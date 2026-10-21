//! Fields a node reports that are not in the consensus bytes but follow from
//! them: fee prices, contract addresses, blob prices.

use super::txs::{Decoded, ARB_DEPOSIT, ARB_INTERNAL};
use alloy::consensus::{Transaction as _, TxEnvelope};
use alloy::primitives::U256;
use logset_blockcheck::receipts::{nitro_allowed, DEPOSIT_TX_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub const ARBITRUM_ONE: u64 = 42161;
pub const ROBINHOOD: u64 = 4663;

/// The `gasPrice` of a transaction object in a block: the legacy price, else
/// what a dynamic-fee transaction paid per gas (`min(maxFee, baseFee + tip)`,
/// geth's `newRPCTransaction`); a Nitro type's fee cap; zero for deposits and
/// Nitro's internal and deposit types.
pub fn tx_gas_price(tx: &Decoded, base_fee: Option<U256>) -> U256 {
    match tx {
        Decoded::Eth(env) => standard_price(env, base_fee),
        Decoded::Table { .. } => tx.uint("maxFeePerGas").unwrap_or_default(),
    }
}

fn standard_price(env: &TxEnvelope, base_fee: Option<U256>) -> U256 {
    match env.gas_price() {
        Some(p) => U256::from(p),
        None => {
            let cap = U256::from(env.max_fee_per_gas());
            let tip = U256::from(env.max_priority_fee_per_gas().unwrap_or_default());
            match base_fee {
                Some(b) => cap.min(b + tip),
                None => cap,
            }
        }
    }
}

/// Whether a Nitro block collects tips, from its header's `mixHash`
/// (OffchainLabs go-ethereum `DeserializeHeaderExtraInformation`): always under
/// ArbOS 9, else the flag ArbOS 60 added at byte 25, set when the chain owner
/// turned tip collection on (Arbitrum One from block 508,190,474) and never
/// on a block of delayed-inbox messages.
pub fn nitro_collects_tips(mix_hash: &[u8; 32]) -> bool {
    let arbos = u64::from_be_bytes(mix_hash[16..24].try_into().expect("8 bytes"));
    arbos == 9 || mix_hash[25] & 1 == 1
}

/// A receipt's `effectiveGasPrice`. Ethereum and the OP stack: the price per
/// gas the transaction paid (zero for a deposit). Nitro (OffchainLabs
/// go-ethereum `Receipt.DeriveFields`): the base fee for every receipt of a
/// block that doesn't collect tips ([`nitro_collects_tips`]); in one that
/// does, each transaction's own price: the standard price for Ethereum's
/// types, zero for the internal and deposit types, the base fee for the other
/// Nitro types.
pub fn effective_gas_price(
    chain_id: u64,
    tx: &Decoded,
    base_fee: Option<U256>,
    collects_tips: bool,
) -> U256 {
    let base = base_fee.unwrap_or_default();
    let nitro = nitro_allowed(chain_id);
    match tx {
        _ if nitro && !collects_tips => base,
        Decoded::Eth(env) => standard_price(env, base_fee),
        Decoded::Table { ty, .. } if *ty == DEPOSIT_TX_TYPE => U256::ZERO,
        Decoded::Table { ty, .. } if nitro && (*ty == ARB_INTERNAL || *ty == ARB_DEPOSIT) => {
            U256::ZERO
        }
        Decoded::Table { .. } if nitro => {
            base_fee.unwrap_or_else(|| tx.uint("maxFeePerGas").unwrap_or_default())
        }
        _ => U256::ZERO,
    }
}

/// `keccak(rlp([sender, nonce]))[12..]`: the address a creation deploys to.
pub fn create_address(sender: &[u8; 20], nonce: u64) -> [u8; 20] {
    alloy::primitives::Address::from(*sender).create(nonce).0 .0
}

/// Blob gas per blob (EIP-4844).
pub const GAS_PER_BLOB: u64 = 131_072;

/// Ethereum mainnet's blob base fee update fraction by fork activation time:
/// Cancun, Prague (Pectra), BPO1 and BPO2 (Fusaka's blob-parameter forks).
/// A store's `blob_schedule` replaces it, so a new fork is a config change.
const MAINNET_BLOB_FRACTIONS: [(u64, u64); 4] = [
    (1_710_338_135, 3_338_477),
    (1_746_612_311, 5_007_716),
    (1_765_290_071, 8_346_193),
    (1_767_747_671, 11_684_671),
];

/// One blob-parameter fork: from `at` (unix seconds) on, blob prices use
/// `update_fraction` (`BLOB_BASE_FEE_UPDATE_FRACTION`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlobFork {
    /// For the reader of the config only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub at: u64,
    pub update_fraction: u64,
}

/// A chain's blob price schedule, oldest fork first; empty on a chain without blobs.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlobSchedule(Arc<[(u64, u64)]>);

impl BlobSchedule {
    /// Ethereum mainnet's built-in table; no blobs on any other chain.
    pub fn builtin(chain_id: u64) -> Self {
        match chain_id {
            1 => Self(MAINNET_BLOB_FRACTIONS.into()),
            _ => Self::default(),
        }
    }

    /// `forks` with strictly increasing activation times and nonzero fractions.
    pub fn new(forks: &[BlobFork]) -> anyhow::Result<Self> {
        for f in forks {
            anyhow::ensure!(
                f.update_fraction > 0,
                "blob_schedule: the fork at {} has update_fraction 0",
                f.at
            );
        }
        for w in forks.windows(2) {
            anyhow::ensure!(
                w[0].at < w[1].at,
                "blob_schedule: forks must be in order of activation time ({} is not before {})",
                w[0].at,
                w[1].at
            );
        }
        Ok(Self(
            forks.iter().map(|f| (f.at, f.update_fraction)).collect(),
        ))
    }

    /// The update fraction in force at `timestamp`; `None` before the first fork.
    pub fn fraction_at(&self, timestamp: u64) -> Option<u64> {
        self.0
            .iter()
            .rev()
            .find(|(at, _)| timestamp >= *at)
            .map(|f| f.1)
    }

    /// The blob base fee of a block (`blobGasPrice` of its blob receipts):
    /// `fake_exponential(1, excessBlobGas, fraction)` with the fraction in force
    /// at `timestamp`; `None` before the first fork.
    pub fn price(&self, timestamp: u64, excess_blob_gas: u64) -> Option<U256> {
        let fraction = self.fraction_at(timestamp)?;
        Some(fake_exponential(
            U256::from(1u8),
            U256::from(excess_blob_gas),
            U256::from(fraction),
        ))
    }
}

/// EIP-4844's `fake_exponential`.
pub fn fake_exponential(factor: U256, numerator: U256, denominator: U256) -> U256 {
    let mut i = U256::from(1u8);
    let mut output = U256::ZERO;
    let mut acc = factor * denominator;
    while acc > U256::ZERO {
        output += acc;
        acc = acc * numerator / (denominator * i);
        i += U256::from(1u8);
    }
    output / denominator
}

/// Blobs a transaction carries.
pub fn blob_count(env: &TxEnvelope) -> Option<usize> {
    env.blob_versioned_hashes().map(<[_]>::len)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Values observed on mainnet on 2026-10-04 (block with excessBlobGas
    /// 177,462,588 reports blobGasPrice 0x3c2d3f).
    #[test]
    fn blob_gas_price_follows_the_fork_schedule() {
        let mainnet = BlobSchedule::builtin(1);
        assert_eq!(
            mainnet.price(1_791_097_883, 177_462_588),
            Some(U256::from(0x3c2d3fu64))
        );
        assert_eq!(mainnet.price(1_700_000_000, 0), None);
        assert_eq!(mainnet.price(1_710_338_135, 0), Some(U256::from(1u8)));
        assert_eq!(BlobSchedule::builtin(10).price(1_791_097_883, 0), None);
    }

    /// A configured schedule replaces the built-in one: a fork the build does
    /// not know prices its blocks from its activation time on.
    #[test]
    fn a_configured_schedule_prices_a_new_fork() {
        let fork = |at, update_fraction| BlobFork {
            name: None,
            at,
            update_fraction,
        };
        let mut forks: Vec<BlobFork> = MAINNET_BLOB_FRACTIONS
            .iter()
            .map(|&(at, f)| fork(at, f))
            .collect();
        forks.push(fork(1_800_000_000, 2 * 11_684_671));
        let s = BlobSchedule::new(&forks).unwrap();
        let x = 177_462_588;
        let builtin = BlobSchedule::builtin(1);
        assert_eq!(s.price(1_799_999_999, x), builtin.price(1_799_999_999, x));
        assert_eq!(s.fraction_at(1_800_000_000), Some(2 * 11_684_671));
        assert_ne!(s.price(1_800_000_000, x), builtin.price(1_800_000_000, x));
        assert!(BlobSchedule::new(&[fork(2, 1), fork(1, 1)]).is_err());
        assert!(BlobSchedule::new(&[fork(1, 1), fork(1, 2)]).is_err());
        assert!(BlobSchedule::new(&[fork(1, 0)]).is_err());
    }

    /// The tip flag: ArbOS 9 always, else byte 25 of the mixHash.
    #[test]
    fn nitro_tip_flag_from_the_mix_hash() {
        let mix = |arbos: u64, flag: u8| {
            let mut m = [0u8; 32];
            m[16..24].copy_from_slice(&arbos.to_be_bytes());
            m[25] = flag;
            m
        };
        assert!(!nitro_collects_tips(&mix(61, 0)));
        assert!(nitro_collects_tips(&mix(61, 1)));
        assert!(nitro_collects_tips(&mix(9, 0)));
        assert!(!nitro_collects_tips(&mix(20, 0)));
    }

    /// Nitro's message types under either flag; Ethereum's types and the
    /// internal transaction are checked on real blocks by the proof tests.
    #[test]
    fn nitro_message_types_price_by_the_tip_flag() {
        use logset_blockcheck::transactions::value_from_rpc;
        let addr = |b: u8| format!("0x{}", hex::encode([b; 20]));
        let deposit = serde_json::json!({
            "type": "0x64", "chainId": "0xa4b1", "requestId": format!("0x{}", "11".repeat(32)),
            "from": addr(1), "to": addr(2), "value": "0x5",
        });
        let internal = serde_json::json!({
            "type": "0x6a", "chainId": "0xa4b1", "input": "0x6bf6a42d",
        });
        let unsigned = serde_json::json!({
            "type": "0x65", "chainId": "0xa4b1", "from": addr(1), "nonce": "0x1",
            "maxFeePerGas": "0x77359400", "gas": "0x5208", "to": addr(2), "value": "0x0", "input": "0x",
        });
        let base = U256::from(20_000_000u64);
        for (json, tipped) in [
            (deposit, U256::ZERO),
            (internal, U256::ZERO),
            (unsigned, base),
        ] {
            let tx =
                crate::txs::decode(&value_from_rpc(&json, ARBITRUM_ONE).unwrap(), ARBITRUM_ONE)
                    .unwrap();
            assert_eq!(
                effective_gas_price(ARBITRUM_ONE, &tx, Some(base), false),
                base
            );
            assert_eq!(
                effective_gas_price(ARBITRUM_ONE, &tx, Some(base), true),
                tipped
            );
            assert_eq!(effective_gas_price(ROBINHOOD, &tx, Some(base), false), base);
        }
    }

    #[test]
    fn create_address_is_keccak_of_sender_and_nonce() {
        // Robinhood Chain, 2026-10-04: 0x10956b2a... from a real deployment
        // is checked by the fixture tests; here, the well-known first
        // deployment of address 0x00..00 at nonce 0.
        let a = create_address(&[0; 20], 0);
        assert_eq!(hex::encode(a), "bd770416a3345f91e4b34576cb804a576fa48eb1");
    }
}
