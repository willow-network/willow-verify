//! Traces as geth writes them where revm-inspectors v0.30 (reth v1.8's) writes them otherwise:
//! the prestate tracer's accounts, slots and code hashes, the call tracer's revert reasons, the
//! 4byte tracer's precompile calls, the struct logger's storage, and parity state diffs of
//! accounts a transaction brought into being (as revm-inspectors' later versions write them).
//! The log RPC's `state/trace_output.rs`, kept in step with it: its trace methods' responses and
//! a proof's must be the same.

use alloy::primitives::map::{HashMap, HashSet};
use alloy::primitives::{Address, B256, U256, U64};
use alloy::rpc::types::trace::parity::{ChangedType, Delta, StateDiff};
use alloy::trie::KECCAK_EMPTY;
use revm::bytecode::opcode;
use revm::context::TxEnv;
use revm::context_interface::{ContextTr, JournalTr};
use revm::interpreter::interpreter_types::{Jumps, LoopControl, MemoryTr, ReturnData};
use revm::interpreter::{CallInputs, CallOutcome, CreateInputs, CreateOutcome, Interpreter};
use revm::primitives::TxKind;
use revm::state::{AccountInfo, EvmState};
use revm::{DatabaseRef, Inspector};
use revm_inspectors::tracing::FourByteInspector;
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// The accounts and slots geth's prestate tracer looks up: the transaction's sender, recipient,
/// fee recipient and authorities, what opcodes name, and the slots SLOAD and SSTORE touch.
#[derive(Default)]
pub struct Lookups {
    accounts: HashSet<Address>,
    slots: HashMap<Address, HashSet<B256>>,
    /// The accounts called by opcodes, and the recipient: their code's delegation targets are
    /// looked up too (the recipient's as it was before the transaction's authorizations).
    called: HashSet<Address>,
    recipient: Option<Address>,
    /// What the step under way names, kept once it doesn't fail (geth looks nothing up at a step
    /// that fails, out of gas say): an account, whether it is called, and a slot of it.
    pending: Option<(Address, bool, Option<B256>)>,
}

impl Lookups {
    /// Those of a transaction itself, before it runs over `db`.
    pub fn of_tx<DB: DatabaseRef>(tx: &TxEnv, coinbase: Address, db: &DB) -> Self {
        let mut l = Self::default();
        l.accounts.insert(tx.caller);
        l.accounts.insert(coinbase);
        match tx.kind {
            TxKind::Call(to) => {
                l.accounts.insert(to);
                l.recipient = Some(to);
            }
            TxKind::Create => {
                let nonce = db
                    .basic_ref(tx.caller)
                    .ok()
                    .flatten()
                    .map_or(0, |i| i.nonce);
                l.accounts.insert(tx.caller.create(nonce));
            }
        }
        for auth in &tx.authorization_list {
            let authority = match auth {
                revm::context_interface::either::Either::Left(signed) => {
                    signed.recover_authority().ok()
                }
                revm::context_interface::either::Either::Right(recovered) => recovered.authority(),
            };
            if let Some(a) = authority {
                l.accounts.insert(a);
            }
        }
        l
    }

    /// Every account looked up, with the accounts the called ones' code delegates to (EIP-7702),
    /// read as geth reads it: the recipient's as it was before the transaction (when it starts),
    /// a called account's as it is when called (after the transaction's authorizations, which is
    /// how the transaction leaves it).
    fn accounts<DB: DatabaseRef>(&self, db: &DB, state: &EvmState) -> HashSet<Address> {
        let target = |code: Option<revm::state::Bytecode>| {
            let bytes = code.map(|c| c.original_bytes()).unwrap_or_default();
            (bytes.len() == 23 && bytes.starts_with(&[0xef, 0x01, 0x00]))
                .then(|| Address::from_slice(&bytes[3..]))
        };
        let code_of = |info: &AccountInfo| {
            info.code
                .clone()
                .or_else(|| db.code_by_hash_ref(info.code_hash).ok())
        };
        let before = |a: &Address| db.basic_ref(*a).ok().flatten();
        let mut all = self.accounts.clone();
        if let Some(info) = self.recipient.as_ref().and_then(before) {
            all.extend(target(code_of(&info)));
        }
        for a in &self.called {
            let info = state.get(a).map(|s| s.info.clone()).or_else(|| before(a));
            if let Some(info) = info {
                all.extend(target(code_of(&info)));
            }
        }
        all
    }
}

impl<CTX: ContextTr> Inspector<CTX> for Lookups {
    fn step(&mut self, interp: &mut Interpreter, _: &mut CTX) {
        let stack = interp.stack.data();
        let at = |i: usize| stack.len().checked_sub(1 + i).map(|j| stack[j]);
        let address = |w: U256| Address::from_word(B256::from(w));
        let contract = interp.input.target_address;
        self.pending = match interp.bytecode.opcode() {
            opcode::SLOAD | opcode::SSTORE => at(0).map(|w| (contract, false, Some(B256::from(w)))),
            opcode::EXTCODECOPY
            | opcode::EXTCODEHASH
            | opcode::EXTCODESIZE
            | opcode::BALANCE
            | opcode::SELFDESTRUCT => at(0).map(|w| (address(w), false, None)),
            opcode::CALL | opcode::CALLCODE | opcode::DELEGATECALL | opcode::STATICCALL
                if stack.len() >= 5 =>
            {
                at(1).map(|w| (address(w), true, None))
            }
            _ => None,
        };
    }

    fn step_end(&mut self, interp: &mut Interpreter, _: &mut CTX) {
        let Some((a, called, slot)) = self.pending.take() else {
            return;
        };
        let failed = interp
            .bytecode
            .action()
            .as_ref()
            .and_then(|x| x.instruction_result())
            .is_some_and(|r| r.is_error());
        if failed {
            return;
        }
        self.accounts.insert(a);
        if called {
            self.called.insert(a);
        }
        if let Some(slot) = slot {
            self.slots.entry(a).or_default().insert(slot);
        }
    }

    fn create_end(&mut self, _: &mut CTX, _: &CreateInputs, outcome: &mut CreateOutcome) {
        if let Some(a) = outcome.address {
            self.accounts.insert(a);
        }
    }
}

/// What geth's prestate tracer reads of one account: its fields, code and code hash.
struct Seen {
    balance: U256,
    nonce: u64,
    code: Vec<u8>,
    code_hash: B256,
    exists: bool,
}

/// Geth's prestate trace of a transaction that ran over `db` (the state before it) to `state`:
/// the accounts and slots geth looks up (`lookups`), as they were before it; in a diff, those that
/// changed, before and after (geth's `processDiffState`, and its OnTxEnd's leaving out of
/// accounts empty before the transaction unless `includeEmpty`).
pub fn geth_prestate<DB: DatabaseRef>(
    db: &DB,
    state: &EvmState,
    lookups: &Lookups,
    config: &alloy::rpc::types::trace::geth::PreStateConfig,
    include_empty: bool,
    budget: &Budget,
) -> Result<Value, DB::Error> {
    let (code_on, storage_on) = (config.code_enabled(), config.storage_enabled());
    let code_of = |info: &AccountInfo| -> Vec<u8> {
        info.code
            .as_ref()
            .map(|c| c.original_bytes().to_vec())
            .or_else(
                || match info.code_hash == KECCAK_EMPTY || info.code_hash.is_zero() {
                    true => None,
                    false => db
                        .code_by_hash_ref(info.code_hash)
                        .ok()
                        .map(|c| c.original_bytes().to_vec()),
                },
            )
            .unwrap_or_default()
    };
    let read = |info: Option<AccountInfo>| match info {
        Some(i) => Seen {
            balance: i.balance,
            nonce: i.nonce,
            code: code_of(&i),
            code_hash: i.code_hash,
            exists: true,
        },
        None => Seen {
            balance: U256::ZERO,
            nonce: 0,
            code: vec![],
            code_hash: B256::ZERO,
            exists: false,
        },
    };
    let has_code = |h: &B256| *h != KECCAK_EMPTY && !h.is_zero();
    let word = |v: U256| json!(B256::from(v));
    let account = |s: &Seen, slots: &BTreeMap<B256, U256>| {
        let mut o = serde_json::Map::new();
        o.insert("balance".into(), json!(format!("{:#x}", s.balance)));
        if s.nonce != 0 {
            o.insert("nonce".into(), json!(s.nonce));
        }
        if code_on && !s.code.is_empty() {
            o.insert(
                "code".into(),
                json!(alloy::primitives::Bytes::from(s.code.clone())),
            );
        }
        if has_code(&s.code_hash) {
            o.insert("codeHash".into(), json!(s.code_hash));
        }
        if storage_on && !slots.is_empty() {
            let m: serde_json::Map<String, Value> = slots
                .iter()
                .map(|(k, v)| (format!("{k:?}"), word(*v)))
                .collect();
            o.insert("storage".into(), Value::Object(m));
        }
        Value::Object(o)
    };
    let mut pre = serde_json::Map::new();
    let mut post = serde_json::Map::new();
    let mut accounts: Vec<Address> = lookups.accounts(db, state).into_iter().collect();
    accounts.sort();
    for a in accounts {
        let before = read(db.basic_ref(a)?);
        budget
            .add(256 + 2 * before.code.len() + 128 * lookups.slots.get(&a).map_or(0, |s| s.len()));
        if budget.over() {
            break;
        }
        let empty = !before.exists
            || (before.balance.is_zero() && before.nonce == 0 && !has_code(&before.code_hash));
        let mut slots = BTreeMap::new();
        for k in lookups.slots.get(&a).into_iter().flatten() {
            slots.insert(*k, db.storage_ref(a, U256::from_be_bytes(k.0))?);
        }
        if !config.is_diff_mode() {
            if include_empty || !empty {
                pre.insert(format!("{a:#x}"), account(&before, &slots));
            }
            continue;
        }
        let after_state = state.get(&a);
        if after_state.is_some_and(|s| s.is_selfdestructed()) {
            // Deleted: kept as it was, never in the post state.
            if include_empty || !empty {
                pre.insert(format!("{a:#x}"), account(&before, &slots));
            }
            continue;
        }
        let after = match after_state {
            Some(s) => read(Some(s.info.clone())),
            None => Seen {
                ..read(db.basic_ref(a)?)
            },
        };
        // Geth reads an absent account's code hash as zero, a codeless one's as the empty hash.
        let after_hash = match after_state {
            Some(s) if !s.info.is_empty() || before.exists => s.info.code_hash,
            Some(_) | None if !before.exists => B256::ZERO,
            _ => after.code_hash,
        };
        let mut changed = serde_json::Map::new();
        if after.balance != before.balance {
            changed.insert("balance".into(), json!(format!("{:#x}", after.balance)));
        }
        if after.nonce != before.nonce && after.nonce != 0 {
            changed.insert("nonce".into(), json!(after.nonce));
        }
        let mut modified = after.balance != before.balance || after.nonce != before.nonce;
        let before_hash = match has_code(&before.code_hash) {
            true => before.code_hash,
            false => KECCAK_EMPTY,
        };
        if after_hash != before_hash {
            modified = true;
            changed.insert("codeHash".into(), json!(after_hash));
        }
        if code_on && after.code != before.code {
            modified = true;
            changed.insert(
                "code".into(),
                json!(alloy::primitives::Bytes::from(after.code.clone())),
            );
        }
        let mut kept = BTreeMap::new();
        let mut written = BTreeMap::new();
        if storage_on {
            for (k, v) in &slots {
                let now = after_state
                    .and_then(|s| s.storage.get(&U256::from_be_bytes(k.0)))
                    .map_or(*v, |s| s.present_value);
                if v.is_zero() && now.is_zero() || *v == now {
                    continue;
                }
                modified = true;
                if !v.is_zero() {
                    kept.insert(*k, *v);
                }
                if !now.is_zero() {
                    written.insert(*k, now);
                }
            }
        }
        if !modified || (!include_empty && empty) {
            if modified {
                post.insert(format!("{a:#x}"), post_account(changed, &written, word));
            }
            continue;
        }
        pre.insert(format!("{a:#x}"), account(&before, &kept));
        post.insert(format!("{a:#x}"), post_account(changed, &written, word));
    }
    Ok(match config.is_diff_mode() {
        true => json!({"pre": pre, "post": post}),
        false => Value::Object(pre),
    })
}

fn post_account(
    mut changed: serde_json::Map<String, Value>,
    written: &BTreeMap<B256, U256>,
    word: impl Fn(U256) -> Value,
) -> Value {
    if !written.is_empty() {
        let m: serde_json::Map<String, Value> = written
            .iter()
            .map(|(k, v)| (format!("{k:?}"), word(*v)))
            .collect();
        changed.insert("storage".into(), Value::Object(m));
    }
    Value::Object(changed)
}

/// Whether a prestate trace's config asks for accounts empty before the transaction (geth's
/// `includeEmpty`, which alloy's config here doesn't name).
pub fn include_empty(config: &alloy::rpc::types::trace::geth::GethDebugTracerConfig) -> bool {
    config.0.get("includeEmpty").and_then(Value::as_bool) == Some(true)
}

/// A call tracer's frames with geth's revert reasons: the reason an `Error(string)` or
/// `Panic(uint256)` revert names (geth's `abi.UnpackRevert`), and none for any other output.
pub fn geth_revert_reasons(frame: &mut Value) {
    if let Some(f) = frame.as_object_mut() {
        f.remove("revertReason");
        if f.get("error").and_then(Value::as_str) == Some("execution reverted") {
            let output = f
                .get("output")
                .and_then(|o| crate::request::data(o).ok())
                .unwrap_or_default();
            if let Some(r) = revert_reason(&output) {
                f.insert("revertReason".into(), json!(r));
            }
        }
        for c in f
            .get_mut("calls")
            .and_then(Value::as_array_mut)
            .into_iter()
            .flatten()
        {
            geth_revert_reasons(c);
        }
    }
}

/// revm-inspectors' 4byte tracer, but calls to precompiles left out, as geth's tracer (and
/// revm-inspectors' own from later versions) leaves them.
#[derive(Default)]
pub struct FourBytes(pub FourByteInspector);

impl<CTX: ContextTr> Inspector<CTX> for FourBytes {
    fn call(&mut self, context: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        match context
            .journal_ref()
            .precompile_addresses()
            .contains(&inputs.bytecode_address)
        {
            true => None,
            false => self.0.call(context, inputs),
        }
    }
}

/// The storage geth's struct logger writes at each step: at every SLOAD and SSTORE, the slots of
/// the running contract read or written so far in the transaction (revm-inspectors v0.30 writes
/// them at SSTOREs alone, without `0x`).
pub struct StorageSteps<'a> {
    steps: Vec<Option<BTreeMap<B256, B256>>>,
    held: HashMap<Address, BTreeMap<B256, B256>>,
    loading: Option<(usize, Address, B256)>,
    budget: &'a Budget,
}

impl<'a> StorageSteps<'a> {
    pub fn new(budget: &'a Budget) -> Self {
        Self {
            steps: vec![],
            held: Default::default(),
            loading: None,
            budget,
        }
    }

    fn note(&mut self, at: usize, contract: Address, key: B256, value: B256) {
        let slots = self.held.entry(contract).or_default();
        slots.insert(key, value);
        self.budget.add(slots.len() * 64);
        if !self.budget.over() {
            self.steps[at] = Some(slots.clone());
        }
    }

    /// Each struct log's `storage` written as geth writes it, when the steps line up.
    pub fn write(&self, frame: &mut Value, enabled: bool) {
        let Some(logs) = frame["structLogs"].as_array_mut() else {
            return;
        };
        if logs.len() != self.steps.len() {
            return;
        }
        for (log, at) in logs.iter_mut().zip(&self.steps) {
            let Some(log) = log.as_object_mut() else {
                continue;
            };
            log.remove("storage");
            if let (true, Some(slots)) = (enabled, at) {
                let slots: serde_json::Map<String, Value> = slots
                    .iter()
                    .map(|(k, v)| (format!("{k:?}"), json!(format!("{v:?}"))))
                    .collect();
                log.insert("storage".into(), Value::Object(slots));
            }
        }
    }
}

impl<CTX> Inspector<CTX> for StorageSteps<'_> {
    fn step(&mut self, interp: &mut Interpreter, _: &mut CTX) {
        let at = self.steps.len();
        self.steps.push(None);
        let contract = interp.input.target_address;
        let stack = interp.stack.data();
        let word = |i: usize| B256::from(stack[stack.len() - 1 - i]);
        match interp.bytecode.opcode() {
            opcode::SLOAD if !stack.is_empty() => self.loading = Some((at, contract, word(0))),
            opcode::SSTORE if stack.len() >= 2 => {
                let (key, value) = (word(0), word(1));
                self.note(at, contract, key, value);
            }
            _ => {}
        }
    }

    fn step_end(&mut self, interp: &mut Interpreter, _: &mut CTX) {
        if let Some((at, contract, key)) = self.loading.take() {
            if let Some(v) = interp.stack.data().last() {
                self.note(at, contract, key, B256::from(*v));
            }
        }
    }
}

/// A parity state diff of a transaction's `state` over `db` (the state before it), as
/// revm-inspectors writes it from later versions on (and erigon): an account absent before and
/// not empty after is added ("+"), not changed from zero; a deleted one is removed ("-").
pub fn populate_state_diff<'a, DB: DatabaseRef>(
    state_diff: &mut StateDiff,
    db: &DB,
    accounts: impl IntoIterator<Item = (&'a Address, &'a revm::state::Account)>,
) -> Result<(), DB::Error> {
    let code_of = |info: &AccountInfo| {
        info.code
            .as_ref()
            .map(|c| c.original_bytes())
            .or_else(|| match info.code_hash == KECCAK_EMPTY {
                true => None,
                false => db
                    .code_by_hash_ref(info.code_hash)
                    .ok()
                    .map(|c| c.original_bytes()),
            })
            .unwrap_or_default()
    };
    for (addr, changed) in accounts {
        let addr = *addr;
        let db_acc = db.basic_ref(addr)?;
        if changed.is_selfdestructed() && db_acc.is_none() {
            continue;
        }
        let existed = db_acc.is_some();
        let db_acc = db_acc.unwrap_or_default();
        let entry = state_diff.entry(addr).or_default();
        if changed.is_selfdestructed() {
            entry.balance = Delta::Removed(db_acc.balance);
            entry.nonce = Delta::Removed(U64::from(db_acc.nonce));
            entry.code = Delta::Removed(code_of(&db_acc));
            continue;
        }
        if !existed && (changed.is_created() || !changed.is_empty()) {
            entry.balance = Delta::Added(changed.info.balance);
            entry.nonce = Delta::Added(U64::from(changed.info.nonce));
            entry.code = Delta::Added(code_of(&changed.info));
            for (key, slot) in changed.storage.iter().filter(|(_, s)| s.is_changed()) {
                entry
                    .storage
                    .insert((*key).into(), Delta::Added(slot.present_value.into()));
            }
            continue;
        }
        if db_acc.code_hash != changed.info.code_hash {
            entry.code = Delta::changed(code_of(&db_acc), code_of(&changed.info));
        }
        for (key, slot) in changed.storage.iter().filter(|(_, s)| s.is_changed()) {
            entry.storage.insert(
                (*key).into(),
                Delta::changed(slot.original_value.into(), slot.present_value.into()),
            );
        }
        if entry.storage.is_empty() && db_acc == changed.info {
            state_diff.remove(&addr);
            continue;
        }
        entry.balance = match db_acc.balance == changed.info.balance {
            true => Delta::Unchanged,
            false => Delta::Changed(ChangedType {
                from: db_acc.balance,
                to: changed.info.balance,
            }),
        };
        entry.nonce = match db_acc.nonce == changed.info.nonce {
            true => Delta::Unchanged,
            false => Delta::Changed(ChangedType {
                from: U64::from(db_acc.nonce),
                to: U64::from(changed.info.nonce),
            }),
        };
    }
    Ok(())
}

/// What one request's traces have recorded so far (roughly, in bytes): every call frame's input
/// and output, logs, and the stack, memory and return data of each step the tracer writes.
pub struct Budget {
    used: std::cell::Cell<usize>,
    limit: usize,
}

impl Budget {
    pub fn new(limit: usize) -> Self {
        Self {
            used: std::cell::Cell::new(0),
            limit,
        }
    }

    pub fn add(&self, bytes: usize) {
        self.used.set(self.used.get().saturating_add(bytes));
    }

    pub fn over(&self) -> bool {
        self.used.get() > self.limit
    }
}

/// What a step the tracer writes holds.
#[derive(Clone, Copy, Default)]
pub struct StepCost {
    pub stack: bool,
    pub memory: bool,
    pub returndata: bool,
}

/// Counts what a tracer beside it records against the request's [`Budget`], and once that is
/// spent ends the run: every further step halts and every call or creation reverts unrun. The
/// run's trace is then refused as too large, so how it ended is never sent.
pub struct Guard<'a> {
    pub budget: &'a Budget,
    pub steps: Option<StepCost>,
}

impl<CTX> Inspector<CTX> for Guard<'_> {
    fn step(&mut self, interp: &mut Interpreter, _: &mut CTX) {
        if let Some(c) = self.steps {
            let mut b = 64;
            if c.stack {
                b += interp.stack.len() * 32;
            }
            if c.memory {
                b += interp.memory.size();
            }
            if c.returndata {
                b += interp.return_data.buffer().len();
            }
            self.budget.add(b);
        }
        if self.budget.over() {
            interp.halt(revm::interpreter::InstructionResult::OutOfGas);
        }
    }

    fn log(&mut self, _: &mut Interpreter, _: &mut CTX, log: alloy::primitives::Log) {
        self.budget
            .add(64 + log.data.data.len() + 32 * log.data.topics().len());
    }

    fn call(&mut self, _: &mut CTX, inputs: &mut CallInputs) -> Option<CallOutcome> {
        self.budget.add(256 + inputs.input.len());
        self.budget.over().then(|| {
            CallOutcome::new(
                revm::interpreter::InterpreterResult::new(
                    revm::interpreter::InstructionResult::Revert,
                    Default::default(),
                    revm::interpreter::Gas::new(inputs.gas_limit),
                ),
                inputs.return_memory_offset.clone(),
            )
        })
    }

    fn call_end(&mut self, _: &mut CTX, _: &CallInputs, outcome: &mut CallOutcome) {
        self.budget.add(outcome.result.output.len());
    }

    fn create(&mut self, _: &mut CTX, inputs: &mut CreateInputs) -> Option<CreateOutcome> {
        self.budget.add(256 + inputs.init_code.len());
        self.budget.over().then(|| {
            CreateOutcome::new(
                revm::interpreter::InterpreterResult::new(
                    revm::interpreter::InstructionResult::Revert,
                    Default::default(),
                    revm::interpreter::Gas::new(inputs.gas_limit),
                ),
                None,
            )
        })
    }

    fn create_end(&mut self, _: &mut CTX, _: &CreateInputs, outcome: &mut CreateOutcome) {
        self.budget.add(outcome.result.output.len());
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
    reason
}
