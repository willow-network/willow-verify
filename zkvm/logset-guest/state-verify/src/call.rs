//! `eth_call` on Ethereum, run over state proven against a verified block's state root, as geth
//! runs it (`eth_call`'s defaults: 50M gas, no nonce, sender-code or block-gas checks, a zero base
//! fee when no gas price is given). Every account, slot and block hash the call reads must be in
//! the witness; a run that reads anything else is answered by nothing, only by what it was
//! missing, so the witness can be filled and the call run again. The server gathers that way
//! ([`Mode::Gather`]); a client runs [`Mode::Strict`], where the first read the witness lacks
//! ends the run.

use crate::proof::{self, Proven};
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::context::result::{
    EVMError, ExecutionResult, HaltReason, InvalidTransaction, OutOfGasError, Output,
};
use revm::context::{BlockEnv, CfgEnv, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::context_interface::transaction::{AccessList, AccessListItem};
use revm::database_interface::DBErrorMarker;
use revm::interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome};
use revm::primitives::hardfork::SpecId;
use revm::primitives::TxKind;
use revm::state::{AccountInfo, Bytecode};
use revm::{Context, Database, ExecuteEvm, InspectEvm, Inspector, MainBuilder, MainContext};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};

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
}

impl Call {
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
#[derive(Default)]
pub struct Witness {
    accounts: HashMap<Address, (Proven, Bytecode)>,
    storage: HashMap<Address, HashMap<U256, U256>>,
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
    /// Blocks whose hash was read (BLOCKHASH).
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
    /// Read state the witness does not hold: the answer waits for it (strict: the first such read).
    Missing(Missing),
    /// Read a block hash the store does not hold.
    NoBlockHash(u64),
    /// The EVM refused the block or itself, not the transaction.
    Internal(String),
}

/// What the EVM asked for that can't be read as empty: a block hash the store doesn't hold, or
/// code by a hash no proven account has.
#[derive(Debug)]
enum Unreadable {
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
struct WitnessDb<'a> {
    witness: &'a Witness,
    missing: &'a RefCell<Missing>,
    reads: &'a RefCell<Reads>,
    hash_of: &'a dyn Fn(u64) -> Option<B256>,
    mode: Mode,
}

impl WitnessDb<'_> {
    fn lacking(&self) -> Result<(), Unreadable> {
        match self.mode {
            Mode::Gather => Ok(()),
            Mode::Strict => Err(Unreadable::Missing),
        }
    }
}

impl Database for WitnessDb<'_> {
    type Error = Unreadable;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Unreadable> {
        let Some((account, code)) = self.witness.accounts.get(&address) else {
            self.missing
                .borrow_mut()
                .accounts
                .entry(address)
                .or_default();
            self.lacking()?;
            return Ok(None);
        };
        self.reads.borrow_mut().accounts.entry(address).or_default();
        Ok(account.exists.then(|| AccountInfo {
            balance: account.balance,
            nonce: account.nonce,
            code_hash: account.code_hash,
            code: Some(code.clone()),
        }))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Unreadable> {
        // Every account comes with its code, so the EVM never asks; refused if it does.
        self.witness
            .accounts
            .values()
            .find(|(a, _)| a.code_hash == code_hash)
            .map(|(_, c)| c.clone())
            .ok_or(Unreadable::Code(code_hash))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Unreadable> {
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

    fn block_hash(&mut self, number: u64) -> Result<B256, Unreadable> {
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

/// `call` run at `block` over `witness` on Ethereum, block hashes from `hash_of`.
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
    let gas = call.gas.unwrap_or(GAS_CAP).min(GAS_CAP);
    execute(witness, block, call, gas, hash_of, mode, reads, false).ran
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

/// `call` with a gas limit of `gas`; `peak` measured only when asked (it runs the inspector).
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
    let spec = spec(block.number, block.timestamp);
    let fees = call.fees_given();
    let mut cfg = CfgEnv::new_with_spec(spec);
    cfg.chain_id = 1;
    cfg.disable_nonce_check = true;
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    // Geth's calls skip EIP-7825's cap, and with no gas price the base fee.
    cfg.tx_gas_limit_cap = Some(u64::MAX);
    cfg.disable_base_fee = !fees;
    let merged = spec.is_enabled_in(SpecId::MERGE);
    let block_env = BlockEnv {
        number: U256::from(block.number),
        beneficiary: block.beneficiary,
        timestamp: U256::from(block.timestamp),
        gas_limit: block.gas_limit,
        // Geth lowers the base fee to zero for a call without a gas price.
        basefee: match fees {
            true => block.base_fee.unwrap_or(0),
            false => 0,
        },
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
    // Geth reads fee fields as a 1559 price only from London on, and an access list from Berlin.
    let eip1559 = spec.is_enabled_in(SpecId::LONDON)
        && (call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some());
    let access_list = match spec.is_enabled_in(SpecId::BERLIN) {
        true => call.access_list.as_slice(),
        false => &[],
    };
    let tx = TxEnv {
        tx_type: match (eip1559, access_list.is_empty()) {
            (true, _) => 2,
            (false, false) => 1,
            (false, true) => 0,
        },
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
        access_list: AccessList(
            access_list
                .iter()
                .map(|(address, keys)| AccessListItem {
                    address: *address,
                    storage_keys: keys.clone(),
                })
                .collect(),
        ),
        ..Default::default()
    };
    let missing = RefCell::new(Missing::default());
    let read = RefCell::new(std::mem::take(reads));
    let db = WitnessDb {
        witness,
        missing: &missing,
        reads: &read,
        hash_of,
        mode,
    };
    let context = Context::mainnet()
        .with_db(db)
        .with_block(block_env)
        .with_cfg(cfg);
    let (out, remaining) = match peak {
        true => {
            let mut evm = context.build_mainnet_with_inspector(Peak::default());
            let out = evm.inspect_one_tx(tx);
            (out, evm.inspector.remaining)
        }
        false => {
            let mut evm = context.build_mainnet();
            (evm.transact(tx).map(|r| r.result), None)
        }
    };
    *reads = read.into_inner();
    let missing = missing.into_inner();
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
    let ran = if !missing.is_empty() {
        Ran::Missing(missing)
    } else {
        match out {
            Ok(r) => match r {
                ExecutionResult::Success { output, .. } => Ran::Returned(match output {
                    Output::Call(b) | Output::Create(b, _) => b,
                }),
                ExecutionResult::Revert { output, .. } => Ran::Reverted(output),
                ExecutionResult::Halt { reason, .. } => Ran::Halted(halted(&reason)),
            },
            Err(EVMError::Database(Unreadable::BlockHash(n))) => Ran::NoBlockHash(n),
            Err(EVMError::Database(e)) => Ran::Internal(e.to_string()),
            Err(EVMError::Transaction(e)) => Ran::Invalid(invalid_message(&e)),
            Err(e) => Ran::Internal(e.to_string()),
        }
    };
    Exec {
        ran,
        used,
        peak,
        out_of_gas,
        intrinsic,
    }
}

/// Geth's `execution reverted` message: with the reason of an `Error(string)` or `Panic(uint256)`
/// revert, as geth's `abi.UnpackRevert` reads them.
pub fn revert_message(data: &[u8]) -> String {
    const ERROR: [u8; 4] = [0x08, 0xc3, 0x79, 0xa0];
    const PANIC: [u8; 4] = [0x4e, 0x48, 0x7b, 0x71];
    let word =
        |at: usize| -> Option<U256> { data.get(at..at.checked_add(32)?).map(U256::from_be_slice) };
    let small = |at: usize| -> Option<usize> { word(at)?.try_into().ok() };
    let reason = match data.get(..4) {
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
    };
    match reason {
        Some(r) => format!("execution reverted: {r}"),
        None => "execution reverted".into(),
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

/// An `eth_call` transaction object, as geth reads one.
pub fn parse(o: &Value) -> Result<Call, Unrunnable> {
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
        if id != U256::from(1) {
            return Err(Unrunnable::Invalid(format!(
                "chainId does not match node's (have={id}, want=1)"
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
    })
}
