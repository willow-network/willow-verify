//! Ethereum's blocks run again and traced as the log RPC's trace methods run and trace them
//! (`state/trace.rs`, kept in step with it): the block's system calls as alloy-evm 0.21 (reth
//! v1.8) makes them, each transaction as a block's, EIP-161 between transactions, the traces
//! written by revm-inspectors v0.30 with geth's output where it differs ([`crate::output`]). A
//! call's trace (`debug_traceCall`, `trace_call`) is the same tracers over a run as `eth_call`
//! runs it ([`crate::call`]: reth's gas rules, `state_verify`'s `CallHandler`).

use crate::block::Block;
use crate::output::{self, Budget, FourBytes, Guard, Lookups, StepCost, StorageSteps};
use crate::request::Tracer;
use crate::state::{Pre, Unreadable};
use crate::{Stop, Traces, MAX_TRACE_BYTES, TOO_LARGE};
use alloy::rpc::types::trace::geth::{
    FourByteFrame, GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
    NoopFrame,
};
use alloy::rpc::types::trace::parity::{TraceResultsWithTransactionHash, TraceType};
use alloy::rpc::types::TransactionInfo;
use alloy_evm::block::SystemCaller;
use alloy_evm::eth::EthEvmBuilder;
use alloy_evm::{Evm, EvmEnv, FromRecoveredTx};
use alloy_primitives::U256;
use logset_blockproof::txs::Decoded;
use revm::context::result::{EVMError, ResultAndState};
use revm::context::{BlockEnv, CfgEnv, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::database::CacheDB;
use revm::inspector::NoOpInspector;
use revm::primitives::hardfork::SpecId;
use revm::{DatabaseCommit, Inspector};
use revm_inspectors::tracing::{MuxInspector, TracingInspector, TracingInspectorConfig};
use serde_json::Value;

/// Ethereum's forks activated by block number, then by time (Constantinople and Petersburg
/// activated together, as Petersburg). The log RPC's `state/call.rs` table.
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

/// The block's EVM environment, as consensus runs it.
pub fn evm_env(block: &Block) -> EvmEnv {
    let h = &block.header;
    let spec = spec(h.number, h.timestamp);
    let mut cfg = CfgEnv::new_with_spec(spec);
    cfg.chain_id = 1;
    let blobs = logset_blockproof::derive::BlobSchedule::builtin(1);
    let blob = h.excess_blob_gas.and_then(|excess| {
        let price = blobs.price(h.timestamp, excess)?;
        Some((excess, u128::try_from(price).unwrap_or(u128::MAX)))
    });
    let block_env = BlockEnv {
        number: U256::from(h.number),
        beneficiary: h.beneficiary,
        timestamp: U256::from(h.timestamp),
        gas_limit: h.gas_limit,
        basefee: h.base_fee_per_gas.unwrap_or(0),
        difficulty: h.difficulty,
        prevrandao: spec.is_enabled_in(SpecId::MERGE).then_some(h.mix_hash),
        blob_excess_gas_and_price: blob.filter(|_| spec.is_enabled_in(SpecId::CANCUN)).map(
            |(excess_blob_gas, blob_gasprice)| BlobExcessGasAndPrice {
                excess_blob_gas,
                blob_gasprice,
            },
        ),
    };
    EvmEnv {
        cfg_env: cfg,
        block_env,
    }
}

type Db<'a> = CacheDB<&'a Pre<'a>>;

/// `tx` run over `db` with `inspector`: as a block's transaction, or `as_call` as `eth_call` runs
/// it (no fee charged, reth's).
fn inspect<'b, DB, I>(
    db: &'b mut DB,
    env: &EvmEnv,
    tx: TxEnv,
    inspector: I,
    as_call: bool,
) -> Result<ResultAndState, EVMError<DB::Error>>
where
    DB: alloy_evm::Database,
    I: Inspector<alloy_evm::eth::EthEvmContext<&'b mut DB>>,
{
    if as_call {
        use revm::context::ContextSetters;
        use revm::inspector::InspectorHandler;
        use revm::{MainBuilder, MainContext};
        let mut evm = revm::Context::mainnet()
            .with_db(db)
            .with_block(env.block_env.clone())
            .with_cfg(env.cfg_env.clone())
            .build_mainnet_with_inspector(inspector);
        evm.ctx.set_tx(tx);
        let result = state_verify::call::CallHandler::<_, EVMError<DB::Error>, _>::default()
            .inspect_run(&mut evm)?;
        let state = revm::handler::ExecuteEvm::finalize(&mut evm);
        return Ok(ResultAndState::new(result, state));
    }
    let mut evm = EthEvmBuilder::new(db, env.clone())
        .activate_inspector(inspector)
        .build();
    evm.transact(tx)
}

/// One transaction (or `as_call`, a call) run under `tracer` over `db` (the state before it): its
/// trace, as JSON, and its outcome; refused as too large once the request's `budget` is spent.
pub(crate) fn traced<DB>(
    tracer: &Tracer,
    db: &mut DB,
    env: &EvmEnv,
    tx: TxEnv,
    info: TransactionInfo,
    budget: &Budget,
    as_call: bool,
) -> Result<(Value, ResultAndState), EVMError<<DB as revm::Database>::Error>>
where
    DB: alloy_evm::Database + revm::DatabaseRef<Error = <DB as revm::Database>::Error>,
{
    let out = traced_within(tracer, db, env, tx, info, budget, as_call);
    match budget.over() {
        true => Err(EVMError::Custom(TOO_LARGE.into())),
        false => out,
    }
}

fn traced_within<DB>(
    tracer: &Tracer,
    db: &mut DB,
    env: &EvmEnv,
    tx: TxEnv,
    info: TransactionInfo,
    budget: &Budget,
    as_call: bool,
) -> Result<(Value, ResultAndState), EVMError<<DB as revm::Database>::Error>>
where
    DB: alloy_evm::Database + revm::DatabaseRef<Error = <DB as revm::Database>::Error>,
{
    let gas_limit = tx.gas_limit;
    let custom = |m: String| EVMError::Custom(m);
    let json = |v: Result<Value, serde_json::Error>| v.map_err(|e| custom(e.to_string()));
    let bad = || custom("invalid tracer config".into());
    let spent = || match budget.over() {
        true => Err(EVMError::Custom(TOO_LARGE.into())),
        false => Ok(()),
    };
    let guard = |steps: Option<StepCost>| Guard { budget, steps };
    let lookups = Lookups::of_tx(&tx, env.block_env.beneficiary, &*db);
    let (trace, res): (Value, ResultAndState) = match tracer {
        Tracer::Flat => {
            let mut i = (
                TracingInspector::new(TracingInspectorConfig::default_parity()),
                guard(None),
            );
            let res = inspect(db, env, tx, &mut i, as_call)?;
            spent()?;
            let traces =
                i.0.into_parity_builder()
                    .into_localized_transaction_traces(info);
            (json(serde_json::to_value(traces))?, res)
        }
        Tracer::Replay(types) => {
            let steps = types
                .contains(&TraceType::VmTrace)
                .then_some(StepCost::default());
            let mut i = (
                TracingInspector::new(TracingInspectorConfig::from_parity_config(types)),
                guard(steps),
            );
            let res = inspect(db, env, tx, &mut i, as_call)?;
            spent()?;
            let mut out =
                i.0.into_parity_builder()
                    .into_trace_results_with_state(&res, types, &*db)
                    .map_err(EVMError::Database)?;
            if let Some(diff) = out.state_diff.as_mut() {
                diff.clear();
                output::populate_state_diff(diff, &*db, res.state.iter())
                    .map_err(EVMError::Database)?;
            }
            let out = match info.hash {
                Some(transaction_hash) => serde_json::to_value(TraceResultsWithTransactionHash {
                    transaction_hash,
                    full_trace: out,
                }),
                None => serde_json::to_value(out),
            };
            (json(out)?, res)
        }
        Tracer::Geth(opts) => {
            let GethDebugTracingOptions {
                config,
                tracer,
                tracer_config,
                ..
            } = opts.as_ref();
            match tracer {
                None => {
                    let steps = StepCost {
                        stack: !config.disable_stack.unwrap_or_default(),
                        memory: config.enable_memory.unwrap_or_default(),
                        returndata: config.enable_return_data.unwrap_or_default(),
                    };
                    let tracing =
                        TracingInspector::new(TracingInspectorConfig::from_geth_config(config));
                    let mut i = ((tracing, StorageSteps::new(budget)), guard(Some(steps)));
                    let res = inspect(db, env, tx, &mut i, as_call)?;
                    spent()?;
                    let ((mut tracing, storage), _) = i;
                    tracing.set_transaction_gas_limit(gas_limit);
                    let gas = res.result.gas_used();
                    let out = res.result.output().cloned().unwrap_or_default();
                    let frame = tracing.geth_builder().geth_traces(gas, out, *config);
                    let mut v = json(serde_json::to_value(frame))?;
                    storage.write(&mut v, config.is_storage_enabled());
                    (v, res)
                }
                Some(GethDebugTracerType::BuiltInTracer(t)) => match t {
                    GethDebugBuiltInTracerType::FourByteTracer => {
                        let mut i = (FourBytes::default(), guard(None));
                        let res = inspect(db, env, tx, &mut i, as_call)?;
                        spent()?;
                        (
                            json(serde_json::to_value(FourByteFrame::from(&i.0 .0)))?,
                            res,
                        )
                    }
                    GethDebugBuiltInTracerType::CallTracer => {
                        let cfg = tracer_config
                            .clone()
                            .into_call_config()
                            .map_err(|_| bad())?;
                        let tracing = TracingInspector::new(
                            TracingInspectorConfig::from_geth_call_config(&cfg),
                        );
                        let mut i = (tracing, guard(None));
                        let res = inspect(db, env, tx, &mut i, as_call)?;
                        spent()?;
                        i.0.set_transaction_gas_limit(gas_limit);
                        let frame =
                            i.0.geth_builder()
                                .geth_call_traces(cfg, res.result.gas_used());
                        let mut v = json(serde_json::to_value(frame))?;
                        output::geth_revert_reasons(&mut v);
                        (v, res)
                    }
                    GethDebugBuiltInTracerType::PreStateTracer => {
                        let cfg = tracer_config
                            .clone()
                            .into_pre_state_config()
                            .map_err(|_| bad())?;
                        let mut i = (lookups, guard(None));
                        let res = inspect(db, env, tx, &mut i, as_call)?;
                        spent()?;
                        let empty = output::include_empty(tracer_config);
                        let v = output::geth_prestate(&*db, &res.state, &i.0, &cfg, empty, budget)
                            .map_err(EVMError::Database)?;
                        spent()?;
                        (v, res)
                    }
                    GethDebugBuiltInTracerType::NoopTracer => {
                        let res = inspect(db, env, tx, guard(None), as_call)?;
                        (json(serde_json::to_value(NoopFrame::default()))?, res)
                    }
                    GethDebugBuiltInTracerType::MuxTracer => {
                        let cfg = tracer_config.clone().into_mux_config().map_err(|_| bad())?;
                        let mux = MuxInspector::try_from_config(cfg)
                            .map_err(|e| custom(e.to_string()))?;
                        let mut i = (mux, guard(Some(StepCost::default())));
                        let res = inspect(db, env, tx, &mut i, as_call)?;
                        spent()?;
                        let frame =
                            i.0.try_into_mux_frame(&res, &*db, info)
                                .map_err(EVMError::Database)?;
                        (json(serde_json::to_value(frame))?, res)
                    }
                    GethDebugBuiltInTracerType::FlatCallTracer => {
                        let cfg = tracer_config
                            .clone()
                            .into_flat_call_config()
                            .map_err(|_| bad())?;
                        let tracing = TracingInspector::new(
                            TracingInspectorConfig::from_flat_call_config(&cfg),
                        );
                        let mut i = (tracing, guard(None));
                        let res = inspect(db, env, tx, &mut i, as_call)?;
                        spent()?;
                        let frame =
                            i.0.with_transaction_gas_limit(gas_limit)
                                .into_parity_builder()
                                .into_localized_transaction_traces(info);
                        (json(serde_json::to_value(frame))?, res)
                    }
                    _ => return Err(custom("unsupported tracer".into())),
                },
                Some(GethDebugTracerType::JsTracer(_)) => {
                    return Err(custom("JavaScript tracers are not run here".into()))
                }
            }
        }
    };
    Ok((trace, res))
}

/// A transaction's state laid over the block's so far, as a block's state is kept: from Spurious
/// Dragon on, an account it touched and left empty is gone (EIP-161).
fn commit(db: &mut Db<'_>, mut state: revm::state::EvmState, env: &EvmEnv) {
    if env.cfg_env.spec.is_enabled_in(SpecId::SPURIOUS_DRAGON) {
        for account in state.values_mut() {
            if account.is_touched() && account.is_empty() && !account.is_selfdestructed() {
                account.mark_selfdestruct();
            }
        }
    }
    db.commit(state);
}

/// `block` run over `pre` (its parent's state) through transaction `last`, the transactions
/// `wanted` says traced under `tracer`; every one checked against its receipt.
pub fn run_block(
    block: &Block,
    pre: &Pre<'_>,
    tracer: &Tracer,
    last: usize,
    wanted: &dyn Fn(usize) -> bool,
) -> Result<Traces, Stop> {
    let mut db: Db<'_> = CacheDB::new(pre);
    let env = evm_env(block);
    let header = alloy::consensus::Header {
        parent_hash: block.header.parent_hash,
        parent_beacon_block_root: block.header.parent_beacon_block_root,
        number: block.header.number,
        timestamp: block.header.timestamp,
        ..Default::default()
    };
    {
        let mut evm = EthEvmBuilder::new(&mut db, env.clone()).build();
        let out = SystemCaller::new(alloy_hardforks::EthereumChainHardforks::mainnet())
            .apply_pre_execution_changes(&header, &mut evm);
        drop(evm);
        if pre.has_missing() {
            return Err(Stop::Failed(
                "the block's system calls read state not held".into(),
            ));
        }
        out.map_err(|e| Stop::Failed(format!("the block's system calls: {e}")))?;
    }
    let budget = Budget::new(MAX_TRACE_BYTES);
    let mut out = Vec::new();
    let mut cumulative = 0u64;
    for (i, tx) in block.txs.iter().enumerate().take(last.saturating_add(1)) {
        let Decoded::Eth(envelope) = &tx.decoded else {
            return Err(Stop::Failed(format!("transaction {i} is not Ethereum's")));
        };
        let sender = tx
            .sender
            .ok_or_else(|| Stop::Failed(format!("transaction {i}: its sender not recovered")))?;
        let tx_env = TxEnv::from_recovered_tx(envelope.as_ref(), sender);
        let info = TransactionInfo {
            hash: Some(tx.hash),
            index: Some(i as u64),
            block_hash: Some(block.hash),
            block_number: Some(block.number),
            base_fee: block.header.base_fee_per_gas,
        };
        // Once something is missing the rest is run only to find what else is.
        let trace = wanted(i) && !pre.has_missing();
        let ran = match trace {
            true => traced(tracer, &mut db, &env, tx_env, info, &budget, false)
                .map(|(v, r)| (Some(v), r)),
            false => inspect(&mut db, &env, tx_env, NoOpInspector, false).map(|r| (None, r)),
        }
        .map_err(|e| match e {
            EVMError::Database(Unreadable::BlockHash(n)) => Stop::NoBlockHash(n),
            EVMError::Custom(m) if m == TOO_LARGE => Stop::TooLarge,
            e => Stop::Failed(e.to_string()),
        });
        let (v, res) = ran?;
        if !pre.has_missing() {
            cumulative += res.result.gas_used();
            let r = &block.receipts[i];
            let ok = r.success.is_none_or(|s| s == res.result.is_success())
                && res.result.logs() == r.logs.as_slice();
            if cumulative != r.cumulative_gas || !ok {
                return Err(Stop::Disagrees(
                    i,
                    format!(
                        "used {} gas in all ({}), the receipt says {} ({:?})",
                        cumulative,
                        res.result.is_success(),
                        r.cumulative_gas,
                        r.success
                    ),
                ));
            }
        }
        if let Some(v) = v {
            out.push((i, v));
        }
        commit(&mut db, res.state, &env);
    }
    Ok(out)
}
