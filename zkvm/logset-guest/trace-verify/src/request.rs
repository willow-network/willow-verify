//! `willow_getTraceProof`'s own params, `[method, params]`: the standard trace method asked for
//! and its params, read as the server's trace methods read them. Only what a proof covers is
//! accepted: a block's or a transaction's traces under the tracers the chain's trace methods run,
//! and on Ethereum a call's (`debug_traceCall`, `trace_call`, read as `eth_call` reads its call
//! and overrides: `state_verify::call_request`).

use crate::block::Block;
use crate::{Failure, Kind, ETHEREUM, UNICHAIN};
use alloy::rpc::types::trace::geth::{
    GethDebugBuiltInTracerType, GethDebugTracerType, GethDebugTracingOptions,
};
use alloy::rpc::types::trace::parity::TraceType;
use alloy_primitives::map::HashSet;
use alloy_primitives::B256;
use serde_json::{json, Value};

/// The trace methods a proof covers.
pub const PROVEN: [&str; 9] = [
    "debug_traceTransaction",
    "debug_traceBlockByNumber",
    "debug_traceBlockByHash",
    "trace_block",
    "trace_transaction",
    "trace_replayBlockTransactions",
    "trace_replayTransaction",
    "debug_traceCall",
    "trace_call",
];

/// How a transaction's trace is written.
#[derive(Clone, Debug, PartialEq)]
pub enum Tracer {
    /// Geth's tracers (`debug_*`).
    Geth(Box<GethDebugTracingOptions>),
    /// Parity's flat traces with their transaction's place (`trace_block`, `trace_transaction`).
    Flat,
    /// Parity's replay of the kinds asked for (`trace_replay*`).
    Replay(HashSet<TraceType>),
}

/// A block as a request names it (number, tag, hash, or an EIP-1898 object).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum At {
    Number(u64),
    Hash(B256),
    /// `latest`, `pending`, `safe`, `finalized`: any block the server resolved it to (the client
    /// decides whether that block is the one the tag means).
    Tag,
}

impl At {
    /// `v` as geth reads a block parameter; absent or null is `latest`.
    pub fn parse(v: &Value) -> Result<At, Failure> {
        let bad = || invalid(format!("not a block: {v}"));
        match v {
            Value::Null => Ok(At::Tag),
            Value::String(s) => match s.as_str() {
                "latest" | "pending" | "safe" | "finalized" => Ok(At::Tag),
                "earliest" => Ok(At::Number(0)),
                h if h.len() == 66 => hash(v).map(At::Hash).map_err(|_| bad()),
                _ => quantity(v).map(At::Number).map_err(|_| bad()),
            },
            Value::Object(o) => match (o.get("blockHash"), o.get("blockNumber")) {
                (Some(h @ Value::String(s)), None) if s.len() == 66 => Self::parse(h),
                (None, Some(n @ Value::String(_))) => Self::parse(n),
                _ => Err(bad()),
            },
            _ => Err(bad()),
        }
    }

    /// Whether `block` is the block this names.
    pub fn check(&self, block: &Block) -> Result<(), Failure> {
        match *self {
            At::Number(n) if n != block.number => Err(Failure::new(
                Kind::WrongBlock,
                format!("the proof is for block {}, not {n}", block.number),
            )),
            At::Hash(h) if h != block.hash => Err(Failure::new(
                Kind::WrongBlock,
                format!("the proof is for block {}, not {h}", block.hash),
            )),
            _ => Ok(()),
        }
    }
}

/// A trace request, parsed.
#[derive(Clone, Debug, PartialEq)]
pub enum Job {
    /// Every transaction of the block `at` names.
    Block { at: At, tracer: Tracer },
    /// One transaction, by hash.
    Tx { hash: B256, tracer: Tracer },
    /// A call at a block, as `eth_call` runs it, under the caller's overrides.
    Call {
        call: Box<state_verify::CallRequest>,
        tracer: Tracer,
    },
}

impl Job {
    pub fn tracer(&self) -> &Tracer {
        match self {
            Job::Block { tracer, .. } | Job::Tx { tracer, .. } | Job::Call { tracer, .. } => tracer,
        }
    }
}

/// `willow_getTraceProof`'s params, read.
#[derive(Clone, Debug, PartialEq)]
pub struct Asked {
    /// The standard method, and its params as asked.
    pub method: String,
    pub params: Value,
    pub job: Job,
}

fn invalid(m: impl Into<String>) -> Failure {
    Failure::new(Kind::InvalidRequest, m)
}

fn unsupported(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Unsupported, m)
}

/// `0x` hex data.
pub fn data(v: &Value) -> Result<Vec<u8>, String> {
    let s = v
        .as_str()
        .ok_or_else(|| format!("expected hex data, not {v}"))?;
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("expected 0x-prefixed hex, not {s}"))?;
    hex::decode(h).map_err(|e| format!("{s}: {e}"))
}

fn hash(v: &Value) -> Result<B256, String> {
    let b = data(v)?;
    (b.len() == 32)
        .then(|| B256::from_slice(&b))
        .ok_or_else(|| "expected 32 bytes".to_string())
}

/// A `0x` quantity that fits in 64 bits.
pub fn quantity(v: &Value) -> Result<u64, String> {
    let s = v
        .as_str()
        .ok_or_else(|| format!("expected a quantity, not {v}"))?;
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("expected 0x-prefixed hex, not {s}"))?;
    if h.is_empty() || h.len() > 16 {
        return Err(format!("not a 64-bit quantity: {s}"));
    }
    u64::from_str_radix(h, 16).map_err(|e| format!("{s}: {e}"))
}

fn geth_options(v: &Value) -> Result<GethDebugTracingOptions, Failure> {
    let opts: GethDebugTracingOptions = match v {
        Value::Null => Default::default(),
        v => serde_json::from_value(v.clone())
            .map_err(|e| invalid(format!("invalid tracing options: {e}")))?,
    };
    match &opts.tracer {
        Some(GethDebugTracerType::JsTracer(_)) => {
            Err(unsupported("JavaScript tracers are not run here"))
        }
        Some(GethDebugTracerType::BuiltInTracer(GethDebugBuiltInTracerType::Erc7562Tracer)) => {
            Err(unsupported("the erc7562Tracer is not run here"))
        }
        _ => Ok(opts),
    }
}

fn trace_types(v: &Value) -> Result<HashSet<TraceType>, Failure> {
    serde_json::from_value(v.clone()).map_err(|e| invalid(format!("invalid trace types: {e}")))
}

fn tx_hash(v: &Value) -> Result<B256, Failure> {
    hash(v).map_err(|e| invalid(format!("expected a 32-byte transaction hash: {e}")))
}

/// `params` (`[method, params]`) read as a trace proof request on chain `chain_id`.
pub fn parse(chain_id: u64, params: &Value) -> Result<Asked, Failure> {
    if chain_id != ETHEREUM && chain_id != UNICHAIN {
        return Err(unsupported(format!(
            "traces are proven on Ethereum and Unichain, not chain {chain_id}"
        )));
    }
    let list = params
        .as_array()
        .ok_or_else(|| invalid("expected [method, params]"))?;
    let (method, inner) = match list.as_slice() {
        [Value::String(m)] => (m.as_str(), json!([])),
        [Value::String(m), p @ Value::Array(_)] => (m.as_str(), p.clone()),
        [Value::String(m), Value::Null] => (m.as_str(), json!([])),
        _ => return Err(invalid("expected [method, params]")),
    };
    let p = |i: usize| inner.get(i).unwrap_or(&Value::Null);
    let job = match method {
        "debug_traceBlockByNumber" | "debug_traceBlockByHash" => Job::Block {
            at: At::parse(p(0))?,
            tracer: Tracer::Geth(Box::new(geth_options(p(1))?)),
        },
        "debug_traceTransaction" => Job::Tx {
            hash: tx_hash(p(0))?,
            tracer: Tracer::Geth(Box::new(geth_options(p(1))?)),
        },
        "trace_block" => Job::Block {
            at: At::parse(p(0))?,
            tracer: Tracer::Flat,
        },
        "trace_transaction" => Job::Tx {
            hash: tx_hash(p(0))?,
            tracer: Tracer::Flat,
        },
        "trace_replayBlockTransactions" => Job::Block {
            at: At::parse(p(0))?,
            tracer: Tracer::Replay(trace_types(p(1))?),
        },
        "trace_replayTransaction" => Job::Tx {
            hash: tx_hash(p(0))?,
            tracer: Tracer::Replay(trace_types(p(1))?),
        },
        "debug_traceCall" | "trace_call" if chain_id != ETHEREUM => {
            return Err(unsupported(format!(
                "{method} is not run on chain {chain_id}: calls' traces are run on Ethereum"
            )))
        }
        "debug_traceCall" => {
            // Its options hold the call's overrides and its place in a block beside the tracer's.
            let mut opts = p(2).clone();
            let mut take = |k: &str| {
                opts.as_object_mut()
                    .and_then(|o| o.remove(k))
                    .unwrap_or(Value::Null)
            };
            let (state, block, index) = (
                take("stateOverrides"),
                take("blockOverrides"),
                take("txIndex"),
            );
            if !index.is_null() {
                return Err(unsupported(
                    "debug_traceCall at a transaction's place in its block is not run here",
                ));
            }
            Job::Call {
                call: Box::new(call_of(chain_id, &json!([p(0), p(1), state, block]))?),
                tracer: Tracer::Geth(Box::new(geth_options(&opts)?)),
            }
        }
        "trace_call" => Job::Call {
            call: Box::new(call_of(chain_id, &json!([p(0), p(2), p(3), p(4)]))?),
            tracer: Tracer::Replay(trace_types(p(1))?),
        },
        "trace_filter" => {
            return Err(unsupported(
                "trace_filter is proven block by block: ask for trace_block of each block",
            ))
        }
        m => return Err(invalid(format!("{m} is not a trace method"))),
    };
    if chain_id == UNICHAIN {
        op_served(job.tracer())?;
    }
    Ok(Asked {
        method: method.to_string(),
        params: inner,
        job,
    })
}

/// A call's `[call, block, stateOverrides, blockOverrides]`, read as `eth_call` reads them.
fn call_of(chain_id: u64, params: &Value) -> Result<state_verify::CallRequest, Failure> {
    state_verify::call_request(chain_id, params).map_err(|f| match f.kind {
        state_verify::Kind::Unsupported => unsupported(f.message),
        _ => invalid(f.message),
    })
}

/// Whether `tracer` is run on Unichain: geth's call, prestate and noop tracers, and parity's flat
/// traces (the log RPC's Unichain trace methods refuse the rest).
fn op_served(tracer: &Tracer) -> Result<(), Failure> {
    match tracer {
        Tracer::Flat => Ok(()),
        Tracer::Geth(o) => {
            let v = serde_json::to_value(o).map_err(|e| invalid(e.to_string()))?;
            willow_op_exec::block::served(&v)
                .map_err(|t| unsupported(format!("{t} traces are not run on Unichain")))
        }
        Tracer::Replay(_) => Err(unsupported("trace replays are not run on Unichain")),
    }
}
