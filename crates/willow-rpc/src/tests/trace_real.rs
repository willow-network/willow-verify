//! Traces (willow_getTraceProof) on trace-verify's recorded proofs (Ethereum and Unichain, and
//! Ethereum calls' traces): the client runs each block or call itself and returns the response
//! the trace method sent; a lying server's proofs never reach the app (trace.real.test.ts).

use super::support::*;
use crate::config::Status;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;

fn dir() -> PathBuf {
    repo().join("zkvm/logset-guest/trace-verify/testdata")
}

fn files(sub: &str) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir().join(sub))
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    v.sort();
    v
}

const BLOCK_TRACES: [&str; 4] = [
    "debug_traceBlockByNumber",
    "debug_traceBlockByHash",
    "trace_block",
    "trace_replayBlockTransactions",
];

struct Fx {
    chain_id: u64,
    number: u64,
    hash: String,
    f: Value,
}

fn load(p: &std::path::Path) -> Fx {
    let f = json_file(p);
    Fx {
        chain_id: n(&f["block"]["chainId"]),
        number: n(&f["block"]["blockNumber"]),
        hash: f["block"]["blockHash"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase(),
        f,
    }
}

/// A case's proof: the block's fields and its own, its witness from the file's pool.
fn proof_of(x: &Fx, c: &Value) -> Value {
    let pick = |field: &str| -> Value {
        Value::Array(
            c["witness"][field]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| x.f["pool"][field][i.as_u64().unwrap() as usize].clone())
                .collect(),
        )
    };
    let mut p = x.f["block"].clone();
    p["ancestors"] = c["ancestors"].clone();
    p["witness"] = json!({"nodes": pick("nodes"), "codes": pick("codes")});
    p["algorithm"] = json!("willow-trace-v1");
    p
}

type Bend = Arc<dyn Fn(&mut Value) + Send + Sync>;

/// A world whose head is the fixture's block, serving its proofs (`bend` changes one).
fn world(x: Arc<Fx>, bend: Option<Bend>) -> Arc<World> {
    let w = World::new();
    w.head(
        x.chain_id,
        x.number,
        &x.hash,
        "signed",
        if x.chain_id == 1 { "head" } else { "sequencer" },
    );
    let xx = x.clone();
    w.intercept(Some(Arc::new(move |chain, m, params| {
        if m != "willow_getTraceProof" || chain != xx.chain_id {
            return None;
        }
        let method = params[0].as_str().unwrap_or("");
        let p = params[1].as_array().cloned().unwrap_or_default();
        let the_block = |v: &Value| {
            if BLOCK_TRACES.contains(&method) && (v == &json!(xx.hash) || v == &json!(q(xx.number))) {
                json!("the block")
            } else {
                v.clone()
            }
        };
        let norm = |q: &[Value]| -> Value {
            let c = canon(&Value::Array(q.to_vec()));
            let a = c.as_array().unwrap();
            let mut out = vec![the_block(a.first().unwrap_or(&Value::Null))];
            out.extend(a.iter().skip(1).cloned());
            Value::Array(out)
        };
        let case = xx.f["cases"].as_array().unwrap().iter().find(|c| {
            c["params"][0] == method && norm(c["params"][1].as_array().unwrap()) == norm(&p)
        });
        let Some(c) = case else {
            return Some(json!({"error": {"code": -32000, "message": format!("not found in this store: no trace proof for {method}")}}));
        };
        let mut proof = proof_of(&xx, c);
        if let Some(b) = &bend {
            b(&mut proof);
        }
        Some(json!({"result": proof}))
    })));
    w
}

#[tokio::test(flavor = "multi_thread")]
async fn every_trace_methods_response_from_the_clients_own_run() {
    for p in files("") {
        let x = Arc::new(load(&p));
        let h = harness_with(x.chain_id, world(x.clone(), None), "test-key", |c| c).await;
        for c in x.f["cases"].as_array().unwrap() {
            let method = c["params"][0].as_str().unwrap();
            let got = h
                .ask(method, c["params"][1].clone())
                .await
                .unwrap_or_else(|e| panic!("{p:?} {method}: {e}"));
            assert_eq!(got, c["expect"], "{p:?} {method}");
        }
        assert!(h.events().iter().all(|e| e.status == Status::Verified
            && e.block.as_ref().map(|b| b.number) == Some(x.number)));
        // A block trace went out by the hash the client resolved its number to.
        for s in h.world.sent("willow_getTraceProof") {
            let m = s[0].as_str().unwrap();
            if BLOCK_TRACES.contains(&m) && m != "debug_traceBlockByHash" {
                assert_eq!(s[1][0], json!({"blockHash": x.hash}), "{m}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lying_servers_trace_proofs_never_reach_the_app() {
    for p in files("") {
        let x = Arc::new(load(&p));
        let cases = x.f["cases"].as_array().unwrap();
        let c = cases
            .iter()
            .find(|c| BLOCK_TRACES.contains(&c["params"][0].as_str().unwrap()))
            .unwrap_or(&cases[0])
            .clone();
        let bends: Vec<(&str, Bend, &str)> = vec![
            (
                "a node left out",
                Arc::new(|p: &mut Value| {
                    p["witness"]["nodes"].as_array_mut().unwrap().pop();
                }),
                "missing-read",
            ),
            (
                "a code left out",
                Arc::new(|p: &mut Value| p["witness"]["codes"] = json!([])),
                "missing-read",
            ),
            (
                "a receipt changed",
                Arc::new(|p: &mut Value| {
                    let r = p["receipts"][0].as_str().unwrap().to_string();
                    let last = if r.ends_with("00") { "01" } else { "00" };
                    p["receipts"][0] = json!(format!("{}{last}", &r[..r.len() - 2]));
                }),
                "receipts",
            ),
            (
                "another parent",
                Arc::new(|p: &mut Value| p["parent"] = p["header"].clone()),
                "parent",
            ),
            (
                "another algorithm",
                Arc::new(|p: &mut Value| p["algorithm"] = json!("willow-trace-v2")),
                "algorithm",
            ),
        ];
        for (what, bend, kind) in bends {
            let h = harness_with(x.chain_id, world(x.clone(), Some(bend)), "test-key", |c| c).await;
            let e = h
                .ask(c["params"][0].as_str().unwrap(), c["params"][1].clone())
                .await
                .unwrap_err();
            assert_eq!(
                code_kind(&e),
                (-32098, Some(kind.to_string())),
                "{p:?} {what}: {e}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn trace_filter_traces_each_block_and_filters_as_the_server_filters() {
    for p in files("") {
        let x = Arc::new(load(&p));
        let Some(tb) = x.f["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["params"][0] == "trace_block")
        else {
            continue;
        };
        let flat = tb["expect"].as_array().unwrap().clone();
        let h = harness_with(x.chain_id, world(x.clone(), None), "test-key", |c| c).await;
        let nq = q(x.number);
        assert_eq!(
            h.ask("trace_filter", json!([{"fromBlock": nq, "toBlock": nq}]))
                .await
                .unwrap(),
            Value::Array(flat.clone())
        );
        let to = flat.iter().find(|t| t["type"] == "call").unwrap()["action"]["to"].clone();
        let want: Vec<Value> = flat
            .iter()
            .filter(|t| t["type"] == "call" && t["action"]["to"] == to)
            .cloned()
            .collect();
        assert_eq!(
            h.ask(
                "trace_filter",
                json!([{"fromBlock": nq, "toBlock": nq, "toAddress": [to]}])
            )
            .await
            .unwrap(),
            Value::Array(want)
        );
        assert_eq!(
            h.ask(
                "trace_filter",
                json!([{"fromBlock": nq, "toBlock": nq, "after": 1, "count": 2}])
            )
            .await
            .unwrap(),
            Value::Array(flat[1..3].to_vec())
        );
    }
}

/// Where a call trace's params name its block.
fn block_at(m: &str) -> usize {
    if m == "debug_traceCall" {
        1
    } else {
        2
    }
}

fn call_world(cases: Arc<Vec<Value>>, bend: Option<Bend>) -> Arc<World> {
    let w = World::new();
    let r = &cases[0]["result"];
    w.head(
        1,
        n(&r["blockNumber"]),
        r["blockHash"].as_str().unwrap(),
        "signed",
        "head",
    );
    w.intercept(Some(Arc::new(move |chain, m, params| {
        if m != "willow_getTraceProof" || chain != 1 {
            return None;
        }
        let method = params[0].as_str().unwrap_or("");
        let asked = |q: &[Value]| -> Value {
            canon(&Value::Array(q.iter().enumerate().map(|(i, v)| if i == block_at(method) { json!("the block") } else { v.clone() }).collect()))
        };
        let p = params[1].as_array().cloned().unwrap_or_default();
        let Some(c) = cases.iter().find(|c| c["params"][0] == method && asked(c["params"][1].as_array().unwrap()) == asked(&p)) else {
            return Some(json!({"error": {"code": -32000, "message": format!("not found in this store: no trace proof for {method}")}}));
        };
        let mut proof = c["result"].clone();
        if let Some(b) = &bend {
            b(&mut proof);
        }
        Some(json!({"result": proof}))
    })));
    w
}

#[tokio::test(flavor = "multi_thread")]
async fn every_calls_trace_from_the_clients_own_run_of_the_call() {
    for p in files("calls") {
        let cases = Arc::new(json_file(&p)["cases"].as_array().unwrap().clone());
        let h = harness_with(1, call_world(cases.clone(), None), "test-key", |c| c).await;
        for c in cases.iter() {
            let m = c["params"][0].as_str().unwrap();
            let got = h
                .ask(m, c["params"][1].clone())
                .await
                .unwrap_or_else(|e| panic!("{p:?} {m}: {e}"));
            assert_eq!(got, c["expect"], "{p:?} {m}");
        }
        let number = n(&cases[0]["result"]["blockNumber"]);
        assert!(h
            .events()
            .iter()
            .all(|e| e.status == Status::Verified
                && e.block.as_ref().map(|b| b.number) == Some(number)));
        assert!(!h.world.sent("willow_getTraceProof").is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_lying_servers_call_trace_proofs_never_reach_the_app() {
    for p in files("calls") {
        let cases = Arc::new(json_file(&p)["cases"].as_array().unwrap().clone());
        let c = cases
            .iter()
            .find(|c| c["params"][0] == "debug_traceCall")
            .unwrap_or(&cases[0])
            .clone();
        let bends: Vec<(&str, Bend, &str)> = vec![
            (
                "an account left out",
                Arc::new(|p: &mut Value| {
                    p["witness"]["accounts"].as_array_mut().unwrap().remove(0);
                }),
                "missing-read",
            ),
            (
                "a code altered",
                Arc::new(|p: &mut Value| {
                    let codes = p["witness"]["codes"].as_object_mut().unwrap();
                    let k = codes.keys().next().unwrap().clone();
                    let v = codes[&k].as_str().unwrap().to_string();
                    let last = if v.ends_with("00") { "01" } else { "00" };
                    codes.insert(k, json!(format!("{}{last}", &v[..v.len() - 2])));
                }),
                "code",
            ),
            (
                "another chain",
                Arc::new(|p: &mut Value| p["chainId"] = json!("0x5")),
                "wrong-chain",
            ),
            (
                "another algorithm",
                Arc::new(|p: &mut Value| p["algorithm"] = json!("willow-trace-v2")),
                "algorithm",
            ),
        ];
        for (what, bend, kind) in bends {
            let h = harness_with(1, call_world(cases.clone(), Some(bend)), "test-key", |c| c).await;
            let e = h
                .ask(c["params"][0].as_str().unwrap(), c["params"][1].clone())
                .await
                .unwrap_err();
            assert_eq!(
                code_kind(&e),
                (-32098, Some(kind.to_string())),
                "{p:?} {what}: {e}"
            );
        }
    }
}
