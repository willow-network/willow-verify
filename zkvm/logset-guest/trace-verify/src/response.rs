


use crate::block::Block;
use crate::Traces;

use alloy::rpc::types::trace::geth::{GethTrace, TraceResult};

use serde_json::Value;


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



























