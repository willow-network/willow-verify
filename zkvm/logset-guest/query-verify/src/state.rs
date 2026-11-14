//! `--state`, `--call`, `--estimate` and `--trace`: account and storage proofs against the block's
//! state root; `eth_call` / `eth_estimateGas` run here over proven state; and every trace method a
//! proof covers (a block's or a transaction's traces, the block run again here; a call's, the call
//! run here over proven state), checked by the verify page's own state module
//! (`willow_state_verify_wasm`, natively, on state-verify and trace-verify).
//!
//! 1. Asks Willow for `willow_getStateProof [address, slots, block, {"code"}]`,
//!    `willow_getCallProof [call, block, stateOverrides?, blockOverrides?]`,
//!    `willow_getEstimateGasProof [call, block, stateOverrides?, blockOverrides?]` or
//!    `willow_getTraceProof [method, params]`.
//! 2. Verifies it as the response to that request: the header hashes to the block hash, the
//!    account and slots are under its state root (absence included), a call or estimate is re-run
//!    here over the proven state only (under the caller's overrides, which are the caller's own:
//!    never proven, never sent back), on Ethereum, Unichain and Robinhood Chain as each chain runs
//!    it; a trace is written here by the client's own run.
//! 3. Anchors the block.
//! 4. `--compare-rpc`: that endpoint's standard methods at the same block (by hash).

use crate::check::{counted, problem, Ctx, Named};
use serde_json::{json, Value};
use willow_state_verify_wasm::{self as sw, StateKind};

/// What to prove, from the command line.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    State {
        address: String,
        slots: Vec<String>,
        code: bool,
        at: String,
    },
    Call {
        call: Value,
        overrides: Option<Value>,
        at: String,
    },
    Estimate {
        call: Value,
        overrides: Option<Value>,
        at: String,
    },
    /// A trace method and its params (`[method, params]`), its block (a block's or a call's
    /// trace) named in its params.
    Trace { method: String, params: Value },
}

/// Where a trace method's params name its block, if they do.
fn trace_block_at(method: &str) -> Option<usize> {
    match method {
        "debug_traceBlockByNumber" | "trace_block" | "trace_replayBlockTransactions" => Some(0),
        "debug_traceCall" => Some(1),
        "trace_call" => Some(2),
        _ => None,
    }
}

/// `--trace`'s JSON: `[method, params]`.
pub fn parse_trace(v: Value) -> Result<(String, Value), String> {
    match v {
        Value::Array(a) => match a.as_slice() {
            [Value::String(m), p @ Value::Array(_)] => Ok((m.clone(), p.clone())),
            [Value::String(m)] => Ok((m.clone(), json!([]))),
            _ => Err("--trace: expected [method, params]".into()),
        },
        _ => Err("--trace: expected [method, params]".into()),
    }
}

/// Storage keys: `k1,k2,…`.
pub fn parse_slots(s: &str) -> Vec<String> {
    s.split(',')
        .map(str::trim)
        .filter(|k| !k.is_empty())
        .map(str::to_string)
        .collect()
}

fn kind_of(ask: &Ask) -> StateKind {
    match ask {
        Ask::State { .. } => StateKind::State,
        Ask::Call { .. } => StateKind::Call,
        Ask::Estimate { .. } => StateKind::Estimate,
        Ask::Trace { .. } => StateKind::Trace,
    }
}

/// Fetch, verify, anchor, compare: the report, and whether everything checked out.
pub fn run(ctx: &mut Ctx, ask: Ask) -> Result<(Value, bool), String> {
    let kind = kind_of(&ask);
    let method = kind.method();
    // The block the request names: `--at`'s, or a block's or a call's trace's own block param (a
    // transaction's trace is in whichever block holds it, which the proof names).
    let at_text = match &ask {
        Ask::State { at, .. } | Ask::Call { at, .. } | Ask::Estimate { at, .. } => Some(at.clone()),
        Ask::Trace { method, params } => trace_block_at(method).map(|i| match params.get(i) {
            Some(Value::String(s)) => s.clone(),
            Some(Value::Object(o)) => o
                .get("blockHash")
                .or_else(|| o.get("blockNumber"))
                .and_then(Value::as_str)
                .unwrap_or("latest")
                .to_string(),
            _ => "latest".into(),
        }),
    };
    let at = match at_text.as_deref().map(|t| ctx.resolve(t)).transpose() {
        Ok(n) => n,
        Err(e) => return e.report(ctx, method).map(|r| (r, false)),
    };
    // A block anchored (or given) by hash goes out as EIP-1898's object, so no other block can
    // take its place.
    let block = match &at {
        Some(Named::Hash(h)) => json!({"blockHash": h}),
        Some(other) => other.param(),
        None => Value::Null,
    };
    let params = match &ask {
        Ask::State {
            address,
            slots,
            code,
            ..
        } => json!([address.to_ascii_lowercase(), slots, block, {"code": code}]),
        Ask::Call {
            call, overrides, ..
        }
        | Ask::Estimate {
            call, overrides, ..
        } => with_overrides(json!([call, block]), overrides),
        Ask::Trace { method, params } => {
            let mut p = params.clone();
            if let (Some(i), Some(list)) = (trace_block_at(method), p.as_array_mut()) {
                if list.len() <= i {
                    list.resize(i + 1, Value::Null);
                }
                list[i] = block.clone();
            }
            json!([method, p])
        }
    };
    let params = match sw::request(ctx.chain_id, kind, &params.to_string()) {
        Ok(p) => p,
        Err(f) => {
            let mut r = ctx.base(method);
            r["failure"] = f.json();
            r["note"] = json!("the request was not sent");
            return Ok((r, false));
        }
    };
    let body = ctx.willow(method, &params)?;
    if let Some(r) = ctx.not_proven(method, &body) {
        return Ok((r, false));
    }
    let t = std::time::Instant::now();
    let verified = match kind {
        StateKind::Trace => sw::verify_trace(ctx.chain_id, &params, &body).map_err(|f| f.json()),
        _ => sw::verify(ctx.chain_id, kind, &params, &body).map_err(|f| f.json()),
    };
    let v = match verified {
        Ok(v) => v,
        Err(failure) => {
            let mut r = ctx.base(method);
            r["proofBytes"] = json!(body.len());
            r["failure"] = failure;
            return Ok((r, false));
        }
    };
    let mut report = v.report.clone();
    report["verified"] = json!(true);
    report["chain"] = json!(crate::check::chain_name(ctx.chain_id));
    report["proofBytes"] = json!(body.len());
    report["verifySeconds"] = json!(t.elapsed().as_secs_f64());
    let number = crate::check::parse_number(report["block"]["number"].as_str().unwrap_or(""))
        .ok_or("a verified report without a block number")?;
    let hash = report["block"]["hash"]
        .as_str()
        .ok_or("a verified report without a block hash")?
        .to_string();
    let mut ok = ctx.anchor_into(&mut report, number, &hash);
    // Every standard response, from the proven data: what an app would have been sent.
    report["responses"] = v.proven.clone();
    if ctx.compare_rpc.is_some() {
        let at = json!({"blockHash": hash});
        let asks: Vec<(String, &str, Value)> = match &ask {
            Ask::State {
                address,
                slots,
                code,
                ..
            } => {
                let a = address.to_ascii_lowercase();
                let mut v = vec![
                    (
                        "eth_getBalance".to_string(),
                        "eth_getBalance",
                        json!([a, at]),
                    ),
                    (
                        "eth_getTransactionCount".to_string(),
                        "eth_getTransactionCount",
                        json!([a, at]),
                    ),
                ];
                if *code {
                    v.push(("eth_getCode".to_string(), "eth_getCode", json!([a, at])));
                }
                for (i, k) in slots.iter().enumerate() {
                    v.push((
                        format!("eth_getStorageAt:{i}"),
                        "eth_getStorageAt",
                        json!([a, k, at]),
                    ));
                }
                v.push((
                    "eth_getProof".to_string(),
                    "eth_getProof",
                    json!([a, slots, at]),
                ));
                v
            }
            Ask::Call {
                call, overrides, ..
            } => vec![(
                "eth_call".to_string(),
                "eth_call",
                with_overrides(json!([call, at]), overrides),
            )],
            Ask::Estimate {
                call, overrides, ..
            } => vec![(
                "eth_estimateGas".to_string(),
                "eth_estimateGas",
                with_overrides(json!([call, at]), overrides),
            )],
            // The trace method itself, its block (where it names one) by hash.
            Ask::Trace { method, .. } => {
                let mut p = params[1].clone();
                if let (Some(i), Some(list)) = (trace_block_at(method), p.as_array_mut()) {
                    list[i] = at.clone();
                }
                vec![(method.clone(), method.as_str(), p)]
            }
        };
        let mut out = serde_json::Map::new();
        for (key, m, p) in asks {
            let c = match ctx.other(m, &p) {
                // A state read's error is the other endpoint's failure, not other state; a call's
                // or an estimate's error is its outcome, compared.
                Ok(theirs)
                    if kind == StateKind::State && crate::rpc::error_of(&theirs).is_some() =>
                {
                    problem(crate::rpc::error_of(&theirs).unwrap_or_default())
                }
                Ok(theirs) => sw::compare(kind, &v.proven, &key, &theirs)
                    .unwrap_or_else(|f| problem(f.json())),
                Err(e) => problem(json!(e)),
            };
            if counted(&c) {
                ok &= c["exact"] == true;
            }
            out.insert(key, c);
        }
        report["compare"] = Value::Object(out);
    }
    report["verified"] = json!(ok);
    Ok((report, ok))
}

/// A call's params with its overrides, if any: `[state, block]` (state overrides third, block
/// overrides fourth, an empty state override before block overrides alone, as Nitro reads them).
fn with_overrides(mut params: Value, overrides: &Option<Value>) -> Value {
    if let (Some(Value::Array(o)), Some(a)) = (overrides, params.as_array_mut()) {
        let mut o = o.clone();
        while o.last().is_some_and(Value::is_null) {
            o.pop();
        }
        if o.len() == 2 && o[0].is_null() {
            o[0] = json!({});
        }
        a.extend(o);
    }
    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overrides_go_third_and_fourth() {
        let call = json!([{"to": "0x01"}, "0x5"]);
        let state = json!({"0x02": {"balance": "0x1"}});
        let block = json!({"time": "0x9"});
        assert_eq!(with_overrides(call.clone(), &None), call);
        assert_eq!(
            with_overrides(call.clone(), &Some(json!([state, null]))),
            json!([{"to": "0x01"}, "0x5", state])
        );
        assert_eq!(
            with_overrides(call.clone(), &Some(json!([null, block]))),
            json!([{"to": "0x01"}, "0x5", {}, block])
        );
        assert_eq!(
            with_overrides(call, &Some(json!([state, block]))),
            json!([{"to": "0x01"}, "0x5", state, block])
        );
    }

    #[test]
    fn traces_name_their_block_where_they_do() {
        assert_eq!(trace_block_at("debug_traceCall"), Some(1));
        assert_eq!(trace_block_at("trace_call"), Some(2));
        assert_eq!(trace_block_at("trace_block"), Some(0));
        assert_eq!(trace_block_at("debug_traceTransaction"), None);
        assert!(parse_trace(json!(["trace_block", ["0x1"]])).is_ok());
        assert!(parse_trace(json!("trace_block")).is_err());
    }

    #[test]
    fn slots_are_a_comma_list() {
        assert_eq!(parse_slots("0x0, 0x1,,"), vec!["0x0", "0x1"]);
        assert!(parse_slots("").is_empty());
    }
}
