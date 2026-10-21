//! OP-stack receipts' L1 fee fields, derived from the block's own L1 info
//! deposit (its first transaction, under transactionsRoot) and each
//! transaction's bytes, as op-reth's receipts report them
//! (`OpReceiptFieldsBuilder::l1_block_info`, op-revm's `L1BlockInfo`): the
//! cost function in force at the block's timestamp ([`FeeFork`]) over the
//! parameters its L1 info layout carries, and from Jovian the DA footprint
//! reported as the receipt's `blobGasUsed`.

use alloy::primitives::U256;

/// `setL1BlockValues` (Bedrock), `...Ecotone`, `...Isthmus`, `...Jovian` selectors.
const BEDROCK: [u8; 4] = [0x01, 0x5d, 0x8e, 0xb9];
const ECOTONE: [u8; 4] = [0x44, 0x0a, 0x5e, 0x20];
const ISTHMUS: [u8; 4] = [0x09, 0x89, 0x99, 0xbe];
const JOVIAN: [u8; 4] = [0x3d, 0xb6, 0xbe, 0x2b];

/// The L1 cost function a block's transactions are priced with.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeeFork {
    /// `(zeros·4 + others·16 + overhead) · l1BaseFee · scalar / 1e6`
    /// (Regolith, active from Bedrock on all four chains).
    Bedrock,
    /// `(zeros·4 + others·16) · (16·l1BaseFee·baseFeeScalar + blobBaseFee·blobBaseFeeScalar) / 16e6`.
    Ecotone,
    /// A FastLZ size estimate at the same prices.
    Fjord,
}

impl FeeFork {
    /// The fork in force at `timestamp`: Base and OP Mainnet activated Ecotone
    /// at 1,710,374,401 (2024-03-14) and Fjord at 1,720,627,201 (2024-07-10);
    /// Unichain and Ink began after both.
    pub fn at(chain_id: u64, timestamp: u64) -> Self {
        let (ecotone, fjord) = match chain_id {
            8453 | 10 => (1_710_374_401, 1_720_627_201),
            _ => (0, 0),
        };
        if timestamp >= fjord {
            Self::Fjord
        } else if timestamp >= ecotone {
            Self::Ecotone
        } else {
            Self::Bedrock
        }
    }
}

/// The L1 attributes an OP-stack block carries in its first transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct L1Info {
    /// Bedrock's fee scalar (`l1FeeScalar` × 1e6), Ecotone's base fee scalar.
    pub base_fee_scalar: U256,
    pub l1_base_fee: U256,
    /// Bedrock layout only.
    pub overhead: Option<U256>,
    /// From the Ecotone layout on.
    pub blob_base_fee_scalar: Option<u32>,
    /// From the Ecotone layout on.
    pub l1_blob_base_fee: Option<U256>,
    /// From Isthmus.
    pub operator_fee: Option<(u32, u64)>,
    /// From Jovian.
    pub da_footprint_gas_scalar: Option<u16>,
}

impl L1Info {
    /// Parse the L1 info deposit's calldata (Bedrock, Ecotone, Isthmus or Jovian layout).
    pub fn parse(input: &[u8]) -> Option<Self> {
        let (sel, rest) = input.split_at_checked(4)?;
        let sel: [u8; 4] = sel.try_into().ok()?;
        if sel == BEDROCK {
            // Eight ABI words: number, timestamp, base fee, hash, sequence
            // number, batcher hash, overhead, scalar.
            if rest.len() != 256 {
                return None;
            }
            let word = |i: usize| U256::from_be_slice(&rest[32 * i..32 * (i + 1)]);
            return Some(Self {
                base_fee_scalar: word(7),
                l1_base_fee: word(2),
                overhead: Some(word(6)),
                blob_base_fee_scalar: None,
                l1_blob_base_fee: None,
                operator_fee: None,
                da_footprint_gas_scalar: None,
            });
        }
        let want = match sel {
            ECOTONE => 160,
            ISTHMUS => 172,
            JOVIAN => 174,
            _ => return None,
        };
        if rest.len() != want {
            return None;
        }
        let u32_at = |i: usize| u32::from_be_bytes(rest[i..i + 4].try_into().unwrap());
        let u64_at = |i: usize| u64::from_be_bytes(rest[i..i + 8].try_into().unwrap());
        // scalars (4+4), sequence number, timestamp, number (8 each),
        // base fee, blob base fee, block hash, batcher hash (32 each).
        Some(Self {
            base_fee_scalar: U256::from(u32_at(0)),
            l1_base_fee: U256::from_be_slice(&rest[32..64]),
            overhead: None,
            blob_base_fee_scalar: Some(u32_at(4)),
            l1_blob_base_fee: Some(U256::from_be_slice(&rest[64..96])),
            operator_fee: (want >= 172).then(|| (u32_at(160), u64_at(164))),
            da_footprint_gas_scalar: (want == 174)
                .then(|| u16::from_be_bytes(rest[172..174].try_into().unwrap())),
        })
    }

    /// Before Ecotone, the receipt's `l1FeeScalar`: the scalar / 1e6 as a
    /// decimal, the way op-reth formats an `f64`.
    pub fn fee_scalar_decimal(&self) -> String {
        format!(
            "{}",
            self.base_fee_scalar.saturating_to::<u64>() as f64 / 1_000_000.0
        )
    }

    /// `(l1GasUsed, l1Fee, DA footprint)` of one transaction's EIP-2718 bytes
    /// under `fork`. A deposit's L1 fee is zero (it was paid on L1); its other
    /// two are computed like any transaction's, as nodes report them.
    pub fn cost(&self, fork: FeeFork, tx: &[u8], deposit: bool) -> (u64, U256, Option<u64>) {
        let fastlz = U256::from(flz_compress_len(tx));
        // max(minTransactionSize, intercept + fastlzCoef * fastlzSize), scaled 1e6.
        let scaled = (U256::from(836_500u64) * fastlz)
            .checked_sub(U256::from(42_585_600u64))
            .unwrap_or_default()
            .max(U256::from(100_000_000u64));
        let data_gas = match fork {
            FeeFork::Fjord => scaled * U256::from(16u8) / U256::from(1_000_000u64),
            FeeFork::Bedrock | FeeFork::Ecotone => U256::from(
                tx.iter()
                    .map(|b| if *b == 0 { 4u64 } else { 16 })
                    .sum::<u64>(),
            ),
        };
        let overhead = self.overhead.unwrap_or_default();
        let fee_scaled = U256::from(16u8) * self.base_fee_scalar * self.l1_base_fee
            + U256::from(self.blob_base_fee_scalar.unwrap_or_default())
                * self.l1_blob_base_fee.unwrap_or_default();
        let fee = match fork {
            _ if deposit => U256::ZERO,
            FeeFork::Fjord => scaled * fee_scaled / U256::from(1_000_000_000_000u64),
            FeeFork::Ecotone => fee_scaled * data_gas / U256::from(16_000_000u64),
            FeeFork::Bedrock => {
                (data_gas + overhead) * self.l1_base_fee * self.base_fee_scalar
                    / U256::from(1_000_000u64)
            }
        };
        let da = self.da_footprint_gas_scalar.map(|s| {
            let size: u64 = (scaled / U256::from(1_000_000u64)).saturating_to();
            size.saturating_mul(u64::from(s))
        });
        ((data_gas + overhead).saturating_to(), fee, da)
    }
}

/// The FastLZ (level 1) compressed length of `ib`, as op-geth's `FlzCompressLen` and op-revm's
/// `flz_compress_len` count it (both follow Solady's MIT-licensed `LibZip.flzCompress`).
pub fn flz_compress_len(ib: &[u8]) -> u32 {
    let len = ib.len() as u32;
    let mut n = 0u32;
    let mut ht = vec![0u32; 8192];
    let u24 = |i: u32| {
        let i = i as usize;
        u32::from(ib[i]) | u32::from(ib[i + 1]) << 8 | u32::from(ib[i + 2]) << 16
    };
    let hash = |v: u32| (2_654_435_769u32.wrapping_mul(v) >> 19) & 0x1fff;
    let literals = |n: &mut u32, r: u32| {
        *n += 0x21 * (r / 0x20);
        if !r.is_multiple_of(0x20) {
            *n += r % 0x20 + 1;
        }
    };
    let ip_limit = len.saturating_sub(13);
    let mut a = 0u32;
    let mut ip = a + 2;
    while ip < ip_limit {
        let mut r;
        loop {
            let s = u24(ip);
            let h = hash(s) as usize;
            r = ht[h];
            ht[h] = ip;
            let d = ip - r;
            if ip >= ip_limit {
                break;
            }
            ip += 1;
            if d <= 0x1fff && s == u24(r) {
                break;
            }
        }
        if ip >= ip_limit {
            break;
        }
        ip -= 1;
        if ip > a {
            literals(&mut n, ip - a);
        }
        // Match length: compare from +3, stopping one past the first difference.
        let (p, q) = (r + 3, ip + 3);
        let mut e = ip_limit + 9 - q;
        let mut l = 0u32;
        while l < e {
            if ib[(p + l) as usize] != ib[(q + l) as usize] {
                e = 0;
            }
            l += 1;
        }
        let m = l - 1;
        n += 3 * (m / 262) + if m % 262 >= 6 { 3 } else { 2 };
        ip += l;
        for _ in 0..2 {
            ht[hash(u24(ip)) as usize] = ip;
            ip += 1;
        }
        a = ip;
    }
    literals(&mut n, len - a);
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Edge sizes around the 13-byte window; real transactions are checked
    /// against node-reported L1 fees by the fixture tests.
    #[test]
    fn fastlz_length_edges() {
        assert_eq!(flz_compress_len(&[]), 0);
        assert_eq!(flz_compress_len(&[1]), 2);
        assert_eq!(flz_compress_len(&[0; 13]), 14);
        // 32 literals take one control byte each 32; long runs compress.
        assert!(flz_compress_len(&[0u8; 1000]) < 100);
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        let random: Vec<u8> = (0..1000)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x >> 32) as u8
            })
            .collect();
        assert!(flz_compress_len(&random) > 900);
    }

    #[test]
    fn l1_info_layouts() {
        let mut jovian = JOVIAN.to_vec();
        jovian.extend_from_slice(&2269u32.to_be_bytes());
        jovian.extend_from_slice(&1_055_762u32.to_be_bytes());
        jovian.extend_from_slice(&[0; 24]);
        jovian.extend_from_slice(&U256::from(61_832_221u64).to_be_bytes::<32>());
        jovian.extend_from_slice(&U256::from(3_988_230u64).to_be_bytes::<32>());
        jovian.extend_from_slice(&[0; 64]);
        jovian.extend_from_slice(&[0; 12]);
        jovian.extend_from_slice(&148u16.to_be_bytes());
        let info = L1Info::parse(&jovian).unwrap();
        assert_eq!(info.base_fee_scalar, U256::from(2269u32));
        assert_eq!(info.l1_blob_base_fee, Some(U256::from(3_988_230u64)));
        assert_eq!(info.operator_fee, Some((0, 0)));
        assert_eq!(info.da_footprint_gas_scalar, Some(148));
        assert_eq!(info.overhead, None);
        assert!(L1Info::parse(&jovian[..177]).is_none());
        // The minimum size: 100 bytes → 1,600 gas, footprint 100 x scalar.
        let (gas, fee, da) = info.cost(FeeFork::Fjord, &[0u8; 20], false);
        assert_eq!((gas, da), (1_600, Some(14_800)));
        assert!(fee > U256::ZERO);
        assert_eq!(info.cost(FeeFork::Fjord, &[0u8; 20], true).1, U256::ZERO);
    }

    /// Base's L1 info before Ecotone (block 1): overhead 188, scalar 684,000.
    #[test]
    fn bedrock_layout_and_cost() {
        let mut bedrock = BEDROCK.to_vec();
        for w in [1u64, 2, 0x3ded24b5e, 4, 0, 6, 188, 684_000] {
            bedrock.extend_from_slice(&U256::from(w).to_be_bytes::<32>());
        }
        let info = L1Info::parse(&bedrock).unwrap();
        assert_eq!(info.l1_base_fee, U256::from(0x3ded24b5eu64));
        assert_eq!(info.overhead, Some(U256::from(188u8)));
        assert_eq!(info.base_fee_scalar, U256::from(684_000u32));
        assert_eq!(
            (info.l1_blob_base_fee, info.blob_base_fee_scalar),
            (None, None)
        );
        assert_eq!(info.fee_scalar_decimal(), "0.684");
        assert!(L1Info::parse(&bedrock[..259]).is_none());
        // Two zero bytes and three others: 2·4 + 3·16 = 56, plus the overhead.
        let tx = [0u8, 1, 0, 2, 3];
        let (gas, fee, _) = info.cost(FeeFork::Bedrock, &tx, false);
        assert_eq!(gas, 56 + 188);
        let want = U256::from(244u32) * U256::from(0x3ded24b5eu64) * U256::from(684_000u32)
            / U256::from(1_000_000u32);
        assert_eq!(fee, want);
        assert_eq!(
            info.cost(FeeFork::Bedrock, &tx, true),
            (244, U256::ZERO, None)
        );
        // Ecotone's first block still carries this layout: the overhead is
        // still counted in l1GasUsed, and the fee is Ecotone's with no blob part.
        let (gas, fee, _) = info.cost(FeeFork::Ecotone, &tx, false);
        assert_eq!(gas, 244);
        assert_eq!(
            fee,
            U256::from(16u8)
                * U256::from(684_000u32)
                * U256::from(0x3ded24b5eu64)
                * U256::from(56u8)
                / U256::from(16_000_000u32)
        );
    }

    #[test]
    fn fee_forks_by_chain_and_time() {
        assert_eq!(FeeFork::at(8453, 1_710_374_400), FeeFork::Bedrock);
        assert_eq!(FeeFork::at(8453, 1_710_374_401), FeeFork::Ecotone);
        assert_eq!(FeeFork::at(10, 1_720_627_200), FeeFork::Ecotone);
        assert_eq!(FeeFork::at(10, 1_720_627_201), FeeFork::Fjord);
        assert_eq!(FeeFork::at(130, 0), FeeFork::Fjord);
        assert_eq!(FeeFork::at(57073, 0), FeeFork::Fjord);
    }
}
