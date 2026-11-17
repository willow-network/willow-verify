//! State, calls and estimates on the state, call and served fixtures
//! (state.test.ts).

use super::support::*;
use crate::config::{Level, Status};
use serde_json::{json, Value};

async fn ethereum() -> Harness {
    let h = harness(1, |c| c).await;
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    h
}

#[tokio::test(flavor = "multi_thread")]
async fn an_eoas_balance_nonce_and_code_at_latest_go_out_as_one_state_proof() {
    let h = ethereum().await;
    let f = fixture("state/eoa.json");
    let a = f["request"]["params"][0].clone();
    let (bal, nonce, code) = tokio::join!(
        h.ask("eth_getBalance", json!([a, "latest"])),
        h.ask("eth_getTransactionCount", json!([a, "latest"])),
        h.ask("eth_getCode", json!([a, "latest"])),
    );
    assert_eq!(
        (bal.unwrap(), nonce.unwrap(), code.unwrap()),
        (
            f["expect"]["eth_getBalance"].clone(),
            f["expect"]["eth_getTransactionCount"].clone(),
            json!("0x")
        )
    );
    let proofs = h.world.sent("willow_getStateProof");
    assert_eq!(proofs.len(), 1);
    assert_eq!(
        proofs[0],
        json!([a, [], {"blockHash": w2_block().1}, {"code": true}])
    );
    for e in h.events() {
        assert_eq!(
            (e.status, e.level, e.block.map(|b| b.number)),
            (Status::Verified, Some(Level::Signed), Some(w2_block().0))
        );
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn an_absent_account_is_proven_absent() {
    let h = ethereum().await;
    let f = fixture("state/absent.json");
    let a = f["request"]["params"][0].clone();
    assert_eq!(
        h.ask("eth_getBalance", json!([a, "latest"])).await.unwrap(),
        json!("0x0")
    );
    assert_eq!(
        h.ask("eth_getTransactionCount", json!([a, "latest"]))
            .await
            .unwrap(),
        json!("0x0")
    );
    assert_eq!(
        h.ask("eth_getStorageAt", json!([a, "0x0", "latest"]))
            .await
            .unwrap(),
        json!(format!("0x{}", "00".repeat(32)))
    );
    assert_eq!(
        h.ask("eth_getCode", json!([a, "latest"])).await.unwrap(),
        json!("0x")
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn usdc_slots_code_get_proof_and_code_from_its_cache() {
    let h = ethereum().await;
    let f = fixture("state/usdc-slots-code.json");
    let a = f["request"]["params"][0].clone();
    let keys = f["request"]["params"][1].as_array().unwrap().clone();
    let slots = futures::future::join_all(
        keys.iter()
            .map(|k| h.ask("eth_getStorageAt", json!([a, k, "latest"]))),
    )
    .await;
    let slots: Vec<Value> = slots.into_iter().map(Result::unwrap).collect();
    assert_eq!(Value::Array(slots), f["expect"]["eth_getStorageAt"]);
    // Slots of one account at one block, asked at once, are one proof.
    assert_eq!(h.world.sent("willow_getStateProof").len(), 1);
    assert_eq!(
        h.ask("eth_getCode", json!([a, "latest"])).await.unwrap(),
        f["expect"]["eth_getCode"]
    );
    assert_eq!(
        h.ask("eth_getProof", json!([a, keys, "latest"]))
            .await
            .unwrap(),
        f["expect"]["eth_getProof"]
    );
    let before = h.world.sent("willow_getStateProof").len();
    assert_eq!(
        h.ask("eth_getCode", json!([a, "latest"])).await.unwrap(),
        f["expect"]["eth_getCode"]
    );
    let again = h.world.sent("willow_getStateProof")[before..].to_vec();
    assert_eq!(again.len(), 1);
    assert_eq!(again[0][3], json!({"code": false}));
}

#[tokio::test(flavor = "multi_thread")]
async fn base_a_slot_at_a_block_named_by_hash() {
    let h = harness(8453, |c| c).await;
    let f = fixture("state/base-usdc.json");
    let p = f["request"]["params"].as_array().unwrap().clone();
    h.world.head(
        8453,
        n(&f["result"]["blockNumber"]),
        p[2]["blockHash"].as_str().unwrap(),
        "signed",
        "sequencer",
    );
    assert_eq!(
        h.ask("eth_getStorageAt", json!([p[0], p[1][0], p[2]]))
            .await
            .unwrap(),
        f["expect"]["eth_getStorageAt"][0]
    );
    assert_eq!(
        h.ask("eth_getBalance", json!([p[0], p[2]])).await.unwrap(),
        f["expect"]["eth_getBalance"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_pending_nonce_is_the_proven_one_until_this_client_sends() {
    let h = ethereum().await;
    let f = fixture("state/eoa.json");
    assert_eq!(
        h.ask(
            "eth_getTransactionCount",
            json!([f["request"]["params"][0], "pending"])
        )
        .await
        .unwrap(),
        f["expect"]["eth_getTransactionCount"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_bad_address_or_slot_is_invalid_params_before_anything_is_sent() {
    let h = ethereum().await;
    assert_eq!(
        h.ask("eth_getBalance", json!(["0x12", "latest"]))
            .await
            .unwrap_err()
            .code(),
        -32602
    );
    assert_eq!(
        h.ask(
            "eth_getStorageAt",
            json!([format!("0x{}", "11".repeat(20)), "zz", "latest"])
        )
        .await
        .unwrap_err()
        .code(),
        -32602
    );
}

fn expect_outcome(r: Result<Value, crate::Error>, expect: &Value, what: &str) {
    if let Some(result) = expect.get("result") {
        assert_eq!(&r.unwrap(), result, "{what}");
        return;
    }
    let e = r.unwrap_err();
    let want = &expect["error"];
    match &e {
        crate::Error::Rpc {
            code,
            message,
            data,
        } => {
            assert_eq!(*code, want["code"].as_i64().unwrap(), "{what}");
            assert_eq!(message, want["message"].as_str().unwrap(), "{what}");
            if !want["data"].is_null() {
                assert_eq!(data.as_ref(), Some(&want["data"]), "{what}");
            }
        }
        e => panic!("{what}: {e}"),
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_and_estimates_on_ethereum() {
    for file in fixture_names("call") {
        let h = ethereum().await;
        let f = fixture(&format!("call/{file}"));
        let call = f["request"]["method"] == "willow_getCallProof";
        let method = if call { "eth_call" } else { "eth_estimateGas" };
        let params = if call {
            json!([f["request"]["params"][0], "latest"])
        } else {
            json!([f["request"]["params"][0]])
        };
        expect_outcome(h.ask(method, params).await, &f["expect"], &file);
        // The client's own outcome equals the public node's (geth v1.17.9), as recorded.
        let reference = &f["reference"][method];
        if let Some(r) = reference.get("result") {
            assert_eq!(&f["expect"]["result"], r, "{file}");
        } else {
            assert_eq!(f["expect"]["error"], reference["error"], "{file}");
        }
        let e = h.last();
        assert_eq!(
            e.block.map(|b| (b.number, b.hash)),
            Some(w2_block()),
            "{file}"
        );
        assert_eq!(e.status, Status::Verified, "{file}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_call_with_state_and_block_overrides_is_verified_over_them() {
    let h = ethereum().await;
    let mut f = fixture("call/call-usdc-balance.json");
    let a = format!("0x{}", "11".repeat(20));
    let overrides = [
        json!({a.clone(): {"balance": "0x1", "nonce": "0x0", "code": "0x"}}),
        json!({"time": "0x5"}),
    ];
    let p0 = f["request"]["params"][0].clone();
    let p1 = f["request"]["params"][1].clone();
    f["request"]["params"] = json!([p0, p1, overrides[0], overrides[1]]);
    h.world.add_call(&f, 1);
    assert_eq!(
        h.ask(
            "eth_call",
            json!([p0, "latest", overrides[0], overrides[1]])
        )
        .await
        .unwrap(),
        f["expect"]["result"]
    );
    assert_eq!(
        h.world.sent("willow_getCallProof"),
        vec![json!([p0, {"blockHash": w2_block().1}, overrides[0], overrides[1]])]
    );
    assert_eq!(h.last().status, Status::Verified);
    // Empty overrides are none: the plain call's proof.
    assert_eq!(
        h.ask("eth_call", json!([p0, "latest", {}, null]))
            .await
            .unwrap(),
        f["expect"]["result"]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn the_same_call_at_the_same_head_is_proven_once() {
    let h = ethereum().await;
    let f = fixture("call/call-usdc-balance.json");
    let p = json!([f["request"]["params"][0], "latest"]);
    h.ask("eth_call", p.clone()).await.unwrap();
    h.ask("eth_call", p).await.unwrap();
    assert_eq!(h.world.sent("willow_getCallProof").len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn calls_the_rpc_served_with_their_overrides_on_ethereum_unichain_and_robinhood() {
    for file in fixture_names("served") {
        let f = fixture(&format!("served/{file}"));
        let r = &f["result"];
        let chain_id = n(&r["chainId"]);
        let number = n(&r["blockNumber"]);
        let hash = r["blockHash"].as_str().unwrap().to_ascii_lowercase();
        let h = harness(chain_id, |c| c).await;
        h.world.add_call(&f, chain_id);
        let now = h.world.now_sec();
        h.world.anchor(AnchorEntry {
            chain_id,
            number,
            hash: hash.clone(),
            level: "signed",
            path: if chain_id == 1 { "head" } else { "sequencer" },
            time: now - 2,
        });
        let call = f["request"]["method"] == "willow_getCallProof";
        let method = if call { "eth_call" } else { "eth_estimateGas" };
        let mut params = vec![
            f["request"]["params"][0].clone(),
            json!({"blockHash": r["blockHash"]}),
        ];
        params.extend(
            f["request"]["params"]
                .as_array()
                .unwrap()
                .iter()
                .skip(2)
                .cloned(),
        );
        expect_outcome(
            h.ask(method, Value::Array(params)).await,
            &f["expect"],
            &file,
        );
        let e = h.last();
        assert_eq!(e.status, Status::Verified, "{file}");
        assert_eq!(
            e.block.map(|b| (b.number, b.hash)),
            Some((number, hash)),
            "{file}"
        );
    }
}
