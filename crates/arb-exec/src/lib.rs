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

/// Robinhood Chain's chain id.
pub const ROBINHOOD: u64 = 4663;
/// Nitro's `--execution.rpc.gas-cap` default (and Robinhood's own RPC's): the gas a call gets
/// when it names none (before its L1 data's), and the most it may name.
pub const GAS_CAP: u64 = 50_000_000;
/// Robinhood Chain's code size limits (its chain config's `MaxCodeSize`, `MaxInitCodeSize`).
pub const MAX_CODE_SIZE: usize = 98_304;
pub const MAX_INITCODE_SIZE: usize = 196_608;
/// The batch poster: the coinbase of every block the sequencer makes.
pub const BATCH_POSTER: Address = address!("0xA4B000000000000000000073657175656e636572");
/// NodeInterface and NodeInterfaceDebug: answered by a Nitro node's RPC, not by its EVM.
pub const NODE_INTERFACES: [Address; 2] = [
    address!("0x00000000000000000000000000000000000000c8"),
    address!("0x00000000000000000000000000000000000000c9"),
];

/// The ArbOS versions whose rules are rebuilt here: the EVM's (Ethereum's rules, the precompile
/// set) and ArbOS's own (precompile methods, fees). Robinhood Chain runs 61.
pub fn rules(arbos_version: u64) -> Option<revm::primitives::hardfork::SpecId> {
    match arbos_version {
        // Osaka's EVM from ArbOS 50 (Amsterdam from 70); ArbOS 60's precompile set.
        61 => Some(revm::primitives::hardfork::SpecId::OSAKA),
        _ => None,
    }
}

/// The ArbOS version a header was made under (its mix hash, bytes 16..24).
pub fn header_arbos_version(mix_hash: &B256) -> u64 {
    u64::from_be_bytes(mix_hash[16..24].try_into().unwrap())
}

/// The ArbOS slots every call reads (its version, the L1 pricing a call's data is charged at,
/// tip collection, the L1 block number NUMBER reads): proven with the call's first round.
pub fn always_read() -> (Address, Vec<B256>) {
    let s = &*state::SLOTS;
    (
        ARBOS_STATE,
        vec![
            s.version,
            s.brotli_level,
            s.collect_tips,
            s.l1_price_per_unit,
            s.l1_units_since_update,
            s.l2_min_base_fee,
            s.l1_block_number,
        ],
    )
}

/// The block a call runs in: its verified header's fields.
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

/// An `eth_call` transaction object, as Nitro reads one.
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
    /// Read only into the L1 data a call is charged for.
    pub nonce: Option<u64>,
    /// Nitro's `skipL1Charging`: no gas charged for the call's L1 data.
    pub skip_l1_charging: bool,
}

/// The block overrides Nitro reads for `eth_call` (`prevRandao` and `blockHash` it leaves be).
#[derive(Clone, Debug, Default)]
pub struct BlockOverrides {
    pub number: Option<U256>,
    pub difficulty: Option<U256>,
    pub time: Option<u64>,
    pub gas_limit: Option<u64>,
    pub coinbase: Option<Address>,
    pub base_fee: Option<U256>,
}

/// What a call came to.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Returned(Bytes),
    /// Reverted with its data, and Nitro's reason where it renders one of ArbOS's own errors.
    Reverted(Bytes, Option<String>),
    /// Stopped by the EVM.
    Halted(HaltReason),
    /// Stopped by BLOBBASEFEE, which Nitro's EVM doesn't run: Nitro's message.
    HaltedSaying(String),
    /// Not run: the call is invalid at this block (Nitro's message).
    Invalid(String),
    /// Not run here: what of it this crate doesn't rebuild.
    Refused(String),
    /// The EVM failed on its own terms.
    Internal(String),
}

/// `call` run at `block` over `db` (the caller's state overrides already laid over it; the
/// accounts it overrides in `overridden`), under the caller's block overrides.
pub fn run<DB: revm::Database>(
    db: DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
    overridden: &HashSet<Address>,
) -> Result<Outcome, DB::Error> {
    evm::run(db, block, call, overrides, overridden)
}

/// One of `eth_estimateGas`'s runs: what it came to, and the gas it took as Nitro counts it.
#[derive(Debug, PartialEq)]
pub struct Measured {
    pub outcome: Outcome,
    /// `UsedGas`: after refunds (the L1 data's gas never refunded), the held gas not used.
    pub used: u64,
    /// `MaxUsedGas`: before refunds.
    pub peak: u64,
    /// Not run for too little gas to start (`ErrIntrinsicGas`): the search raises the gas.
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

/// `call` run as one of `eth_estimateGas`'s runs (Nitro's gas estimation mode) with a gas limit
/// of `gas`, over `db` and under the overrides as [`run`]: its L1 data priced as an estimate
/// prices it (the stand-in's gas filled, 7/8 of the price at least the minimum base fee, the cost
/// padded 10%), the computation held to ArbOS's per-transaction gas limit, an invalid message in
/// the state transition's own words.
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

/// The gas `eth_estimateGas` raises the RPC's gas cap by for `call` (`RPCPostingGasHook` in
/// `DoEstimateGas`): its L1 data's, priced as an estimate prices it at the header's base fee
/// (under the caller's override). `Err` within: refused, and why.
pub fn estimate_posting_gas<DB: revm::Database>(
    db: &mut DB,
    block: &Block,
    call: &Call,
    overrides: &BlockOverrides,
) -> Result<Result<u64, String>, DB::Error> {
    evm::estimate_posting_gas(db, block, call, overrides)
}

/// The ArbOS slots an estimate's runs read besides a call's: the per-transaction gas limit the
/// computation is held to, and the features (EIP-7623's floor is refused where it is on).
pub fn estimate_reads() -> (Address, Vec<B256>) {
    let s = &*state::SLOTS;
    (ARBOS_STATE, vec![s.l2_per_tx_gas_limit, s.features])
}

#[cfg(test)]
mod tests;
