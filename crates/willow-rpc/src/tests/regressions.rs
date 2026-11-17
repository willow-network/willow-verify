//! Faults found in review, each with the test that showed it.

use crate::anchor::{self, Anchors};
use crate::config::Level;
use crate::error::Error;
use crate::http::Reply;
use crate::verifiers::{AnchorApi, Backend, OpenVm};
use futures::FutureExt;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

/// A light client that needs a bootstrap once, then passes every anchor at level final.
struct NeedsBootstrap(Arc<AtomicUsize>);

impl AnchorApi for NeedsBootstrap {
    fn restore(&mut self, _: &str, _: f64) -> String {
        json!({"ok": true}).to_string()
    }
    fn bootstrap(&mut self, _: &str, _: f64) -> String {
        json!({"ok": true, "period": 1}).to_string()
    }
    fn updates(&mut self, _: &str) -> String {
        json!({"ok": true, "period": 1}).to_string()
    }
    fn verify(&mut self, _: u32, _: f64, hash: &str, _: &str, now: f64) -> String {
        if self.0.fetch_add(1, Ordering::SeqCst) == 0 {
            return json!({"ok": false, "kind": "need-bootstrap", "message": "no committee"})
                .to_string();
        }
        json!({"ok": true, "level": "final", "path": "history", "hash": hash, "matches": true, "headTime": now as u64}).to_string()
    }
    fn saved(&self) -> String {
        "null".into()
    }
    fn reset(&mut self) {}
    fn start(&mut self, _: &str, _: &str) -> String {
        json!({"ok": true}).to_string()
    }
    fn bootstrap_for(&mut self, _: &str, _: &str, _: f64) -> String {
        json!({"ok": true}).to_string()
    }
    fn updates_for(&mut self, _: &str, _: &str) -> String {
        json!({"ok": true, "period": 1}).to_string()
    }
    fn restore_for(&mut self, _: &str, _: &str, _: f64) -> String {
        json!({"ok": true}).to_string()
    }
    fn saved_for(&self, _: &str) -> String {
        "null".into()
    }
    fn reset_for(&mut self, _: &str) {}
}

/// A request whose caller gave up (a timeout, a dropped future) while its anchor was catching the
/// light client up must not leave the light client locked: the catch-up finishes on its own, and
/// the next anchor proceeds.
#[tokio::test(flavor = "multi_thread")]
async fn a_cancelled_anchor_does_not_leave_the_light_client_locked() {
    let calls = Arc::new(AtomicUsize::new(0));
    let c = calls.clone();
    let factory: crate::verifiers::AnchorFactory =
        Arc::new(move |_, _| Box::new(NeedsBootstrap(c.clone())));
    let rpc: anchor::Rpc = Arc::new(|_: u64, m: &str, _: Value| {
        let slow = m == "willow_lightClientBootstrap";
        async move {
            if slow {
                tokio::time::sleep(Duration::from_millis(300)).await;
            }
            let v = json!({"jsonrpc": "2.0", "id": 1, "result": {"blockNumber": "0x7"}});
            Ok::<_, Error>(Reply {
                text: v.to_string().into(),
                value: Arc::new(v),
            })
        }
        .boxed()
    });
    let a = Anchors::new(anchor::Options {
        backend: Arc::new(Backend::new(factory, OpenVm::None)),
        rpc,
        storage: None,
        starts: Arc::new(|| async { Ok::<_, Error>(crate::checkpoint::shipped()) }.boxed()),
        min_level: Level::Signed,
        head_max_age_ms: Some(2000),
        max_head_age_sec: 600,
        now_ms: Arc::new(|| 1_794_400_000_000),
        waiting_ms: Some(0),
        anchor_wait_ms: 50,
    });
    // The head's caller gives up while the light client waits for its bootstrap.
    assert!(
        tokio::time::timeout(Duration::from_millis(50), a.head(1, "latest", false, None))
            .await
            .is_err()
    );
    // Another block's anchor still gets the light client once the bootstrap is in.
    let hash = format!("0x{}", "ab".repeat(32));
    let r = tokio::time::timeout(Duration::from_secs(5), a.block(1, 7, &hash, None, None)).await;
    let r = r.expect("the light client stayed locked by the cancelled request");
    assert_eq!(r.unwrap().level, Some(Level::Final));
}
