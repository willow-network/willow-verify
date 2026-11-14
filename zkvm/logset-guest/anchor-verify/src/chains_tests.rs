//! The chains added after the first seven, on anchors eth-anchor's `record_new_chain_anchors`
//! saved from public endpoints (`web/verify/test/anchor/chains/`), verified with the light client
//! they were saved with, then tampered.

use crate::Anchor;
use serde_json::{json, Value};
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../web/verify/test/anchor/chains")
}

fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir().join(name)).unwrap()).unwrap()
}

fn body(result: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string()
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// The fixtures `l2-<chain>-*.json`, by name.
fn fixtures(chain: u64) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = std::fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.starts_with(&format!("l2-{chain}-"))
                .then(|| (name.clone(), read(&name)))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn synced(now: f64) -> Anchor {
    let c = read("checkpoint.json");
    let mut a = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    let r = parse(&a.bootstrap(&body(&read("bootstrap.json")), now));
    assert_eq!(r["ok"], true, "{r}");
    let r = parse(&a.updates(&body(&read("updates.json"))));
    assert_eq!(r["ok"], true, "{r}");
    a
}

fn verify(chain: u64, f: &Value, result: &Value, hash: &str) -> Value {
    let now = f["savedAt"].as_f64().unwrap();
    let n = f["block"].as_u64().unwrap();
    parse(&synced(now).verify(chain as u32, n as f64, hash, &body(result), now))
}

/// Flips the last hex digit of the first long hex string in `v` (a proof node or code).
fn flip_first_long(v: &mut Value) -> bool {
    match v {
        Value::String(s) if s.starts_with("0x") && s.len() > 200 => {
            let last = if s.ends_with('0') { "1" } else { "0" };
            s.truncate(s.len() - 1);
            s.push_str(last);
            true
        }
        Value::Array(a) => a.iter_mut().any(flip_first_long),
        Value::Object(o) => o.values_mut().any(flip_first_long),
        _ => false,
    }
}

/// Every saved anchor of `chain` verifies at the level saved, with the public node's hash, and
/// is refused once tampered; `others` are chains of the same family it must not pass as.
fn check(chain: u64, others: &[u64], levels: &[&str]) {
    let all = fixtures(chain);
    assert!(!all.is_empty(), "no saved anchors of chain {chain}");
    let mut seen = Vec::new();
    for (name, f) in &all {
        let hash = f["hash"].as_str().unwrap();
        let result = &f["result"];
        let r = verify(chain, f, result, hash);
        assert_eq!(
            (r["ok"].as_bool(), r["matches"].as_bool(), &r["level"]),
            (Some(true), Some(true), &f["expect"]["level"]),
            "{name}: {r}"
        );
        seen.push(r["level"].as_str().unwrap().to_string());
        // Another hash is not the block's.
        let other = format!("0x{}", "ab".repeat(32));
        assert_eq!(verify(chain, f, result, &other)["matches"], false, "{name}");
        // The top header with one byte changed (its state root): not the one claimed or signed.
        let mut t = result.clone();
        let h = t["headers"][0].as_str().unwrap().to_string();
        let i = h.len() - 300;
        let flipped = format!(
            "{}{}{}",
            &h[..i],
            if &h[i..i + 1] == "0" { "1" } else { "0" },
            &h[i + 1..]
        );
        t["headers"][0] = json!(flipped);
        let r = verify(chain, f, &t, hash);
        assert_ne!(r["ok"], true, "{name}: a changed header passed: {r}");
        // The Ethereum state witness with a proof node (or code) changed.
        let mut t = result.clone();
        assert!(flip_first_long(&mut t["l1"]), "{name}: no witness");
        let r = verify(chain, f, &t, hash);
        assert_ne!(r["ok"], true, "{name}: a changed state witness passed: {r}");
        // Another chain's anchor.
        for o in others {
            let r = verify(*o, f, result, hash);
            assert_ne!(r["ok"], true, "{name} passed as chain {o}: {r}");
        }
    }
    for l in levels {
        assert!(
            seen.iter().any(|s| s == l),
            "chain {chain}: no {l} anchor saved ({seen:?})"
        );
    }
}

#[test]
fn soneium_blocks_anchor_on_ethereum() {
    check(1868, &[10, 130, 7777777], &["committed", "final", "signed"]);
}

#[test]
fn zora_blocks_anchor_on_ethereum() {
    check(
        7777777,
        &[8453, 480, 1868],
        &["committed", "final", "signed"],
    );
}

#[test]
fn world_chain_blocks_anchor_on_ethereum() {
    check(
        480,
        &[8453, 7777777, 1868],
        &["committed", "final", "signed"],
    );
}
