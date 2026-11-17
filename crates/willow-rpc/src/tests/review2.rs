//! Faults found by the independent review (round 2), each with the test that showed it.

use super::support::*;
use crate::config::{Anchoring, Status, Unverifiable};
use crate::{Config, Verifier};
use serde_json::json;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// A runtime of its own for the mock server, which outlives the runtimes the client runs on.
fn server_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
}

fn client_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
}

/// A client shared by runtimes (a `static`, tests each with their own runtime): one that ends
/// in the middle of a batch window, a send, an anchor fetch or a state read must not leave the
/// client hung or failing for the next.
#[test]
fn a_client_outlives_a_runtime_that_ended_mid_request() {
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c.batch_window(Duration::from_millis(200))));
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    let eoa = fixture("state/eoa.json")["request"]["params"][0].clone();
    // A request by hash reaches the state read (and its flush) at once; the head needs a batch.
    for _ in 0..2 {
        let rt = client_runtime();
        let c = h.client.clone();
        let p = json!([eoa, {"blockHash": b.1}]);
        let r = rt.block_on(async {
            tokio::time::timeout(Duration::ZERO, c.request("eth_getBalance", p)).await
        });
        assert!(r.is_err(), "the first poll should still be waiting");
        let c = h.client.clone();
        let r = rt.block_on(async {
            tokio::time::timeout(
                Duration::from_millis(20),
                c.request("eth_blockNumber", json!([])),
            )
            .await
        });
        assert!(r.is_err(), "the head should still be waiting for its batch");
        drop(rt);
    }
    let rt = client_runtime();
    let c = h.client.clone();
    let r = rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            (
                c.request("eth_blockNumber", json!([])).await,
                c.request("eth_getBalance", json!([eoa, {"blockHash": b.1}]))
                    .await,
            )
        })
        .await
    });
    let (n, bal) = r.expect("the client stayed hung by a runtime that ended");
    assert_eq!(n.unwrap(), json!(q(b.0)));
    assert_eq!(
        bal.unwrap(),
        fixture("state/eoa.json")["expect"]["eth_getBalance"]
    );
    drop(h);
    drop(srv);
}

/// The same, for a runtime that ended before its tasks first ran: a state read's flush, an anchor
/// fetch and the chain check, all spawned and never polled.
#[test]
fn a_client_outlives_a_runtime_that_ended_before_its_tasks_ran() {
    use futures::FutureExt;
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c));
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    let eoa = fixture("state/eoa.json")["request"]["params"][0].clone();
    let rt = client_runtime();
    let c = h.client.clone();
    let p = json!([eoa, {"blockHash": b.1}]);
    assert!(rt
        .block_on(async { c.request("eth_getBalance", p).now_or_never() })
        .is_none());
    let c = h.client.clone();
    assert!(rt
        .block_on(async { c.request("eth_blockNumber", json!([])).now_or_never() })
        .is_none());
    drop(rt);
    let rt = client_runtime();
    let c = h.client.clone();
    let r = rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), async {
            (
                c.request("eth_blockNumber", json!([])).await,
                c.request("eth_getBalance", json!([eoa, {"blockHash": b.1}]))
                    .await,
            )
        })
        .await
    });
    let (n, bal) = r.expect("the client stayed hung by a runtime that ended");
    assert_eq!(n.unwrap(), json!(q(b.0)));
    assert_eq!(
        bal.unwrap(),
        fixture("state/eoa.json")["expect"]["eth_getBalance"]
    );
    drop(h);
    drop(srv);
}

/// A batch whose send ended with its runtime is asked again; the error its cut-off left isn't
/// served from then on.
#[test]
fn a_send_cut_off_by_its_runtime_is_asked_again() {
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c));
    let first = Arc::new(AtomicUsize::new(0));
    let f = first.clone();
    h.world.intercept(Some(Arc::new(move |_, m, _| {
        if m == "eth_chainId" && f.fetch_add(1, Ordering::SeqCst) == 0 {
            std::thread::sleep(Duration::from_millis(400));
        }
        None
    })));
    let rt = client_runtime();
    let c = h.client.clone();
    let r = rt.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(100),
            c.request("eth_chainId", json!([])),
        )
        .await
    });
    assert!(r.is_err(), "the first chain check should still be waiting");
    drop(rt);
    let rt = client_runtime();
    let c = h.client.clone();
    let r = rt.block_on(async {
        tokio::time::timeout(Duration::from_secs(5), c.request("eth_chainId", json!([]))).await
    });
    assert_eq!(r.expect("hung").unwrap(), json!("0x1"));
    assert_eq!(first.load(Ordering::SeqCst), 2);
    drop(h);
    drop(srv);
}

/// Every HTTP request has the timeout, also on a client passed in. (The mock server blocks its
/// thread, so it runs on a runtime of its own.)
#[test]
fn a_request_times_out_also_with_an_http_client_passed_in() {
    let srv = server_runtime();
    let w = World::new();
    w.s().raw = Some(Arc::new(|_| {
        std::thread::sleep(Duration::from_secs(3));
        None
    }));
    let s = srv.block_on(serve(w.clone()));
    let url = s.url(1, "k");
    let rt = client_runtime();
    let (e, took) = rt.block_on(async {
        let v = Verifier::new(
            Config::new(1)
                .url(url)
                .http_client(reqwest::Client::new())
                .timeout(Duration::from_millis(300))
                .storage(crate::StorageChoice::None),
        )
        .unwrap();
        let t = Instant::now();
        let e = v.request("eth_chainId", json!([])).await.unwrap_err();
        (e, t.elapsed())
    });
    assert_eq!(e.code(), -32603, "{e}");
    assert!(took < Duration::from_millis(2500), "{took:?}");
    drop(s);
    drop(srv);
}

/// An OpenVM helper that started and then failed on a proof is a failed check (as a throw from
/// the page's OpenVM module is), not a missing verifier.
#[cfg(feature = "logs")]
#[tokio::test(flavor = "multi_thread")]
async fn an_openvm_helper_that_dies_on_a_proof_fails_the_logs() {
    use crate::config::OpenVmVerifier;
    let helper = super::logs::script("echo \"ready $KEYS\"\nread line\nexit 3");
    let dirl = repo().join("web/verify/test/logs/base-52969856");
    let v = json_file(&dirl.join("segment.json"));
    let resp = v["result"].clone();
    let w = World::new();
    let to = n(&v["filter"]["toBlock"]);
    w.head(
        8453,
        to + 10,
        &format!("0x{}", "77".repeat(32)),
        "signed",
        "sequencer",
    );
    let store = dirl.join("store");
    w.intercept(Some(Arc::new(move |_, m, params| match m {
        "willow_getLogsProof" => Some(json!({"result": resp.clone()})),
        "willow_getProof" => {
            let b = std::fs::read(store.join(format!("{}.proof", params[0].as_str().unwrap())))
                .unwrap();
            Some(json!({"result": {"proof": format!("0x{}", hex::encode(b))}}))
        }
        _ => None,
    })));
    let h = harness_with(8453, w, "test-key", move |c| {
        c.openvm_verifier(OpenVmVerifier::Path(helper))
    })
    .await;
    let e = h
        .ask("eth_getLogs", json!([v["filter"]]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("logs".into())));
}

/// A failed chain check is let go as soon as it fails, not when a later request polls it.
#[tokio::test(flavor = "multi_thread")]
async fn a_transient_chain_check_failure_does_not_fail_the_next_request() {
    let h = harness(8453, |c| c).await;
    h.world.head_at(8453, &block_of("base-56049971.json"));
    let asked = Arc::new(AtomicUsize::new(0));
    let a = asked.clone();
    h.world.intercept(Some(Arc::new(move |_, m, _| {
        (m == "eth_chainId" && a.fetch_add(1, Ordering::SeqCst) == 0)
            .then(|| json!({"error": {"code": -32603, "message": "busy"}}))
    })));
    // A request that fails before it waits for the chain check.
    assert_eq!(
        h.ask("eth_getBalance", json!(["0x12", "latest"]))
            .await
            .unwrap_err()
            .code(),
        -32602
    );
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        h.ask("eth_getBlockByNumber", json!(["latest", false]))
            .await
            .unwrap()["hash"],
        json!(block_of("base-56049971.json").1)
    );
}

/// `anchor_wait` counts from when the anchor is asked for, as the npm client counts it, not
/// from when its response is first read. (The mock server blocks its thread, so it runs on a
/// runtime of its own.)
#[test]
fn the_anchor_wait_counts_from_the_ask() {
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c.anchor_wait(Duration::from_millis(300))));
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let rt = client_runtime();
    rt.block_on(h.ask("eth_blockNumber", json!([]))).unwrap();
    let old = block_of("ethereum-17034874.json");
    let want = json!(q(old.0));
    h.world.intercept(Some(Arc::new(move |_, m, p| {
        if m == "willow_getBlockProof" {
            std::thread::sleep(Duration::from_millis(400));
            return None;
        }
        (m == "willow_getBlockAnchor" && p.first() == Some(&want))
            .then(|| json!({"error": {"code": -32000, "message": "waiting", "data": {"status": "waiting"}}}))
    })));
    let t = Instant::now();
    let e = rt
        .block_on(h.ask("eth_getBlockByNumber", json!([q(old.0), false])))
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-waiting".into())));
    assert!(
        t.elapsed() < Duration::from_millis(650),
        "{:?}",
        t.elapsed()
    );
    drop(rt);
    drop(h);
    drop(srv);
}

/// Under `flag`, a log filter's unverifiable range falls back to the server's eth_getLogs over
/// the same range, never the filter method with this client's local id; the filter moves on.
#[tokio::test(flavor = "multi_thread")]
async fn a_log_filters_fallback_asks_the_server_for_the_logs_not_its_filter() {
    let eth = block_of("ethereum-26391511.json");
    let h = harness(1, |c| {
        c.unverifiable(Unverifiable::Flag)
            .head_max_age(Duration::ZERO)
    })
    .await;
    h.world.head(
        1,
        eth.0 - 20,
        &format!("0x{}", "12".repeat(32)),
        "signed",
        "head",
    );
    let id = h.ask("eth_newFilter", json!([{}])).await.unwrap();
    h.world.head_at(1, &eth);
    h.world.intercept(Some(Arc::new(|_, m, _| match m {
        "willow_getLogsProof" => {
            Some(json!({"error": {"code": -32601, "message": "no log proofs on this plan"}}))
        }
        "eth_getLogs" => Some(json!({"result": [{"plain": true}]})),
        _ => None,
    })));
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([{"plain": true}])
    );
    assert_eq!(h.last().status, Status::Unverifiable);
    assert!(h.world.sent("eth_getFilterChanges").is_empty());
    let asked = h.world.sent("eth_getLogs");
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0][0]["fromBlock"], json!(q(eth.0 - 19)));
    assert_eq!(asked[0][0]["toBlock"], json!(q(eth.0)));
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([])
    );
    assert_eq!(h.world.sent("eth_getLogs").len(), 1);
}

/// The same over a few blocks (receipt proofs, refused for the plan), and for eth_getFilterLogs
/// (the filter as it was made).
#[tokio::test(flavor = "multi_thread")]
async fn a_log_filters_fallback_over_receipts_asks_for_the_logs_too() {
    let c = json_file(&items_dir().join("uni-block.json"));
    let number = n(&c["block"]["number"]);
    let w = World::new();
    w.add_item(&c, true);
    let h = harness_with(130, w, "test-key", |x| {
        x.unverifiable(Unverifiable::Flag)
            .head_max_age(Duration::ZERO)
    })
    .await;
    h.world.head(
        130,
        number - 1,
        &format!("0x{}", "34".repeat(32)),
        "signed",
        "sequencer",
    );
    let made = json!({"fromBlock": q(number - 5), "address": "0x00000000000000000000000000000000000000aa"});
    let id = h.ask("eth_newFilter", json!([made])).await.unwrap();
    h.world.head(
        130,
        number,
        c["block"]["hash"].as_str().unwrap(),
        "signed",
        "sequencer",
    );
    h.world.intercept(Some(Arc::new(|_, m, _| match m {
        "willow_getBlockReceiptsProof" => {
            Some(json!({"error": {"code": -32601, "message": "not on this plan"}}))
        }
        "eth_getLogs" => Some(json!({"result": [{"plain": true}]})),
        _ => None,
    })));
    assert_eq!(
        h.ask("eth_getFilterChanges", json!([id])).await.unwrap(),
        json!([{"plain": true}])
    );
    assert_eq!(h.last().status, Status::Unverifiable);
    assert_eq!(
        h.world.sent("eth_getLogs")[0][0]["fromBlock"],
        json!(q(number))
    );
    assert_eq!(
        h.ask("eth_getFilterLogs", json!([id])).await.unwrap(),
        json!([{"plain": true}])
    );
    assert_eq!(h.world.sent("eth_getLogs")[1][0], made);
    assert!(h.world.sent("eth_getFilterChanges").is_empty());
    assert!(h.world.sent("eth_getFilterLogs").is_empty());
}

/// A block filter has no server request to stand for it: under `flag`, a refused proof is
/// unverifiable, and nothing is sent in its place.
#[tokio::test(flavor = "multi_thread")]
async fn a_block_filters_refused_proof_is_unverifiable_even_under_flag() {
    let b = block_of("base-56049971.json");
    let h = harness(8453, |c| {
        c.unverifiable(Unverifiable::Flag)
            .head_max_age(Duration::ZERO)
    })
    .await;
    h.world.head(
        8453,
        b.0 - 1,
        &format!("0x{}", "12".repeat(32)),
        "signed",
        "sequencer",
    );
    let id = h.ask("eth_newBlockFilter", json!([])).await.unwrap();
    h.world.head_at(8453, &b);
    h.world.intercept(Some(Arc::new(|_, m, _| {
        (m == "willow_getBlockProof")
            .then(|| json!({"error": {"code": -32601, "message": "not on this plan"}}))
    })));
    let e = h
        .ask("eth_getFilterChanges", json!([id]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("not-on-plan".into())));
    assert!(h.world.sent("eth_getFilterChanges").is_empty());
}

/// A block filter asks for at most `BLOCK_FILTER_MAX` blocks a poll, the oldest first, whatever
/// the newest block says.
#[tokio::test(flavor = "multi_thread")]
async fn a_block_filter_asks_for_at_most_block_filter_max_blocks_a_poll() {
    let h = harness(8453, |c| c.anchoring(Anchoring::None)).await;
    h.world
        .standard("8453|eth_blockNumber|*", json!({"result": "0x64"}));
    let id = h.ask("eth_newBlockFilter", json!([])).await.unwrap();
    h.world
        .standard("8453|eth_blockNumber|*", json!({"result": "0x100000"}));
    let _ = h.ask("eth_getFilterChanges", json!([id])).await;
    // The poll fails at its first block (the mock has no proofs for them); every proof was asked
    // for at once, so let the last batches land.
    let until = Instant::now() + Duration::from_secs(3);
    while (h.world.sent("willow_getBlockProof").len() as u64) < crate::BLOCK_FILTER_MAX
        && Instant::now() < until
    {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(100)).await;
    let asked: Vec<u64> = h
        .world
        .sent("willow_getBlockProof")
        .iter()
        .map(|p| n(&p[0]))
        .collect();
    assert_eq!(asked.len() as u64, crate::BLOCK_FILTER_MAX);
    assert_eq!(asked.iter().min(), Some(&0x65));
    assert_eq!(
        asked.iter().max(),
        Some(&(0x65 + crate::BLOCK_FILTER_MAX - 1))
    );
}

/// A call on one runtime that joins a request queued by another still goes out when that runtime
/// ends before its flush.
#[test]
fn a_call_joining_another_runtimes_batch_goes_out_when_that_runtime_ends() {
    let srv = server_runtime();
    let w = World::new();
    w.s().raw = Some(Arc::new(|_| {
        Some((
            200,
            vec![],
            json!([{"jsonrpc": "2.0", "id": 1, "result": "0x1"}]).to_string(),
        ))
    }));
    let s = srv.block_on(serve(w.clone()));
    let url = format!("{}/x", s.base);
    let b = crate::http::Batcher::new(
        reqwest::Client::new(),
        Duration::from_millis(300),
        100,
        Duration::from_secs(5),
    );
    // Runtime A queues the request and the batch's flush.
    let a = client_runtime();
    let _queued = {
        let _in = a.enter();
        b.call(&url, "m", json!([]))
    };
    // Runtime B joins it, on its own thread, and A ends inside the window.
    let (b2, u2) = (b.clone(), url.clone());
    let joined = std::thread::spawn(move || {
        client_runtime().block_on(async {
            tokio::time::timeout(Duration::from_secs(3), b2.call(&u2, "m", json!([]))).await
        })
    });
    std::thread::sleep(Duration::from_millis(50));
    drop(a);
    let r = joined.join().unwrap().expect("the call on B hung");
    assert_eq!(r.unwrap().result(), &json!("0x1"));
    drop(s);
    drop(srv);
}

/// A runtime that is shutting down drops a task as it is spawned, on the spawning thread: an
/// anchor fetch started then (here from a guard dropped in the shutdown) must neither deadlock on
/// the in-flight map nor stay in it.
#[test]
fn an_anchor_fetch_spawned_while_its_runtime_shuts_down_lets_go() {
    use futures::FutureExt;
    struct AskOnDrop(crate::Verifier);
    impl Drop for AskOnDrop {
        fn drop(&mut self) {
            let _ = self.0.request("eth_blockNumber", json!([])).now_or_never();
        }
    }
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c));
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let c = h.client.clone();
    let ended = std::thread::spawn(move || {
        let rt = client_runtime();
        rt.spawn(async move {
            let _g = AskOnDrop(c);
            std::future::pending::<()>().await;
        });
        rt.block_on(tokio::task::yield_now());
        drop(rt);
    });
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || tx.send(ended.join().is_ok()));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(5)),
        Ok(true),
        "the runtime's shutdown hung"
    );
    let rt = client_runtime();
    let r = rt.block_on(async {
        tokio::time::timeout(
            Duration::from_secs(5),
            h.client.request("eth_blockNumber", json!([])),
        )
        .await
    });
    assert!(r.expect("hung").is_ok());
    drop(rt);
    drop(h);
    drop(srv);
}

/// A caller that joined an anchor fetch whose runtime then ended gets a transport error
/// (-32603), not a failed proof.
#[test]
fn a_joined_anchor_fetch_cut_off_by_its_runtime_is_not_a_failed_proof() {
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c.batch_window(Duration::from_millis(300))));
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let a = client_runtime();
    let c = h.client.clone();
    let r = a.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(20),
            c.request("eth_blockNumber", json!([])),
        )
        .await
    });
    assert!(r.is_err());
    let c = h.client.clone();
    let joined = std::thread::spawn(move || {
        client_runtime().block_on(async {
            tokio::time::timeout(
                Duration::from_secs(5),
                c.request("eth_blockNumber", json!([])),
            )
            .await
        })
    });
    // B has joined the fetch A started (the map's future and B's) before A ends.
    let anchors = h.client.inner.anchors().clone();
    let until = Instant::now() + Duration::from_secs(3);
    while anchors.sharing("1:latest") < 2 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(anchors.sharing("1:latest"), 2);
    drop(a);
    let e = joined.join().unwrap().expect("hung").unwrap_err();
    assert_eq!(e.code(), -32603, "{e}");
    drop(h);
    drop(srv);
}

/// A response's key is out of flight before its caller has it: a request asked again at once
/// goes to the server again, never served from the one just settled. (The batcher pauses after
/// each response is handed over, so the caller, on the test's own thread, asks inside that pause.)
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_request_asked_again_once_settled_is_sent_again() {
    let w = World::new();
    w.s().raw = Some(Arc::new(move |_| {
        Some((
            200,
            vec![],
            json!([{"jsonrpc": "2.0", "id": 1, "result": "0x1"}]).to_string(),
        ))
    }));
    let s = serve(w.clone()).await;
    let url = format!("{}/x", s.base);
    let b = crate::http::Batcher::new(
        reqwest::Client::new(),
        Duration::from_millis(1),
        100,
        Duration::from_secs(5),
    );
    b.pause_after_send(Duration::from_millis(100));
    for i in 0..3u64 {
        b.call(&url, "m", json!([i])).await.unwrap();
        let again = b.call(&url, "m", json!([i]));
        assert!(
            again.peek().is_none(),
            "round {i}: served from the request just settled"
        );
        again.await.unwrap();
    }
}

/// The same for anchor fetches: a head asked for again once settled is fetched again.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_head_asked_again_once_settled_is_fetched_again() {
    let h = harness(8453, |c| c.head_max_age(Duration::ZERO)).await;
    let anchors = h.client.inner.anchors().clone();
    anchors.pause_after_send(Duration::from_millis(100));
    let base = block_of("base-56049971.json").0;
    for i in 0..3u64 {
        h.world.head(
            8453,
            base + i,
            &format!("0x{:064x}", i + 1),
            "signed",
            "sequencer",
        );
        let a = anchors.head(8453, "latest", false, None).await.unwrap();
        assert_eq!(
            a.number,
            base + i,
            "round {i}: served from the head just settled"
        );
    }
}

/// A run memoized under a key leaves the key only if the entry there is still its own: a slow
/// joiner of a failed run must not drop a newer run in flight (that would run it twice).
#[tokio::test(flavor = "multi_thread")]
async fn a_late_joiner_of_a_memoized_run_leaves_a_newer_run_in_place() {
    use futures::FutureExt;
    let h = harness(1, |c| c).await;
    let c = h.client.inner.clone();
    let ran = Arc::new(AtomicUsize::new(0));
    let (tx1, rx1) = tokio::sync::oneshot::channel::<()>();
    let (_tx2, rx2) = tokio::sync::oneshot::channel::<()>();
    let k = || "test:key".to_string();
    let r1 = ran.clone();
    let mut first = Box::pin(c.once(k(), move || {
        r1.fetch_add(1, Ordering::SeqCst);
        async move {
            let _ = rx1.await;
            Err(crate::Error::rpc(-32603, "busy"))
        }
        .boxed()
    }));
    assert!(futures::poll!(first.as_mut()).is_pending());
    let mut joiner = Box::pin(c.once(k(), || unreachable!("joins the first")));
    assert!(futures::poll!(joiner.as_mut()).is_pending());
    tx1.send(()).unwrap();
    assert!(first.await.is_err());
    let r2 = ran.clone();
    let mut second = Box::pin(c.once(k(), move || {
        r2.fetch_add(1, Ordering::SeqCst);
        async move {
            let _ = rx2.await;
            Ok(json!(1))
        }
        .boxed()
    }));
    assert!(futures::poll!(second.as_mut()).is_pending());
    // The first run's late joiner finishes now.
    assert!(joiner.await.is_err());
    let r3 = ran.clone();
    let mut third = Box::pin(c.once(k(), move || {
        r3.fetch_add(1, Ordering::SeqCst);
        async move { Ok(json!(3)) }.boxed()
    }));
    let _ = futures::poll!(third.as_mut());
    assert_eq!(
        ran.load(Ordering::SeqCst),
        2,
        "the second run was dropped and run again"
    );
}

/// Polled off a tokio runtime, a request is an error, not a panic with the client's locks held
/// (which would poison them for every later request).
#[test]
fn a_request_off_a_tokio_runtime_is_an_error_and_the_client_goes_on() {
    let srv = server_runtime();
    let h = srv.block_on(harness(1, |c| c));
    h.world.head_at(1, &block_of("ethereum-26391511.json"));
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        futures::executor::block_on(h.client.request("eth_blockNumber", json!([])))
    }));
    assert_eq!(r.expect("panicked").unwrap_err().code(), -32603);
    let rt = client_runtime();
    assert!(rt
        .block_on(h.client.request("eth_blockNumber", json!([])))
        .is_ok());
    drop(rt);
    drop(h);
    drop(srv);
}

/// A batch call made off a tokio runtime (a request future moved to another executor) is an
/// error, not a panic with the batcher's lock held; the batcher goes on.
#[test]
fn a_batch_call_off_a_tokio_runtime_is_an_error_and_the_batcher_goes_on() {
    let srv = server_runtime();
    let w = World::new();
    w.s().raw = Some(Arc::new(|_| {
        Some((
            200,
            vec![],
            json!([{"jsonrpc": "2.0", "id": 1, "result": "0x1"}]).to_string(),
        ))
    }));
    let s = srv.block_on(serve(w.clone()));
    let url = format!("{}/x", s.base);
    let b = crate::http::Batcher::new(
        reqwest::Client::new(),
        Duration::from_millis(1),
        100,
        Duration::from_secs(5),
    );
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        futures::executor::block_on(b.call(&url, "m", json!([])))
    }));
    assert_eq!(r.expect("panicked").unwrap_err().code(), -32603);
    let rt = client_runtime();
    let r = rt.block_on(async { b.call(&url, "m", json!([])).await });
    assert!(r.is_ok(), "{:?}", r.err());
    drop(rt);
    drop(s);
    drop(srv);
}

/// A memoized proof run (here eth_gasPrice's fee proof) joined from another runtime: when the
/// runtime that started it ends, the joiner gets -32603 at once instead of hanging on a batch
/// whose flush ended with that runtime.
#[test]
fn a_memoized_run_joined_from_another_runtime_ends_with_its_runtime() {
    let srv = server_runtime();
    let h = srv.block_on(harness(8453, |c| {
        c.batch_window(Duration::from_millis(300))
    }));
    h.world.head_at(8453, &block_of("base-56049971.json"));
    let a = client_runtime();
    let c = h.client.clone();
    // The head first (so the fee run is reached at once), then the fee run, cut off inside the window.
    a.block_on(c.request("eth_blockNumber", json!([]))).unwrap();
    let c = h.client.clone();
    let r = a.block_on(async {
        tokio::time::timeout(
            Duration::from_millis(20),
            c.request("eth_gasPrice", json!([])),
        )
        .await
    });
    assert!(r.is_err());
    let c = h.client.clone();
    let joined = std::thread::spawn(move || {
        client_runtime().block_on(async {
            tokio::time::timeout(Duration::from_secs(5), c.request("eth_gasPrice", json!([]))).await
        })
    });
    let until = Instant::now() + Duration::from_secs(3);
    while h.client.inner.memo_sharing() < 2 && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(h.client.inner.memo_sharing(), 2);
    drop(a);
    let e = joined
        .join()
        .unwrap()
        .expect("the joiner hung")
        .unwrap_err();
    assert_eq!(e.code(), -32603, "{e}");
    drop(h);
    drop(srv);
}

/// A memoized run that panics is an error, and its key takes a new run afterwards.
#[tokio::test(flavor = "multi_thread")]
async fn a_memoized_run_that_panics_is_an_error_and_the_key_runs_again() {
    use futures::FutureExt;
    let h = harness(1, |c| c).await;
    let c = h.client.inner.clone();
    let e = c
        .once("test:panics".into(), || async { panic!("boom") }.boxed())
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("internal".into())));
    let v = c
        .once("test:panics".into(), || async { Ok(json!(7)) }.boxed())
        .await
        .unwrap();
    assert_eq!(v, json!(7));
}

/// A verifier run asked for on a runtime that is shutting down is dropped before it runs: that's
/// no verdict on the server (-32603), not a failed proof.
#[test]
fn a_verifier_dropped_before_it_ran_is_not_a_failed_proof() {
    use futures::FutureExt;
    let a = client_runtime();
    let handle = a.handle().clone();
    drop(a);
    let mut queued = Box::pin(crate::verifiers::blocking(|| 1u8));
    let first = {
        let _in = handle.enter();
        let waker = futures::task::noop_waker();
        let mut cx = std::task::Context::from_waker(&waker);
        queued.poll_unpin(&mut cx)
    };
    let e = match first {
        std::task::Poll::Ready(r) => r.unwrap_err(),
        std::task::Poll::Pending => client_runtime().block_on(queued).unwrap_err(),
    };
    assert_eq!(e.code(), -32603, "{e}");
}

/// A chain id past what the verifiers take (u32) is refused, not truncated to another chain.
#[test]
fn a_chain_id_past_u32_is_refused() {
    let e = Verifier::new(Config::new((1u64 << 32) | 1)).unwrap_err();
    assert_eq!(e.code(), -32602);
}
