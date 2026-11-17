//! The lying server (lying.test.ts and support/mutations.ts): one mutation per field
//! of every proof shape, and the listed attacks on anchors and batches. Each must reach the app as
//! a -32098 (or, for denials, -32099 or an absence) and never as a value. Items and state are
//! verified by the real verifiers; anchors by the mock anchor.

use super::support::*;
use crate::Verifier;
use serde_json::{json, Value};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type Fut = Pin<Box<dyn Future<Output = Result<Value, crate::Error>> + Send>>;
type Ask = Arc<dyn Fn(Verifier) -> Fut + Send + Sync>;

enum Expect {
    Err(i64, &'static str),
    Null,
    Values(Arc<dyn Fn(&Value) -> bool + Send + Sync>),
}

struct Case {
    name: String,
    chain_id: u64,
    /// The head: "w1" the block fixture's, "w2" the state fixtures' Ethereum block.
    head: &'static str,
    ask: Ask,
    mutation: Mutation,
    expect: Expect,
    replay_after_sec: u64,
    warm: bool,
    setup: Option<Arc<dyn Fn(&World) + Send + Sync>>,
}

fn ask(method: &'static str, params: Value) -> Ask {
    Arc::new(move |c: Verifier| -> Fut {
        let p = params.clone();
        Box::pin(async move { c.request(method, p).await })
    })
}

/// A mutation of `method`'s result: `f` changes it in place (false when it does not apply).
fn on(
    method: &'static str,
    f: impl Fn(&mut Value, &[Value]) -> bool + Send + Sync + 'static,
) -> Mutation {
    Mutation::Item(Arc::new(move |resp: &mut Value, m: &str, p: &[Value]| {
        m == method && resp.get("result").is_some_and(|r| !r.is_null()) && f(&mut resp["result"], p)
    }))
}

fn flip_at(v: &mut Value, from_end: usize) -> bool {
    let Some(s) = v.as_str() else { return false };
    *v = json!(flip(s, from_end));
    true
}

fn swap(v: &mut Value, i: usize, j: usize) -> bool {
    match v.as_array_mut() {
        Some(a) if a.len() > i.max(j) => {
            a.swap(i, j);
            true
        }
        _ => false,
    }
}

fn eoa() -> Value {
    fixture("state/eoa.json")["request"]["params"][0].clone()
}

fn cases() -> Vec<Case> {
    let mut out = vec![];
    let mut add = |name: &str,
                   chain_id: u64,
                   head: &'static str,
                   ask: Ask,
                   mutation: Mutation,
                   expect: Expect| {
        out.push(Case {
            name: name.into(),
            chain_id,
            head,
            ask,
            mutation,
            expect,
            replay_after_sec: 0,
            warm: true,
            setup: None,
        });
    };
    let v = |kind: &'static str| Expect::Err(-32098, kind);
    let ask_block = || ask("eth_getBlockByNumber", json!(["latest", false]));
    let ask_balance = || ask("eth_getBalance", json!([eoa(), "latest"]));

    // ---- willow_getBlockProof ----
    let block = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
                 name: &str,
                 f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
                 kind: &'static str| {
        add(
            &format!("block: {name}"),
            8453,
            "w1",
            ask_block(),
            on("willow_getBlockProof", move |r, _| f(r)),
            v(kind),
        );
    };
    block(
        &mut add,
        "a byte of the header",
        Box::new(|r| flip_at(&mut r["header"], 10)),
        "header",
    );
    block(
        &mut add,
        "blockHash",
        Box::new(|r| flip_at(&mut r["blockHash"], 1)),
        "header",
    );
    block(
        &mut add,
        "blockNumber",
        Box::new(|r| {
            r["blockNumber"] = json!(q(n(&r["blockNumber"]) + 1));
            true
        }),
        "header",
    );
    block(
        &mut add,
        "chainId",
        Box::new(|r| {
            r["chainId"] = json!("0x1");
            true
        }),
        "wrong-chain",
    );
    block(
        &mut add,
        "a transaction changed",
        Box::new(|r| flip_at(&mut r["transactions"][1], 6)),
        "transactions",
    );
    block(
        &mut add,
        "a transaction dropped",
        Box::new(|r| {
            r["transactions"]
                .as_array_mut()
                .is_some_and(|a| a.pop().is_some())
        }),
        "transactions",
    );
    block(
        &mut add,
        "transactions reordered",
        Box::new(|r| swap(&mut r["transactions"], 1, 2)),
        "transactions",
    );
    block(
        &mut add,
        "no transactions",
        Box::new(|r| r.as_object_mut().unwrap().remove("transactions").is_some()),
        "missing",
    );
    block(
        &mut add,
        "a withdrawal on the OP stack",
        Box::new(|r| {
            r["withdrawals"] = json!("0xc4c3808080");
            true
        }),
        "withdrawals",
    );
    block(
        &mut add,
        "a deposit's receipt",
        Box::new(|r| flip_at(&mut r["receiptProofs"][0]["value"], 4)),
        "receipts",
    );
    let e17 = fixture("block/ethereum-17034874.json")["result"].clone();
    let e26 = fixture("block/ethereum-26391511.json")["result"].clone();
    {
        let other = e17.clone();
        add(
            "block: another block for a blockHash request",
            1,
            "w1",
            ask("eth_getBlockByHash", json!([e26["blockHash"], false])),
            on("willow_getBlockProof", move |r, _| {
                *r = other.clone();
                true
            }),
            v("wrong-block"),
        );
    }
    let pre = fixture("block/ethereum-1500000.json")["result"].clone();
    add(
        "block: an uncle changed",
        1,
        "w1",
        ask("eth_getBlockByHash", json!([pre["blockHash"], false])),
        on("willow_getBlockProof", |r, _| {
            flip_at(&mut r["uncles"][0], 8)
        }),
        v("uncles"),
    );
    add(
        "block: uncles marked unproven",
        1,
        "w1",
        ask("eth_getBlockByHash", json!([pre["blockHash"], false])),
        on("willow_getBlockProof", |r, _| {
            r["unproven"] = json!(["uncles", "size"]);
            true
        }),
        v("malformed"),
    );
    {
        let ink = fixture("block/ink-60901963.json")["result"].clone();
        add(
            "block: a response from another chain",
            8453,
            "w1",
            ask_block(),
            on("willow_getBlockProof", move |r, _| {
                *r = ink.clone();
                true
            }),
            v("wrong-chain"),
        );
    }

    // ---- willow_getFeeProof and willow_getFeeHistoryProof ----
    let fee = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
               name: &str,
               f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
               kind: &'static str| {
        add(
            &format!("fees: {name}"),
            1,
            "w1",
            ask("eth_gasPrice", json!([])),
            on("willow_getFeeProof", move |r, _| f(r)),
            v(kind),
        );
    };
    fee(
        &mut add,
        "another formula",
        Box::new(|r| {
            r["formula"] = json!("willow-fee-v2");
            true
        }),
        "unknown-formula",
    );
    fee(
        &mut add,
        "chainId",
        Box::new(|r| {
            r["chainId"] = json!("0xa");
            true
        }),
        "wrong-chain",
    );
    fee(
        &mut add,
        "blockHash",
        Box::new(|r| flip_at(&mut r["blockHash"], 1)),
        "header",
    );
    fee(
        &mut add,
        "blockNumber",
        Box::new(|r| {
            r["blockNumber"] = json!(q(n(&r["blockNumber"]) - 1));
            true
        }),
        "header",
    );
    fee(
        &mut add,
        "a byte of the newest header",
        Box::new(|r| flip_at(&mut r["headers"][3], 10)),
        "header",
    );
    fee(
        &mut add,
        "two headers swapped",
        Box::new(|r| swap(&mut r["headers"], 0, 1)),
        "broken-link",
    );
    fee(
        &mut add,
        "a header dropped",
        Box::new(|r| {
            r["headers"].as_array_mut().is_some_and(|a| {
                a.remove(0);
                true
            })
        }),
        "wrong-range",
    );
    fee(
        &mut add,
        "a body for another block",
        Box::new(|r| swap(&mut r["bodies"], 0, 1)),
        "body",
    );
    fee(
        &mut add,
        "a body dropped",
        Box::new(|r| {
            r["bodies"]
                .as_array_mut()
                .is_some_and(|a| a.pop().is_some())
        }),
        "missing",
    );
    fee(
        &mut add,
        "a transaction of a body",
        Box::new(|r| flip_at(&mut r["bodies"][0]["transactions"][0], 6)),
        "transactions",
    );
    let history = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
                   name: &str,
                   f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
                   kind: &'static str| {
        add(
            &format!("fee history: {name}"),
            8453,
            "w1",
            ask("eth_feeHistory", json!(["0x4", "latest", [10, 50, 90]])),
            on("willow_getFeeHistoryProof", move |r, _| f(r)),
            v(kind),
        );
    };
    history(
        &mut add,
        "two headers swapped",
        Box::new(|r| swap(&mut r["headers"], 1, 2)),
        "broken-link",
    );
    history(
        &mut add,
        "a receipt's cumulative gas",
        Box::new(|r| flip_at(&mut r["bodies"][0]["receipts"][0], 6)),
        "receipts",
    );
    history(
        &mut add,
        "another formula",
        Box::new(|r| {
            r["formula"] = json!("x");
            true
        }),
        "unknown-formula",
    );
    history(
        &mut add,
        "chainId",
        Box::new(|r| {
            r["chainId"] = json!("0x1");
            true
        }),
        "wrong-chain",
    );

    // ---- willow_getStateProof ----
    let usdc = fixture("state/usdc-slots-code.json");
    let state = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
                 name: &str,
                 a: Ask,
                 f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
                 kind: &'static str| {
        add(
            &format!("state: {name}"),
            1,
            "w2",
            a,
            on("willow_getStateProof", move |r, _| f(r)),
            v(kind),
        );
    };
    state(
        &mut add,
        "a byte of the header",
        ask_balance(),
        Box::new(|r| flip_at(&mut r["header"], 10)),
        "header",
    );
    state(
        &mut add,
        "blockHash",
        ask_balance(),
        Box::new(|r| flip_at(&mut r["blockHash"], 1)),
        "header",
    );
    state(
        &mut add,
        "blockNumber",
        ask_balance(),
        Box::new(|r| {
            r["blockNumber"] = json!("0x1");
            true
        }),
        "header",
    );
    state(
        &mut add,
        "chainId",
        ask_balance(),
        Box::new(|r| {
            r["chainId"] = json!("0x2");
            true
        }),
        "wrong-chain",
    );
    state(
        &mut add,
        "balance",
        ask_balance(),
        Box::new(|r| {
            r["account"]["balance"] = json!("0x1");
            true
        }),
        "account",
    );
    state(
        &mut add,
        "a lower pending nonce",
        ask("eth_getTransactionCount", json!([eoa(), "pending"])),
        Box::new(|r| {
            r["account"]["nonce"] = json!("0x1");
            true
        }),
        "account",
    );
    state(
        &mut add,
        "codeHash",
        ask_balance(),
        Box::new(|r| flip_at(&mut r["account"]["codeHash"], 1)),
        "account",
    );
    state(
        &mut add,
        "storageHash",
        ask_balance(),
        Box::new(|r| flip_at(&mut r["account"]["storageHash"], 1)),
        "account",
    );
    state(
        &mut add,
        "an account proof node",
        ask_balance(),
        Box::new(|r| flip_at(&mut r["account"]["accountProof"][1], 10)),
        "account",
    );
    {
        let acct = usdc["result"]["account"].clone();
        state(
            &mut add,
            "a proof of another account",
            ask_balance(),
            Box::new(move |r| {
                r["account"] = acct.clone();
                true
            }),
            "account",
        );
    }
    let slot = || {
        ask(
            "eth_getStorageAt",
            json!([
                usdc["request"]["params"][0],
                usdc["request"]["params"][1][0],
                "latest"
            ]),
        )
    };
    state(
        &mut add,
        "a slot value",
        slot(),
        Box::new(|r| {
            r["account"]["storageProof"][0]["value"] = json!("0x1");
            true
        }),
        "slot",
    );
    state(
        &mut add,
        "the proof of another slot",
        slot(),
        Box::new(|r| {
            r["account"]["storageProof"][0]["key"] = json!("0x2");
            true
        }),
        "slot",
    );
    state(
        &mut add,
        "a storage proof node",
        slot(),
        Box::new(|r| flip_at(&mut r["account"]["storageProof"][0]["proof"][1], 10)),
        "slot",
    );
    let code = || {
        ask(
            "eth_getCode",
            json!([usdc["request"]["params"][0], "latest"]),
        )
    };
    state(
        &mut add,
        "code swapped (same length, other hash)",
        code(),
        Box::new(|r| flip_at(&mut r["code"], 1)),
        "code",
    );
    state(
        &mut add,
        "code left out",
        code(),
        Box::new(|r| r.as_object_mut().unwrap().remove("code").is_some()),
        "code",
    );
    {
        let other = e26.clone();
        state(
            &mut add,
            "another block for the block asked",
            ask_balance(),
            Box::new(move |r| {
                for k in ["chainId", "blockNumber", "blockHash", "header"] {
                    r[k] = other[k].clone();
                }
                true
            }),
            "wrong-block",
        );
    }

    // ---- willow_getCallProof and willow_getEstimateGasProof ----
    let call_f = fixture("call/call-usdc-balance.json");
    let ask_call = || {
        ask(
            "eth_call",
            json!([call_f["request"]["params"][0], "latest"]),
        )
    };
    let call = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
                name: &str,
                f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
                kind: &'static str| {
        add(
            &format!("call: {name}"),
            1,
            "w2",
            ask_call(),
            on("willow_getCallProof", move |r, _| f(r)),
            v(kind),
        );
    };
    call(
        &mut add,
        "an account dropped from the witness",
        Box::new(|r| {
            r["witness"]["accounts"]
                .as_array_mut()
                .is_some_and(|a| a.pop().is_some())
        }),
        "missing-read",
    );
    call(
        &mut add,
        "a shared node",
        Box::new(|r| flip_at(&mut r["witness"]["nodes"][0], 10)),
        "account",
    );
    call(
        &mut add,
        "code left out",
        Box::new(|r| {
            r["witness"]["codes"] = json!({});
            true
        }),
        "code",
    );
    call(
        &mut add,
        "another result",
        Box::new(|r| {
            let x = flip(r["result"]["returned"].as_str().unwrap_or("0x00"), 1);
            r["result"] = json!({"returned": x});
            true
        }),
        "outcome",
    );
    call(
        &mut add,
        "a revert for a result",
        Box::new(|r| {
            r["result"] = json!({"reverted": "0x"});
            true
        }),
        "outcome",
    );
    call(
        &mut add,
        "a byte of the header",
        Box::new(|r| flip_at(&mut r["header"], 10)),
        "header",
    );
    call(
        &mut add,
        "chainId",
        Box::new(|r| {
            r["chainId"] = json!("0x2");
            true
        }),
        "wrong-chain",
    );
    call(
        &mut add,
        "blockHash",
        Box::new(|r| flip_at(&mut r["blockHash"], 1)),
        "header",
    );
    call(
        &mut add,
        "blockNumber",
        Box::new(|r| {
            r["blockNumber"] = json!(q(n(&r["blockNumber"]) + 1));
            true
        }),
        "header",
    );
    let transfer = fixture("call/estimate-transfer.json");
    let ask_est = || ask("eth_estimateGas", json!([transfer["request"]["params"][0]]));
    let est = |add: &mut dyn FnMut(&str, u64, &'static str, Ask, Mutation, Expect),
               name: &str,
               f: Box<dyn Fn(&mut Value) -> bool + Send + Sync>,
               kind: &'static str| {
        add(
            &format!("estimate: {name}"),
            1,
            "w2",
            ask_est(),
            on("willow_getEstimateGasProof", move |r, _| f(r)),
            v(kind),
        );
    };
    est(
        &mut add,
        "another algorithm",
        Box::new(|r| {
            r["algorithm"] = json!("geth-v1.16.0-estimate");
            true
        }),
        "algorithm",
    );
    est(
        &mut add,
        "the estimate + 1",
        Box::new(|r| {
            r["estimate"] = json!({"gas": q(n(&r["estimate"]["gas"]) + 1)});
            true
        }),
        "estimate",
    );
    est(
        &mut add,
        "the estimate - 1",
        Box::new(|r| {
            r["estimate"] = json!({"gas": q(n(&r["estimate"]["gas"]) - 1)});
            true
        }),
        "estimate",
    );
    est(
        &mut add,
        "an account dropped from the witness",
        Box::new(|r| {
            r["witness"]["accounts"]
                .as_array_mut()
                .is_some_and(|a| a.pop().is_some())
        }),
        "missing-read",
    );

    // ---- transaction, receipt and block-receipts proofs ----
    let items = |f: &str| json_file(&items_dir().join(f));
    let receipt = items("eth-receipt.json");
    let ask_receipt = || ask("eth_getTransactionReceipt", json!([receipt["target"]]));
    add(
        "receipt: a proof node",
        1,
        "w1",
        ask_receipt(),
        on("willow_getReceiptProof", |r, _| {
            r["receiptProofs"]
                .as_array_mut()
                .and_then(|a| a.last_mut())
                .is_some_and(|x| flip_at(&mut x["proof"][0], 40))
        }),
        v("receipts"),
    );
    add(
        "receipt: a byte of the header",
        1,
        "w1",
        ask_receipt(),
        on("willow_getReceiptProof", |r, _| {
            flip_at(&mut r["header"], 100)
        }),
        v("header"),
    );
    let tx = items("arb-tx.json");
    add(
        "transaction: a proof node",
        42161,
        "w1",
        ask("eth_getTransactionByHash", json!([tx["target"]])),
        on("willow_getTransactionProof", |r, _| {
            r["transactionProofs"]
                .as_array_mut()
                .and_then(|a| a.last_mut())
                .is_some_and(|x| flip_at(&mut x["proof"][0], 40))
        }),
        v("transactions"),
    );
    let uni = items("uni-block.json");
    add(
        "block receipts: a receipt",
        130,
        "w1",
        ask("eth_getBlockReceipts", json!([uni["target"]])),
        on("willow_getBlockReceiptsProof", |r, _| {
            flip_at(&mut r["receipts"][0], 20)
        }),
        v("receipts"),
    );
    {
        let target = receipt["target"].as_str().unwrap().to_string();
        add(
            "receipt: \"not found\" for a receipt the server has is an absence",
            1,
            "w1",
            ask_receipt(),
            Mutation::Item(Arc::new(move |resp: &mut Value, m: &str, _: &[Value]| {
                if m != "willow_getReceiptProof" {
                    return false;
                }
                let o = resp.as_object_mut().unwrap();
                o.remove("result");
                o.insert("error".into(), json!({"code": -32000, "message": format!("not found in this store: transaction {target} is not in the full-block tier")}));
                true
            })),
            Expect::Null,
        );
    }

    // ---- anchors ----
    let old_number = e17["blockNumber"].clone();
    {
        let num = old_number.clone();
        add(
            "anchor: the anchor of another block",
            1,
            "w1",
            ask("eth_getBlockByNumber", json!([old_number, false])),
            Mutation::Item(Arc::new(move |resp: &mut Value, m: &str, p: &[Value]| {
                if m != "willow_getBlockAnchor" || p.first() != Some(&num) {
                    return false;
                }
                resp["result"] = REPLAY.lock().unwrap().clone().unwrap_or(Value::Null);
                true
            })),
            v("anchor-rejected"),
        );
    }
    {
        let num = old_number.clone();
        add(
            "anchor: a hash the validators did not sign",
            1,
            "w1",
            ask("eth_getBlockByNumber", json!([old_number, false])),
            on("willow_getBlockAnchor", move |r, p| {
                p.first() == Some(&num) && flip_at(&mut r["mock"]["hash"], 1)
            }),
            v("anchor-rejected"),
        );
    }
    out.push(Case {
        name: "anchor: a stale head replayed (a denial: unverifiable)".into(),
        chain_id: 1,
        head: "w1",
        ask: ask("eth_blockNumber", json!([])),
        mutation: Mutation::Item(Arc::new(|resp: &mut Value, m: &str, p: &[Value]| {
            let replay = REPLAY.lock().unwrap().clone();
            if m != "willow_getBlockAnchor"
                || p.first() != Some(&json!("latest"))
                || replay.is_none()
            {
                return false;
            }
            resp["result"] = replay.unwrap();
            true
        })),
        expect: Expect::Err(-32099, "anchor-stale-head"),
        replay_after_sec: 1200,
        warm: true,
        setup: None,
    });

    // ---- batches and endpoints ----
    let both = || -> Ask {
        let bal = ask_balance();
        let c = ask(
            "eth_call",
            json!([
                fixture("call/call-usdc-balance.json")["request"]["params"][0],
                "latest"
            ]),
        );
        Arc::new(move |v: Verifier| -> Fut {
            let (b, c) = (bal(v.clone()), c(v));
            Box::pin(async move {
                let (x, y) = tokio::join!(b, c);
                Ok(json!([x?, y?]))
            })
        })
    };
    let mut push = |c: Case| out.push(c);
    let mk = |name: &str,
              chain_id: u64,
              head: &'static str,
              ask: Ask,
              mutation: Mutation,
              expect: Expect| Case {
        name: name.into(),
        chain_id,
        head,
        ask,
        mutation,
        expect,
        replay_after_sec: 0,
        warm: true,
        setup: None,
    };
    push(mk(
        "batch: an item dropped",
        1,
        "w2",
        both(),
        Mutation::Batch(Arc::new(
            |resps: &mut Vec<Value>, reqs: &[(Value, String, Vec<Value>)]| {
                let Some((id, _, _)) = reqs.iter().find(|(_, m, _)| m == "willow_getCallProof")
                else {
                    return false;
                };
                let Some(i) = resps.iter().position(|x| x["id"] == *id) else {
                    return false;
                };
                resps.remove(i);
                true
            },
        )),
        Expect::Err(-32098, "batch"),
    ));
    push(mk(
        "batch: an item twice",
        1,
        "w2",
        both(),
        Mutation::Batch(Arc::new(
            |resps: &mut Vec<Value>, reqs: &[(Value, String, Vec<Value>)]| {
                if !reqs.iter().any(|(_, m, _)| m == "willow_getCallProof") {
                    return false;
                }
                let first = resps[0].clone();
                resps.push(first);
                true
            },
        )),
        Expect::Err(-32098, "batch"),
    ));
    let two = || -> Ask {
        let a = ask(
            "eth_getBlockByHash",
            json!([
                fixture("block/ethereum-17034874.json")["result"]["blockHash"],
                false
            ]),
        );
        let b = ask(
            "eth_getBlockByHash",
            json!([
                fixture("block/ethereum-1500000.json")["result"]["blockHash"],
                false
            ]),
        );
        Arc::new(move |v: Verifier| -> Fut {
            let (x, y) = (a(v.clone()), b(v));
            Box::pin(async move {
                let (x, y) = tokio::join!(x, y);
                Ok(json!([x?, y?]))
            })
        })
    };
    push(mk(
        "batch: two items' ids swapped",
        1,
        "w1",
        two(),
        Mutation::Batch(Arc::new(
            |resps: &mut Vec<Value>, reqs: &[(Value, String, Vec<Value>)]| {
                let ids: Vec<Value> = reqs
                    .iter()
                    .filter(|(_, m, _)| m == "willow_getBlockProof")
                    .map(|(id, _, _)| id.clone())
                    .collect();
                if ids.len() < 2 {
                    return false;
                }
                let i = resps.iter().position(|x| x["id"] == ids[0]).unwrap();
                let j = resps.iter().position(|x| x["id"] == ids[1]).unwrap();
                resps[i]["id"] = ids[1].clone();
                resps[j]["id"] = ids[0].clone();
                true
            },
        )),
        Expect::Err(-32098, "wrong-block"),
    ));
    {
        let (h17, h15) = (e17["blockHash"].clone(), pre["blockHash"].clone());
        push(mk(
            "batch: items reordered, ids kept (valid: the values are right)",
            1,
            "w1",
            two(),
            Mutation::Batch(Arc::new(
                |resps: &mut Vec<Value>, _: &[(Value, String, Vec<Value>)]| {
                    if resps.len() < 2 {
                        return false;
                    }
                    resps.reverse();
                    true
                },
            )),
            Expect::Values(Arc::new(move |v: &Value| {
                v[0]["hash"] == h17 && v[1]["hash"] == h15
            })),
        ));
    }
    let mut c = mk(
        "endpoint: eth_chainId of another chain",
        8453,
        "w1",
        ask_block(),
        Mutation::Item(Arc::new(|resp: &mut Value, m: &str, _: &[Value]| {
            if m != "eth_chainId" {
                return false;
            }
            resp["result"] = json!("0xa");
            true
        })),
        Expect::Err(-32098, "wrong-chain"),
    );
    c.warm = false;
    push(c);
    let raw = super::tx::raw_1559();
    let mut c = mk(
        "eth_sendRawTransaction: another hash",
        1,
        "w2",
        ask("eth_sendRawTransaction", json!([raw])),
        Mutation::Item(Arc::new(|resp: &mut Value, m: &str, _: &[Value]| {
            if m != "eth_sendRawTransaction" {
                return false;
            }
            resp["result"] = json!(format!("0x{}", "00".repeat(32)));
            true
        })),
        Expect::Err(-32098, "tx-hash"),
    );
    c.setup = Some(Arc::new(|w: &World| {
        w.intercept(Some(Arc::new(|_, m, p| {
            (m == "eth_sendRawTransaction").then(|| {
                let b = crate::hex::to_bytes(p[0].as_str().unwrap()).unwrap();
                json!({"result": crate::hex::keccak_hex(&b)})
            })
        })));
    }));
    push(c);
    out
}

/// The `latest` anchor the warm-up saw, for replays.
static REPLAY: std::sync::Mutex<Option<Value>> = std::sync::Mutex::new(None);

#[tokio::test(flavor = "multi_thread")]
async fn the_lying_server() {
    let table = cases();
    let names: Vec<&str> = table.iter().map(|c| c.name.as_str()).collect();
    for want in [
        "the anchor of another block",
        "stale head",
        "another chain",
        "another block for a blockHash request",
        "an item dropped",
        "ids swapped",
        "a lower pending nonce",
    ] {
        assert!(names.iter().any(|n| n.contains(want)), "{want}");
    }
    let mut failures = vec![];
    for c in table {
        let world = full_world();
        if let Some(s) = &c.setup {
            s(&world);
        }
        if c.head == "w2" {
            let b = w2_block();
            world.head(1, b.0, &b.1, "signed", "head");
        } else {
            world.head_at(c.chain_id, &block_of(head_fixture(c.chain_id)));
        }
        let h = harness_with(c.chain_id, world, "test-key", |x| x).await;
        *REPLAY.lock().unwrap() = None;
        if c.warm {
            h.ask("eth_blockNumber", json!([]))
                .await
                .unwrap_or_else(|e| panic!("{}: warm-up: {e}", c.name));
            let seen = h
                .world
                .s()
                .seen
                .get(&format!("willow_getBlockAnchor {}", json!(["latest"])))
                .and_then(|v| v.last().cloned());
            *REPLAY.lock().unwrap() = seen.map(|s| s["result"].clone());
        }
        if c.replay_after_sec > 0 {
            let mut s = h.world.s();
            s.now_sec += c.replay_after_sec;
            let now = s.now_sec;
            // The honest server's head moves on with the clock: only the replay is stale.
            if let Some(head) = s.heads.get_mut(&c.chain_id) {
                head.time = now - 2;
            }
        }
        {
            let mut s = h.world.s();
            s.mutation = Some(c.mutation.clone());
            s.applied = 0;
        }
        let r = (c.ask)(h.client.clone()).await;
        let applied = h.world.s().applied;
        h.world.s().mutation = None;
        if applied == 0 {
            failures.push(format!("{}: the mutation did not apply", c.name));
            continue;
        }
        let ok = match (&c.expect, &r) {
            (Expect::Err(code, kind), Err(e)) => e.code() == *code && e.kind() == Some(*kind),
            (Expect::Null, Ok(v)) => v.is_null(),
            (Expect::Values(f), Ok(v)) => f(v),
            _ => false,
        };
        if !ok {
            failures.push(format!("{}: got {:?}", c.name, r.map_err(|e| e.to_json())));
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}
