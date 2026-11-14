//! Recorded trace proofs (`testdata/*.json`, written by the log RPC's own tests from Ethereum and
//! Unichain mainnet blocks): each verifies to the response the log RPC's trace method sent (and
//! the public node's, recorded beside it), and each tampering fails with its kind.
//!
//! A fixture file is one block, `{"block": {chainId, blockNumber, blockHash, header, parent,
//! transactions, receipts}, "pool": {nodes, codes}, "cases": [{"params": [method, params],
//! "ancestors", "witness": {nodes, codes}, "expect", "node"}]}`: a case's proof is the block's
//! fields with its own, its witness the pool's entries it indexes; `node`: a public node's own
//! response was recorded and is `expect`.

use super::*;
use serde_json::{json, Value};
use std::path::PathBuf;

struct Case {
    file: String,
    chain: u64,
    params: Value,
    result: Value,
    expect: Value,
}

fn testdata() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata")
}

/// A case's proof: the block's fields and its own, its witness from the file's pool.
pub fn proof_of(file: &Value, case: &Value) -> Value {
    let mut p = file["block"].clone();
    p["ancestors"] = case["ancestors"].clone();
    let pick = |field: &str| -> Value {
        let at = |i: &Value| file["pool"][field][i.as_u64().unwrap() as usize].clone();
        Value::Array(
            case["witness"][field]
                .as_array()
                .unwrap()
                .iter()
                .map(at)
                .collect(),
        )
    };
    p["witness"] = json!({"nodes": pick("nodes"), "codes": pick("codes")});
    p["algorithm"] = json!(ALGORITHM);
    p
}

fn cases() -> Vec<Case> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(testdata())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let v: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
        let chain = request::quantity(&v["block"]["chainId"]).unwrap();
        for c in v["cases"].as_array().unwrap() {
            out.push(Case {
                file: f.file_name().unwrap().to_string_lossy().into(),
                chain,
                params: c["params"].clone(),
                result: proof_of(&v, c),
                expect: c["expect"].clone(),
            });
        }
    }
    assert!(out.len() >= 10, "{} cases", out.len());
    out
}

#[test]
fn every_proof_verifies_to_the_trace_methods_response() {
    let mut differ = Vec::new();
    let mut chains = std::collections::BTreeSet::new();
    for c in cases() {
        match verify_trace(c.chain, &c.params, &c.result) {
            Ok(p) if p.result == c.expect => {
                chains.insert(c.chain);
            }
            Ok(p) => differ.push(format!(
                "{} {}:\n proven {}\n served {}",
                c.file,
                c.params,
                p.result.to_string().chars().take(400).collect::<String>(),
                c.expect.to_string().chars().take(400).collect::<String>()
            )),
            Err(f) => differ.push(format!("{} {}: {f}", c.file, c.params)),
        }
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
    assert_eq!(
        chains.into_iter().collect::<Vec<_>>(),
        vec![ETHEREUM, UNICHAIN]
    );
}

/// The first case of `chain` whose method is `method`.
fn case(chain: u64, method: &str) -> Case {
    cases()
        .into_iter()
        .find(|c| c.chain == chain && c.params[0] == json!(method))
        .unwrap_or_else(|| panic!("no {method} case on chain {chain}"))
}

fn kind_of(chain: u64, params: &Value, result: &Value) -> Result<(), Kind> {
    verify_trace(chain, params, result)
        .map(|_| ())
        .map_err(|f| f.kind)
}

/// A change made to a proof (or a block), and what it must fail with.
type Mutation = (&'static str, Box<dyn Fn(&mut Value)>, Kind);
type Bend = Box<dyn Fn(&mut Block)>;

fn flip(hex: &str, at: usize) -> String {
    let mut b = hex::decode(&hex[2..]).unwrap();
    let i = at.min(b.len() - 1);
    b[i] ^= 1;
    format!("0x{}", hex::encode(b))
}

#[test]
fn tampered_proofs_fail_with_their_kind() {
    for (chain, method) in [
        (ETHEREUM, "debug_traceTransaction"),
        (UNICHAIN, "debug_traceBlockByNumber"),
    ] {
        let c = case(chain, method);
        assert_eq!(kind_of(chain, &c.params, &c.result), Ok(()));
        let mutations: Vec<Mutation> = vec![
            (
                "chain id",
                Box::new(|p| p["chainId"] = json!("0x5")),
                Kind::WrongChain,
            ),
            (
                "block hash",
                Box::new(|p| {
                    let h = p["blockHash"].as_str().unwrap().to_string();
                    p["blockHash"] = json!(flip(&h, 5));
                }),
                Kind::Header,
            ),
            (
                "block number",
                Box::new(|p| p["blockNumber"] = json!("0x1")),
                Kind::Header,
            ),
            (
                "header",
                Box::new(|p| {
                    let h = p["header"].as_str().unwrap().to_string();
                    p["header"] = json!(flip(&h, 40));
                }),
                Kind::Header,
            ),
            (
                "parent's state root",
                Box::new(|p| {
                    let h = p["parent"].as_str().unwrap().to_string();
                    // The state root's first byte (after the list, parent hash and ommers hash
                    // and beneficiary prefixes).
                    p["parent"] = json!(flip(&h, 3 + 33 + 33 + 21 + 1));
                }),
                Kind::Parent,
            ),
            (
                "a transaction left out",
                Box::new(|p| {
                    p["transactions"].as_array_mut().unwrap().pop();
                }),
                Kind::Transactions,
            ),
            (
                "transactions swapped",
                Box::new(|p| {
                    let t = p["transactions"].as_array_mut().unwrap();
                    let n = t.len();
                    t.swap(0, n - 1);
                }),
                Kind::Transactions,
            ),
            (
                "a receipt changed",
                Box::new(|p| {
                    let r = p["receipts"][0].as_str().unwrap().to_string();
                    p["receipts"][0] = json!(flip(&r, 8));
                }),
                Kind::Receipts,
            ),
            (
                "a receipt left out",
                Box::new(|p| {
                    p["receipts"].as_array_mut().unwrap().pop();
                }),
                Kind::Receipts,
            ),
            (
                "the trie's top node left out",
                Box::new(|p| {
                    let nodes = p["witness"]["nodes"].as_array_mut().unwrap();
                    let root_len = nodes
                        .iter()
                        .map(|n| n.as_str().unwrap().len())
                        .max()
                        .unwrap();
                    let i = nodes
                        .iter()
                        .position(|n| n.as_str().unwrap().len() == root_len)
                        .unwrap();
                    nodes.remove(i);
                }),
                Kind::MissingRead,
            ),
            (
                "every node left out",
                Box::new(|p| p["witness"]["nodes"] = json!([])),
                Kind::MissingRead,
            ),
            (
                "every node altered",
                Box::new(|p| {
                    for n in p["witness"]["nodes"].as_array_mut().unwrap() {
                        let s = n.as_str().unwrap().to_string();
                        *n = json!(flip(&s, s.len() / 4));
                    }
                }),
                Kind::MissingRead,
            ),
            (
                "every code left out",
                Box::new(|p| p["witness"]["codes"] = json!([])),
                Kind::MissingRead,
            ),
            (
                "a node not hex",
                Box::new(|p| p["witness"]["nodes"][0] = json!("0xzz")),
                Kind::Malformed,
            ),
            (
                "a node too long",
                Box::new(|p| p["witness"]["nodes"][0] = json!(format!("0x{}", "00".repeat(2000)))),
                Kind::Limits,
            ),
            (
                "too many ancestors",
                Box::new(|p| p["ancestors"] = json!(vec!["0x00"; 300])),
                Kind::Limits,
            ),
            (
                "an ancestor not linked",
                Box::new(|p| {
                    let parent = p["parent"].clone();
                    p["ancestors"] = json!([parent]);
                }),
                Kind::Parent,
            ),
            (
                "another algorithm",
                Box::new(|p| p["algorithm"] = json!("willow-trace-v0")),
                Kind::Algorithm,
            ),
            (
                "no witness",
                Box::new(|p| p["witness"] = json!(null)),
                Kind::Malformed,
            ),
        ];
        for (what, m, want) in mutations {
            let mut r = c.result.clone();
            m(&mut r);
            assert_eq!(
                kind_of(chain, &c.params, &r),
                Err(want),
                "chain {chain} {method}: {what}"
            );
        }
        // Nodes and codes nothing reads change nothing.
        let mut more = c.result.clone();
        more["witness"]["nodes"]
            .as_array_mut()
            .unwrap()
            .push(json!(format!("0x{}", "ab".repeat(100))));
        more["witness"]["codes"]
            .as_array_mut()
            .unwrap()
            .push(json!("0x6001600101"));
        assert_eq!(
            kind_of(chain, &c.params, &more),
            Ok(()),
            "{chain}: extra nodes"
        );
    }
}

#[test]
fn a_proof_of_another_block_or_transaction_is_refused() {
    let c = case(UNICHAIN, "debug_traceBlockByNumber");
    let n = request::quantity(&c.result["blockNumber"]).unwrap();
    let opts = c.params[1][1].clone();
    let other = json!(["debug_traceBlockByNumber", [format!("0x{:x}", n + 1), opts]]);
    assert_eq!(kind_of(UNICHAIN, &other, &c.result), Err(Kind::WrongBlock));
    let by_hash = json!([
        "debug_traceBlockByHash",
        [format!("0x{}", "11".repeat(32)), opts]
    ]);
    assert_eq!(
        kind_of(UNICHAIN, &by_hash, &c.result),
        Err(Kind::WrongBlock)
    );
    let right = json!([
        "debug_traceBlockByHash",
        [c.result["blockHash"].clone(), opts]
    ]);
    assert_eq!(kind_of(UNICHAIN, &right, &c.result), Ok(()));
    let tx = json!([
        "debug_traceTransaction",
        [format!("0x{}", "22".repeat(32)), opts]
    ]);
    assert_eq!(kind_of(UNICHAIN, &tx, &c.result), Err(Kind::WrongBlock));
}

#[test]
fn requests_a_proof_does_not_cover_are_refused() {
    let tx = format!("0x{}", "33".repeat(32));
    for (chain, params, want) in [
        (10, json!(["trace_block", ["0x1"]]), Kind::Unsupported),
        // Calls' traces: on Ethereum only, never at a transaction's place in a block or by a
        // JavaScript tracer, with calls and overrides as eth_call reads them.
        (
            130,
            json!(["debug_traceCall", [{}, "latest"]]),
            Kind::Unsupported,
        ),
        (
            130,
            json!(["trace_call", [{}, ["trace"], "latest"]]),
            Kind::Unsupported,
        ),
        (
            1,
            json!(["debug_traceCall", [{}, "latest", {"txIndex": "0x0"}]]),
            Kind::Unsupported,
        ),
        (
            1,
            json!(["debug_traceCall", [{}, "latest", {"tracer": "{ result: function() {} }"}]]),
            Kind::Unsupported,
        ),
        (
            1,
            json!(["debug_traceCall", [{"authorizationList": [{}]}, "latest"]]),
            Kind::Unsupported,
        ),
        (
            1,
            json!(["debug_traceCall", [{"to": "0x12"}, "latest"]]),
            Kind::InvalidRequest,
        ),
        (
            1,
            json!(["trace_call", [{}, ["nothing"], "latest"]]),
            Kind::InvalidRequest,
        ),
        (1, json!(["trace_filter", [{}]]), Kind::Unsupported),
        (
            1,
            json!(["debug_traceTransaction", [tx, {"tracer": "{ result: function() {} }"}]]),
            Kind::Unsupported,
        ),
        (
            1,
            json!(["debug_traceTransaction", [tx, {"tracer": "erc7562Tracer"}]]),
            Kind::Unsupported,
        ),
        (
            130,
            json!(["debug_traceTransaction", [tx, {}]]),
            Kind::Unsupported,
        ),
        (
            130,
            json!(["debug_traceTransaction", [tx, {"tracer": "4byteTracer"}]]),
            Kind::Unsupported,
        ),
        (
            130,
            json!(["trace_replayTransaction", [tx, ["trace"]]]),
            Kind::Unsupported,
        ),
        (1, json!(["eth_call", [{}]]), Kind::InvalidRequest),
        (1, json!("trace_block"), Kind::InvalidRequest),
        (
            1,
            json!(["debug_traceTransaction", ["0x12"]]),
            Kind::InvalidRequest,
        ),
        (1, json!(["trace_block", ["bad"]]), Kind::InvalidRequest),
    ] {
        assert_eq!(
            request::parse(chain, &params)
                .map(|_| ())
                .map_err(|f| f.kind),
            Err(want),
            "{chain} {params}"
        );
    }
}

/// A run here that differs from a receipt (an EVM that runs the chain otherwise) is refused:
/// the receipts are the block's own, so the proof can't carry another; the block is bent here.
#[test]
fn a_run_unlike_its_receipts_is_refused() {
    for (chain, method) in [
        (ETHEREUM, "debug_traceTransaction"),
        (UNICHAIN, "debug_traceBlockByNumber"),
    ] {
        let c = case(chain, method);
        let proof: wire::TraceProof = serde_json::from_value(c.result.clone()).unwrap();
        let mut block = Block::from_proof(chain, &proof).unwrap();
        let trie = trie::Trie::from_wire(&proof.witness).unwrap();
        let asked = request::parse(chain, &c.params).unwrap();
        let (last, _) = span(&block, &asked.job).unwrap();
        block.recover_through(last).unwrap();
        let bends: Vec<Bend> = vec![
            Box::new(|b| b.receipts[0].cumulative_gas += 1),
            Box::new(|b| b.receipts[0].success = b.receipts[0].success.map(|s| !s)),
            Box::new(|b| b.receipts[0].logs.push(Default::default())),
        ];
        for bend in bends {
            let mut bent = block.clone();
            bend(&mut bent);
            let hash_of = |n: u64| bent.hash_of(n);
            let pre = state::Pre::new(&trie, bent.parent_root, &hash_of);
            let r = run(&bent, &pre, asked.job.tracer(), last, &|_| true);
            assert!(matches!(r, Err(Stop::Disagrees(0, _))), "{chain}: {r:?}");
        }
        if chain == UNICHAIN {
            // A deposit's nonce, or its receipt's version, otherwise.
            let nonce = block.receipts[0].deposit.unwrap().0;
            for d in [
                (nonce.map(|x| x + 1), Some(1)),
                (None, Some(1)),
                (nonce, Some(0)),
            ] {
                let mut bent = block.clone();
                bent.receipts[0].deposit = Some(d);
                let hash_of = |n: u64| bent.hash_of(n);
                let pre = state::Pre::new(&trie, bent.parent_root, &hash_of);
                let r = run(&bent, &pre, asked.job.tracer(), last, &|_| true);
                assert!(
                    matches!(r, Err(Stop::Disagrees(0, _))),
                    "deposit {d:?}: {r:?}"
                );
            }
        }
    }
}

/// The recorded call trace proofs (`testdata/calls/*.json`, written by the log RPC's own tests
/// from Ethereum mainnet): `(file, params, result, expect, node)`.
fn call_cases() -> Vec<(String, Value, Value, Value, bool)> {
    let dir = testdata().join("calls");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    files.sort();
    let mut out = Vec::new();
    for f in files {
        let v: Value = serde_json::from_slice(&std::fs::read(&f).unwrap()).unwrap();
        for c in v["cases"].as_array().unwrap() {
            out.push((
                f.file_name().unwrap().to_string_lossy().into(),
                c["params"].clone(),
                c["result"].clone(),
                c["expect"].clone(),
                c["node"] == json!(true),
            ));
        }
    }
    assert!(out.len() >= 10, "{} call cases", out.len());
    out
}

#[test]
fn every_call_trace_proof_verifies_to_the_trace_methods_response() {
    let mut differ = Vec::new();
    let (mut methods, mut nodes) = (std::collections::BTreeSet::new(), 0);
    for (file, params, result, expect, node) in call_cases() {
        match verify_trace(ETHEREUM, &params, &result) {
            Ok(p) if p.result == expect => {
                methods.insert(p.method);
                nodes += usize::from(node);
            }
            Ok(p) => differ.push(format!(
                "{file} {params}:\n proven {}\n served {}",
                p.result.to_string().chars().take(400).collect::<String>(),
                expect.to_string().chars().take(400).collect::<String>()
            )),
            Err(f) => differ.push(format!("{file} {params}: {f}")),
        }
    }
    assert!(differ.is_empty(), "{}", differ.join("\n"));
    assert_eq!(
        methods.into_iter().collect::<Vec<_>>(),
        vec!["debug_traceCall".to_string(), "trace_call".to_string()]
    );
    // Geth's and a parity-trace node's own responses, recorded, are among them.
    assert!(nodes >= 4, "{nodes}");
}

#[test]
fn tampered_call_trace_proofs_fail_with_their_kind() {
    let (_, params, result, _, _) = call_cases()
        .into_iter()
        .find(|c| c.1[0] == json!("debug_traceCall") && c.1[1][2]["tracer"] == json!("callTracer"))
        .expect("a callTracer case");
    assert_eq!(kind_of(ETHEREUM, &params, &result), Ok(()));
    let first_long = |p: &mut Value| {
        let a = &mut p["witness"]["accounts"][0]["accountProof"];
        let i = a
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n.as_str().is_some_and(|s| s.len() > 200))
            .unwrap();
        let s = a[i].as_str().unwrap().to_string();
        a[i] = json!(flip(&s, 50));
    };
    let mutations: Vec<Mutation> = vec![
        (
            "chain id",
            Box::new(|p| p["chainId"] = json!("0x5")),
            Kind::WrongChain,
        ),
        (
            "block hash",
            Box::new(|p| {
                let h = p["blockHash"].as_str().unwrap().to_string();
                p["blockHash"] = json!(flip(&h, 5));
            }),
            Kind::Header,
        ),
        (
            "header",
            Box::new(|p| {
                let h = p["header"].as_str().unwrap().to_string();
                p["header"] = json!(flip(&h, 40));
            }),
            Kind::Header,
        ),
        (
            "an account's proof node",
            Box::new(first_long),
            Kind::Account,
        ),
        (
            "an account left out",
            Box::new(|p| {
                p["witness"]["accounts"].as_array_mut().unwrap().remove(0);
            }),
            Kind::MissingRead,
        ),
        (
            "a code altered",
            Box::new(|p| {
                let codes = p["witness"]["codes"].as_object_mut().unwrap();
                let k = codes.keys().next().unwrap().clone();
                let c = codes[&k].as_str().unwrap().to_string();
                codes[&k] = json!(flip(&c, 10));
            }),
            Kind::Code,
        ),
        (
            "another algorithm",
            Box::new(|p| p["algorithm"] = json!("willow-trace-v0")),
            Kind::Algorithm,
        ),
        (
            "no witness",
            Box::new(|p| {
                p.as_object_mut().unwrap().remove("witness");
            }),
            Kind::Malformed,
        ),
    ];
    for (what, bend, want) in mutations {
        let mut bad = result.clone();
        bend(&mut bad);
        assert_eq!(kind_of(ETHEREUM, &params, &bad), Err(want), "{what}");
    }
    // The proof of another block than the one asked for.
    let mut other = params.clone();
    let n = request::quantity(&result["blockNumber"]).unwrap();
    other[1][1] = json!(format!("0x{:x}", n + 1));
    assert_eq!(kind_of(ETHEREUM, &other, &result), Err(Kind::WrongBlock));
    // Asked for on another chain.
    assert_eq!(kind_of(UNICHAIN, &params, &result), Err(Kind::Unsupported));
}
