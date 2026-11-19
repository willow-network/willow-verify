







use alloy::primitives::U256;


const BEDROCK: [u8; 4] = [0x01, 0x5d, 0x8e, 0xb9];
const ECOTONE: [u8; 4] = [0x44, 0x0a, 0x5e, 0x20];
const ISTHMUS: [u8; 4] = [0x09, 0x89, 0x99, 0xbe];
const JOVIAN: [u8; 4] = [0x3d, 0xb6, 0xbe, 0x2b];

const MANTLE_ARSIA: [u8; 4] = [0x49, 0xe7, 0x23, 0x83];


#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum FeeFork {


    Bedrock,

    Ecotone,

    Fjord,
}

impl FeeFork {



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


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct L1Info {

    pub base_fee_scalar: U256,
    pub l1_base_fee: U256,

    pub overhead: Option<U256>,

    pub blob_base_fee_scalar: Option<u32>,

    pub l1_blob_base_fee: Option<U256>,

    pub operator_fee: Option<(u32, u64)>,

    pub da_footprint_gas_scalar: Option<u16>,
}

impl L1Info {

    pub fn parse(input: &[u8]) -> Option<Self> {
        let (sel, rest) = input.split_at_checked(4)?;
        let sel: [u8; 4] = sel.try_into().ok()?;
        if sel == BEDROCK {


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



    pub fn parse_for(chain_id: u64, input: &[u8]) -> Option<Self> {
        if logset_blockcheck::header::mantle(chain_id) && input.get(..4) == Some(&MANTLE_ARSIA[..])
        {
            let mut jovian = input.to_vec();
            jovian[..4].copy_from_slice(&JOVIAN);
            return Self::parse(&jovian);
        }
        Self::parse(input)
    }



    pub fn fee_scalar_decimal(&self) -> String {
        format!(
            "{}",
            self.base_fee_scalar.saturating_to::<u64>() as f64 / 1_000_000.0
        )
    }




    pub fn cost(&self, fork: FeeFork, tx: &[u8], deposit: bool) -> (u64, U256, Option<u64>) {
        let fastlz = U256::from(flz_compress_len(tx));

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

/// The FastLZ (level 1) compressed length of `input`: a port of op-revm's `flz_compress_len`
/// (`op_revm::fast_lz`, MIT), which follows Solady's MIT-licensed `LibZip.flzCompress`.
pub fn flz_compress_len(input: &[u8]) -> u32 {
    let mut idx: u32 = 2;
    let idx_limit: u32 = if input.len() < 13 {
        0
    } else {
        input.len() as u32 - 13
    };
    let mut anchor = 0;
    let mut size = 0;
    let mut htab = vec![0u32; 8192];
    while idx < idx_limit {
        let mut r: u32;
        let mut distance: u32;
        loop {
            let seq = u24(input, idx);
            let hash = fastlz_hash(seq);
            r = htab[hash as usize];
            htab[hash as usize] = idx;
            distance = idx - r;
            if idx >= idx_limit {
                break;
            }
            idx += 1;
            if distance < 8192 && seq == u24(input, r) {
                break;
            }
        }
        if idx >= idx_limit {
            break;
        }
        idx -= 1;
        if idx > anchor {
            size = literals(idx - anchor, size);
        }
        let len = cmp(input, r + 3, idx + 3, idx_limit + 9);
        size = flz_match(len, size);
        idx = set_next_hash(&mut htab, input, idx + len);
        idx = set_next_hash(&mut htab, input, idx);
        anchor = idx;
    }
    literals(input.len() as u32 - anchor, size)
}

const fn literals(r: u32, size: u32) -> u32 {
    let size = size + 0x21 * (r / 0x20);
    let r = r % 0x20;
    if r != 0 {
        size + r + 1
    } else {
        size
    }
}

fn cmp(input: &[u8], p: u32, q: u32, r: u32) -> u32 {
    let mut l = 0;
    let mut r = r - q;
    while l < r {
        if input[(p + l) as usize] != input[(q + l) as usize] {
            r = 0;
        }
        l += 1;
    }
    l
}

const fn flz_match(l: u32, size: u32) -> u32 {
    let l = l - 1;
    let size = size + (3 * (l / 262));
    if l % 262 >= 6 {
        size + 3
    } else {
        size + 2
    }
}

fn set_next_hash(htab: &mut [u32], input: &[u8], idx: u32) -> u32 {
    htab[fastlz_hash(u24(input, idx)) as usize] = idx;
    idx + 1
}

const fn fastlz_hash(v: u32) -> u16 {
    let hash = (v as u64 * 2654435769) >> 19;
    hash as u16 & 0x1fff
}

fn u24(input: &[u8], idx: u32) -> u32 {
    u32::from(input[idx as usize])
        + (u32::from(input[(idx + 1) as usize]) << 8)
        + (u32::from(input[(idx + 2) as usize]) << 16)
}

#[cfg(test)]
mod tests {
    use super::*;



    #[test]
    fn fastlz_length_edges() {
        assert_eq!(flz_compress_len(&[]), 0);
        assert_eq!(flz_compress_len(&[1]), 2);
        assert_eq!(flz_compress_len(&[0; 13]), 14);

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

        assert_eq!(flz_compress_len(&[0; 1000]), 21);
        assert_eq!(flz_compress_len(&[42; 1000]), 21);
        assert_eq!(flz_compress_len(&[0xfa, 0xca, 0xde]), 4);
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

        let (gas, fee, da) = info.cost(FeeFork::Fjord, &[0u8; 20], false);
        assert_eq!((gas, da), (1_600, Some(14_800)));
        assert!(fee > U256::ZERO);
        assert_eq!(info.cost(FeeFork::Fjord, &[0u8; 20], true).1, U256::ZERO);
    }


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
