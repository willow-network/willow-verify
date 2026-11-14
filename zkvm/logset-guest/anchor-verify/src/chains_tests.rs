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

/// Both light clients synced from the saved data: Ethereum's, and Sepolia's when saved.
fn synced(now: f64) -> Anchor {
    let c = read("checkpoint.json");
    let mut a = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    let r = parse(&a.bootstrap(&body(&read("bootstrap.json")), now));
    assert_eq!(r["ok"], true, "{r}");
    let r = parse(&a.updates(&body(&read("updates.json"))));
    assert_eq!(r["ok"], true, "{r}");
    if dir().join("sepolia-checkpoint.json").exists() {
        sync_sepolia(&mut a, now);
    }
    a
}

fn sync_sepolia(a: &mut Anchor, now: f64) {
    let r = parse(&a.start("sepolia", &read("sepolia-checkpoint.json").to_string()));
    assert_eq!(r["ok"], true, "{r}");
    let r = parse(&a.bootstrap_for("sepolia", &body(&read("sepolia-bootstrap.json")), now));
    assert_eq!(r["ok"], true, "{r}");
    let r = parse(&a.updates_for("sepolia", &body(&read("sepolia-updates.json"))));
    assert_eq!(r["ok"], true, "{r}");
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

#[test]
fn op_sepolia_blocks_anchor_on_sepolia() {
    check(11155420, &[10, 1301, 763373], LEVELS_OP_SEPOLIA);
}

#[test]
fn base_sepolia_blocks_anchor_on_sepolia() {
    check(84532, &[8453, 11155420], LEVELS_BASE_SEPOLIA);
}

#[test]
fn unichain_sepolia_blocks_anchor_on_sepolia() {
    check(1301, &[130, 11155420, 763373], LEVELS_UNICHAIN_SEPOLIA);
}

#[test]
fn ink_sepolia_blocks_anchor_on_sepolia() {
    check(763373, &[57073, 11155420, 1301], LEVELS_INK_SEPOLIA);
}

#[test]
fn arbitrum_sepolia_blocks_anchor_on_sepolia() {
    check(421614, &[42161, 46630], LEVELS_ARBITRUM_SEPOLIA);
}

#[test]
fn robinhood_testnet_blocks_anchor_on_sepolia() {
    check(46630, &[4663, 421614], LEVELS_ROBINHOOD_TESTNET);
}

/// The levels saved for each testnet (2026-11-14): its claims where a free RPC serves the state, and
/// its sequencer's newest block.
const LEVELS_OP_SEPOLIA: &[&str] = &["committed", "signed"];
const LEVELS_BASE_SEPOLIA: &[&str] = &["committed", "signed"];
const LEVELS_UNICHAIN_SEPOLIA: &[&str] = &["committed", "signed"];
const LEVELS_INK_SEPOLIA: &[&str] = &["committed", "signed"];
const LEVELS_ARBITRUM_SEPOLIA: &[&str] = &["committed", "signed"];
const LEVELS_ROBINHOOD_TESTNET: &[&str] = &["committed", "signed"];

fn fin(r: &mut Value) -> &mut Value {
    &mut r["update"]["data"]["finalized_header"]
}

/// A Sepolia anchor's finality update, tampered where only Sepolia's light client reads it.
#[test]
fn sepolia_finality_updates_are_checked_as_gloas_data() {
    let (name, f) = fixtures(1301)
        .into_iter()
        .next()
        .expect("a Unichain Sepolia anchor");
    let now = f["savedAt"].as_f64().unwrap();
    let n = f["block"].as_u64().unwrap() as f64;
    let hash = f["hash"].as_str().unwrap();
    let run = |result: &Value| parse(&synced(now).verify(1301, n, hash, &body(result), now));
    let kind = |v: &Value| v["kind"].as_str().unwrap_or("").to_string();
    assert_eq!(run(&f["result"])["ok"], true, "{name}");
    // The execution header with a byte changed: not the block the beacon block names.
    let mut r = f["result"].clone();
    let h = fin(&mut r);
    let x = h["execution_header"].as_str().unwrap().to_string();
    h["execution_header"] = json!(format!(
        "{}{}",
        &x[..x.len() - 1],
        if x.ends_with('0') { "1" } else { "0" }
    ));
    assert_eq!(kind(&run(&r)), "rejected", "{name}");
    // No execution header.
    let mut r = f["result"].clone();
    fin(&mut r)
        .as_object_mut()
        .unwrap()
        .remove("execution_header");
    assert_eq!(kind(&run(&r)), "malformed-input", "{name}");
    // A node of the block hash's branch changed.
    let mut r = f["result"].clone();
    fin(&mut r)["execution_branch"][3] = json!(format!("0x{}", "22".repeat(32)));
    assert_eq!(kind(&run(&r)), "rejected", "{name}");
    // Named as a fork Sepolia's light client does not read.
    let mut r = f["result"].clone();
    r["update"]["version"] = json!("fulu");
    assert_eq!(kind(&run(&r)), "unknown-fork", "{name}");
    // Ethereum's light client knows no Gloas: the same update is not Ethereum's.
    let mut a = synced(now);
    let r = parse(&a.verify(1301, n, hash, &body(&f["result"]), now));
    assert_eq!(r["ok"], true);
    let c = read("checkpoint.json");
    let mut m = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    parse(&m.bootstrap(&body(&read("bootstrap.json")), now));
    parse(&m.updates(&body(&read("updates.json"))));
    let r = parse(&m.verify(1, n, hash, &body(&f["result"]), now));
    assert_ne!(r["ok"], true, "{r}");
}

#[test]
fn sepolias_light_client_asks_for_its_own_data_and_ages() {
    let (_, f) = fixtures(1301)
        .into_iter()
        .next()
        .expect("a Unichain Sepolia anchor");
    let now = f["savedAt"].as_f64().unwrap();
    let n = f["block"].as_u64().unwrap() as f64;
    let hash = f["hash"].as_str().unwrap();
    // Ethereum's light client synced, Sepolia's not: Sepolia's bootstrap is asked for.
    let c = read("checkpoint.json");
    let mut a = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    parse(&a.bootstrap(&body(&read("bootstrap.json")), now));
    parse(&a.updates(&body(&read("updates.json"))));
    parse(&a.start("sepolia", &read("sepolia-checkpoint.json").to_string()));
    let r = parse(&a.verify(1301, n, hash, &body(&f["result"]), now));
    assert_eq!(
        (r["kind"].as_str(), r["lightClient"].as_str()),
        (Some("need-bootstrap"), Some("sepolia")),
        "{r}"
    );
    // Ethereum's bootstrap is not Sepolia's.
    let r = parse(&a.bootstrap_for("sepolia", &body(&read("bootstrap.json")), now));
    assert_ne!(r["ok"], true, "{r}");
    // Kept and restored.
    sync_sepolia(&mut a, now);
    let saved = a.saved_for("sepolia");
    let mut b = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    parse(&b.bootstrap(&body(&read("bootstrap.json")), now));
    parse(&b.updates(&body(&read("updates.json"))));
    parse(&b.start("sepolia", &read("sepolia-checkpoint.json").to_string()));
    assert_eq!(parse(&b.restore_for("sepolia", &saved, now))["ok"], true);
    assert_eq!(
        parse(&b.verify(1301, n, hash, &body(&f["result"]), now))["ok"],
        true
    );
    // A start more than 30 days old.
    let later = now + 31.0 * 86_400.0;
    let mut d = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    parse(&d.start("sepolia", &read("sepolia-checkpoint.json").to_string()));
    let r = parse(&d.bootstrap_for("sepolia", &body(&read("sepolia-bootstrap.json")), later));
    assert_eq!(r["kind"], "stale-checkpoint", "{r}");
}
