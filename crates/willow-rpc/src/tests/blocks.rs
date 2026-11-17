//! Blocks: eth_getBlockByNumber / ByHash, false and true, and the transaction
//! counts, on the block fixtures from all seven chains (blocks.test.ts).

use super::support::*;
use crate::config::{Level, Status};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `got` without the transactions' proven `blockTimestamp` where the node's `want` leaves it out
/// (Nitro's nodes do).
fn without_omitted(got: &Value, want: &Value) -> Value {
    let mut out = got.clone();
    if let Some(txs) = out["transactions"].as_array_mut() {
        for (i, t) in txs.iter_mut().enumerate() {
            if t.is_object() && want["transactions"][i].get("blockTimestamp").is_none() {
                t.as_object_mut().unwrap().remove("blockTimestamp");
            }
        }
    }
    out
}

#[tokio::test(flavor = "multi_thread")]
async fn blocks_on_every_chain() {
    for file in fixture_names("block") {
        if file == "ethereum-1500000-uncles-unproven.json" {
            continue;
        }
        let chain_id = chain_of(&file);
        let f = fixture(&format!("block/{file}"));
        let b = block_of(&file);
        let h = harness(chain_id, |c| c).await;
        h.world.head_at(chain_id, &b);
        let entry = h
            .world
            .proof(chain_id, "willow_getBlockProof", json!([b.1]));
        let full = entry.full.clone().unwrap();
        assert_eq!(
            h.ask("eth_getBlockByNumber", json!(["latest", false]))
                .await
                .unwrap(),
            entry.proven,
            "{file}"
        );
        let got = h
            .ask("eth_getBlockByNumber", json!(["latest", true]))
            .await
            .unwrap();
        assert_eq!(without_omitted(&got, &full), full, "{file} full");
        assert_eq!(
            h.ask("eth_getBlockByHash", json!([b.1, false]))
                .await
                .unwrap(),
            entry.proven,
            "{file}"
        );
        assert_eq!(
            h.ask("eth_getBlockByNumber", json!([q(b.0), false]))
                .await
                .unwrap(),
            entry.proven,
            "{file}"
        );
        let count = f["node"]["eth_getBlockByNumber"]["transactions"]
            .as_array()
            .unwrap()
            .len() as u64;
        assert_eq!(
            h.ask("eth_getBlockTransactionCountByHash", json!([b.1]))
                .await
                .unwrap(),
            json!(q(count))
        );
        assert_eq!(
            h.ask("eth_getBlockTransactionCountByNumber", json!(["latest"]))
                .await
                .unwrap(),
            json!(q(count))
        );
        for e in h.events() {
            assert_eq!(e.status, Status::Verified, "{file}");
            assert_eq!(
                e.block.as_ref().map(|x| (x.number, x.hash.clone())),
                Some(b.clone()),
                "{file}"
            );
            assert_eq!(e.level, Some(Level::Signed), "{file}");
        }
        // The block is fetched once, then served from the cache by hash.
        let proofs = h
            .world
            .methods_sent()
            .iter()
            .filter(|m| *m == "willow_getBlockProof")
            .count();
        assert!(proofs <= 2, "{file}: {proofs} block proofs");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_block_that_is_not_the_head_has_its_anchor_in_the_same_batch_as_its_proof() {
    let h = harness(1, |c| c).await;
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    h.ask("eth_blockNumber", json!([])).await.unwrap();
    h.world.s().log.clear();
    let old = block_of("ethereum-17034874.json");
    h.ask("eth_getBlockByNumber", json!([q(old.0), false]))
        .await
        .unwrap();
    let log = h.world.s().log.clone();
    let batch = log
        .iter()
        .find(|(_, items)| items.iter().any(|(m, _)| m == "willow_getBlockProof"))
        .unwrap();
    let mut methods: Vec<&str> = batch.1.iter().map(|(m, _)| m.as_str()).collect();
    methods.sort();
    assert_eq!(
        methods,
        vec!["willow_getBlockAnchor", "willow_getBlockProof"]
    );
    assert_eq!(h.last().block.map(|b| (b.number, b.hash)), Some(old));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_block_newer_than_the_signed_head_waits_for_its_anchor_then_is_anchored() {
    let h = harness(1, |c| c).await;
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let old = block_of("ethereum-17034874.json");
    let asked = Arc::new(AtomicUsize::new(0));
    let a = asked.clone();
    let want = json!(q(old.0));
    h.world.intercept(Some(Arc::new(move |_, m, p| {
        if m == "willow_getBlockAnchor" && p.first() == Some(&want) && a.fetch_add(1, Ordering::SeqCst) + 1 < 3 {
            return Some(json!({"error": {"code": -32000, "message": "newer than the signed head", "data": {"status": "waiting", "retryAfterSeconds": 4}}}));
        }
        None
    })));
    h.ask("eth_getBlockByNumber", json!([q(old.0), false]))
        .await
        .unwrap();
    assert_eq!(asked.load(Ordering::SeqCst), 3);
    let e = h.last();
    assert_eq!(
        (e.status, e.block.map(|b| (b.number, b.hash))),
        (Status::Verified, Some(old))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_block_whose_anchor_still_waits_past_anchor_wait_is_unverifiable() {
    let h = harness(1, |c| c.anchor_wait(Duration::from_millis(40))).await;
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let old = block_of("ethereum-17034874.json");
    let asked = Arc::new(AtomicUsize::new(0));
    let a = asked.clone();
    let want = json!(q(old.0));
    h.world.intercept(Some(Arc::new(move |_, m, p| {
        if m == "willow_getBlockAnchor" && p.first() == Some(&want) {
            a.fetch_add(1, Ordering::SeqCst);
            return Some(json!({"error": {"code": -32000, "message": "newer than the signed head", "data": {"status": "waiting", "retryAfterSeconds": 4}}}));
        }
        None
    })));
    let started = Instant::now();
    let e = h
        .ask("eth_getBlockByNumber", json!([q(old.0), false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-waiting".into())));
    assert!(asked.load(Ordering::SeqCst) > 1);
    assert!(started.elapsed() >= Duration::from_millis(40));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_block_by_hash_is_anchored_in_a_second_round_trip() {
    let h = harness(1, |c| c).await;
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let pre = block_of("ethereum-1500000.json");
    h.ask("eth_getBlockByHash", json!([pre.1, false]))
        .await
        .unwrap();
    let log = h.world.s().log.clone();
    let idx = |m: &str| {
        log.iter()
            .position(|(_, items)| items.iter().any(|(x, _)| x == m))
            .unwrap()
    };
    assert!(idx("willow_getBlockAnchor") > idx("willow_getBlockProof"));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_pre_merge_block_without_its_uncles_leaves_them_out_and_lists_them() {
    let w = World::new();
    let b = w.add_block(
        &fixture("block/ethereum-1500000-uncles-unproven.json"),
        None,
    );
    let h = harness_with(1, w, "test-key", |c| c).await;
    h.world.head_at(1, &b);
    let block = h
        .ask("eth_getBlockByHash", json!([b.1, false]))
        .await
        .unwrap();
    assert!(block.get("uncles").is_none());
    assert!(block.get("size").is_none());
    assert_eq!(
        h.last().unproven,
        vec!["size".to_string(), "uncles".to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_block_the_server_does_not_have_is_refused_as_sent() {
    let h = harness(8453, |c| c).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    let e = h
        .ask(
            "eth_getBlockByHash",
            json!([format!("0x{}", "ab".repeat(32)), false]),
        )
        .await
        .unwrap_err();
    assert_eq!(e.code(), -32000);
    assert!(matches!(e, crate::Error::Rpc { .. }));
    assert_eq!(h.last().status, Status::Refused);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_app_that_changes_a_returned_block_does_not_change_the_cached_one() {
    let h = harness(8453, |c| c).await;
    let head = block_of("base-56049971.json");
    h.world.head_at(8453, &head);
    let mut a = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap();
    a["hash"] = json!("0xforged");
    let b = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(head.1));
}
