//! What the crates.io release holds back (`client::HELD`): the fee formula, the gas estimate and
//! Robinhood Chain's calls are refused as unverifiable, before any proof is asked for.

use super::support::*;
use crate::config::{Status, Unverifiable};
use serde_json::json;
use std::sync::Arc;

fn held(c: crate::Config) -> crate::Config {
    let mut c = c;
    c.unheld = false;
    c
}

#[tokio::test(flavor = "multi_thread")]
async fn held_methods_are_unverifiable_and_ask_no_proof() {
    let h = harness(1, held).await;
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    for (m, p) in [
        ("eth_gasPrice", json!([])),
        ("eth_maxPriorityFeePerGas", json!([])),
        ("eth_blobBaseFee", json!([])),
        ("eth_feeHistory", json!(["0x4", "latest", []])),
        (
            "eth_estimateGas",
            json!([{"to": "0x0000000000000000000000000000000000000001"}]),
        ),
    ] {
        let e = h.ask(m, p).await.unwrap_err();
        assert_eq!(code_kind(&e), (-32099, Some("no-verifier".into())), "{m}");
        assert!(
            e.message().contains("not verified by this release"),
            "{m}: {e}"
        );
    }
    assert!(h.world.sent("willow_getFeeProof").is_empty());
    assert!(h.world.sent("willow_getFeeHistoryProof").is_empty());
    assert!(h.world.sent("willow_getEstimateGasProof").is_empty());
    // Ethereum's calls stay verified (they're asked for as usual).
    let _ = h
        .ask(
            "eth_call",
            json!([{"to": "0x0000000000000000000000000000000000000001"}, "latest"]),
        )
        .await;
    assert!(!h.world.sent("willow_getCallProof").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn robinhood_calls_are_held_and_flag_returns_the_servers_value() {
    let h = harness(4663, |c| held(c).unverifiable(Unverifiable::Flag)).await;
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "eth_call").then(|| json!({"result": "0x2a"}))
    })));
    let v = h
        .ask(
            "eth_call",
            json!([{"to": "0x0000000000000000000000000000000000000001"}, "latest"]),
        )
        .await
        .unwrap();
    assert_eq!(v, json!("0x2a"));
    assert_eq!(h.last().status, Status::Unverifiable);
    assert_eq!(h.last().kind.as_deref(), Some("no-verifier"));
    assert!(h.world.sent("willow_getCallProof").is_empty());
}
