//! Local filters over verified blocks and logs (filters.test.ts), and batching
//! (batching.test.ts): calls within the window go out as one batch of at most 100, identical
//! calls share one item, and a batch response must hold every item.

use super::support::*;
use crate::http::Batcher;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[tokio::test(flavor = "multi_thread")]
async fn a_block_filter_returns_each_new_blocks_hash_then_nothing() {
    let b = block_of("base-56049971.json");
    let h = harness(8453, |c| c.head_max_age(Duration::ZERO)).await;
    h.world.head(
        8453,
        b.0 - 1,
        &format!("0x{}", "12".repeat(32)),
        "signed",
        "sequencer",
    );
    let id = h.ask("eth_newBlockFilter", json!([])).await.unwrap();
    h.world.head_at(8453, &b);
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([b.1])
    );
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([])
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_log_filter_returns_the_matching_logs_of_new_blocks_then_nothing() {
    let c = json_file(&items_dir().join("uni-block.json"));
    let number = n(&c["block"]["number"]);
    let w = World::new();
    w.add_item(&c, true);
    let h = harness_with(130, w, "test-key", |x| x.head_max_age(Duration::ZERO)).await;
    h.world.head(
        130,
        number - 1,
        &format!("0x{}", "34".repeat(32)),
        "signed",
        "sequencer",
    );
    let all: Vec<Value> = c["answers"][0]["result"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["logs"].as_array().cloned().unwrap_or_default())
        .collect();
    let address = all[0]["address"].as_str().unwrap().to_string();
    let want: Vec<Value> = all
        .iter()
        .filter(|l| {
            l["address"]
                .as_str()
                .unwrap()
                .eq_ignore_ascii_case(&address)
        })
        .cloned()
        .collect();
    let id = h
        .ask("eth_newFilter", json!([{"address": address}]))
        .await
        .unwrap();
    h.world.head(
        130,
        number,
        c["block"]["hash"].as_str().unwrap(),
        "signed",
        "sequencer",
    );
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        Value::Array(want.clone())
    );
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([])
    );
    assert_eq!(
        h.ask("eth_getFilterLogs", json!([id])).await.unwrap(),
        Value::Array(want)
    );
    assert_eq!(
        h.ask("eth_uninstallFilter", json!([id])).await.unwrap(),
        json!(true)
    );
    assert_eq!(
        h.ask("eth_uninstallFilter", json!([id])).await.unwrap(),
        json!(false)
    );
    let e = h
        .ask("eth_getFilterChanges", json!([id]))
        .await
        .unwrap_err();
    assert_eq!(e.message(), "filter not found");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_log_filters_failed_poll_is_asked_again_from_the_same_block() {
    let c = json_file(&items_dir().join("uni-block.json"));
    let number = n(&c["block"]["number"]);
    let w = World::new();
    w.add_item(&c, true);
    let h = harness_with(130, w, "test-key", |x| x.head_max_age(Duration::ZERO)).await;
    h.world.head(
        130,
        number - 1,
        &format!("0x{}", "34".repeat(32)),
        "signed",
        "sequencer",
    );
    let id = h.ask("eth_newFilter", json!([{}])).await.unwrap();
    h.world.head(
        130,
        number,
        c["block"]["hash"].as_str().unwrap(),
        "signed",
        "sequencer",
    );
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getBlockReceiptsProof")
            .then(|| json!({"error": {"code": -32603, "message": "busy"}}))
    })));
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id]))
            .await
            .unwrap_err()
            .code(),
        -32603
    );
    h.world.intercept(None);
    let logs = h.ask("eth_getFilterChanges", json!([id])).await.unwrap();
    assert!(!logs.as_array().unwrap().is_empty());
    assert!(logs
        .as_array()
        .unwrap()
        .iter()
        .all(|l| n(&l["blockNumber"]) == number));
}

#[tokio::test(flavor = "multi_thread")]
async fn identical_calls_in_flight_share_one_item() {
    let h = harness(8453, |c| c.batch_window(Duration::from_millis(20))).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    h.ask("eth_blockNumber", json!([])).await.unwrap();
    h.world.s().log.clear();
    let r = futures::future::join_all(
        (0..5).map(|_| h.ask("eth_getBlockByNumber", json!(["latest", false]))),
    )
    .await;
    assert!(r.iter().all(Result::is_ok));
    let items: usize = h.world.s().log.iter().map(|(_, i)| i.len()).sum();
    assert_eq!(items, 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn more_than_100_items_go_out_as_batches_of_at_most_100() {
    let h = harness(8453, |c| c.batch_window(Duration::from_millis(100))).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    h.ask("eth_blockNumber", json!([])).await.unwrap();
    h.world.s().log.clear();
    let hashes: Vec<String> = (0..130).map(|i| format!("0x{i:064x}")).collect();
    let out = futures::future::join_all(
        hashes
            .iter()
            .map(|x| h.ask("eth_getTransactionReceipt", json!([x]))),
    )
    .await;
    assert!(out.iter().all(|r| r.as_ref().is_ok_and(Value::is_null)));
    let mut sizes: Vec<usize> = h.world.s().log.iter().map(|(_, i)| i.len()).collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(sizes, vec![100, 30]);
}

#[tokio::test(flavor = "multi_thread")]
async fn eth_block_number_in_a_burst_refreshes_the_head_once() {
    let h = harness(8453, |c| c).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    let r = futures::future::join_all((0..4).map(|_| h.ask("eth_blockNumber", json!([])))).await;
    assert!(r.iter().all(Result::is_ok));
    assert_eq!(h.world.sent("willow_getBlockAnchor").len(), 1);
}

async fn raw_server(
    f: impl Fn(&str) -> Option<(u16, Vec<(String, String)>, String)> + Send + Sync + 'static,
) -> (Arc<World>, Server) {
    let w = World::new();
    w.s().raw = Some(Arc::new(f));
    let s = serve(w.clone()).await;
    (w, s)
}

fn batcher() -> Batcher {
    Batcher::new(
        reqwest::Client::new(),
        Duration::from_millis(1),
        100,
        Duration::from_secs(120),
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn an_item_missing_from_a_batch_response_fails_verification_and_the_others_resolve() {
    let (_w, s) = raw_server(|_| {
        Some((
            200,
            vec![],
            json!([{"jsonrpc": "2.0", "id": 1, "result": "0x1"}]).to_string(),
        ))
    })
    .await;
    let b = batcher();
    let url = format!("{}/x", s.base);
    let (one, two) = tokio::join!(b.call(&url, "a", json!([])), b.call(&url, "b", json!([])));
    assert_eq!(one.unwrap().result(), &json!("0x1"));
    assert_eq!(code_kind(&two.unwrap_err()), (-32098, Some("batch".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_item_twice_in_a_batch_response_fails_verification() {
    let (_w, s) = raw_server(|_| {
        Some((200, vec![], json!([{"jsonrpc": "2.0", "id": 1, "result": "0x1"}, {"jsonrpc": "2.0", "id": 1, "result": "0x2"}]).to_string()))
    })
    .await;
    let e = batcher()
        .call(&format!("{}/x", s.base), "a", json!([]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("batch".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_whole_request_error_reaches_every_item_as_sent() {
    let (_w, s) = raw_server(|_| Some((200, vec![], json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32001, "message": "invalid API key"}}).to_string()))).await;
    let e = batcher()
        .call(&format!("{}/x", s.base), "a", json!([]))
        .await
        .unwrap_err();
    assert_eq!(e, crate::Error::rpc(-32001, "invalid API key"));
}

#[tokio::test(flavor = "multi_thread")]
async fn http_429_is_asked_again_after_its_retry_after() {
    let n = Arc::new(AtomicUsize::new(0));
    let m = n.clone();
    let (_w, s) = raw_server(move |_| {
        Some(if m.fetch_add(1, Ordering::SeqCst) == 0 {
            (429, vec![("retry-after".into(), "1".into())], String::new())
        } else {
            (
                200,
                vec![],
                json!([{"jsonrpc": "2.0", "id": 1, "result": "ok"}]).to_string(),
            )
        })
    })
    .await;
    let r = batcher()
        .call(&format!("{}/x", s.base), "a", json!([]))
        .await
        .unwrap();
    assert_eq!(r.result(), &json!("ok"));
    assert_eq!(n.load(Ordering::SeqCst), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn an_http_error_without_json_rpc_is_a_server_error() {
    let (_w, s) = raw_server(|_| Some((500, vec![], "oops".into()))).await;
    let e = batcher()
        .call(&format!("{}/x", s.base), "a", json!([]))
        .await
        .unwrap_err();
    assert_eq!(e.code(), -32603);
    assert!(e.message().contains("HTTP 500"), "{e}");
}
