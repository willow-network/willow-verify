//! Traces without a proof (trace.test.ts): calls' traces off Ethereum and other chains' traces are
//! unverifiable, never asked for as a proof; and trace_filter's matching is the server's. Plus the
//! anchor expiries anchor.routes.test.ts pins.

use super::support::*;
use crate::client::trace_filtered;
use crate::config::{Status, Unverifiable};
use serde_json::{json, Value};

fn a() -> String {
    format!("0x{}", "aa".repeat(20))
}
fn b() -> String {
    format!("0x{}", "bb".repeat(20))
}
fn c() -> String {
    format!("0x{}", "cc".repeat(20))
}

fn traces() -> Vec<Value> {
    let call = |from: String, to: String| json!({"type": "call", "action": {"from": from, "to": to, "callType": "call"}, "result": {}});
    let create = |from: String, address: Option<String>| json!({"type": "create", "action": {"from": from}, "result": address.map(|x| json!({"address": x})).unwrap_or(Value::Null)});
    vec![
        call(a(), b()),
        call(b(), c()),
        create(a(), Some(c())),
        create(b(), None),
        json!({"type": "suicide", "action": {"address": c(), "refundAddress": a()}}),
        json!({"type": "reward", "action": {"author": b()}}),
    ]
}

fn pick(t: &[Value], i: &[usize]) -> Vec<Value> {
    i.iter().map(|&i| t[i].clone()).collect()
}

#[test]
fn trace_filter_matches_as_the_server_does() {
    let t = traces();
    let all = [Value::Array(t.clone())];
    assert_eq!(trace_filtered(&json!({}), &all), t);
    assert_eq!(
        trace_filtered(&json!({"fromAddress": [a()]}), &all),
        pick(&t, &[0, 2])
    );
    assert_eq!(
        trace_filtered(&json!({"toAddress": [c()]}), &all),
        pick(&t, &[1, 2])
    );
    assert_eq!(
        trace_filtered(&json!({"fromAddress": [a()], "toAddress": [c()]}), &all),
        pick(&t, &[0, 1, 2])
    );
    // A creation with no address matches a to-filter only when there is none; a reward's author is its "to".
    assert_eq!(
        trace_filtered(&json!({"toAddress": [b()]}), &all),
        pick(&t, &[0, 5])
    );
    assert_eq!(
        trace_filtered(&json!({"toAddress": [a()]}), &all),
        pick(&t, &[4])
    );
    assert_eq!(
        trace_filtered(
            &json!({"fromAddress": [a()], "toAddress": [c()], "mode": "intersection"}),
            &all
        ),
        pick(&t, &[2])
    );
    assert_eq!(
        trace_filtered(&json!({"fromAddress": [b()], "mode": "intersection"}), &all),
        pick(&t, &[1, 3])
    );
    let split = [Value::Array(t[..2].to_vec()), Value::Array(t[2..].to_vec())];
    assert_eq!(
        trace_filtered(&json!({"after": 1, "count": 2}), &split),
        pick(&t, &[1, 2])
    );
    assert_eq!(
        trace_filtered(&json!({"after": "0x2", "count": "0x1"}), &all),
        pick(&t, &[2])
    );
    assert_eq!(
        trace_filtered(&json!({"after": 6}), &all),
        Vec::<Value>::new()
    );
    assert_eq!(
        trace_filtered(
            &json!({"fromAddress": [a().to_ascii_uppercase().replace("0X", "0x")]}),
            &all
        ),
        pick(&t, &[0, 2])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_traces_on_unichain_are_unverifiable_and_with_flag_pass_marked() {
    for (method, params) in [
        (
            "debug_traceCall",
            json!([{"to": a(), "data": "0x"}, "latest", {"tracer": "callTracer"}]),
        ),
        (
            "trace_call",
            json!([{"to": a(), "data": "0x"}, ["trace"], "latest"]),
        ),
    ] {
        for flag in [false, true] {
            let w = World::new();
            w.standard(
                &format!("130|{method}|*"),
                json!({"result": {"proven": false}}),
            );
            w.head(
                130,
                100,
                &format!("0x{}", "12".repeat(32)),
                "signed",
                "sequencer",
            );
            let h = harness_with(130, w, "test-key", move |x| {
                x.unverifiable(if flag {
                    Unverifiable::Flag
                } else {
                    Unverifiable::Throw
                })
            })
            .await;
            let r = h.ask(method, params.clone()).await;
            if flag {
                assert_eq!(r.unwrap(), json!({"proven": false}));
                assert_eq!(h.last().status, Status::Unverifiable);
            } else {
                assert_eq!(code_kind(&r.unwrap_err()), (-32099, Some("method".into())));
            }
            assert!(h.world.sent("willow_getTraceProof").is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn another_chains_traces_are_unverifiable_never_asked_as_a_proof() {
    let w = World::new();
    w.standard("8453|trace_transaction|*", json!({"result": []}));
    let h = harness_with(8453, w, "test-key", |x| x).await;
    let e = h
        .ask(
            "trace_transaction",
            json!([format!("0x{}", "11".repeat(32))]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("method".into())));
    assert!(h.world.sent("willow_getTraceProof").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bnb_block_one_validator_sealed_counts_10_minutes_from_its_time() {
    let w = World::new();
    let now = w.now_sec();
    let hash = format!("0x{}", "11".repeat(32));
    w.head(56, 20, &format!("0x{}", "22".repeat(32)), "final", "parlia");
    w.anchor(AnchorEntry {
        chain_id: 56,
        number: 16,
        hash: hash.clone(),
        level: "signed",
        path: "parlia",
        time: now - 10,
    });
    let h = harness_with(56, w, "test-key", |x| x).await;
    let a = h.client.inner.anchors();
    let got = a.block(56, 16, &hash, None, None).await.unwrap();
    assert_eq!(got.expires, (now - 10 + 600) * 1000);
    // BNB's newest block is final, so it's served at min level final; an L2's newest is not.
    let h2 = harness_with(56, h.world.clone(), "test-key", |x| {
        x.min_level(crate::Level::Final)
    })
    .await;
    assert_eq!(
        h2.ask("eth_blockNumber", json!([])).await.unwrap(),
        json!("0x14")
    );
    let w3 = World::new();
    w3.head(
        1868,
        50,
        &format!("0x{}", "33".repeat(32)),
        "signed",
        "sequencer",
    );
    let h3 = harness_with(1868, w3, "test-key", |x| x.min_level(crate::Level::Final)).await;
    assert_eq!(
        code_kind(&h3.ask("eth_blockNumber", json!([])).await.unwrap_err()),
        (-32099, Some("anchor-level".into()))
    );
}
