







mod exec;
mod ops;
pub mod poster;
mod precompiles;
pub mod state;

use alloy_primitives::{address, Address, Bytes, B256, U256};
use revm::context::result::HaltReason;
use std::collections::HashSet;

pub use state::ARBOS_STATE;


pub const ROBINHOOD: u64 = 4663;

pub const ARBITRUM_ONE: u64 = 42161;

pub const GAS_CAP: u64 = 50_000_000;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArbChain {
    pub id: u64,
    pub max_code_size: usize,
    pub max_initcode_size: usize,
}


pub const CHAINS: [ArbChain; 2] = [
    ArbChain {
        id: ROBINHOOD,
        max_code_size: 98_304,
        max_initcode_size: 196_608,
    },
    ArbChain {
        id: ARBITRUM_ONE,
        max_code_size: 24_576,
        max_initcode_size: 49_152,
    },
];


pub fn chain(id: u64) -> Option<&'static ArbChain> {
    CHAINS.iter().find(|c| c.id == id)
}




pub const NODE_INTERFACES: [Address; 2] = [
    address!("0x00000000000000000000000000000000000000c8"),
    address!("0x00000000000000000000000000000000000000c9"),
];


pub fn rules(arbos_version: u64) -> Option<revm::primitives::hardfork::SpecId> {
    (arbos_version == 61).then_some(revm::primitives::hardfork::SpecId::OSAKA)
}


pub fn header_arbos_version(mix_hash: &B256) -> u64 {
    let mut be = [0u8; 8];
    be.copy_from_slice(&mix_hash[16..24]);
    u64::from_be_bytes(be)
}

























#[derive(Clone, Debug)]
pub struct Block {
    pub chain_id: u64,
    pub number: u64,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub base_fee: u64,
    pub difficulty: U256,
    pub mix_hash: B256,
    pub beneficiary: Address,
}


#[derive(Clone, Debug, Default)]
pub struct Call {
    pub from: Address,
    pub to: Option<Address>,
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub value: U256,
    pub data: Bytes,
    pub access_list: Vec<(Address, Vec<B256>)>,
    pub nonce: Option<u64>,
    pub skip_l1_charging: bool,
}


#[derive(Clone, Debug, Default)]
pub struct BlockOverrides {
    pub number: Option<U256>,
    pub difficulty: Option<U256>,
    pub time: Option<u64>,
    pub gas_limit: Option<u64>,
    pub coinbase: Option<Address>,
    pub base_fee: Option<U256>,
}


#[derive(Debug, PartialEq)]
pub enum Outcome {
    Returned(Bytes),
    Reverted(Bytes, Option<String>),
    Halted(HaltReason),
    HaltedSaying(String),
    Invalid(String),
    Refused(String),
    Internal(String),
}


#[derive(Debug, PartialEq)]
pub struct Measured {
    pub outcome: Outcome,
    pub used: u64,
    pub peak: u64,
    pub intrinsic: bool,
}


pub fn run<DB: revm::Database>(
    db: DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
    overridden: &HashSet<Address>,
) -> Result<Outcome, DB::Error> {
    exec::execute(db, block, call, overrides, overridden, None).map(|m| m.outcome)
}


pub fn estimate_run<DB: revm::Database>(
    db: DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
    overridden: &HashSet<Address>,
    gas: u64,
) -> Result<Measured, DB::Error> {
    exec::execute(db, block, call, overrides, overridden, Some(gas))
}


pub fn estimate_posting_gas<DB: revm::Database>(
    db: &mut DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
) -> Result<Result<u64, String>, DB::Error> {
    exec::posting_gas(db, block, call, overrides)
}



