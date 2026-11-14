//! A block's traces as the standard trace method's `result` writes them, and `trace_filter`'s over
//! block traces: as the log RPC's trace methods write them (`state/trace.rs`).

use crate::block::Block;
use crate::Traces;
use alloy::rpc::types::trace::filter::TraceFilter;
use alloy::rpc::types::trace::geth::{GethTrace, TraceResult};
use alloy::rpc::types::trace::parity::LocalizedTransactionTrace;
use serde_json::{json, Value};

/// A block's traces as `method` writes them, from each traced transaction's.
pub fn block_response(method: &str, block: &Block, traces: Traces) -> Value {
    match method {
        "trace_block" | "trace_filter" | "trace_transaction" => Value::Array(
            traces
                .into_iter()
                .flat_map(|(_, v)| match v {
                    Value::Array(a) => a,
                    v => vec![v],
                })
                .collect(),
        ),
        "trace_replayBlockTransactions" => {
            Value::Array(traces.into_iter().map(|(_, v)| v).collect())
        }
        "debug_traceTransaction" | "trace_replayTransaction" => {
            traces.into_iter().next().map_or(Value::Null, |(_, v)| v)
        }
        _ => Value::Array(
            traces
                .into_iter()
                .map(|(i, v)| {
                    let r: TraceResult = TraceResult::Success {
                        result: GethTrace::JS(v),
                        tx_hash: Some(block.txs[i].hash),
                    };
                    serde_json::to_value(r).unwrap_or(Value::Null)
                })
                .collect(),
        ),
    }
}

/// `trace_filter`'s traces of the blocks run (each `trace_block`'s result, in block order): those
/// its filter matches, `after` skipped, at most `count`.
pub fn filtered(filter: &TraceFilter, blocks: Vec<Value>) -> Value {
    let matcher = filter.matcher();
    let mut all: Vec<Value> = blocks
        .into_iter()
        .flat_map(|v| match v {
            Value::Array(a) => a,
            _ => vec![],
        })
        .filter(|t| {
            serde_json::from_value::<LocalizedTransactionTrace>(t.clone())
                .is_ok_and(|l| matcher.matches(&l.trace))
        })
        .collect();
    if let Some(after) = filter.after.map(|a| a as usize) {
        if after >= all.len() {
            return json!([]);
        }
        all.drain(..after);
    }
    if let Some(count) = filter.count {
        all.truncate(count as usize);
    }
    Value::Array(all)
}
