//! Calls on Robinhood Chain (Arbitrum Orbit, chain 4663) run as a Nitro node runs `eth_call`, over
//! a state the caller of this crate proves (every account, slot and code the call reads, ArbOS's
//! own state included, comes from its database). Where Nitro's EVM differs from Ethereum's it is
//! rebuilt from Nitro's source: NUMBER and BLOCKHASH read the L1 block number and hashes ArbOS
//! keeps, GASPRICE is the base fee, BLOBBASEFEE fails, PREVRANDAO is the difficulty; the call is
//! charged gas for its L1 data as Nitro charges it; ArbSys and ArbGasInfo answer from ArbOS's
//! storage. What isn't rebuilt (Stylus programs, the other ArbOS precompiles, ArbOS versions not
//! in [`rules`]) is refused, never approximated.

mod evm;
pub mod poster;
mod precompiles;
pub mod state;

use alloy_primitives::{address, Address, Bytes, B256, U256};
use revm::context::result::HaltReason;
use std::collections::HashSet;

pub use state::ARBOS_STATE;


pub const ROBINHOOD: u64 = 4663;


pub const GAS_CAP: u64 = 50_000_000;

pub const MAX_CODE_SIZE: usize = 98_304;
pub const MAX_INITCODE_SIZE: usize = 196_608;



pub const NODE_INTERFACES: [Address; 2] = [
    address!("0x00000000000000000000000000000000000000c8"),
    address!("0x00000000000000000000000000000000000000c9"),
];



pub fn rules(arbos_version: u64) -> Option<revm::primitives::hardfork::SpecId> {
    match arbos_version {

        61 => Some(revm::primitives::hardfork::SpecId::OSAKA),
        _ => None,
    }
}


pub fn header_arbos_version(mix_hash: &B256) -> u64 {
    u64::from_be_bytes(mix_hash[16..24].try_into().unwrap())
}




















#[derive(Clone, Debug)]
pub struct Block {
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



pub fn run<DB: revm::Database>(
    db: DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
    overridden: &HashSet<Address>,
) -> Result<Outcome, DB::Error> {
    evm::run(db, block, call, overrides, overridden)
}


#[derive(Debug, PartialEq)]
pub struct Measured {
    pub outcome: Outcome,

    pub used: u64,

    pub peak: u64,

    pub intrinsic: bool,
}

impl Measured {
    pub(crate) fn not_run(outcome: Outcome, intrinsic: bool) -> Self {
        Measured {
            outcome,
            used: 0,
            peak: 0,
            intrinsic,
        }
    }
}






pub fn estimate_run<DB: revm::Database>(
    db: DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
    overridden: &HashSet<Address>,
    gas: u64,
) -> Result<Measured, DB::Error> {
    evm::run_as(
        db,
        block,
        call,
        overrides,
        overridden,
        evm::Mode::Estimate(gas),
    )
}




pub fn estimate_posting_gas<DB: revm::Database>(
    db: &mut DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
) -> Result<Result<u64, String>, DB::Error> {
    evm::estimate_posting_gas(db, block, call, overrides)
}










