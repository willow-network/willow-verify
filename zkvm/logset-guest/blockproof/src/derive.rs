


use super::txs::{Decoded, ARB_DEPOSIT, ARB_INTERNAL};
use alloy::consensus::{Transaction as _, TxEnvelope};
use alloy::primitives::U256;
use logset_blockcheck::receipts::{nitro_allowed, DEPOSIT_TX_TYPE};

use std::sync::Arc;








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






pub fn nitro_collects_tips(mix_hash: &[u8; 32]) -> bool {
    let arbos = u64::from_be_bytes(mix_hash[16..24].try_into().expect("8 bytes"));
    arbos == 9 || mix_hash[25] & 1 == 1
}








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


pub fn create_address(sender: &[u8; 20], nonce: u64) -> [u8; 20] {
    alloy::primitives::Address::from(*sender).create(nonce).0 .0
}


pub const GAS_PER_BLOB: u64 = 131_072;






const MAINNET_BLOBS: [(u64, u64, u64, u64); 4] = [
    (1_710_338_135, 3, 6, 3_338_477),
    (1_746_612_311, 6, 9, 5_007_716),
    (1_765_290_071, 10, 15, 8_346_193),
    (1_767_747_671, 14, 21, 11_684_671),
];



const BSC_BLOB_FRACTIONS: [(u64, u64); 1] = [(1_718_863_500, 3_338_477)];



















#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlobParams {
    pub at: u64,
    pub update_fraction: u64,
    pub target: Option<u64>,
    pub max: Option<u64>,
}


#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlobSchedule(Arc<[BlobParams]>);

impl BlobSchedule {

    pub fn builtin(chain_id: u64) -> Self {
        match chain_id {
            1 => Self(
                MAINNET_BLOBS
                    .iter()
                    .map(|&(at, target, max, update_fraction)| BlobParams {
                        at,
                        update_fraction,
                        target: Some(target),
                        max: Some(max),
                    })
                    .collect(),
            ),


            56 => Self(
                BSC_BLOB_FRACTIONS
                    .iter()
                    .map(|&(at, update_fraction)| BlobParams {
                        at,
                        update_fraction,
                        target: None,
                        max: None,
                    })
                    .collect(),
            ),
            _ => Self::default(),
        }
    }








































    pub fn at(&self, timestamp: u64) -> Option<BlobParams> {
        self.0.iter().rev().find(|f| timestamp >= f.at).copied()
    }


    pub fn fraction_at(&self, timestamp: u64) -> Option<u64> {
        self.at(timestamp).map(|f| f.update_fraction)
    }






    pub fn price(&self, timestamp: u64, excess_blob_gas: u64) -> Option<U256> {
        let fraction = self.fraction_at(timestamp)?;
        if excess_blob_gas / fraction > MAX_EXCESS_RATIO {
            return None;
        }
        Some(fake_exponential(
            U256::from(1u8),
            U256::from(excess_blob_gas),
            U256::from(fraction),
        ))
    }
}



pub const MAX_EXCESS_RATIO: u64 = 64;


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


pub fn blob_count(env: &TxEnvelope) -> Option<usize> {
    env.blob_versioned_hashes().map(<[_]>::len)
}












































































































