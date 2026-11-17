//! Against Willow's live RPC (ignored by default): `WILLOW_LIVE=1 cargo test --release -- --ignored
//! live`. Keyless on the public path; with `WILLOW_KEY`, state and calls too. Every response through
//! the real light clients from the shipped checkpoint, nothing mocked.

use crate::config::{Config, Event, Status, StorageChoice};
use crate::Verifier;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

fn live(chain_id: u64) -> (Verifier, Arc<Mutex<Vec<Event>>>) {
    let events = Arc::new(Mutex::new(vec![]));
    let ev = events.clone();
    let mut c = Config::new(chain_id)
        .storage(StorageChoice::None)
        .on_verification(move |e| ev.lock().unwrap().push(e.clone()));
    if let Ok(k) = std::env::var("WILLOW_KEY") {
        c = c.key(k);
    }
    // The repo's verifiers verify what the release holds back (fees here).
    c.unheld = true;
    (Verifier::new(c).unwrap(), events)
}

fn report(
    chain: u64,
    method: &str,
    r: &Result<Value, crate::Error>,
    events: &Arc<Mutex<Vec<Event>>>,
    t: std::time::Instant,
) {
    let e = events.lock().unwrap().last().cloned();
    println!(
        "chain {chain} {method}: {} | {:?} block {:?} level {:?} {:.2}s",
        match r {
            Ok(v) => format!("ok {}", crate::hex::shown(v, 70)),
            Err(e) => format!("ERR {}", e.to_json()),
        },
        e.as_ref().map(|e| e.status),
        e.as_ref().and_then(|e| e.block.as_ref().map(|b| b.number)),
        e.as_ref().and_then(|e| e.level),
        t.elapsed().as_secs_f64()
    );
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn live_smoke() {
    if std::env::var("WILLOW_LIVE").is_err() {
        return;
    }
    let keyed = std::env::var("WILLOW_KEY").is_ok();
    let mut failed = vec![];
    for chain in [1u64, 8453, 42161, 10, 56, 84532] {
        let (v, ev) = live(chain);
        let t = std::time::Instant::now();
        let n = v.request("eth_blockNumber", json!([])).await;
        report(chain, "eth_blockNumber", &n, &ev, t);
        let mut asks: Vec<(&str, Value)> = vec![
            ("eth_getBlockByNumber", json!(["latest", false])),
            ("eth_getBlockReceipts", json!(["latest"])),
        ];
        if chain == 1 || chain == 8453 {
            asks.push(("eth_gasPrice", json!([])));
            asks.push(("eth_feeHistory", json!(["0x4", "latest", [25, 75]])));
        }
        if keyed && chain == 1 {
            asks.push((
                "eth_getBalance",
                json!(["0x0000000000000000000000000000000000000000", "latest"]),
            ));
            asks.push(("eth_call", json!([{"to": "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48", "data": "0x18160ddd"}, "latest"])));
        }
        for (m, p) in asks {
            let t = std::time::Instant::now();
            let r = v.request(m, p).await;
            report(chain, m, &r, &ev, t);
            if r.is_err() || ev.lock().unwrap().last().map(|e| e.status) != Some(Status::Verified) {
                failed.push(format!("{chain} {m}"));
            }
        }
        if n.is_err() {
            failed.push(format!("{chain} eth_blockNumber"));
        }
    }
    println!("not verified: {failed:?}");
}
