//! What can't be verified: absences and refusals pass through; values that can't be
//! verified raise -32099 or, with `flag`, pass marked; a proof that fails always raises -32098
//! (policy.test.ts).

use super::support::*;
use crate::config::{Level, OpenVmVerifier, Status, Unverifiable};
use crate::{Error, UNVERIFIABLE, VERIFICATION_FAILED};
use serde_json::{json, Value};
use std::sync::Arc;

fn base() -> (u64, String) {
    block_of("base-56049971.json")
}
fn eth() -> (u64, String) {
    block_of("ethereum-26391511.json")
}

async fn on(chain_id: u64, u: Unverifiable) -> Harness {
    let h = harness(chain_id, move |c| c.unverifiable(u)).await;
    h.world
        .head_at(chain_id, &if chain_id == 1 { eth() } else { base() });
    h
}

#[test]
fn error_codes_are_32098_and_32099_with_data_kind() {
    assert_eq!(VERIFICATION_FAILED, -32098);
    assert_eq!(UNVERIFIABLE, -32099);
    let e = Error::verification("header", "x", Some("eth_getBlockByNumber"));
    assert_eq!(
        e.to_json(),
        json!({"code": -32098, "message": "Willow's response failed verification (header): x",
            "data": {"kind": "header", "detail": "x", "method": "eth_getBlockByNumber"}})
    );
    assert_eq!(
        Error::unverifiable("tag", "", None).to_json()["data"],
        json!({"kind": "tag", "detail": ""})
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn l2_safe_and_finalized_throw_by_default_and_pass_marked_with_flag() {
    let h = on(8453, Unverifiable::Throw).await;
    let e = h
        .ask("eth_getBlockByNumber", json!(["safe", false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("tag".into())));
    let h = on(8453, Unverifiable::Flag).await;
    h.world.standard(
        &key(8453, "eth_getBlockByNumber", &json!(["finalized", false])),
        json!({"result": {"number": q(base().0)}}),
    );
    let b = h
        .ask("eth_getBlockByNumber", json!(["finalized", false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(base().1));
    let e = h.last();
    assert_eq!(
        (e.status, e.kind.as_deref(), e.level),
        (Status::Unverifiable, Some("tag"), Some(Level::Signed))
    );
    assert_eq!(e.block.map(|b| (b.number, b.hash)), Some(base()));
}

#[tokio::test(flavor = "multi_thread")]
async fn ethereum_safe_is_refused_as_the_server_refuses_it() {
    let h = on(1, Unverifiable::Throw).await;
    let e = h
        .ask(
            "eth_getBalance",
            json!([format!("0x{}", "11".repeat(20)), "safe"]),
        )
        .await
        .unwrap_err();
    assert_eq!(
        e,
        Error::rpc(-32000, "not found in this store: the \"safe\" block tag")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_method_without_a_proof_throws_passes_marked_with_flag_and_its_errors_pass_through() {
    let h = on(1, Unverifiable::Throw).await;
    h.world.standard(
        "1|eth_coinbase|*",
        json!({"result": format!("0x{}", "22".repeat(20))}),
    );
    assert_eq!(
        code_kind(&h.ask("eth_coinbase", json!([])).await.unwrap_err()),
        (-32099, Some("method".into()))
    );
    let e = h.ask("eth_mining", json!([])).await.unwrap_err();
    assert!(matches!(e, Error::Rpc { code: -32601, .. }));
    assert_eq!(h.last().status, Status::Refused);
    let h = on(1, Unverifiable::Flag).await;
    h.world.standard(
        "1|eth_coinbase|*",
        json!({"result": format!("0x{}", "22".repeat(20))}),
    );
    assert_eq!(
        h.ask("eth_coinbase", json!([])).await.unwrap(),
        json!(format!("0x{}", "22".repeat(20)))
    );
    let e = h.last();
    assert_eq!(
        (e.status, e.kind.as_deref()),
        (Status::Unverifiable, Some("method"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn l2_eth_call_and_estimate_refusals_pass_through() {
    let h = on(8453, Unverifiable::Throw).await;
    h.world.standard(
        "8453|eth_call|*",
        json!({"error": {"code": -32601, "message": "eth_call is not served on chain 8453"}}),
    );
    h.world.standard(
        "8453|eth_estimateGas|*",
        json!({"error": {"code": -32601, "message": "not served"}}),
    );
    let to = json!({"to": format!("0x{}", "11".repeat(20))});
    assert_eq!(
        h.ask("eth_call", json!([to, "latest"]))
            .await
            .unwrap_err()
            .code(),
        -32601
    );
    assert_eq!(
        h.ask("eth_estimateGas", json!([to]))
            .await
            .unwrap_err()
            .code(),
        -32601
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn overrides_the_verifier_does_not_run_make_the_servers_value_unverifiable() {
    let h = on(1, Unverifiable::Throw).await;
    h.world.standard("1|eth_call|*", json!({"result": "0x01"}));
    let a = format!("0x{}", "11".repeat(20));
    let e = h
        .ask("eth_call", json!([{"to": a}, "latest", {a.clone(): {"movePrecompileToAddress": format!("0x{}", "22".repeat(20))}}]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("method".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn local_methods_are_served_here() {
    let h = on(8453, Unverifiable::Throw).await;
    assert_eq!(
        h.ask("eth_chainId", json!([])).await.unwrap(),
        json!("0x2105")
    );
    assert_eq!(
        h.ask("net_version", json!([])).await.unwrap(),
        json!("8453")
    );
    assert_eq!(h.ask("eth_accounts", json!([])).await.unwrap(), json!([]));
    assert_eq!(h.ask("eth_syncing", json!([])).await.unwrap(), json!(false));
    assert!(h
        .ask("web3_clientVersion", json!([]))
        .await
        .unwrap()
        .as_str()
        .unwrap()
        .starts_with("willow-rpc/"));
    assert!(h.events().iter().all(|e| e.status == Status::Local));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proof_method_not_served_for_this_key_is_unverifiable_and_with_flag_the_standard_response_passes(
) {
    let h = on(8453, Unverifiable::Throw).await;
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getBlockProof")
            .then(|| json!({"error": {"code": -32601, "message": "no proofs on this plan"}}))
    })));
    let e = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("not-on-plan".into())));
    let h = on(8453, Unverifiable::Flag).await;
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getBlockProof")
            .then(|| json!({"error": {"code": -32601, "message": "no proofs"}}))
    })));
    h.world.standard(
        &key(8453, "eth_getBlockByNumber", &json!(["latest", false])),
        json!({"result": {"number": "0x1", "plain": true}}),
    );
    assert_eq!(
        h.ask("eth_getBlockByNumber", json!(["latest", false]))
            .await
            .unwrap(),
        json!({"number": "0x1", "plain": true})
    );
    let e = h.last();
    assert_eq!(
        (e.status, e.kind.as_deref()),
        (Status::Unverifiable, Some("not-on-plan"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn state_on_the_public_path_is_unverifiable() {
    let h = harness_with(1, full_world(), "public", |c| c).await;
    h.world.head_at(1, &eth());
    let e = h
        .ask(
            "eth_getBalance",
            json!([format!("0x{}", "11".repeat(20)), "latest"]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("not-on-plan".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn any_other_refusal_passes_through_as_sent() {
    let h = on(8453, Unverifiable::Throw).await;
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getBlockProof").then(|| json!({"error": {"code": -32005, "message": "rate limited", "data": {"retryAfterSeconds": 1}}}))
    })));
    let e = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap_err();
    assert_eq!(
        e,
        Error::Rpc {
            code: -32005,
            message: "rate limited".into(),
            data: Some(json!({"retryAfterSeconds": 1}))
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_anchor_the_server_cant_give_makes_the_block_unverifiable_and_with_flag_it_passes_marked(
) {
    let old = block_of("ethereum-17034874.json");
    let h = on(1, Unverifiable::Throw).await;
    h.world.s().anchors.remove(&format!("1:{}", old.0));
    let e = h
        .ask("eth_getBlockByHash", json!([old.1, false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-refused".into())));
    let h = on(1, Unverifiable::Flag).await;
    h.world.s().anchors.remove(&format!("1:{}", old.0));
    let b = h
        .ask("eth_getBlockByHash", json!([old.1, false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(old.1));
    let e = h.last();
    assert_eq!(
        (e.status, e.kind.as_deref()),
        (Status::Unverifiable, Some("anchor-refused"))
    );
    assert_eq!(e.block.map(|b| (b.number, b.hash)), Some(old));
}

#[tokio::test(flavor = "multi_thread")]
async fn eth_get_logs_over_more_than_8_blocks_without_log_proofs_is_unverifiable() {
    let h = on(1, Unverifiable::Throw).await;
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getLogsProof")
            .then(|| json!({"error": {"code": -32601, "message": "its plan has no proofs"}}))
    })));
    let e = h
        .ask(
            "eth_getLogs",
            json!([{"fromBlock": q(eth().0 - 20), "toBlock": "latest"}]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("logs-range".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_openvm_log_proof_without_its_verifier_is_unverifiable_not_a_failure() {
    let h = harness(1, |c| c.openvm_verifier(OpenVmVerifier::None)).await;
    h.world.head_at(1, &eth());
    let (from, to) = (eth().0 - 20, eth().0);
    h.world.intercept(Some(Arc::new(move |_, m, _| match m {
        "willow_getLogsProof" => Some(
            json!({"result": {"chainId": "0x1", "fromBlock": q(from), "toBlock": q(to),
            "parts": [{"type": "block", "publicValues": "0x", "proof": "p1", "records": "0x"}]}}),
        ),
        "willow_getProof" => {
            Some(json!({"result": {"proof": format!("0x{}", hex::encode(b"OVM2xxxx"))}}))
        }
        _ => None,
    })));
    let e = h
        .ask(
            "eth_getLogs",
            json!([{"fromBlock": q(from), "toBlock": "latest"}]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("no-verifier".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn levels_a_block_below_min_level_is_unverifiable_and_an_l2_latest_cant_be_committed() {
    let h = harness(8453, |c| c.min_level(Level::Committed)).await;
    h.world.head_at(8453, &base());
    let e = h
        .ask("eth_getBlockByHash", json!([base().1, false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-level".into())));
    let e = h.ask("eth_blockNumber", json!([])).await.unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-level".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn on_ethereum_at_min_level_final_latest_is_the_finalized_block() {
    let h = harness(1, |c| c.min_level(Level::Final)).await;
    let old = block_of("ethereum-17034874.json");
    let now = h.world.now_sec();
    let e = AnchorEntry {
        chain_id: 1,
        number: old.0,
        hash: old.1.clone(),
        level: "final",
        path: "history",
        time: now - 10,
    };
    h.world.s().finalized.insert(1, e.clone());
    h.world.anchor(e);
    assert_eq!(
        h.ask("eth_blockNumber", json!([])).await.unwrap(),
        json!(q(old.0))
    );
    let b = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(old.1));
    assert_eq!(h.last().level, Some(Level::Final));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_proof_that_fails_always_throws_also_with_flag() {
    let h = on(8453, Unverifiable::Flag).await;
    let e = h
        .world
        .proof(8453, "willow_getBlockProof", json!([base().1]));
    let mut lie = e.result.clone();
    lie["transactions"].as_array_mut().unwrap().remove(0);
    h.world.intercept(Some(Arc::new(move |_, m, _| {
        (m == "willow_getBlockProof").then(|| json!({"result": lie}))
    })));
    let err = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&err), (-32098, Some("transactions".into())));
    let ev = h.last();
    assert_eq!(
        (ev.status, ev.kind.as_deref()),
        (Status::Failed, Some("transactions"))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_endpoint_for_another_chain_fails_every_request() {
    let h = on(8453, Unverifiable::Throw).await;
    h.world
        .standard("8453|eth_chainId|*", json!({"result": "0xa"}));
    assert_eq!(
        code_kind(&h.ask("eth_chainId", json!([])).await.unwrap_err()),
        (-32098, Some("wrong-chain".into()))
    );
    assert!(matches!(
        h.ask("eth_getBlockByNumber", json!(["latest", false]))
            .await
            .unwrap_err(),
        Error::Verification(_)
    ));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_block_parameter_is_invalid_params() {
    let h = on(1, Unverifiable::Throw).await;
    for b in [
        json!(5),
        json!("0xzz"),
        json!({"blockHash": "0x12"}),
        json!(true),
    ] {
        let e = h
            .ask("eth_getBlockByNumber", json!([b, false]))
            .await
            .unwrap_err();
        assert_eq!(e.code(), -32602, "{b}");
    }
    let _: Value = h
        .ask(
            "eth_getBlockByNumber",
            json!([{"blockNumber": q(eth().0)}, false]),
        )
        .await
        .unwrap();
}
