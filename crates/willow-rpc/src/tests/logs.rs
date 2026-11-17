//! eth_getLogs by log proofs (filters.test.ts "over more than 8 blocks"), on the
//! real responses in web/verify/test/logs: SP1 (Ethereum 26226600-601) through the logs verifier,
//! and OpenVM (Base 52969856-983, 128 blocks) through the whole client, its proof job polled, its
//! proofs fetched one per request, the last block anchored. OpenVM proofs go to the
//! `willow-logs-verify-openvm` helper: the real one when WILLOW_LOGS_VERIFY_OPENVM names it, else a
//! stand-in that trusts the pinned programs and passes or fails every proof as told.

#![cfg(feature = "logs")]

use super::support::*;
use crate::config::{OpenVmVerifier, Status};
use crate::verifiers::{Backend, OpenVm, Ran};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use willow_logs_verify_openvm::pinned::{self, Kind};

fn logs_dir() -> PathBuf {
    repo().join("web/verify/test/logs")
}

/// Every proof a response names, read from its store.
fn proofs(response: &Value, store: &Path) -> HashMap<String, Vec<u8>> {
    response["parts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            let id = p["proof"].as_str().unwrap().to_string();
            let bytes = std::fs::read(store.join(format!("{id}.proof"))).unwrap();
            (id, bytes)
        })
        .collect()
}

/// A stand-in OpenVM helper that trusts exactly the pinned programs and says `verdict` to each proof.
fn stand_in(verdict: &str) -> PathBuf {
    script(&format!(
        "echo \"ready $KEYS\"\nwhile read line; do echo '{verdict}'; done"
    ))
}

/// A stand-in helper running `body` (sh), with the pinned keys in `$KEYS` and its own
/// directory in `$DIR`. Written by `install`, so no file this process holds open for writing is
/// exec'd (a fork elsewhere in the test run could still hold it: ETXTBSY).
pub(super) fn script(body: &str) -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!(
        "willow-rpc-ovm-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let keys: Vec<String> = Kind::ALL
        .iter()
        .map(|k| hex::encode(pinned::key(*k).unwrap()))
        .collect();
    let p = dir.join("willow-logs-verify-openvm");
    let text = dir.join("script.txt");
    std::fs::write(
        &text,
        format!(
            "#!/bin/sh\nKEYS='{}'\nDIR='{}'\n{body}\n",
            keys.join(" "),
            dir.display()
        ),
    )
    .unwrap();
    let ok = std::process::Command::new("install")
        .args(["-m", "755"])
        .arg(&text)
        .arg(&p)
        .status()
        .unwrap();
    assert!(ok.success());
    p
}

fn helper() -> (PathBuf, bool) {
    match std::env::var("WILLOW_LOGS_VERIFY_OPENVM")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(h) => (PathBuf::from(h), true),
        None => (stand_in("ok"), false),
    }
}

#[test]
fn a_real_sp1_response_verifies() {
    let resp = json_file(&logs_dir().join("eth-26226600/answer.json"));
    let store = logs_dir().join("eth-26226600/store");
    let filter = json!({"fromBlock": resp["fromBlock"], "toBlock": resp["toBlock"]});
    let body = json!({"jsonrpc": "2.0", "id": 1, "result": resp}).to_string();
    let b = Backend::new(
        crate::verifiers::real_anchor(),
        OpenVm::Helper(Some("/nonexistent/ovm".into())),
    );
    let Ran::Done(v) = b.logs_verify(1, &body, &filter.to_string(), &proofs(&resp, &store)) else {
        panic!("no-verifier")
    };
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["report"]["proofs"], json!({"sp1": 2, "openvm": 0}));
    assert!(!v["logs"].as_array().unwrap().is_empty());
    // A byte of a proof changed: rejected.
    let mut bad = proofs(&resp, &store);
    for p in bad.values_mut() {
        let i = p.len() / 2;
        p[i] ^= 1;
    }
    let Ran::Done(v) = b.logs_verify(1, &body, &filter.to_string(), &bad) else {
        panic!("no-verifier")
    };
    assert_eq!(
        (v["ok"].clone(), v["kind"].clone()),
        (json!(false), json!("rejected")),
        "{v}"
    );
}

#[test]
fn an_openvm_response_from_before_ethereums_pinned_cutover_is_the_wrong_system() {
    let v = json_file(&logs_dir().join("eth-26069632/segment.json"));
    let store = logs_dir().join("eth-26069632/store");
    let body = json!({"jsonrpc": "2.0", "id": 1, "result": v["result"]}).to_string();
    let b = Backend::new(
        crate::verifiers::real_anchor(),
        OpenVm::Helper(Some(stand_in("ok"))),
    );
    let Ran::Done(r) = b.logs_verify(
        1,
        &body,
        &v["filter"].to_string(),
        &proofs(&v["result"], &store),
    ) else {
        panic!("no-verifier")
    };
    assert_eq!(r["kind"], "rejected", "{r}");
    assert!(r["message"].as_str().unwrap().contains("OpenVM"), "{r}");
}

/// The Base response through the whole client: queued once, then served.
async fn base_flow(
    verdict_helper: Option<PathBuf>,
    tamper: impl Fn(&mut Value) + Send + Sync + 'static,
) -> (Harness, Result<Value, crate::Error>, Value, usize, usize) {
    base_flow_with(verdict_helper, None, tamper).await
}

/// [`base_flow`], with the client's timeout (the helper's limit too) set.
async fn base_flow_with(
    verdict_helper: Option<PathBuf>,
    timeout: Option<std::time::Duration>,
    tamper: impl Fn(&mut Value) + Send + Sync + 'static,
) -> (Harness, Result<Value, crate::Error>, Value, usize, usize) {
    let v = json_file(&logs_dir().join("base-52969856/segment.json"));
    let store = logs_dir().join("base-52969856/store");
    let mut resp = v["result"].clone();
    tamper(&mut resp);
    let filter = v["filter"].clone();
    let (from, to) = (n(&filter["fromBlock"]), n(&filter["toBlock"]));
    // The last block's hash, as the logs verifier proves it, is what the mock anchor serves.
    let (h_path, _) = helper();
    let b = Backend::new(
        crate::verifiers::real_anchor(),
        OpenVm::Helper(Some(h_path.clone())),
    );
    let honest = json!({"jsonrpc": "2.0", "id": 1, "result": v["result"]}).to_string();
    let Ran::Done(r) = b.logs_verify(
        8453,
        &honest,
        &filter.to_string(),
        &proofs(&v["result"], &store),
    ) else {
        panic!("no-verifier")
    };
    assert_eq!(r["ok"], true, "{r}");
    let last = r["report"]["lastProvenBlock"].clone();
    let w = World::new();
    w.head(
        8453,
        to + 10,
        &format!("0x{}", "77".repeat(32)),
        "signed",
        "sequencer",
    );
    let now = w.now_sec();
    w.anchor(AnchorEntry {
        chain_id: 8453,
        number: to,
        hash: last["hash"].as_str().unwrap().into(),
        level: "final",
        path: "game",
        time: now - 3600,
    });
    let asked = Arc::new(AtomicUsize::new(0));
    let polled = Arc::new(AtomicUsize::new(0));
    let (a, p) = (asked.clone(), polled.clone());
    let store2 = store.clone();
    w.intercept(Some(Arc::new(move |_, m, params| match m {
        "willow_getLogsProof" => Some(if a.fetch_add(1, Ordering::SeqCst) == 0 {
            json!({"error": {"code": -32000, "message": "proof queued", "data": {"status": "queued", "jobId": "j-1", "jobIds": ["j-1"], "retryAfterSeconds": 2}}})
        } else {
            json!({"result": resp.clone()})
        }),
        "willow_getProofJob" => Some(json!({"result": {"id": "j-1", "status": if p.fetch_add(1, Ordering::SeqCst) > 0 { "done" } else { "running" }}})),
        "willow_getProof" => {
            let id = params[0].as_str().unwrap();
            let bytes = std::fs::read(store2.join(format!("{id}.proof"))).unwrap();
            Some(json!({"result": {"proof": format!("0x{}", hex::encode(bytes))}}))
        }
        _ => None,
    })));
    let path = verdict_helper.unwrap_or(h_path);
    let h = harness_with(8453, w, "test-key", move |c| {
        let c = c.openvm_verifier(OpenVmVerifier::Path(path));
        match timeout {
            Some(t) => c.timeout(t),
            None => c,
        }
    })
    .await;
    let mut f = filter.clone();
    f["toBlock"] = json!(q(to));
    f["fromBlock"] = json!(q(from));
    let r = h.ask("eth_getLogs", json!([f])).await;
    let (x, y) = (asked.load(Ordering::SeqCst), polled.load(Ordering::SeqCst));
    (h, r, v, x, y)
}

#[tokio::test(flavor = "multi_thread")]
async fn logs_over_more_than_8_blocks_wait_for_the_job_fetch_each_proof_alone_verify_and_anchor_the_last_block(
) {
    let (h, r, v, asked, polled) = base_flow(None, |_| {}).await;
    let logs = r.unwrap();
    assert_eq!(
        logs.as_array().unwrap().len() as u64,
        v["logs"].as_u64().unwrap()
    );
    assert_eq!((asked, polled), (2, 2));
    let log = h.world.s().log.clone();
    let proof_req = log
        .iter()
        .find(|(_, items)| items.iter().any(|(m, _)| m == "willow_getProof"))
        .unwrap();
    assert_eq!(proof_req.1.len(), 1);
    let e = h.last();
    assert_eq!(e.status, Status::Verified);
    assert_eq!(e.block.map(|b| b.number), Some(n(&v["filter"]["toBlock"])));
    assert_eq!(
        e.unproven,
        vec!["[].blockHash".to_string(), "[].transactionHash".to_string()]
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tampered_response_or_a_failed_openvm_proof_never_reaches_the_app() {
    // A log's data changed in the response's data: it no longer hashes to what the proof commits to.
    let (_h, r, _, _, _) = base_flow(None, |a| {
        let rec = &mut a["parts"][0]["ranges"][0]["records"][0]["record"];
        let x = rec.as_str().unwrap().to_string();
        *rec = json!(flip(&x, 3));
    })
    .await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    // A record left out of a range.
    let (_h, r, _, _, _) = base_flow(None, |a| {
        a["parts"][0]["ranges"][0]["records"]
            .as_array_mut()
            .unwrap()
            .pop();
    })
    .await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    // The OpenVM verifier says the proof does not hold.
    let (_h, r, _, _, _) =
        base_flow(Some(stand_in("fail the proof does not verify")), |_| {}).await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    // No OpenVM verifier on this machine: unverifiable, not a failure.
    let (_h, r, _, _, _) = base_flow(
        Some(PathBuf::from("/nonexistent/willow-logs-verify-openvm")),
        |_| {},
    )
    .await;
    assert_eq!(
        code_kind(&r.unwrap_err()),
        (-32099, Some("no-verifier".into()))
    );
}

/// How the helper starts decides: one trusting other programs fails the check (`internal`); one
/// that says `error` and exits is unavailable; one that fails on a proof fails that check and is
/// started afresh for the next.
#[tokio::test(flavor = "multi_thread")]
async fn the_helpers_start_and_failures() {
    let other = script("echo 'ready 00 11 22'\nwhile read line; do echo ok; done");
    let (_h, r, _, _, _) = base_flow(Some(other), |_| {}).await;
    assert_eq!(
        code_kind(&r.unwrap_err()),
        (-32098, Some("internal".into()))
    );
    let broken = script("echo 'error no verifying keys'\nexit 1");
    let (_h, r, _, _, _) = base_flow(Some(broken), |_| {}).await;
    assert_eq!(
        code_kind(&r.unwrap_err()),
        (-32099, Some("no-verifier".into()))
    );
    let once = script(
        "echo \"ready $KEYS\"\nif [ -e \"$DIR/died\" ]; then while read line; do echo ok; done; \
         else touch \"$DIR/died\"; read line; exit 3; fi",
    );
    let (h, r, _, _, _) = base_flow(Some(once), |_| {}).await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    let v = json_file(&logs_dir().join("base-52969856/segment.json"));
    let mut f = v["filter"].clone();
    f["fromBlock"] = json!(q(n(&v["filter"]["fromBlock"])));
    f["toBlock"] = json!(q(n(&v["filter"]["toBlock"])));
    let r = h.ask("eth_getLogs", json!([f])).await.unwrap();
    assert_eq!(
        r.as_array().unwrap().len() as u64,
        v["logs"].as_u64().unwrap()
    );
}

/// A helper that stops answering is stopped at the client's timeout: one that never says it's
/// ready is unavailable; one that never gives its verdict on a proof fails that check (closed).
#[tokio::test(flavor = "multi_thread")]
async fn a_helper_that_hangs_is_stopped_at_the_timeout() {
    let t = std::time::Duration::from_millis(500);
    let mute = script("sleep 30");
    let started = std::time::Instant::now();
    let (_h, r, _, _, _) = base_flow_with(Some(mute), Some(t), |_| {}).await;
    assert_eq!(
        code_kind(&r.unwrap_err()),
        (-32099, Some("no-verifier".into()))
    );
    let stuck = script("echo \"ready $KEYS\"\nread line\nsleep 30");
    let (_h, r, _, _, _) = base_flow_with(Some(stuck), Some(t), |_| {}).await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    // One that stops reading: the proof (hundreds of KB of hex) fills the pipe, and the write
    // must not hold the client past the timeout either.
    let deaf = script("echo \"ready $KEYS\"\nsleep 30");
    let (_h, r, _, _, _) = base_flow_with(Some(deaf), Some(t), |_| {}).await;
    assert_eq!(code_kind(&r.unwrap_err()), (-32098, Some("logs".into())));
    assert!(
        started.elapsed() < std::time::Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
}

/// A helper that exits after a verdict, while idle, is started again for the next proof, not
/// asked (which would fail that proof).
#[tokio::test(flavor = "multi_thread")]
async fn a_helper_that_exits_while_idle_is_started_again() {
    let one_shot = script("echo \"ready $KEYS\"\nread line\necho ok");
    let (h, r, v, _, _) = base_flow(Some(one_shot), |_| {}).await;
    assert!(r.is_ok(), "{r:?}");
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let mut f = v["filter"].clone();
    f["fromBlock"] = json!(q(n(&v["filter"]["fromBlock"])));
    f["toBlock"] = json!(q(n(&v["filter"]["toBlock"])));
    let r = h.ask("eth_getLogs", json!([f])).await;
    assert!(r.is_ok(), "{r:?}");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_response_for_another_range_fails_verification() {
    let (_h, r, _, _, _) = base_flow(None, |a| {
        a["fromBlock"] = json!(q(n(&a["fromBlock"]) + 1));
    })
    .await;
    let e = r.unwrap_err();
    assert_eq!(e.code(), -32098, "{e}");
}

#[test]
fn the_helper_in_use_is_reported() {
    let (_, real) = helper();
    if !real {
        eprintln!(
            "WILLOW_LOGS_VERIFY_OPENVM is not set: OpenVM proofs went to the stand-in helper"
        );
    }
}
