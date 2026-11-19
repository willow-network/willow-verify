//! Nothing is held back from this release (`client::HELD` is false): with the release's own setting,
//! the fee methods, gas estimates and Robinhood Chain's and Arbitrum One's calls ask for their proofs
//! as every other method does, and none is refused as a method this release doesn't verify.

use super::support::*;
use serde_json::json;

fn released(c: crate::Config) -> crate::Config {
    let mut c = c;
    c.unheld = false;
    c
}

fn not_held(m: &str, r: &Result<serde_json::Value, crate::Error>) {
    if let Err(e) = r {
        assert!(
            !e.message().contains("not verified by this release"),
            "{m}: {e}"
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn fee_methods_and_estimates_ask_their_proofs() {
    let h = harness(1, released).await;
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
        not_held(m, &h.ask(m, p).await);
    }
    assert!(!h.world.sent("willow_getFeeProof").is_empty());
    assert!(!h.world.sent("willow_getFeeHistoryProof").is_empty());
    assert!(!h.world.sent("willow_getEstimateGasProof").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn robinhood_and_arbitrum_one_calls_ask_their_proofs() {
    for chain in [4663, 42161] {
        let h = harness(chain, released).await;
        h.world.head_at(chain, &block_of(head_fixture(chain)));
        let r = h
            .ask(
                "eth_call",
                json!([{"to": "0x0000000000000000000000000000000000000001"}, "latest"]),
            )
            .await;
        not_held("eth_call", &r);
        assert!(
            !h.world.sent("willow_getCallProof").is_empty(),
            "chain {chain}"
        );
    }
}
