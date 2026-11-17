//! The CLI's other anchorings (a node the user trusts; none) and the checkpoint choice
//! (regressions.test.ts, "the checkpoint").

use super::support::*;
use crate::checkpoint::{self, slot_time, Checkpoint, Choice, MAX_START_AGE_SECS};
use crate::config::{AnchoredBy, Anchoring, Status};
use serde_json::json;

#[tokio::test(flavor = "multi_thread")]
async fn anchoring_rpc_checks_each_block_hash_with_the_trusted_node() {
    let head = block_of("base-56049971.json");
    let world = full_world();
    // The trusted node is the mock server's other key path; it serves blocks by number and "latest".
    world.standard(
        &key(8453, "eth_getBlockByNumber", &json!(["latest", false])),
        json!({"result": {"number": q(head.0), "hash": head.1}}),
    );
    world.standard(
        &key(8453, "eth_getBlockByNumber", &json!([q(head.0), false])),
        json!({"result": {"number": q(head.0), "hash": head.1}}),
    );
    let server2 = serve(world.clone()).await;
    let trusted = server2.url(8453, "trusted");
    let h = harness_with(8453, world, "test-key", move |c| {
        c.anchoring(Anchoring::Rpc(trusted))
    })
    .await;
    let b = h
        .ask("eth_getBlockByNumber", json!(["latest", false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(head.1));
    let e = h.last();
    assert_eq!(
        (e.status, e.anchored_by, e.level),
        (Status::Verified, AnchoredBy::Rpc, None)
    );
    assert!(h.world.sent("willow_getBlockAnchor").is_empty());
    // A trusted node that names another hash for the block: shown wrong.
    h.world.standard(
        &key(8453, "eth_getBlockByNumber", &json!([q(head.0), false])),
        json!({"result": {"number": q(head.0), "hash": format!("0x{}", "ab".repeat(32))}}),
    );
    let h2 = harness_with(8453, h.world.clone(), "test-key", {
        let t = server2.url(8453, "trusted");
        move |c| c.anchoring(Anchoring::Rpc(t))
    })
    .await;
    let e = h2
        .ask("eth_getBlockByHash", json!([head.1, false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("anchor-mismatch".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn anchoring_none_verifies_the_proof_and_says_the_block_is_not_anchored() {
    let head = block_of("base-56049971.json");
    let world = full_world();
    world.standard("8453|eth_blockNumber|*", json!({"result": q(head.0)}));
    let h = harness_with(8453, world, "test-key", |c| c.anchoring(Anchoring::None)).await;
    assert_eq!(
        h.ask("eth_blockNumber", json!([])).await.unwrap(),
        json!(q(head.0))
    );
    let b = h
        .ask("eth_getBlockByNumber", json!([q(head.0), false]))
        .await
        .unwrap();
    assert_eq!(b["hash"], json!(head.1));
    let e = h.last();
    assert_eq!(
        (e.status, e.anchored_by, e.level),
        (Status::Verified, AnchoredBy::None, None)
    );
    assert_eq!(e.block.map(|b| (b.number, b.hash)), Some(head));
    assert!(h.world.sent("willow_getBlockAnchor").is_empty());
    // The proof is still verified.
    let mut lie = h
        .world
        .proof(
            8453,
            "willow_getBlockProof",
            json!([q(block_of("base-56049971.json").0)]),
        )
        .result;
    lie["transactions"].as_array_mut().unwrap().remove(0);
    h.world.intercept(Some(std::sync::Arc::new(move |_, m, _| {
        (m == "willow_getBlockProof").then(|| json!({"result": lie}))
    })));
    let h2 = harness_with(8453, h.world.clone(), "test-key", |c| {
        c.anchoring(Anchoring::None)
    })
    .await;
    let e = h2
        .ask(
            "eth_getBlockByNumber",
            json!([q(block_of("base-56049971.json").0), false]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("transactions".into())));
}

fn newer() -> Checkpoint {
    let s = checkpoint::shipped();
    Checkpoint {
        root: format!("0x{}", "ab".repeat(32)),
        slot: s.ethereum.slot + 8192 * 4,
    }
}

async fn choose(
    world: &World,
    base: &str,
    apis: &[&str],
    cp: Option<&Checkpoint>,
    now: u64,
) -> Result<Checkpoint, crate::Error> {
    let http = reqwest::Client::new();
    let apis: Vec<String> = apis.iter().map(|a| format!("{base}/{a}")).collect();
    let url = format!("{base}/verify/checkpoint.json");
    let _ = world;
    checkpoint::choose(&Choice {
        http: &http,
        checkpoint: cp,
        sepolia: None,
        bnb: None,
        checkpoint_url: &url,
        beacon_apis: &apis,
        now_sec: now,
    })
    .await
    .map(|s| s.ethereum)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_checkpoint_passed_in_else_the_shipped_one_else_willows_confirmed_by_the_beacon_apis() {
    let shipped = checkpoint::shipped().ethereum;
    let fresh = slot_time(shipped.slot) + 3600;
    let stale = slot_time(shipped.slot) + 31 * 24 * 3600;
    assert!(stale > slot_time(shipped.slot) + MAX_START_AGE_SECS);
    let w = World::new();
    let s = serve(w.clone()).await;
    let nw = newer();
    let file = |root: &str| {
        (
            200u16,
            json!({"data": {"root": root, "canonical": true}}).to_string(),
        )
    };
    {
        let mut st = w.s();
        st.files.insert(
            "/verify/checkpoint.json".into(),
            (200, json!({"root": nw.root, "slot": nw.slot}).to_string()),
        );
        st.files.insert(
            format!("/beacon-a/eth/v1/beacon/headers/{}", nw.slot),
            file(&nw.root),
        );
        st.files.insert(
            format!("/beacon-b/eth/v1/beacon/headers/{}", nw.slot),
            file(&nw.root),
        );
        st.files.insert(
            format!("/beacon-c/eth/v1/beacon/headers/{}", nw.slot),
            file(&format!("0x{}", "cd".repeat(32))),
        );
    }
    // The one passed in; the shipped one while it is young enough.
    assert_eq!(
        choose(&w, &s.base, &[], Some(&nw), stale).await.unwrap(),
        nw
    );
    assert_eq!(
        choose(&w, &s.base, &["beacon-a"], None, fresh)
            .await
            .unwrap(),
        shipped
    );
    // Old: Willow's, confirmed (an API that can't be asked is skipped).
    assert_eq!(
        choose(
            &w,
            &s.base,
            &["beacon-a", "beacon-b", "beacon-x"],
            None,
            stale
        )
        .await
        .unwrap(),
        nw
    );
    // An API that disagrees, or none that can be asked: unverifiable.
    let e = choose(&w, &s.base, &["beacon-a", "beacon-c"], None, stale)
        .await
        .unwrap_err();
    assert_eq!(
        code_kind(&e),
        (-32099, Some("anchor-stale-checkpoint".into()))
    );
    let e = choose(&w, &s.base, &["beacon-x"], None, stale)
        .await
        .unwrap_err();
    assert_eq!(
        code_kind(&e),
        (-32099, Some("anchor-stale-checkpoint".into()))
    );
    // Nothing newer published.
    w.s().files.insert(
        "/verify/checkpoint.json".into(),
        (
            200,
            json!({"root": shipped.root, "slot": shipped.slot}).to_string(),
        ),
    );
    let e = choose(&w, &s.base, &[], None, stale).await.unwrap_err();
    assert_eq!(
        code_kind(&e),
        (-32099, Some("anchor-stale-checkpoint".into()))
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_shipped_bnb_start_a_day_old_gives_way_to_a_newer_published_one() {
    let s0 = checkpoint::shipped();
    let w = World::new();
    let s = serve(w.clone()).await;
    let newer_bnb = json!({"number": s0.bnb.number + 1000, "hash": format!("0x{}", "12".repeat(32)), "time": s0.bnb.time + 3000});
    w.s().files.insert(
        "/verify/checkpoint.json".into(),
        (
            200,
            json!({"root": s0.ethereum.root, "slot": s0.ethereum.slot, "bnb": newer_bnb})
                .to_string(),
        ),
    );
    let http = reqwest::Client::new();
    let url = format!("{}/verify/checkpoint.json", s.base);
    let pick = |now: u64| {
        let (http, url) = (http.clone(), url.clone());
        async move {
            checkpoint::choose(&Choice {
                http: &http,
                checkpoint: None,
                sepolia: None,
                bnb: None,
                checkpoint_url: &url,
                beacon_apis: &[],
                now_sec: now,
            })
            .await
            .unwrap()
            .bnb
        }
    };
    assert_eq!(pick(s0.bnb.time + 60).await, s0.bnb);
    assert_eq!(
        pick(s0.bnb.time + 2 * 86_400).await.number,
        s0.bnb.number + 1000
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_trusted_node_without_the_block_is_unverifiable_not_a_verdict() {
    let head = block_of("base-56049971.json");
    let world = full_world();
    world.standard(
        &key(8453, "eth_getBlockByNumber", &json!([q(head.0), false])),
        json!({"result": null}),
    );
    let server2 = serve(world.clone()).await;
    let trusted = server2.url(8453, "trusted");
    let h = harness_with(8453, world, "test-key", move |c| {
        c.anchoring(Anchoring::Rpc(trusted))
    })
    .await;
    let e = h
        .ask("eth_getBlockByHash", json!([head.1, false]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32099, Some("anchor-refused".into())));
}
