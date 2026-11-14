//! `eth_call` run over state proven against a verified block's state root, as the log RPC serves
//! it: on Ethereum as reth (v2) runs a call (50M gas, no nonce, sender-code or block-gas checks,
//! nothing charged for gas, a zero base fee when no gas price is given), on Unichain as op-reth
//! runs it (`willow_op_exec`), on Robinhood Chain as Nitro runs it (`willow_arb_exec`); in each
//! case under the caller's state and block overrides, laid over the witness as reth lays them.
//! `eth_estimateGas`'s runs on Ethereum are geth's ([`execute`]: gas bought at the price named).
//! Every account, slot and block hash a call reads must be in the witness; a run that reads
//! anything else comes to no outcome, only to what it was missing, so the witness can be
//! filled and the call run again. The server gathers that way ([`Mode::Gather`]); a client runs
//! [`Mode::Strict`], where the first read the witness lacks ends the run.

use crate::overrides::Overrides;
use crate::proof::{self, Proven};
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::context::result::{
    EVMError, ExecutionResult, HaltReason, InvalidTransaction, OutOfGasError, Output,
};
use revm::context::{BlockEnv, CfgEnv, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::context_interface::transaction::{AccessList, AccessListItem};
use revm::database::CacheDB;
use revm::database_interface::DBErrorMarker;
use revm::interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome};
use revm::primitives::hardfork::SpecId;
use revm::primitives::TxKind;
use revm::state::{AccountInfo, Bytecode};
use revm::{Context, DatabaseRef, ExecuteEvm, InspectEvm, Inspector, MainBuilder, MainContext};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// Geth's `--rpc.gascap` default: the gas a call gets when it names none, and the most it may name.
pub const GAS_CAP: u64 = 50_000_000;

/// Ethereum's forks activated by block number, then by time (Constantinople and Petersburg
/// activated together, as Petersburg).
const BLOCK_FORKS: [(u64, SpecId); 15] = [
    (0, SpecId::FRONTIER),
    (200_000, SpecId::FRONTIER_THAWING),
    (1_150_000, SpecId::HOMESTEAD),
    (1_920_000, SpecId::DAO_FORK),
    (2_463_000, SpecId::TANGERINE),
    (2_675_000, SpecId::SPURIOUS_DRAGON),
    (4_370_000, SpecId::BYZANTIUM),
    (7_280_000, SpecId::PETERSBURG),
    (9_069_000, SpecId::ISTANBUL),
    (9_200_000, SpecId::MUIR_GLACIER),
    (12_244_000, SpecId::BERLIN),
    (12_965_000, SpecId::LONDON),
    (13_773_000, SpecId::ARROW_GLACIER),
    (15_050_000, SpecId::GRAY_GLACIER),
    (15_537_394, SpecId::MERGE),
];
const TIME_FORKS: [(u64, SpecId); 4] = [
    (1_681_338_455, SpecId::SHANGHAI),
    (1_710_338_135, SpecId::CANCUN),
    (1_746_612_311, SpecId::PRAGUE),
    (1_764_798_551, SpecId::OSAKA),
];

/// The Ethereum rules in force at a block.
pub fn spec(number: u64, timestamp: u64) -> SpecId {
    let by_time = TIME_FORKS
        .iter()
        .rev()
        .find(|(at, _)| number >= 15_537_394 && timestamp >= *at)
        .map(|f| f.1);
    by_time.unwrap_or_else(|| {
        BLOCK_FORKS
            .iter()
            .rev()
            .find(|(at, _)| number >= *at)
            .map_or(SpecId::FRONTIER, |f| f.1)
    })
}

/// The EVM a chain's calls run in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Engine {
    /// Ethereum's (chain 1).
    Ethereum,
    /// The OP stack's, on Unichain (chain 130).
    Op,
    /// Arbitrum Nitro's, on Robinhood Chain (chain 4663).
    Nitro,
}

/// The EVM `chain_id`'s calls run in, where they are proven.
pub fn engine(chain_id: u64) -> Option<Engine> {
    match chain_id {
        1 => Some(Engine::Ethereum),
        willow_op_exec::UNICHAIN => Some(Engine::Op),
        willow_arb_exec::ROBINHOOD => Some(Engine::Nitro),
        _ => None,
    }
}

/// The block a call runs in: its verified header's fields.
#[derive(Clone, Debug)]
pub struct Block {
    pub number: u64,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub base_fee: Option<u64>,
    pub difficulty: U256,
    pub mix_hash: B256,
    pub beneficiary: Address,
    /// `excessBlobGas` and the blob base fee it sets (from Cancun on).
    pub blob: Option<(u64, u128)>,
}

impl Block {
    /// The environment of a verified header, blob prices from `blobs`.
    pub fn from_header(
        h: &alloy::consensus::Header,
        blobs: &logset_blockproof::derive::BlobSchedule,
    ) -> Self {
        let blob = h.excess_blob_gas.and_then(|excess| {
            let price = blobs.price(h.timestamp, excess)?;
            Some((excess, u128::try_from(price).unwrap_or(u128::MAX)))
        });
        Block {
            number: h.number,
            timestamp: h.timestamp,
            gas_limit: h.gas_limit,
            base_fee: h.base_fee_per_gas,
            difficulty: h.difficulty,
            mix_hash: h.mix_hash,
            beneficiary: h.beneficiary,
            blob,
        }
    }
}

/// An `eth_call` transaction object.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Call {
    pub from: Address,
    /// `None`: a contract creation, run for the code it returns.
    pub to: Option<Address>,
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub value: U256,
    pub data: Bytes,
    pub access_list: Vec<(Address, Vec<B256>)>,
    /// Robinhood Chain's: Nitro prices a call's L1 data with its nonce, or charges none.
    pub nonce: Option<u64>,
    pub skip_l1_charging: bool,
}

impl Call {
    /// The gas price the call names, as reth reads it: a legacy price, or a 1559 price at the
    /// block's base fee (no fee cap: none), or zero.
    pub fn price(&self, base_fee: u64) -> u128 {
        if let Some(p) = self.gas_price {
            return p;
        }
        match (self.max_fee_per_gas, self.max_priority_fee_per_gas) {
            (None, None) => 0,
            (cap, tip) => cap
                .unwrap_or(u128::MAX)
                .min(u128::from(base_fee).saturating_add(tip.unwrap_or(0))),
        }
    }

    fn fees_given(&self) -> bool {
        [
            self.gas_price,
            self.max_fee_per_gas,
            self.max_priority_fee_per_gas,
        ]
        .iter()
        .any(|f| f.is_some_and(|f| f > 0))
    }
}

/// The proven state a call may read.
#[derive(Clone, Default)]
pub struct Witness {
    accounts: HashMap<Address, (Proven, Bytecode)>,
    storage: HashMap<Address, HashMap<U256, U256>>,
    /// Accounts held without their code (the caller's replaces it): their code is never run or
    /// served by hash.
    codeless: HashSet<Address>,
}

impl Witness {
    /// `account` (proven at the call's block) with its code (checked against its code hash).
    pub fn add(&mut self, account: Proven, code: Bytes) {
        let code =
            Bytecode::new_raw_checked(code.clone()).unwrap_or_else(|_| Bytecode::new_legacy(code));
        let slots = self.storage.entry(account.address).or_default();
        for (k, v) in &account.slots {
            slots.insert(U256::from_be_bytes(k.0), *v);
        }
        match self.accounts.get_mut(&account.address) {
            Some(held) => held.0.slots.extend(account.slots),
            None => {
                self.accounts.insert(account.address, (account, code));
            }
        }
    }

    /// `account` (proven at the call's block) without its code, which the caller's code replaces.
    pub fn add_without_code(&mut self, account: Proven) {
        if !self.accounts.contains_key(&account.address) {
            self.codeless.insert(account.address);
        }
        self.add(account, Bytes::new());
    }

    pub fn has(&self, address: &Address) -> bool {
        self.accounts.contains_key(address)
    }

    pub fn accounts(&self) -> usize {
        self.accounts.len()
    }

    pub fn slots(&self) -> usize {
        self.storage.values().map(HashMap::len).sum()
    }

    pub fn account(&self, address: &Address) -> Option<&Proven> {
        self.accounts.get(address).map(|(a, _)| a)
    }

    /// A slot of an account the witness holds, if it holds the slot.
    pub fn slot(&self, address: &Address, key: B256) -> Option<U256> {
        self.storage
            .get(address)
            .and_then(|s| s.get(&U256::from_be_bytes(key.0)))
            .copied()
    }

    /// Whether `address` is held without its code.
    pub fn codeless(&self, address: &Address) -> bool {
        self.codeless.contains(address)
    }
}

/// What a run read that the witness does not hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Missing {
    /// Accounts, each with the slots read of it.
    pub accounts: BTreeMap<Address, BTreeSet<B256>>,
    /// Slots read of accounts the witness holds.
    pub slots: BTreeMap<Address, BTreeSet<B256>>,
}

impl Missing {
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty() && self.slots.is_empty()
    }
}

/// What runs read of the witness: what a proof of them must hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Reads {
    /// Accounts, each with the slots read of it (none of an absent account: they are zero).
    pub accounts: BTreeMap<Address, BTreeSet<B256>>,
    /// Blocks whose hash was read (BLOCKHASH, `ArbSys.arbBlockHash`).
    pub block_hashes: BTreeSet<u64>,
}

/// How a run treats a read the witness lacks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// As empty, noted in [`Ran::Missing`]: the server gathers it and runs again.
    Gather,
    /// As the end of the run, [`Ran::Missing`]: a proof that lacks it does not verify.
    Strict,
}

/// What a run of the call came to.
#[derive(Debug, PartialEq)]
pub enum Ran {
    /// The bytes returned (a creation's: the code it deploys).
    Returned(Bytes),
    /// Reverted, with the revert data.
    Reverted(Bytes),
    /// Stopped by the EVM (out of gas, an invalid opcode, ...): geth's message for it.
    Halted(String),
    /// Not run: the transaction is invalid at this block (geth's message for why).
    Invalid(String),
    /// Read state the witness does not hold: the response waits for it (strict: the first such
    /// read).
    Missing(Missing),
    /// Read a block hash the store (or the witness) does not hold.
    NoBlockHash(u64),
    /// Reverted with one of ArbOS's own errors: the data, and Nitro's reason for it.
    RevertedSaying(Bytes, String),
    /// Not run here: what of the call isn't rebuilt (a Stylus program, an ArbOS precompile...).
    Refused(String),
    /// The EVM refused the block or itself, not the transaction.
    Internal(String),
}

/// What the EVM asked for that can't be read as empty: a block hash the store doesn't hold, or
/// code by a hash no proven account has.
#[derive(Debug)]
pub enum Unreadable {
    BlockHash(u64),
    Code(B256),
    /// Strict: state the witness lacks (noted in the run's missing reads).
    Missing,
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockHash(n) => write!(f, "the hash of block {n}"),
            Self::Code(h) => write!(f, "code {h}"),
            Self::Missing => write!(f, "state the witness does not hold"),
        }
    }
}
impl std::error::Error for Unreadable {}
impl DBErrorMarker for Unreadable {}

/// The witness as the EVM's database: what it lacks is noted and read as empty (gathering) or
/// ends the run (strict); what it holds and is read is noted in `reads`.
pub struct WitnessDb<'a> {
    witness: &'a Witness,
    missing: &'a RefCell<Missing>,
    reads: &'a RefCell<Reads>,
    hash_of: &'a dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    /// Accounts whose every field the caller overrides: read, but never needed proven.
    whole: HashSet<Address>,
}

impl std::fmt::Debug for WitnessDb<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WitnessDb").finish_non_exhaustive()
    }
}

impl WitnessDb<'_> {
    fn lacking(&self) -> Result<(), Unreadable> {
        match self.mode {
            Mode::Gather => Ok(()),
            Mode::Strict => Err(Unreadable::Missing),
        }
    }
}

impl DatabaseRef for WitnessDb<'_> {
    type Error = Unreadable;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Unreadable> {
        let Some((account, code)) = self.witness.accounts.get(&address) else {
            if self.whole.contains(&address) {
                return Ok(None);
            }
            self.missing
                .borrow_mut()
                .accounts
                .entry(address)
                .or_default();
            self.lacking()?;
            return Ok(None);
        };
        self.reads.borrow_mut().accounts.entry(address).or_default();
        // Held without its code: the caller's code is laid over it, and nothing else may run it.
        let code = (!self.witness.codeless(&address)).then(|| code.clone());
        Ok(account.exists.then(|| AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
            code,
        }))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Unreadable> {
        // Every account comes with its code, so the EVM never asks; refused if it does.
        self.witness
            .accounts
            .iter()
            .filter(|(a, _)| !self.witness.codeless(a))
            .map(|(_, held)| held)
            .find(|(a, _)| a.code_hash == code_hash)
            .map(|(_, c)| c.clone())
            .ok_or(Unreadable::Code(code_hash))
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Unreadable> {
        let key = B256::from(index);
        match self.witness.accounts.get(&address) {
            None => {
                self.missing
                    .borrow_mut()
                    .accounts
                    .entry(address)
                    .or_default()
                    .insert(key);
                self.lacking()?;
                Ok(U256::ZERO)
            }
            Some((a, _)) if !a.exists => Ok(U256::ZERO),
            Some(_) => match self
                .witness
                .storage
                .get(&address)
                .and_then(|s| s.get(&index))
            {
                Some(v) => {
                    let mut reads = self.reads.borrow_mut();
                    reads.accounts.entry(address).or_default().insert(key);
                    Ok(*v)
                }
                None => {
                    self.missing
                        .borrow_mut()
                        .slots
                        .entry(address)
                        .or_default()
                        .insert(key);
                    self.lacking()?;
                    Ok(U256::ZERO)
                }
            },
        }
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Unreadable> {
        let hash = (self.hash_of)(number).ok_or(Unreadable::BlockHash(number))?;
        self.reads.borrow_mut().block_hashes.insert(number);
        Ok(hash)
    }
}

/// Geth's message for an EVM halt.
pub fn halted(reason: &HaltReason) -> String {
    match reason {
        HaltReason::OutOfGas(_) => "out of gas".into(),
        HaltReason::OpcodeNotFound | HaltReason::InvalidFEOpcode | HaltReason::NotActivated => {
            "invalid opcode".into()
        }
        HaltReason::InvalidJump => "invalid jump destination".into(),
        HaltReason::StackUnderflow => "stack underflow".into(),
        // Geth says "stack limit reached 1024 (1023)": "limit" would have venn bench Willow.
        HaltReason::StackOverflow => "stack overflow (1024 entries)".into(),
        HaltReason::OutOfOffset => "return data out of bounds".into(),
        HaltReason::CreateCollision => "contract address collision".into(),
        HaltReason::PrecompileError => "precompile failed".into(),
        HaltReason::NonceOverflow => "nonce uint64 overflow".into(),
        HaltReason::CreateContractSizeLimit => "max code size exceeded".into(),
        HaltReason::CreateContractStartingWithEF => "invalid code: must not begin with 0xef".into(),
        HaltReason::CreateInitCodeSizeLimit => "max initcode size exceeded".into(),
        HaltReason::CallTooDeep => "max call depth exceeded".into(),
        HaltReason::OutOfFunds => "insufficient balance for transfer".into(),
        other => routable(&format!("{other:?}"), "the call halted"),
    }
}

/// `m`, unless it holds words that have a router bench an upstream for every method (venn:
/// "limit", "rate") or mark the method unsupported there (erpc: "not supported"): then `instead`.
fn routable(m: &str, instead: &str) -> String {
    let lower = m.to_lowercase();
    match ["limit", "rate", "not supported", "unsupported"]
        .iter()
        .any(|w| lower.contains(w))
    {
        true => instead.to_string(),
        false => m.to_string(),
    }
}

/// Geth's message for a transaction the EVM won't run, after "invalid transaction: " where it
/// isn't "insufficient funds" (erpc hands both back; some of geth's own wording it reads as a
/// failing upstream).
pub fn invalid_message(e: &InvalidTransaction) -> String {
    let m = geth_invalid(e);
    match m.starts_with("insufficient funds") {
        true => m,
        false => format!("invalid transaction: {m}"),
    }
}

pub(crate) fn geth_invalid(e: &InvalidTransaction) -> String {
    use InvalidTransaction as I;
    match e {
        I::CallGasCostMoreThanGasLimit {
            initial_gas,
            gas_limit,
        } => format!("intrinsic gas too low: have {gas_limit}, want {initial_gas}"),
        I::GasFloorMoreThanGasLimit {
            gas_floor,
            gas_limit,
        } => {
            format!("insufficient gas for floor data gas cost: have {gas_limit}, want {gas_floor}")
        }
        I::LackOfFundForMaxFee { fee, balance } => {
            format!("insufficient funds for gas * price + value: have {balance} want {fee}")
        }
        I::PriorityFeeGreaterThanMaxFee => {
            "max priority fee per gas higher than max fee per gas".into()
        }
        I::GasPriceLessThanBasefee => "max fee per gas less than block base fee".into(),
        I::CreateInitCodeSizeLimit => "max initcode size exceeded".into(),
        other => routable(
            &other.to_string(),
            "the transaction can't run at this block",
        ),
    }
}

/// Nitro's words for a message it won't run, read by routers as the call's own failure (as they
/// read geth's): marked an invalid transaction unless they already read so.
pub fn nitro_invalid(m: String) -> String {
    let m = match m.contains("insufficient funds") || m.contains("gas too low") {
        true => m,
        false => format!("invalid transaction: {m}"),
    };
    routable(
        &m,
        "invalid transaction: the transaction can't run at this block",
    )
}

/// The EVM environment of `block` as the chain runs its transactions.
fn chain_env(block: &Block) -> (CfgEnv, BlockEnv) {
    let spec = spec(block.number, block.timestamp);
    let mut cfg = CfgEnv::new_with_spec(spec);
    cfg.chain_id = 1;
    let merged = spec.is_enabled_in(SpecId::MERGE);
    let block_env = BlockEnv {
        number: U256::from(block.number),
        beneficiary: block.beneficiary,
        timestamp: U256::from(block.timestamp),
        gas_limit: block.gas_limit,
        basefee: block.base_fee.unwrap_or(0),
        difficulty: block.difficulty,
        prevrandao: merged.then_some(block.mix_hash),
        blob_excess_gas_and_price: block
            .blob
            .filter(|_| spec.is_enabled_in(SpecId::CANCUN))
            .map(|(excess_blob_gas, blob_gasprice)| BlobExcessGasAndPrice {
                excess_blob_gas,
                blob_gasprice,
            }),
    };
    (cfg, block_env)
}

/// The block's fields as the caller sets them (reth's: the fork stays the block's own).
fn block_overrides(block_env: &mut BlockEnv, overrides: &Overrides) {
    let o = &overrides.block;
    if let Some(n) = o.number {
        block_env.number = n;
    }
    if let Some(d) = o.difficulty {
        block_env.difficulty = d;
    }
    if let Some(t) = o.time {
        block_env.timestamp = U256::from(t);
    }
    if let Some(g) = o.gas_limit {
        block_env.gas_limit = g;
    }
    if let Some(c) = o.coinbase {
        block_env.beneficiary = c;
    }
    if let Some(r) = o.random {
        block_env.prevrandao = Some(r);
    }
    if let Some(b) = o.base_fee {
        block_env.basefee = b.saturating_to();
    }
    if let Some(b) = overrides.blob_base_fee {
        let excess_blob_gas = block_env
            .blob_excess_gas_and_price
            .map_or(0, |b| b.excess_blob_gas);
        block_env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
            excess_blob_gas,
            blob_gasprice: b.saturating_to(),
        });
    }
}

/// The access list as the block's rules read it (none before Berlin), and the transaction type.
fn tx_shape(spec: SpecId, call: &Call) -> (bool, AccessList, u8) {
    // Geth reads fee fields as a 1559 price only from London on, and an access list from Berlin.
    let eip1559 = spec.is_enabled_in(SpecId::LONDON)
        && (call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some());
    let access_list = match spec.is_enabled_in(SpecId::BERLIN) {
        true => call.access_list.as_slice(),
        false => &[],
    };
    let ty = match (eip1559, access_list.is_empty()) {
        (true, _) => 2,
        (false, false) => 1,
        (false, true) => 0,
    };
    let list = AccessList(
        access_list
            .iter()
            .map(|(address, keys)| AccessListItem {
                address: *address,
                storage_keys: keys.clone(),
            })
            .collect(),
    );
    (eip1559, list, ty)
}

/// The EVM environment `call` runs in at `block` as reth's `eth_call` (the log RPC's), with the
/// caller's block overrides.
pub fn call_env(block: &Block, call: &Call, overrides: &Overrides) -> (CfgEnv, BlockEnv, TxEnv) {
    let spec = spec(block.number, block.timestamp);
    let (mut cfg, mut block_env) = chain_env(block);
    cfg.disable_nonce_check = true;
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    // Calls skip EIP-7825's cap, and the base fee's check (reth's checks a 1559 cap itself).
    cfg.tx_gas_limit_cap = Some(u64::MAX);
    cfg.disable_base_fee = true;
    block_overrides(&mut block_env, overrides);
    // A call without a gas price runs at a zero base fee (reth and geth).
    let fees = call.fees_given() && call.price(block_env.basefee) > 0;
    if !fees {
        block_env.basefee = 0;
    }
    let (eip1559, access_list, tx_type) = tx_shape(spec, call);
    let tx = TxEnv {
        tx_type,
        caller: call.from,
        gas_limit: call.gas.unwrap_or(GAS_CAP).min(GAS_CAP),
        // A tip alone has no cap (reth's).
        gas_price: match eip1559 {
            true => call
                .max_fee_per_gas
                .unwrap_or(match call.max_priority_fee_per_gas {
                    Some(_) => u128::MAX,
                    None => 0,
                }),
            false => call.gas_price.unwrap_or(0),
        },
        gas_priority_fee: eip1559.then(|| call.max_priority_fee_per_gas.unwrap_or(0)),
        kind: call.to.map_or(TxKind::Create, TxKind::Call),
        value: call.value,
        data: call.data.clone(),
        chain_id: Some(1),
        access_list,
        ..Default::default()
    };
    (cfg, block_env, tx)
}

/// The EVM environment of one of `eth_estimateGas`'s runs at `gas`, as geth runs them (the gas
/// bought at the price named, the base fee checked when one is named, zero when none is), with
/// the caller's block overrides.
fn estimate_env(
    block: &Block,
    call: &Call,
    gas: u64,
    overrides: &Overrides,
) -> (CfgEnv, BlockEnv, TxEnv) {
    let spec = spec(block.number, block.timestamp);
    let fees = call.fees_given();
    let (mut cfg, mut block_env) = chain_env(block);
    cfg.disable_nonce_check = true;
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    // Geth's calls skip EIP-7825's cap, and with no gas price the base fee.
    cfg.tx_gas_limit_cap = Some(u64::MAX);
    cfg.disable_base_fee = !fees;
    block_overrides(&mut block_env, overrides);
    // Geth lowers the base fee to zero for a call without a gas price.
    if !fees {
        block_env.basefee = 0;
    }
    let (eip1559, access_list, tx_type) = tx_shape(spec, call);
    let tx = TxEnv {
        tx_type,
        caller: call.from,
        gas_limit: gas,
        gas_price: match eip1559 {
            true => call.max_fee_per_gas.unwrap_or(0),
            false => call.gas_price.unwrap_or(0),
        },
        gas_priority_fee: eip1559.then(|| call.max_priority_fee_per_gas.unwrap_or(0)),
        kind: call.to.map_or(TxKind::Create, TxKind::Call),
        value: call.value,
        data: call.data.clone(),
        chain_id: Some(1),
        access_list,
        ..Default::default()
    };
    (cfg, block_env, tx)
}

/// What an EVM run came to, as a call's response: what it was missing first.
pub fn ran<H>(
    out: Result<ExecutionResult<H>, EVMError<Unreadable>>,
    missing: Missing,
    halted: impl Fn(&H) -> String,
) -> Ran {
    if !missing.is_empty() {
        return Ran::Missing(missing);
    }
    match out {
        Ok(ExecutionResult::Success { output, .. }) => Ran::Returned(match output {
            Output::Call(b) | Output::Create(b, _) => b,
        }),
        Ok(ExecutionResult::Revert { output, .. }) => Ran::Reverted(output),
        Ok(ExecutionResult::Halt { reason, .. }) => Ran::Halted(halted(&reason)),
        Err(EVMError::Database(Unreadable::BlockHash(n))) => Ran::NoBlockHash(n),
        Err(EVMError::Database(e)) => Ran::Internal(e.to_string()),
        Err(EVMError::Transaction(e)) => Ran::Invalid(invalid_message(&e)),
        Err(e) => Ran::Internal(e.to_string()),
    }
}

/// Reth's (v2) handling of a call's gas price: the caller pays nothing up front and is refunded
/// nothing, and the fee recipient is paid nothing (no balance check either: a value the caller
/// can't send fails as the transfer runs). GASPRICE and BASEFEE read as named.
pub struct CallHandler<EVM, ERROR, FRAME>(std::marker::PhantomData<(EVM, ERROR, FRAME)>);

impl<EVM, ERROR, FRAME> Default for CallHandler<EVM, ERROR, FRAME> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<EVM, ERROR, FRAME> revm::handler::Handler for CallHandler<EVM, ERROR, FRAME>
where
    EVM: revm::handler::EvmTr<
        Context: revm::context_interface::ContextTr<
            Journal: revm::context_interface::JournalTr<State = revm::state::EvmState>,
        >,
        Frame = FRAME,
    >,
    ERROR: revm::handler::EvmTrError<EVM>,
    FRAME: revm::handler::FrameTr<
        FrameResult = revm::handler::FrameResult,
        FrameInit = revm::interpreter::interpreter_action::FrameInit,
    >,
{
    type Evm = EVM;
    type Error = ERROR;
    type HaltReason = HaltReason;

    fn validate_against_state_and_deduct_caller(
        &self,
        evm: &mut Self::Evm,
    ) -> Result<(), Self::Error> {
        use revm::context_interface::{ContextTr, JournalTr, Transaction};
        let (tx, journal) = evm.ctx().tx_journal_mut();
        let (caller, is_call) = (tx.caller(), tx.kind().is_call());
        let account = journal.load_account_code(caller)?.data;
        let balance = account.info.balance;
        account.mark_touch();
        if is_call {
            account.info.nonce = account.info.nonce.saturating_add(1);
        }
        journal.caller_accounting_journal_entry(caller, balance, is_call);
        Ok(())
    }

    fn reimburse_caller(
        &self,
        _: &mut Self::Evm,
        _: &mut <<Self::Evm as revm::handler::EvmTr>::Frame as revm::handler::FrameTr>::FrameResult,
    ) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reward_beneficiary(
        &self,
        _: &mut Self::Evm,
        _: &mut <<Self::Evm as revm::handler::EvmTr>::Frame as revm::handler::FrameTr>::FrameResult,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// A call run with an inspector (a call's trace) as it runs without one.
impl<EVM, ERROR> revm::inspector::InspectorHandler
    for CallHandler<
        EVM,
        ERROR,
        revm::handler::EthFrame<revm::interpreter::interpreter::EthInterpreter>,
    >
where
    EVM: revm::inspector::InspectorEvmTr<
        Context: revm::context_interface::ContextTr<
            Journal: revm::context_interface::JournalTr<State = revm::state::EvmState>,
        >,
        Frame = revm::handler::EthFrame<revm::interpreter::interpreter::EthInterpreter>,
        Inspector: revm::Inspector<
            <EVM as revm::handler::EvmTr>::Context,
            revm::interpreter::interpreter::EthInterpreter,
        >,
    >,
    ERROR: revm::handler::EvmTrError<EVM>,
{
    type IT = revm::interpreter::interpreter::EthInterpreter;
}

/// The call's gas, as reth sets it: what it names (at most the cap), else the cap, or with a gas
/// price what the caller's balance less the value pays for, at most the block's gas limit; and
/// reth's refusal of a 1559 fee cap under the base fee.
pub fn call_gas<DB: revm::Database<Error = Unreadable>>(
    db: &mut DB,
    call: &Call,
    block_env: &BlockEnv,
    tx: &mut TxEnv,
) -> Result<(), Ran> {
    if call
        .max_fee_per_gas
        .is_some_and(|cap| block_env.basefee > 0 && cap < u128::from(block_env.basefee))
    {
        return Err(Ran::Invalid(invalid_message(
            &InvalidTransaction::GasPriceLessThanBasefee,
        )));
    }
    // The price the allowance is counted at: a legacy price, or a 1559 cap (reth's).
    let price = tx.gas_price;
    if call.gas.is_some() || price == 0 {
        return Ok(());
    }
    let balance = db
        .basic(call.from)
        .map_err(|e| Ran::Internal(e.to_string()))?
        .map_or(U256::ZERO, |a| a.balance);
    let Some(left) = balance.checked_sub(call.value) else {
        return Err(Ran::Invalid(invalid_message(
            &InvalidTransaction::LackOfFundForMaxFee {
                fee: Box::new(call.value),
                balance: Box::new(balance),
            },
        )));
    };
    let allowance: u64 = (left / U256::from(price)).saturating_to();
    tx.gas_limit = tx.gas_limit.min(allowance).min(block_env.gas_limit);
    Ok(())
}

/// A proven state (a call's witness) as the OP stack's EVM reads it; what it lacks is noted by
/// the state itself.
struct OpState<'a, D>(&'a D);

impl<D: DatabaseRef<Error = Unreadable>> willow_op_exec::State for OpState<'_, D> {
    fn basic(&self, address: Address) -> Option<willow_op_exec::Account> {
        self.0
            .basic_ref(address)
            .ok()
            .flatten()
            .map(|a| willow_op_exec::Account {
                balance: a.balance,
                nonce: a.nonce,
                code_hash: a.code_hash,
                code: a.code.map(|c| c.original_bytes()),
            })
    }

    fn storage(&self, address: Address, slot: U256) -> U256 {
        self.0.storage_ref(address, slot).unwrap_or_default()
    }

    fn code(&self, hash: B256) -> Option<Bytes> {
        self.0
            .code_by_hash_ref(hash)
            .ok()
            .map(|c| c.original_bytes())
    }

    fn block_hash(&self, number: u64) -> Option<B256> {
        self.0.block_hash_ref(number).ok()
    }
}

/// A run's outcome with the gas it took: `used` after refunds, `peak` before them (geth's
/// `MaxUsedGas`, raised to the EIP-7623 floor), `out_of_gas` when it halted for lack of gas,
/// `intrinsic` when it was not run for gas below its intrinsic cost (geth's `ErrIntrinsicGas`).
pub(crate) struct Exec {
    pub ran: Ran,
    pub used: u64,
    pub peak: u64,
    pub out_of_gas: bool,
    pub intrinsic: bool,
}

impl Exec {
    fn not_run(ran: Ran) -> Exec {
        Exec {
            ran,
            used: 0,
            peak: 0,
            out_of_gas: false,
            intrinsic: false,
        }
    }
}

/// The gas left in the transaction's outermost frame when it ends.
#[derive(Default)]
struct Peak {
    depth: usize,
    remaining: Option<u64>,
}

impl Peak {
    fn end(&mut self, remaining: u64) {
        self.depth = self.depth.saturating_sub(1);
        if self.depth == 0 {
            self.remaining = Some(remaining);
        }
    }
}

impl<CTX> Inspector<CTX> for Peak {
    fn call(&mut self, _: &mut CTX, _: &mut CallInputs) -> Option<CallOutcome> {
        self.depth += 1;
        None
    }

    fn call_end(&mut self, _: &mut CTX, _: &CallInputs, outcome: &mut CallOutcome) {
        self.end(outcome.result.gas.remaining());
    }

    fn create(&mut self, _: &mut CTX, _: &mut CreateInputs) -> Option<CreateOutcome> {
        self.depth += 1;
        None
    }

    fn create_end(&mut self, _: &mut CTX, _: &CreateInputs, outcome: &mut CreateOutcome) {
        self.end(outcome.result.gas.remaining());
    }
}

/// One call to run: the chain's EVM, the proven state it reads (block hashes from `hash_of`), the
/// block, the call, and the caller's overrides.
#[derive(Clone, Copy)]
pub struct Run<'a> {
    pub engine: Engine,
    pub witness: &'a Witness,
    pub block: &'a Block,
    pub call: &'a Call,
    pub overrides: &'a Overrides,
    pub hash_of: &'a dyn Fn(u64) -> Option<B256>,
    pub mode: Mode,
}

impl<'a> Run<'a> {
    fn db<'b>(&self, missing: &'b RefCell<Missing>, reads: &'b RefCell<Reads>) -> WitnessDb<'b>
    where
        'a: 'b,
    {
        WitnessDb {
            witness: self.witness,
            missing,
            reads,
            hash_of: self.hash_of,
            mode: self.mode,
            whole: self.overrides.whole_accounts(),
        }
    }

    /// The witness under the caller's state overrides (as reth lays them over its state) and
    /// block hash overrides.
    pub fn overridden<'b>(
        &self,
        missing: &'b RefCell<Missing>,
        reads: &'b RefCell<Reads>,
        block_hashes: bool,
    ) -> Result<CacheDB<WitnessDb<'b>>, String>
    where
        'a: 'b,
    {
        let mut db = CacheDB::new(self.db(missing, reads));
        if block_hashes {
            for (n, h) in self.overrides.block.block_hash.iter().flatten() {
                db.cache.block_hashes.insert(U256::from(*n), *h);
            }
        }
        alloy_evm::overrides::apply_state_overrides(self.overrides.state.clone(), &mut db)
            .map_err(|e| e.to_string())?;
        Ok(db)
    }

    /// The call as `eth_call` runs it on its chain; what it read of the witness added to `reads`.
    pub fn call(&self, reads: &mut Reads) -> Ran {
        match self.engine {
            Engine::Ethereum => self.eth_call(reads),
            Engine::Op => self.op(None, reads).ran,
            Engine::Nitro => self.nitro(reads),
        }
    }

    /// One of `eth_estimateGas`'s runs, with a gas limit of `gas`; `peak` measured only when asked
    /// (on Ethereum it runs an inspector).
    pub(crate) fn at_gas(&self, gas: u64, reads: &mut Reads, peak: bool) -> Exec {
        match self.engine {
            Engine::Ethereum => self.eth_estimate_run(gas, reads, peak),
            Engine::Op => self.op(Some(gas), reads),
            Engine::Nitro => self.nitro_at(gas, reads),
        }
    }

    fn eth_call(&self, reads: &mut Reads) -> Ran {
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let out = self.eth_call_over(&missing, &read);
        *reads = read.into_inner();
        let missing = missing.into_inner();
        match out {
            Ok(out) => ran(out, missing, halted),
            Err(_) if !missing.is_empty() => Ran::Missing(missing),
            Err(r) => r,
        }
    }

    /// What an Ethereum call runs with, as `eth_call` runs it (reth's): its environment and
    /// transaction (the gas as reth sets it) and the witness under the caller's overrides, as
    /// `missing` and `read` note it; for a run of its own (a call's trace runs it with a tracer).
    /// `Err`: what the call came to without running.
    #[allow(clippy::type_complexity)]
    pub fn eth_prepared<'b>(
        &'b self,
        missing: &'b RefCell<Missing>,
        read: &'b RefCell<Reads>,
    ) -> Result<(CfgEnv, BlockEnv, TxEnv, CacheDB<WitnessDb<'b>>), Ran> {
        let (cfg, block_env, mut tx) = call_env(self.block, self.call, self.overrides);
        let mut db = self
            .overridden(missing, read, true)
            .map_err(Ran::Internal)?;
        call_gas(&mut db, self.call, &block_env, &mut tx)?;
        Ok((cfg, block_env, tx, db))
    }

    /// [`Run::eth_call`] over the witness as `missing` and `read` note it.
    fn eth_call_over<'b>(
        &'b self,
        missing: &'b RefCell<Missing>,
        read: &'b RefCell<Reads>,
    ) -> Result<Result<ExecutionResult<HaltReason>, EVMError<Unreadable>>, Ran> {
        use revm::context::ContextSetters;
        use revm::handler::Handler;
        let (cfg, block_env, tx, db) = self.eth_prepared(missing, read)?;
        let mut evm = Context::mainnet()
            .with_db(db)
            .with_block(block_env)
            .with_cfg(cfg)
            .build_mainnet();
        evm.ctx.set_tx(tx);
        Ok(CallHandler::default().run(&mut evm))
    }

    fn eth_estimate_run(&self, gas: u64, reads: &mut Reads, peak: bool) -> Exec {
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let out = self.eth_estimate_over(gas, peak, &missing, &read);
        *reads = read.into_inner();
        let missing = missing.into_inner();
        let (out, remaining) = match out {
            Ok(o) => o,
            Err(e) => {
                return Exec::not_run(match missing.is_empty() {
                    true => Ran::Internal(e),
                    false => Ran::Missing(missing),
                })
            }
        };
        let (used, out_of_gas) = match &out {
            Ok(r) => (
                r.gas_used(),
                matches!(r, ExecutionResult::Halt { reason: HaltReason::OutOfGas(e), .. }
                    if *e != OutOfGasError::InvalidOperand),
            ),
            Err(_) => (0, false),
        };
        let intrinsic = matches!(
            out,
            Err(EVMError::Transaction(
                InvalidTransaction::CallGasCostMoreThanGasLimit { .. }
            ))
        );
        // Geth's peak is the gas used before refunds, or the EIP-7623 floor when that is more: the
        // floor is what revm counts as used when it applies.
        let peak = remaining.map_or(used, |left| gas.saturating_sub(left).max(used));
        Exec {
            ran: ran(out, missing, halted),
            used,
            peak,
            out_of_gas,
            intrinsic,
        }
    }

    /// [`Run::eth_estimate_run`]'s run over the witness as `missing` and `read` note it, and the
    /// gas left in its outermost frame (with `peak`).
    #[allow(clippy::type_complexity)]
    fn eth_estimate_over<'b>(
        &'b self,
        gas: u64,
        peak: bool,
        missing: &'b RefCell<Missing>,
        read: &'b RefCell<Reads>,
    ) -> Result<
        (
            Result<ExecutionResult<HaltReason>, EVMError<Unreadable>>,
            Option<u64>,
        ),
        String,
    > {
        let (cfg, block_env, tx) = estimate_env(self.block, self.call, gas, self.overrides);
        let db = self.overridden(missing, read, true)?;
        let context = Context::mainnet()
            .with_db(db)
            .with_block(block_env)
            .with_cfg(cfg);
        Ok(match peak {
            true => {
                let mut evm = context.build_mainnet_with_inspector(Peak::default());
                let out = evm.inspect_one_tx(tx);
                (out, evm.inspector.remaining)
            }
            false => {
                let mut evm = context.build_mainnet();
                (evm.transact(tx).map(|r| r.result), None)
            }
        })
    }

    /// On Unichain, as op-reth runs it (`gas`: one of an estimate's runs, at that limit).
    fn op(&self, gas: Option<u64>, reads: &mut Reads) -> Exec {
        use willow_op_exec::Outcome;
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let db = self.db(&missing, &read);
        let (block, call) = (self.block, self.call);
        let op_block = willow_op_exec::Block {
            number: block.number,
            timestamp: block.timestamp,
            gas_limit: block.gas_limit,
            base_fee: block.base_fee.unwrap_or(0),
            mix_hash: block.mix_hash,
            beneficiary: block.beneficiary,
        };
        let op_call = willow_op_exec::Call {
            from: call.from,
            to: call.to,
            gas: gas.or(call.gas),
            gas_price: call.gas_price,
            max_fee_per_gas: call.max_fee_per_gas,
            max_priority_fee_per_gas: call.max_priority_fee_per_gas,
            value: call.value,
            data: call.data.clone(),
            access_list: call.access_list.clone(),
        };
        let op_overrides = willow_op_exec::Overrides {
            state: &self.overrides.state,
            block: &self.overrides.block,
            blob_base_fee: self.overrides.blob_base_fee,
        };
        let m = willow_op_exec::measured(&OpState(&db), &op_block, &op_call, op_overrides);
        drop(db);
        *reads = read.into_inner();
        let missing = missing.into_inner();
        let ran = match m.outcome {
            _ if !missing.is_empty() => Ran::Missing(missing),
            Outcome::Returned(b) => Ran::Returned(b),
            Outcome::Reverted(b) => Ran::Reverted(b),
            Outcome::Halted(m) => Ran::Halted(m),
            Outcome::Invalid(m) => Ran::Invalid(m),
            Outcome::NoBlockHash(n) => Ran::NoBlockHash(n),
            Outcome::Internal(m) => Ran::Internal(m),
        };
        Exec {
            ran,
            used: m.used,
            peak: m.peak,
            out_of_gas: m.out_of_gas,
            intrinsic: m.intrinsic,
        }
    }

    /// On Robinhood Chain, as a Nitro node runs it: the caller's state overrides laid over the
    /// witness (block hash overrides Nitro leaves be).
    fn nitro(&self, reads: &mut Reads) -> Ran {
        use willow_arb_exec::Outcome;
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let out = self.nitro_over(&missing, &read);
        *reads = read.into_inner();
        let missing = missing.into_inner();
        if !missing.is_empty() {
            return Ran::Missing(missing);
        }
        match out {
            Err(e) => Ran::Internal(e),
            Ok(Ok(Outcome::Returned(b))) => Ran::Returned(b),
            Ok(Ok(Outcome::Reverted(b, None))) => Ran::Reverted(b),
            // Error(string) and Panic(uint256) read first, as geth's RPC reads them.
            Ok(Ok(Outcome::Reverted(b, Some(_)))) if revert_reason(&b).is_some() => {
                Ran::Reverted(b)
            }
            Ok(Ok(Outcome::Reverted(b, Some(said)))) => Ran::RevertedSaying(b, said),
            Ok(Ok(Outcome::Halted(h))) => Ran::Halted(halted(&h)),
            Ok(Ok(Outcome::HaltedSaying(m))) => {
                Ran::Halted(routable(&m, "BLOBBASEFEE is not run on Arbitrum"))
            }
            Ok(Ok(Outcome::Invalid(m))) => Ran::Invalid(nitro_invalid(m)),
            Ok(Ok(Outcome::Refused(m))) => Ran::Refused(m),
            Ok(Ok(Outcome::Internal(m))) => Ran::Internal(m),
            Ok(Err(Unreadable::BlockHash(n))) => Ran::NoBlockHash(n),
            Ok(Err(e)) => Ran::Internal(e.to_string()),
        }
    }

    /// [`Run::nitro`] over the witness as `missing` and `read` note it.
    fn nitro_over<'b>(
        &'b self,
        missing: &'b RefCell<Missing>,
        read: &'b RefCell<Reads>,
    ) -> Result<Result<willow_arb_exec::Outcome, Unreadable>, String> {
        let db = self.overridden(missing, read, false)?;
        let (arb_block, arb_call, arb_overrides, overridden) = self.nitro_inputs();
        Ok(willow_arb_exec::run(
            db,
            &arb_block,
            &arb_call,
            &arb_overrides,
            &overridden,
        ))
    }

    /// One of `eth_estimateGas`'s runs on Robinhood Chain at `gas`, as Nitro's gas estimation mode
    /// runs it (the caller's state overrides laid over the witness).
    fn nitro_at(&self, gas: u64, reads: &mut Reads) -> Exec {
        use willow_arb_exec::Outcome;
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let out = self.overridden(&missing, &read, false).map(|db| {
            let (arb_block, arb_call, arb_overrides, overridden) = self.nitro_inputs();
            willow_arb_exec::estimate_run(
                db,
                &arb_block,
                &arb_call,
                &arb_overrides,
                &overridden,
                gas,
            )
        });
        *reads = read.into_inner();
        let missing = missing.into_inner();
        if !missing.is_empty() {
            return Exec::not_run(Ran::Missing(missing));
        }
        let m = match out {
            Err(e) => return Exec::not_run(Ran::Internal(e)),
            Ok(Err(Unreadable::BlockHash(n))) => return Exec::not_run(Ran::NoBlockHash(n)),
            Ok(Err(e)) => return Exec::not_run(Ran::Internal(e.to_string())),
            Ok(Ok(m)) => m,
        };
        let out_of_gas = matches!(m.outcome, Outcome::Halted(HaltReason::OutOfGas(e))
            if e != OutOfGasError::InvalidOperand);
        let ran = match m.outcome {
            Outcome::Returned(b) => Ran::Returned(b),
            Outcome::Reverted(b, None) => Ran::Reverted(b),
            Outcome::Reverted(b, Some(_)) if revert_reason(&b).is_some() => Ran::Reverted(b),
            Outcome::Reverted(b, Some(said)) => Ran::RevertedSaying(b, said),
            Outcome::Halted(h) => Ran::Halted(halted(&h)),
            Outcome::HaltedSaying(m) => {
                Ran::Halted(routable(&m, "BLOBBASEFEE is not run on Arbitrum"))
            }
            // The state transition's own words, as the search wraps them.
            Outcome::Invalid(m) => Ran::Invalid(nitro_invalid(m)),
            Outcome::Refused(m) => Ran::Refused(m),
            Outcome::Internal(m) => Ran::Internal(m),
        };
        Exec {
            ran,
            used: m.used,
            peak: m.peak,
            out_of_gas,
            intrinsic: m.intrinsic,
        }
    }

    /// The gas `eth_estimateGas` on Robinhood Chain raises its gas cap by (`RPCPostingGasHook`),
    /// read under the caller's overrides: `Err` what the search comes to instead.
    pub(crate) fn nitro_posting(&self, reads: &mut Reads) -> Result<u64, Ran> {
        let missing = RefCell::new(Missing::default());
        let read = RefCell::new(std::mem::take(reads));
        let out = self.overridden(&missing, &read, false).map(|mut db| {
            let (arb_block, arb_call, arb_overrides, _) = self.nitro_inputs();
            willow_arb_exec::estimate_posting_gas(&mut db, &arb_block, &arb_call, &arb_overrides)
        });
        *reads = read.into_inner();
        let missing = missing.into_inner();
        if !missing.is_empty() {
            return Err(Ran::Missing(missing));
        }
        match out {
            Err(e) => Err(Ran::Internal(e)),
            Ok(Err(e)) => Err(Ran::Internal(e.to_string())),
            Ok(Ok(Err(why))) => Err(Ran::Refused(why)),
            Ok(Ok(Ok(gas))) => Ok(gas),
        }
    }

    /// The block, call and block overrides as Nitro's run takes them, and the accounts the
    /// caller's state overrides set.
    fn nitro_inputs(
        &self,
    ) -> (
        willow_arb_exec::Block,
        willow_arb_exec::Call,
        willow_arb_exec::BlockOverrides,
        HashSet<Address>,
    ) {
        let (block, call) = (self.block, self.call);
        let arb_block = willow_arb_exec::Block {
            number: block.number,
            timestamp: block.timestamp,
            gas_limit: block.gas_limit,
            base_fee: block.base_fee.unwrap_or(0),
            difficulty: block.difficulty,
            mix_hash: block.mix_hash,
            beneficiary: block.beneficiary,
        };
        let arb_call = willow_arb_exec::Call {
            from: call.from,
            to: call.to,
            gas: call.gas,
            gas_price: call.gas_price,
            max_fee_per_gas: call.max_fee_per_gas,
            max_priority_fee_per_gas: call.max_priority_fee_per_gas,
            value: call.value,
            data: call.data.clone(),
            access_list: call.access_list.clone(),
            nonce: call.nonce,
            skip_l1_charging: call.skip_l1_charging,
        };
        let o = &self.overrides.block;
        let arb_overrides = willow_arb_exec::BlockOverrides {
            number: o.number,
            difficulty: o.difficulty,
            time: o.time,
            gas_limit: o.gas_limit,
            coinbase: o.coinbase,
            base_fee: o.base_fee,
        };
        let overridden: HashSet<Address> = self.overrides.state.keys().copied().collect();
        (arb_block, arb_call, arb_overrides, overridden)
    }
}

/// `call` run at `block` over `witness` on Ethereum, block hashes from `hash_of`, gathering.
pub fn run(
    witness: &Witness,
    block: &Block,
    call: &Call,
    hash_of: &dyn Fn(u64) -> Option<B256>,
) -> Ran {
    run_with(
        witness,
        block,
        call,
        hash_of,
        Mode::Gather,
        &mut Reads::default(),
    )
}

/// [`run`] in `mode`, adding what it read of the witness to `reads`.
pub fn run_with(
    witness: &Witness,
    block: &Block,
    call: &Call,
    hash_of: &dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    reads: &mut Reads,
) -> Ran {
    let none = Overrides::default();
    Run {
        engine: Engine::Ethereum,
        witness,
        block,
        call,
        overrides: &none,
        hash_of,
        mode,
    }
    .call(reads)
}

/// One of `eth_estimateGas`'s runs on Ethereum (geth's rules), without overrides, with a gas
/// limit of `gas`; `peak` measured only when asked (it runs the inspector).
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
pub(crate) fn execute(
    witness: &Witness,
    block: &Block,
    call: &Call,
    gas: u64,
    hash_of: &dyn Fn(u64) -> Option<B256>,
    mode: Mode,
    reads: &mut Reads,
    peak: bool,
) -> Exec {
    let none = Overrides::default();
    Run {
        engine: Engine::Ethereum,
        witness,
        block,
        call,
        overrides: &none,
        hash_of,
        mode,
    }
    .at_gas(gas, reads, peak)
}

/// Geth's `execution reverted` message: with the reason of an `Error(string)` or `Panic(uint256)`
/// revert, as geth's `abi.UnpackRevert` reads them.
pub fn revert_message(data: &[u8]) -> String {
    match revert_reason(data) {
        Some(r) => format!("execution reverted: {r}"),
        None => "execution reverted".into(),
    }
}

/// The reason of an `Error(string)` or `Panic(uint256)` revert, as geth's `abi.UnpackRevert`
/// reads it.
pub fn revert_reason(data: &[u8]) -> Option<String> {
    const ERROR: [u8; 4] = [0x08, 0xc3, 0x79, 0xa0];
    const PANIC: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];
    let word =
        |at: usize| -> Option<U256> { data.get(at..at.checked_add(32)?).map(U256::from_be_slice) };
    let small = |at: usize| -> Option<usize> { word(at)?.try_into().ok() };
    match data.get(..4) {
        Some(s) if s == ERROR => (|| {
            let at = 4usize.checked_add(small(4)?)?;
            let len = small(at)?;
            let start = at.checked_add(32)?;
            String::from_utf8(data.get(start..start.checked_add(len)?)?.to_vec()).ok()
        })(),
        Some(s) if s == PANIC => word(4).map(|code| {
            let known = match u64::try_from(code) {
                Ok(0x00) => Some("generic panic"),
                Ok(0x01) => Some("assert(false)"),
                Ok(0x11) => Some("arithmetic underflow or overflow"),
                Ok(0x12) => Some("division or modulo by zero"),
                Ok(0x21) => Some("enum overflow"),
                Ok(0x22) => Some("invalid encoded storage byte array accessed"),
                Ok(0x31) => Some("out-of-bounds array access; popping on an empty array"),
                Ok(0x32) => Some("out-of-bounds access of an array or bytesN"),
                Ok(0x41) => Some("out of memory"),
                Ok(0x51) => Some("uninitialized function"),
                _ => None,
            };
            known.map_or_else(|| format!("unknown panic code: {code:#x}"), str::to_string)
        }),
        _ => None,
    }
}

/// A halted call's message: geth's, worded so routers hand it back rather than ask elsewhere
/// (erpc reads an unknown -32000 as a failing upstream).
pub fn halt_message(m: &str) -> String {
    match m {
        "out of gas" => m.to_string(),
        m => format!("VM execution error: {m}"),
    }
}

/// Why a call object is not run: not valid (-32602), or valid but not served (its state can't be
/// proven here).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Unrunnable {
    Invalid(String),
    NotServed(String),
}

/// An `eth_call` transaction object, as geth reads one on Ethereum.
pub fn parse(o: &Value) -> Result<Call, Unrunnable> {
    parse_on(o, 1)
}

/// [`parse`] on chain `chain_id` (on Robinhood Chain with Nitro's own fields, `nonce` and
/// `skipL1Charging`).
pub fn parse_on(o: &Value, chain_id: u64) -> Result<Call, Unrunnable> {
    if !o.is_object() {
        return Err(Unrunnable::Invalid("expected a transaction object".into()));
    }
    let bad = |k: &str, e: String| Unrunnable::Invalid(format!("{k}: {e}"));
    let opt_address = |k: &str| match &o[k] {
        Value::Null => Ok(None),
        v => proof::address(v).map(Some).map_err(|e| bad(k, e)),
    };
    let opt_quantity = |k: &str| match &o[k] {
        Value::Null => Ok(None),
        v => proof::quantity(v).map(Some).map_err(|e| bad(k, e)),
    };
    let small = |k: &str| -> Result<Option<u128>, Unrunnable> {
        opt_quantity(k)?
            .map(|v| u128::try_from(v).map_err(|_| bad(k, "too large".into())))
            .transpose()
    };
    let data = match (&o["data"], &o["input"]) {
        (Value::Null, Value::Null) => Bytes::new(),
        (d, Value::Null) | (Value::Null, d) => proof::data(d).map_err(|e| bad("data", e))?,
        (d, i) => {
            let (d, i) = (
                proof::data(d).map_err(|e| bad("data", e))?,
                proof::data(i).map_err(|e| bad("input", e))?,
            );
            if d != i {
                return Err(Unrunnable::Invalid(
                    "both \"data\" and \"input\" are set and not equal. Please use \"input\" to \
                     pass transaction call data"
                        .into(),
                ));
            }
            i
        }
    };
    let gas_price = small("gasPrice")?;
    let (max_fee, tip) = (small("maxFeePerGas")?, small("maxPriorityFeePerGas")?);
    if gas_price.is_some() && (max_fee.is_some() || tip.is_some()) {
        return Err(Unrunnable::Invalid(
            "both gasPrice and (maxFeePerGas or maxPriorityFeePerGas) specified".into(),
        ));
    }
    if let Some(id) = opt_quantity("chainId")? {
        if id != U256::from(chain_id) {
            return Err(Unrunnable::Invalid(format!(
                "chainId does not match node's (have={id}, want={chain_id})"
            )));
        }
    }
    if ["blobVersionedHashes", "authorizationList"]
        .iter()
        .any(|k| o[*k].as_array().is_some_and(|l| !l.is_empty()))
    {
        return Err(Unrunnable::NotServed(
            "eth_call with blobs or authorizations: such calls' state is not proven here".into(),
        ));
    }
    let mut access_list = Vec::new();
    if let Some(items) = o["accessList"].as_array() {
        for item in items {
            let address = proof::address(&item["address"]).map_err(|e| bad("accessList", e))?;
            let keys = item["storageKeys"]
                .as_array()
                .map(|ks| ks.iter().map(proof::slot).collect::<Result<Vec<_>, _>>())
                .transpose()
                .map_err(|e| bad("accessList", e))?
                .unwrap_or_default();
            access_list.push((address, keys));
        }
    }
    let (nonce, skip_l1_charging) = match chain_id {
        willow_arb_exec::ROBINHOOD => {
            let nonce = opt_quantity("nonce")?
                .map(|n| u64::try_from(n).map_err(|_| bad("nonce", "too large".into())))
                .transpose()?;
            let skip = match &o["skipL1Charging"] {
                Value::Null => false,
                Value::Bool(b) => *b,
                _ => return Err(bad("skipL1Charging", "expected a boolean".into())),
            };
            (nonce, skip)
        }
        _ => (None, false),
    };
    Ok(Call {
        from: opt_address("from")?.unwrap_or(Address::ZERO),
        to: opt_address("to")?,
        gas: opt_quantity("gas")?.map(|g| u64::try_from(g).unwrap_or(u64::MAX)),
        gas_price,
        max_fee_per_gas: max_fee,
        max_priority_fee_per_gas: tip,
        value: opt_quantity("value")?.unwrap_or_default(),
        data,
        access_list,
        nonce,
        skip_l1_charging,
    })
}
