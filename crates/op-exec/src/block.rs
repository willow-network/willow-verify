//! Unichain blocks run again over their parent's state (as the caller of this crate proves it),
//! each transaction as op-reth runs it in the block, traced as op-geth writes its traces.

use crate::{spec, Block, Db, State, Unreadable, UNICHAIN};
use alloy_primitives::{Address, Bytes, Log, B256, U256};
use alloy_rpc_types_eth_2::TransactionInfo;
use alloy_rpc_types_trace_2::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions, GethTrace, NoopFrame,
};
use op_revm::{DefaultOp, OpBuilder, OpSpecId, OpTransaction, OpTransactionError};
use revm::context::result::{EVMError, ExecutionResult};
use revm::context::{BlockEnv, CfgEnv, Context, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::context_interface::either::Either;
use revm::context_interface::transaction::{AccessList, AccessListItem, SignedAuthorization};
use revm::database::CacheDB;
use revm::handler::SystemCallEvm;
use revm::inspector::InspectEvm;
use revm::primitives::TxKind;
use revm::state::EvmState;
use revm::{Database, DatabaseCommit, ExecuteEvm};
use revm_inspectors::tracing::{
    TraceLimitBehavior, TraceLimits, TracingInspector, TracingInspectorConfig,
};
use serde_json::Value;

/// EIP-4788's and EIP-2935's contracts, which a block calls before its transactions.
const BEACON_ROOTS: Address =
    alloy_primitives::address!("0x000F3df6D732807Ef1319fB7B8bB8522d0Beac02");
const HISTORY: Address = alloy_primitives::address!("0x0000F90827F1C53a10cb7A02335B175320002935");
const SYSTEM: Address = alloy_primitives::address!("0xfffffffffffffffffffffffffffffffffffffffe");

/// How a transaction's trace is written.
#[derive(Clone, Debug)]
pub enum Tracer {
    /// Geth's built-in tracers (`debug_*`), from the request's options as given.
    Geth(Value),
    /// Parity's flat traces with their transaction's place (`trace_block`, `trace_transaction`).
    Flat,
}

/// The tracers run here; others are refused by the caller of this crate.
pub fn served(options: &Value) -> Result<(), String> {
    let o: GethDebugTracingOptions =
        serde_json::from_value(options.clone()).map_err(|e| e.to_string())?;
    match &o.tracer {
        Some(GethDebugTracerType::BuiltInTracer(
            GethDebugBuiltInTracerType::CallTracer
            | GethDebugBuiltInTracerType::PreStateTracer
            | GethDebugBuiltInTracerType::NoopTracer,
        )) => Ok(()),
        Some(t) => Err(format!("{t:?}")),
        None => Err("the struct logger".into()),
    }
}

/// A transaction of the block, as the block holds it.
#[derive(Clone, Debug, Default)]
pub struct Tx {
    pub ty: u8,
    pub hash: B256,
    pub from: Address,
    pub to: Option<Address>,
    pub nonce: u64,
    pub gas_limit: u64,
    /// A legacy or 2930 price, or a 1559 fee cap.
    pub gas_price: u128,
    pub priority_fee: Option<u128>,
    pub value: U256,
    pub data: Bytes,
    pub chain_id: Option<u64>,
    pub access_list: Vec<(Address, Vec<B256>)>,
    pub authorizations: Vec<SignedAuthorization>,
    /// Its EIP-2718 encoding (the L1 data fee is counted from it).
    pub encoded: Bytes,
    pub deposit: Option<Deposit>,
}

/// A deposit's own fields.
#[derive(Clone, Debug, Default)]
pub struct Deposit {
    pub source_hash: B256,
    pub mint: Option<u128>,
    pub is_system: bool,
}

/// What a transaction came to, to be checked against its receipt.
#[derive(Clone, Debug)]
pub struct Ran {
    pub gas_used: u64,
    pub success: bool,
    pub logs: Vec<Log>,
    /// A deposit's sender's nonce before it ran (its receipt's `depositNonce`).
    pub deposit_nonce: Option<u64>,
    pub trace: Option<Value>,
}

/// Why a block's run stopped.
#[derive(Debug)]
pub enum Stop {
    NoBlockHash(u64),
    /// The traces record more than the budget allows.
    TooLarge,
    Failed(String),
}

/// `block` run again over `state` (its parent's) through transaction `last`: the system calls
/// before its transactions (EIP-4788, and EIP-2935 from Isthmus), then each transaction, those
/// `wanted` says traced under `tracer`. Traces are bounded at `budget` bytes in all.
#[allow(clippy::too_many_arguments)]
pub fn run_block(
    state: &dyn State,
    block: &Block,
    hash: B256,
    parent_hash: B256,
    beacon_root: Option<B256>,
    txs: &[Tx],
    last: usize,
    wanted: &dyn Fn(usize) -> bool,
    tracer: &Tracer,
    budget: usize,
) -> Result<Vec<Ran>, Stop> {
    let spec = spec(block.timestamp);
    let cfg = CfgEnv::new()
        .with_chain_id(UNICHAIN)
        .with_spec_and_mainnet_gas_params(spec);
    let block_env = BlockEnv {
        number: U256::from(block.number),
        beneficiary: block.beneficiary,
        timestamp: U256::from(block.timestamp),
        gas_limit: block.gas_limit,
        basefee: block.base_fee,
        difficulty: U256::ZERO,
        prevrandao: Some(block.mix_hash),
        blob_excess_gas_and_price: Some(BlobExcessGasAndPrice {
            excess_blob_gas: 0,
            blob_gasprice: 1,
        }),
        slot_num: 0,
    };
    let mut db = CacheDB::new(Db(state));
    let failed = |e: EVMError<Unreadable, OpTransactionError>| match e {
        EVMError::Database(Unreadable::BlockHash(n)) => Stop::NoBlockHash(n),
        EVMError::Custom(m) if m.contains("budget") => Stop::TooLarge,
        e => Stop::Failed(e.to_string()),
    };
    // Before the transactions: the parent beacon root, and from Isthmus the parent's hash.
    let mut system = Vec::new();
    if spec.is_enabled_in(OpSpecId::ECOTONE) && block.number > 0 {
        let root = beacon_root.ok_or(Stop::Failed("no parent beacon block root".into()))?;
        system.push((BEACON_ROOTS, Bytes::from(root.0)));
    }
    if spec.is_enabled_in(OpSpecId::ISTHMUS) && block.number > 0 {
        system.push((HISTORY, Bytes::from(parent_hash.0)));
    }
    for (contract, data) in system {
        let mut evm = Context::op()
            .with_db(&mut db)
            .with_block(block_env.clone())
            .with_cfg(cfg.clone())
            .build_op();
        let out = evm.system_call_with_caller(SYSTEM, contract, data);
        drop(evm);
        let out = out.map_err(failed)?;
        commit(&mut db, out.state);
    }
    let mut left = budget;
    let mut out = Vec::with_capacity(txs.len().min(last.saturating_add(1)));
    for (i, tx) in txs.iter().enumerate().take(last.saturating_add(1)) {
        let deposit_nonce = match &tx.deposit {
            Some(_) => Some(
                db.basic(tx.from)
                    .map_err(|e| Stop::Failed(e.to_string()))?
                    .map_or(0, |a| a.nonce),
            ),
            None => None,
        };
        let op_tx = op_tx(tx);
        let (res, trace) = match wanted(i) {
            false => {
                let mut evm = Context::op()
                    .with_db(&mut db)
                    .with_block(block_env.clone())
                    .with_cfg(cfg.clone())
                    .build_op();
                let r = evm.transact(op_tx);
                drop(evm);
                (r.map_err(failed)?, None)
            }
            true => {
                let (config, geth) = match tracer {
                    Tracer::Flat => (TracingInspectorConfig::default_parity(), None),
                    Tracer::Geth(o) => {
                        let o: GethDebugTracingOptions = serde_json::from_value(o.clone())
                            .map_err(|e| Stop::Failed(e.to_string()))?;
                        (geth_config(&o), Some(o))
                    }
                };
                let limits = TraceLimits::default()
                    .set_max_recorded_bytes(Some(left))
                    .set_behavior(TraceLimitBehavior::Halt);
                let inspector = TracingInspector::new(config).with_limits(limits);
                let mut evm = Context::op()
                    .with_db(&mut db)
                    .with_block(block_env.clone())
                    .with_cfg(cfg.clone())
                    .build_op_with_inspector(inspector);
                let r = evm.inspect_tx(op_tx);
                let inspector = std::mem::take(&mut evm.0.inspector);
                drop(evm);
                if inspector.limit_exceeded() {
                    return Err(Stop::TooLarge);
                }
                let res = r.map_err(failed)?;
                let recorded = inspector.recorded_bytes();
                let info = TransactionInfo {
                    hash: Some(tx.hash),
                    index: Some(i as u64),
                    block_hash: Some(hash),
                    block_number: Some(block.number),
                    base_fee: Some(block.base_fee),
                    block_timestamp: Some(block.timestamp),
                };
                let v = match geth {
                    None => serde_json::to_value(
                        inspector
                            .into_parity_builder()
                            .into_localized_transaction_traces(info),
                    ),
                    Some(o) => {
                        let t = geth_trace(&o, &inspector, &res, &db, tx.gas_limit)?;
                        serde_json::to_value(t)
                    }
                }
                .map_err(|e| Stop::Failed(e.to_string()))?;
                let size = recorded.max(approx_size(&v));
                left = left.checked_sub(size).ok_or(Stop::TooLarge)?;
                (res, Some(v))
            }
        };
        out.push(Ran {
            gas_used: res.result.tx_gas_used(),
            success: res.result.is_success(),
            logs: res.result.logs().to_vec(),
            deposit_nonce,
            trace,
        });
        commit(&mut db, res.state);
    }
    Ok(out)
}

/// A transaction as op-revm runs it in a block.
fn op_tx(tx: &Tx) -> OpTransaction<TxEnv> {
    let base = TxEnv {
        tx_type: tx.ty,
        caller: tx.from,
        gas_limit: tx.gas_limit,
        gas_price: tx.gas_price,
        gas_priority_fee: tx.priority_fee,
        kind: tx.to.map_or(TxKind::Create, TxKind::Call),
        value: tx.value,
        data: tx.data.clone(),
        nonce: tx.nonce,
        chain_id: tx.chain_id,
        access_list: AccessList(
            tx.access_list
                .iter()
                .map(|(address, keys)| AccessListItem {
                    address: *address,
                    storage_keys: keys.clone(),
                })
                .collect(),
        ),
        authorization_list: tx
            .authorizations
            .iter()
            .cloned()
            .map(Either::Left)
            .collect(),
        ..Default::default()
    };
    let mut op = OpTransaction {
        base,
        enveloped_tx: Some(tx.encoded.clone()),
        deposit: Default::default(),
    };
    if let Some(d) = &tx.deposit {
        op.deposit.source_hash = d.source_hash;
        op.deposit.mint = d.mint;
        op.deposit.is_system_transaction = d.is_system;
    }
    op
}

/// What a geth tracer records as the transaction runs.
fn geth_config(o: &GethDebugTracingOptions) -> TracingInspectorConfig {
    match &o.tracer {
        Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer)) => {
            let c = o
                .tracer_config
                .clone()
                .into_call_config()
                .unwrap_or_default();
            TracingInspectorConfig::from_geth_call_config(&c)
        }
        Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::PreStateTracer)) => {
            TracingInspectorConfig::default_geth().set_state_diffs(true)
        }
        _ => TracingInspectorConfig::none(),
    }
}

/// A geth tracer's output for the transaction just run (`db` still its state before).
fn geth_trace<HR: revm::context::result::HaltReasonTr>(
    o: &GethDebugTracingOptions,
    inspector: &TracingInspector,
    res: &revm::context::result::ExecResultAndState<ExecutionResult<HR>, EvmState>,
    db: &CacheDB<Db<'_>>,
    gas_limit: u64,
) -> Result<GethTrace, Stop> {
    let builder = inspector.geth_builder();
    Ok(match &o.tracer {
        Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::CallTracer)) => {
            let c = o
                .tracer_config
                .clone()
                .into_call_config()
                .unwrap_or_default();
            {
                let mut frame = builder.geth_call_traces(c, res.result.tx_gas_used());
                // The transaction's gas, as geth writes it, not what was left after intrinsic gas.
                frame.gas = U256::from(gas_limit);
                frame.into()
            }
        }
        Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::PreStateTracer)) => {
            let c = o
                .tracer_config
                .clone()
                .into_pre_state_config()
                .unwrap_or_default();
            let r = revm::context::result::ResultAndState {
                result: res.result.clone(),
                state: res.state.clone(),
            };
            builder
                .geth_prestate_traces(&r, &c, db)
                .map_err(|e| Stop::Failed(e.to_string()))?
                .into()
        }
        _ => NoopFrame::default().into(),
    })
}

/// A run's changes kept for the next transaction, accounts it touched and left empty removed
/// (EIP-161).
fn commit(db: &mut CacheDB<Db<'_>>, mut state: EvmState) {
    for account in state.values_mut() {
        if account.is_touched() && account.is_empty() && !account.is_selfdestructed() {
            account.mark_selfdestruct();
        }
    }
    db.commit(state);
}

/// About how many bytes `v` takes written out.
fn approx_size(v: &Value) -> usize {
    match v {
        Value::String(s) => s.len() + 2,
        Value::Array(a) => 2 + a.iter().map(approx_size).sum::<usize>() + a.len(),
        Value::Object(o) => {
            2 + o
                .iter()
                .map(|(k, v)| k.len() + 4 + approx_size(v))
                .sum::<usize>()
        }
        _ => 8,
    }
}
