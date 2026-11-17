//! Fees: eth_gasPrice, eth_maxPriorityFeePerGas, eth_blobBaseFee and
//! eth_feeHistory, on the fee fixtures from all seven chains (fees.test.ts).

use super::support::*;
use serde_json::{json, Value};

/// gasUsedRatio within a ulp: serde_json's float parse of the fixture and the verifier's own
/// computation may differ in the last bit; every other field exactly.
fn same_history(got: &Value, want: &Value) -> bool {
    let (Some(g), Some(w)) = (got.as_object(), want.as_object()) else {
        return false;
    };
    if g.len() != w.len() {
        return false;
    }
    for (k, wv) in w {
        let Some(gv) = g.get(k) else { return false };
        if k.ends_with("Ratio") {
            let (Some(ga), Some(wa)) = (gv.as_array(), wv.as_array()) else {
                return false;
            };
            if ga.len() != wa.len()
                || ga
                    .iter()
                    .zip(wa)
                    .any(|(a, b)| (a.as_f64().unwrap() - b.as_f64().unwrap()).abs() > 1e-12)
            {
                return false;
            }
        } else if gv != wv {
            return false;
        }
    }
    true
}

#[tokio::test(flavor = "multi_thread")]
async fn fee_methods_on_every_chain() {
    for file in fixture_names("fees")
        .into_iter()
        .filter(|f| f.ends_with(".fee.json"))
    {
        let chain_id = chain_of(&file);
        let f = fixture(&format!("fees/{file}"));
        let h = harness(chain_id, |c| c).await;
        h.world.head_at(chain_id, &block_of(head_fixture(chain_id)));
        let (gas, tip) = tokio::join!(
            h.ask("eth_gasPrice", json!([])),
            h.ask("eth_maxPriorityFeePerGas", json!([]))
        );
        let (gas, tip) = (gas.unwrap(), tip.unwrap());
        assert_eq!(gas, f["verified"]["eth_gasPrice"], "{file}");
        assert_eq!(tip, f["verified"]["eth_maxPriorityFeePerGas"], "{file}");
        // Both come from one fee proof at the head.
        assert_eq!(h.world.sent("willow_getFeeProof").len(), 1, "{file}");
        let blob = h.ask("eth_blobBaseFee", json!([])).await;
        if f["verified"]["eth_blobBaseFee"].is_null() {
            assert_eq!(blob.unwrap_err().code(), -32601, "{file}");
        } else {
            assert_eq!(blob.unwrap(), f["verified"]["eth_blobBaseFee"], "{file}");
        }
        // Where the public node runs the same formula (the OP stack's tip), the values agree.
        if [8453, 10, 130, 57073].contains(&chain_id) {
            assert_eq!(
                tip, f["node"]["responses"]["eth_maxPriorityFeePerGas"],
                "{file}"
            );
        }
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn fee_history_on_every_chain() {
    for file in fixture_names("fees")
        .into_iter()
        .filter(|f| f.ends_with(".history.json"))
    {
        let chain_id = chain_of(&file);
        let f = fixture(&format!("fees/{file}"));
        let h = harness(chain_id, |c| c).await;
        h.world.head_at(chain_id, &block_of(head_fixture(chain_id)));
        let p = f["request"]["params"].as_array().unwrap().clone();
        let got = h
            .ask("eth_feeHistory", json!([p[0], p[1], p[2]]))
            .await
            .unwrap();
        assert!(
            same_history(&got, &f["verified"]),
            "{file}: {got} vs {}",
            f["verified"]
        );
        // "latest" is the anchored head, the same block.
        let got = h
            .ask("eth_feeHistory", json!([p[0], "latest", p[2]]))
            .await
            .unwrap();
        assert!(same_history(&got, &f["verified"]), "{file} latest");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn nitro_fee_history_with_rewards_is_one_proof_whatever_the_count() {
    let h = harness(42161, |c| c).await;
    h.world
        .head_at(42161, &block_of("arbitrum-one-523278185.json"));
    let _ = h
        .ask("eth_feeHistory", json!(["0x20", "latest", [50]]))
        .await;
    let asked: Vec<Value> = h
        .world
        .sent("willow_getFeeHistoryProof")
        .iter()
        .map(|p| p[0].clone())
        .collect();
    assert_eq!(asked, vec![json!("0x20")]);
}

#[tokio::test(flavor = "multi_thread")]
async fn with_rewards_more_than_16_blocks_go_as_proofs_of_16() {
    let h = harness(8453, |c| c).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    let _ = h
        .ask("eth_feeHistory", json!(["0x14", "latest", [50]]))
        .await;
    let head = block_of("base-56049971.json").0;
    // Two proofs, in one batch or two (whose order on the wire is the network's).
    let mut asked = h.world.sent("willow_getFeeHistoryProof");
    asked.sort_by_key(|p| p.to_string());
    assert_eq!(
        asked,
        vec![
            json!(["0x10", q(head), [50]]),
            json!(["0x4", q(head - 16), [50]])
        ]
    );
}
