//! The npm client's test world (web/client/test/support), ported: the recorded fixtures as one
//! world a mock server serves, a mock anchor (the stand-in signature: the server can't sign for
//! Ethereum's validators), and a harness. Items and state are verified by the real verifiers.

use crate::config::{Config, Event, StorageChoice};
use crate::verifiers::{AnchorApi, AnchorFactory};
use crate::{Checkpoint, Verifier};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

/// The repo root: `WILLOW_REPO`, else up from this crate.
pub fn repo() -> PathBuf {
    if let Some(r) = std::env::var_os("WILLOW_REPO") {
        return PathBuf::from(r);
    }
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if p.join("web/client/test/fixtures").is_dir() {
            return p;
        }
        assert!(
            p.pop(),
            "the repo's fixtures are not above {}: set WILLOW_REPO",
            env!("CARGO_MANIFEST_DIR")
        );
    }
}

pub fn fixtures() -> PathBuf {
    repo().join("web/client/test/fixtures")
}

pub fn items_dir() -> PathBuf {
    repo().join("web/verify/test/items")
}

pub fn json_file(p: &Path) -> Value {
    let t = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    serde_json::from_str(&t).unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

pub fn fixture(rel: &str) -> Value {
    json_file(&fixtures().join(rel))
}

pub fn fixture_names(dir: &str) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(fixtures().join(dir))
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|f| f.ends_with(".json"))
        .collect();
    v.sort();
    v
}

/// Chains by fixture prefix.
pub fn chain_of(file: &str) -> u64 {
    const CHAIN: [(&str, u64); 7] = [
        ("ethereum", 1),
        ("base", 8453),
        ("op", 10),
        ("unichain", 130),
        ("ink", 57073),
        ("arbitrum-one", 42161),
        ("robinhood", 4663),
    ];
    CHAIN
        .iter()
        .find(|(p, _)| file.starts_with(&format!("{p}-")))
        .map(|(_, c)| *c)
        .unwrap_or_else(|| panic!("no chain for {file}"))
}

pub fn empty_object(x: &Value) -> bool {
    x.is_null() || x.as_object().is_some_and(Map::is_empty)
}

pub fn n(v: &Value) -> u64 {
    crate::hex::to_number(v).unwrap_or_else(|| panic!("not a number: {v}"))
}

pub fn q(x: u64) -> String {
    format!("0x{x:x}")
}

/// A JSON value with hex lowercased, `{blockHash}` read as the hash, keys sorted, nulls dropped:
/// for matching requests.
pub fn canon(v: &Value) -> Value {
    match v {
        Value::String(s)
            if s.starts_with("0x") && s[2..].bytes().all(|b| b.is_ascii_hexdigit()) =>
        {
            json!(s.to_ascii_lowercase())
        }
        Value::Array(a) => Value::Array(a.iter().map(canon).collect()),
        Value::Object(o) => {
            if o.len() == 1 && o.get("blockHash").is_some_and(Value::is_string) {
                return canon(&o["blockHash"]);
            }
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort();
            let mut out = Map::new();
            for k in keys {
                if !o[k].is_null() {
                    out.insert(k.clone(), canon(&o[k]));
                }
            }
            Value::Object(out)
        }
        // JSON numbers as JavaScript reads them: 10.0 is 10.
        Value::Number(x)
            if x.is_f64()
                && x.as_f64().unwrap().fract() == 0.0
                && x.as_f64().unwrap().abs() < 9e15 =>
        {
            json!(x.as_f64().unwrap() as i64)
        }
        v => v.clone(),
    }
}

/// The payloads of an RLP list's items.
fn rlp_items(b: &[u8]) -> Vec<&[u8]> {
    fn head(b: &[u8], at: usize) -> (usize, usize) {
        let p = b[at];
        let be = |n: usize| {
            b[at + 1..at + 1 + n]
                .iter()
                .fold(0usize, |v, x| (v << 8) | *x as usize)
        };
        match p {
            0..=0x7f => (at, 1),
            0x80..=0xb7 => (at + 1, (p - 0x80) as usize),
            0xb8..=0xbf => {
                let n = (p - 0xb7) as usize;
                (at + 1 + n, be(n))
            }
            0xc0..=0xf7 => (at + 1, (p - 0xc0) as usize),
            _ => {
                let n = (p - 0xf7) as usize;
                (at + 1 + n, be(n))
            }
        }
    }
    let (start, len) = head(b, 0);
    let mut out = vec![];
    let mut at = start;
    while at < start + len {
        let (s, l) = head(b, at);
        out.push(&b[s..s + l]);
        at = s + l;
    }
    out
}

pub struct Header {
    pub number: u64,
}

pub fn header(rlp_hex: &str) -> Header {
    let b = crate::hex::to_bytes(rlp_hex).unwrap();
    let f = rlp_items(&b);
    let uint = |x: &[u8]| x.iter().fold(0u64, |v, y| (v << 8) | *y as u64);
    Header { number: uint(f[8]) }
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub chain_id: u64,
    pub method: String,
    pub result: Value,
    pub proven: Value,
    pub full: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct AnchorEntry {
    pub chain_id: u64,
    pub number: u64,
    pub hash: String,
    pub level: &'static str,
    pub path: &'static str,
    /// Seconds.
    pub time: u64,
}

pub type Intercept = Arc<dyn Fn(u64, &str, &[Value]) -> Option<Value> + Send + Sync>;

/// The lying server's mutation of a response: one item (with its request), or the whole batch.
#[derive(Clone)]
pub enum Mutation {
    Item(Arc<dyn Fn(&mut Value, &str, &[Value]) -> bool + Send + Sync>),
    Batch(Arc<dyn Fn(&mut Vec<Value>, &[(Value, String, Vec<Value>)]) -> bool + Send + Sync>),
}

/// The mock anchor's stand-in for a signature.
pub const ANCHOR_SECRET: &str = "mock-validators";

pub fn anchor_sig(m: &Value) -> String {
    crate::hex::keccak_hex(format!("{ANCHOR_SECRET}{}", canon(m)).as_bytes())
}

#[derive(Default)]
pub struct State {
    /// Proof responses by `{chainId}|{method}|{canon(params)}`.
    pub proofs: HashMap<String, Entry>,
    pub states: HashMap<String, Value>,
    pub hashes: HashMap<String, String>,
    pub heads: HashMap<u64, AnchorEntry>,
    pub finalized: HashMap<u64, AnchorEntry>,
    pub anchors: HashMap<String, AnchorEntry>,
    /// Standard methods: `{chainId}|{method}|{canon(params)}` or `{chainId}|{method}|*`.
    pub standard: HashMap<String, Value>,
    /// Real anchor bodies by `{chainId}:{param}`.
    pub real_anchors: HashMap<String, Value>,
    pub bootstrap: Option<Value>,
    pub updates: Option<Value>,
    /// Every HTTP request: chain and its items' (method, params).
    pub log: Vec<(u64, Vec<(String, Value)>)>,
    pub now_sec: u64,
    pub intercept: Option<Intercept>,
    pub mutation: Option<Mutation>,
    pub applied: usize,
    /// Every response item served, by `{method} {params}` (for replays).
    pub seen: HashMap<String, Vec<Value>>,
    /// Other paths served as they are (status, body): checkpoint.json, beacon APIs.
    pub files: HashMap<String, (u16, String)>,
    /// A test's own HTTP response to a path (status, headers, body), before everything else.
    pub raw: Option<Raw>,
}

pub type Raw = Arc<dyn Fn(&str) -> Option<(u16, Vec<(String, String)>, String)> + Send + Sync>;

pub struct World(pub Mutex<State>);

pub fn key(chain_id: u64, method: &str, params: &Value) -> String {
    let mut params = params.clone();
    // A call is matched on what decides it in the fixtures (libraries add fee, nonce and gas fields).
    if method == "willow_getCallProof" || method == "willow_getEstimateGasProof" {
        if let Some(c) = params.get(0).filter(|c| c.is_object()).cloned() {
            let value = c.get("value").filter(|v| *v != "0x0").cloned();
            let data = c.get("data").or_else(|| c.get("input")).cloned();
            let mut o = Map::new();
            for (k, v) in [
                ("from", c.get("from").cloned()),
                ("to", c.get("to").cloned()),
                ("value", value),
                ("data", data),
            ] {
                if let Some(v) = v {
                    o.insert(k.into(), v);
                }
            }
            params[0] = Value::Object(o);
        }
    }
    format!("{chain_id}|{method}|{}", canon(&params))
}

impl World {
    pub fn new() -> Arc<World> {
        Arc::new(World(Mutex::new(State {
            now_sec: 1_794_400_000,
            ..Default::default()
        })))
    }

    pub fn s(&self) -> std::sync::MutexGuard<'_, State> {
        self.0.lock().unwrap()
    }

    pub fn now_sec(&self) -> u64 {
        self.s().now_sec
    }

    pub fn proof(&self, chain_id: u64, method: &str, params: Value) -> Entry {
        self.s().proofs[&key(chain_id, method, &params)].clone()
    }

    fn add_proof(&self, params: &Value, e: Entry) {
        let k = key(e.chain_id, &e.method, params);
        self.s().proofs.insert(k, e);
    }

    pub fn head(
        &self,
        chain_id: u64,
        number: u64,
        hash: &str,
        level: &'static str,
        path: &'static str,
    ) {
        let e = AnchorEntry {
            chain_id,
            number,
            hash: hash.to_ascii_lowercase(),
            level,
            path,
            time: self.now_sec() - 2,
        };
        self.s().heads.insert(chain_id, e.clone());
        self.anchor(e);
    }

    /// The head at a block, signed: Ethereum's by path head, an L2's by its sequencer.
    pub fn head_at(&self, chain_id: u64, b: &(u64, String)) {
        self.head(
            chain_id,
            b.0,
            &b.1,
            "signed",
            if chain_id == 1 { "head" } else { "sequencer" },
        );
    }

    pub fn anchor(&self, e: AnchorEntry) {
        let mut s = self.s();
        s.hashes.insert(
            format!("{}:{}", e.chain_id, e.number),
            e.hash.to_ascii_lowercase(),
        );
        s.anchors.insert(format!("{}:{}", e.chain_id, e.number), e);
    }

    pub fn standard(&self, key: &str, v: Value) {
        self.s().standard.insert(key.to_string(), v);
    }

    pub fn intercept(&self, f: Option<Intercept>) {
        self.s().intercept = f;
    }

    pub fn methods_sent(&self) -> Vec<String> {
        self.s()
            .log
            .iter()
            .flat_map(|(_, items)| items.iter().map(|(m, _)| m.clone()))
            .collect()
    }

    pub fn sent(&self, method: &str) -> Vec<Value> {
        self.s()
            .log
            .iter()
            .flat_map(|(_, items)| {
                items
                    .iter()
                    .filter(|(m, _)| m == method)
                    .map(|(_, p)| p.clone())
            })
            .collect()
    }

    /// A block fixture: willow_getBlockProof by hash and by number, rendered as publicnode's block.
    pub fn add_block(&self, f: &Value, anchor: Option<AnchorEntry>) -> (u64, String) {
        let r = &f["result"];
        let h = header(r["header"].as_str().unwrap());
        let mut node = f["node"]["eth_getBlockByNumber"].clone();
        node.as_object_mut().unwrap().remove("totalDifficulty");
        let unproven: Vec<String> = r["unproven"]
            .as_array()
            .map(|a| a.iter().map(|x| x.as_str().unwrap().to_string()).collect())
            .unwrap_or_default();
        for k in &unproven {
            node.as_object_mut().unwrap().remove(k);
        }
        let full = node.clone();
        let txs: Vec<Value> = node["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                if t.is_string() {
                    t.clone()
                } else {
                    t["hash"].clone()
                }
            })
            .collect();
        node["transactions"] = Value::Array(txs);
        let chain_id = n(&f["chainId"]);
        let e = Entry {
            chain_id,
            method: "willow_getBlockProof".into(),
            result: r.clone(),
            proven: node,
            full: Some(full),
        };
        let hash = r["blockHash"].as_str().unwrap().to_ascii_lowercase();
        self.add_proof(&json!([r["blockHash"]]), e.clone());
        self.add_proof(&json!([r["blockNumber"]]), e);
        self.s()
            .hashes
            .insert(format!("{chain_id}:{}", h.number), hash.clone());
        let now = self.now_sec();
        let a = anchor.unwrap_or(AnchorEntry {
            chain_id,
            number: h.number,
            hash: hash.clone(),
            level: "signed",
            path: if chain_id == 1 {
                "history"
            } else {
                "sequencer"
            },
            time: now - 5,
        });
        if a.level != "none" {
            self.anchor(AnchorEntry {
                number: h.number,
                hash: hash.clone(),
                ..a
            });
        }
        (h.number, hash)
    }

    pub fn add_fees(&self, f: &Value) {
        let r = &f["result"];
        let chain_id = n(&f["chainId"]);
        let e = Entry {
            chain_id,
            method: "willow_getFeeProof".into(),
            result: r.clone(),
            proven: f["verified"].clone(),
            full: None,
        };
        self.add_proof(&json!([r["blockHash"]]), e.clone());
        self.add_proof(&json!([r["blockNumber"]]), e);
    }

    pub fn add_fee_history(&self, f: &Value) {
        let e = Entry {
            chain_id: n(&f["chainId"]),
            method: "willow_getFeeHistoryProof".into(),
            result: f["result"].clone(),
            proven: f["verified"].clone(),
            full: None,
        };
        self.add_proof(&f["request"]["params"], e);
    }

    /// A state fixture (any subset of its keys is served).
    pub fn add_state(&self, f: &Value, chain_id: u64) {
        let r = &f["result"];
        let mut s = self.s();
        s.states.insert(
            format!(
                "{chain_id}|{}|{}",
                r["account"]["address"]
                    .as_str()
                    .unwrap()
                    .to_ascii_lowercase(),
                r["blockHash"].as_str().unwrap().to_ascii_lowercase()
            ),
            f.clone(),
        );
        s.hashes.insert(
            format!("{chain_id}:{}", n(&r["blockNumber"])),
            r["blockHash"].as_str().unwrap().to_ascii_lowercase(),
        );
    }

    /// A call or estimate fixture.
    pub fn add_call(&self, f: &Value, chain_id: u64) {
        let r = &f["result"];
        let params = f["request"]["params"].as_array().unwrap();
        let mut overrides: Vec<Value> = params.iter().skip(2).cloned().collect();
        while overrides.last().is_some_and(empty_object) {
            overrides.pop();
        }
        let mut p = vec![params[0].clone(), r["blockHash"].clone()];
        p.extend(overrides);
        let e = Entry {
            chain_id,
            method: f["request"]["method"].as_str().unwrap().to_string(),
            result: r.clone(),
            proven: f["expect"].clone(),
            full: None,
        };
        self.add_proof(&Value::Array(p), e);
        let h = header(r["header"].as_str().unwrap());
        self.s().hashes.insert(
            format!("{chain_id}:{}", h.number),
            r["blockHash"].as_str().unwrap().to_ascii_lowercase(),
        );
    }

    /// A shipped items fixture (web/verify/test/items): a transaction, receipt or block-receipts proof.
    pub fn add_item(&self, c: &Value, anchor: bool) {
        let h = header(c["proof"]["result"]["header"].as_str().unwrap());
        let chain_id = n(&c["chainId"]);
        let e = Entry {
            chain_id,
            method: c["proof"]["method"].as_str().unwrap().to_string(),
            result: c["proof"]["result"].clone(),
            proven: c["answers"][0]["result"].clone(),
            full: None,
        };
        self.add_proof(&c["proof"]["params"], e.clone());
        if c["kind"] == "blockReceipts" {
            self.add_proof(&json!([c["block"]["number"]]), e);
        }
        let hash = c["block"]["hash"].as_str().unwrap().to_ascii_lowercase();
        self.s()
            .hashes
            .insert(format!("{chain_id}:{}", h.number), hash.clone());
        if anchor {
            let now = self.now_sec();
            self.anchor(AnchorEntry {
                chain_id,
                number: h.number,
                hash,
                level: "final",
                path: if chain_id == 1 { "beacon" } else { "game" },
                time: now - 3600,
            });
        }
    }

    fn block_hash(&self, chain_id: u64, b: &Value) -> Option<String> {
        let c = canon(b);
        if let Some(s) = c.as_str().filter(|s| s.len() == 66) {
            return Some(s.to_string());
        }
        let num = crate::hex::to_number(&c)?;
        self.s().hashes.get(&format!("{chain_id}:{num}")).cloned()
    }

    /// The state proof result for `[address, keys, block, {code}]`, or None.
    fn state(&self, chain_id: u64, params: &[Value]) -> Option<Value> {
        let address = params.first()?.as_str()?;
        let keys = params.get(1)?.as_array()?;
        let hash = self.block_hash(chain_id, params.get(2).unwrap_or(&Value::Null))?;
        let code = params
            .get(3)
            .and_then(|o| o["code"].as_bool())
            .unwrap_or(false);
        let f = self
            .s()
            .states
            .get(&format!(
                "{chain_id}|{}|{hash}",
                address.to_ascii_lowercase()
            ))?
            .clone();
        let r = &f["result"];
        let by_key: HashMap<String, Value> = r["account"]["storageProof"]
            .as_array()?
            .iter()
            .map(|s| {
                (
                    crate::hex::slot_key(s["key"].as_str().unwrap()).unwrap(),
                    s.clone(),
                )
            })
            .collect();
        let mut proofs = vec![];
        for k in keys {
            let mut s = by_key.get(&crate::hex::slot_key(k.as_str()?)?)?.clone();
            s["key"] = k.clone();
            proofs.push(s);
        }
        if code && f["expect"]["eth_getCode"].is_null() {
            return None;
        }
        let mut result = r.clone();
        result["account"]["storageProof"] = Value::Array(proofs);
        let o = result.as_object_mut().unwrap();
        o.remove("code");
        if code && !r["code"].is_null() {
            o.insert("code".into(), r["code"].clone());
        }
        Some(result)
    }

    /// The mock anchor body: `{blockNumber, mock, sig}`.
    pub fn anchor_body(e: &AnchorEntry) -> Value {
        let mock = json!({"chainId": e.chain_id, "number": e.number, "hash": e.hash.to_ascii_lowercase(), "level": e.level, "path": e.path, "time": e.time});
        json!({"blockNumber": q(e.number), "sig": anchor_sig(&mock), "mock": mock})
    }
}

/// Every fixture in one world: blocks and fees, state and calls (Ethereum, Base), the shipped items.
pub fn full_world() -> Arc<World> {
    let w = World::new();
    for f in fixture_names("block") {
        if f != "ethereum-1500000-uncles-unproven.json" {
            w.add_block(&fixture(&format!("block/{f}")), None);
        }
    }
    for f in fixture_names("fees") {
        let x = fixture(&format!("fees/{f}"));
        if f.ends_with(".fee.json") {
            w.add_fees(&x);
        } else {
            w.add_fee_history(&x);
        }
    }
    for f in fixture_names("state") {
        w.add_state(
            &fixture(&format!("state/{f}")),
            if f == "base-usdc.json" { 8453 } else { 1 },
        );
    }
    for f in fixture_names("call") {
        w.add_call(&fixture(&format!("call/{f}")), 1);
    }
    let mut items: Vec<PathBuf> = std::fs::read_dir(items_dir())
        .unwrap()
        .filter_map(|e| Some(e.ok()?.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    items.sort();
    for p in items {
        w.add_item(&json_file(&p), true);
    }
    w
}

/// The block fixture whose block is a chain's head in tests (fee fixtures are at the same blocks).
pub fn head_fixture(chain_id: u64) -> &'static str {
    match chain_id {
        1 => "ethereum-26391511.json",
        8453 => "base-56049971.json",
        10 => "op-159400794.json",
        130 => "unichain-63652013.json",
        57073 => "ink-60901963.json",
        42161 => "arbitrum-one-523278185.json",
        4663 => "robinhood-111702114.json",
        c => panic!("no head fixture for {c}"),
    }
}

/// The block of a block fixture: number and hash.
pub fn block_of(file: &str) -> (u64, String) {
    let r = fixture(&format!("block/{file}"))["result"].clone();
    (
        n(&r["blockNumber"]),
        r["blockHash"].as_str().unwrap().to_ascii_lowercase(),
    )
}

/// Ethereum's block in the state fixtures (26391733).
pub fn w2_block() -> (u64, String) {
    (
        0x192b4b5,
        "0x8a27150df15c313310e346ad53e1e04825a7004e0e01096b5cf095e6c7d89552".into(),
    )
}

const STATE_METHODS: [&str; 7] = [
    "willow_getStateProof",
    "willow_getCallProof",
    "willow_getEstimateGasProof",
    "willow_getTraceProof",
    "eth_getBalance",
    "eth_call",
    "eth_estimateGas",
];

fn respond(world: &World, chain_id: u64, key_: &str, it: &Value) -> Value {
    let id = it["id"].clone();
    let ok = |result: Value| json!({"jsonrpc": "2.0", "id": id, "result": result});
    let err = |code: i64, message: String, data: Option<Value>| match data {
        Some(d) => {
            json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message, "data": d}})
        }
        None => json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}}),
    };
    let method = it["method"].as_str().unwrap_or("");
    let params: Vec<Value> = it["params"].as_array().cloned().unwrap_or_default();
    let hook = world.s().intercept.clone();
    if let Some(r) = hook.and_then(|h| h(chain_id, method, &params)) {
        return match r.get("error") {
            Some(e) => err(
                e["code"].as_i64().unwrap(),
                e["message"].as_str().unwrap_or("").into(),
                e.get("data").cloned(),
            ),
            None => ok(r["result"].clone()),
        };
    }
    if key_ == "public" && STATE_METHODS.contains(&method) {
        return err(
            -32601,
            format!("the method {method} is not served on the public path, use an API key"),
            None,
        );
    }
    let pv = Value::Array(params.clone());
    let std = {
        let s = world.s();
        s.standard
            .get(&key(chain_id, method, &pv))
            .or_else(|| s.standard.get(&format!("{chain_id}|{method}|*")))
            .cloned()
    };
    if let Some(r) = std {
        return match r.get("error") {
            Some(e) => err(
                e["code"].as_i64().unwrap(),
                e["message"].as_str().unwrap_or("").into(),
                e.get("data").cloned(),
            ),
            None => ok(r["result"].clone()),
        };
    }
    match method {
        "eth_chainId" => ok(json!(q(chain_id))),
        "willow_getBlockAnchor" => {
            let p = params.first().cloned().unwrap_or(Value::Null);
            let shown = match &p {
                Value::String(s) => s.clone(),
                v => v.to_string(),
            };
            let s = world.s();
            if let Some(real) = s.real_anchors.get(&format!("{chain_id}:{shown}")) {
                return ok(real.clone());
            }
            let e = if p == "latest" {
                s.heads.get(&chain_id)
            } else if p == "finalized" {
                s.finalized.get(&chain_id)
            } else {
                crate::hex::to_number(&p).and_then(|x| s.anchors.get(&format!("{chain_id}:{x}")))
            };
            match e {
                Some(e) => ok(World::anchor_body(e)),
                None => err(
                    -32000,
                    format!("not found in this store: no anchor for block {shown}"),
                    None,
                ),
            }
        }
        "willow_lightClientBootstrap" => match world.s().bootstrap.clone() {
            Some(b) => ok(b),
            None => err(-32601, "no light client here".into(), None),
        },
        "willow_lightClientUpdates" => match world.s().updates.clone() {
            Some(b) => ok(b),
            None => err(-32601, "no light client here".into(), None),
        },
        "willow_getStateProof" => match world.state(chain_id, &params) {
            Some(r) => ok(r),
            None => err(
                -32000,
                "not found in this store: no state fixture".into(),
                None,
            ),
        },
        "willow_getTransactionProof"
        | "willow_getReceiptProof"
        | "willow_getBlockReceiptsProof"
        | "willow_getBlockProof"
        | "willow_getFeeProof"
        | "willow_getFeeHistoryProof"
        | "willow_getCallProof"
        | "willow_getEstimateGasProof"
        | "willow_getTraceProof" => {
            if let Some(e) = world.s().proofs.get(&key(chain_id, method, &pv)) {
                return ok(e.result.clone());
            }
            if (method == "willow_getTransactionProof" || method == "willow_getReceiptProof")
                && params.len() == 1
            {
                return err(
                    -32000,
                    format!(
                        "not found in this store: transaction {} is not in the full-block tier",
                        params[0].as_str().unwrap_or("")
                    ),
                    None,
                );
            }
            err(
                -32000,
                format!(
                    "not found in this store: no fixture for {method} {}",
                    crate::hex::shown(&pv, 200)
                ),
                None,
            )
        }
        _ => err(
            -32601,
            format!("the method {method} does not exist/is not available"),
            None,
        ),
    }
}

/// The world's response to one HTTP body, with the lying server's mutation applied.
fn respond_http(world: &World, chain_id: u64, key_: &str, body: &str) -> String {
    let Ok(v) = serde_json::from_str::<Value>(body) else {
        return json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": "parse error"}}).to_string();
    };
    let batch = v.is_array();
    let items: Vec<Value> = if batch {
        v.as_array().unwrap().clone()
    } else {
        vec![v]
    };
    let asked: Vec<(Value, String, Vec<Value>)> = items
        .iter()
        .map(|i| {
            (
                i["id"].clone(),
                i["method"].as_str().unwrap_or("").to_string(),
                i["params"].as_array().cloned().unwrap_or_default(),
            )
        })
        .collect();
    world.s().log.push((
        chain_id,
        asked
            .iter()
            .map(|(_, m, p)| (m.clone(), Value::Array(p.clone())))
            .collect(),
    ));
    let mut out: Vec<Value> = items
        .iter()
        .map(|i| respond(world, chain_id, key_, i))
        .collect();
    {
        let mut s = world.s();
        for it in &out {
            if let Some((_, m, p)) = asked.iter().find(|(id, _, _)| *id == it["id"]) {
                s.seen
                    .entry(format!("{m} {}", Value::Array(p.clone())))
                    .or_default()
                    .push(it.clone());
            }
        }
    }
    let mutation = world.s().mutation.clone();
    let mut applied = 0;
    match mutation {
        Some(Mutation::Batch(f)) if f(&mut out, &asked) => applied += 1,
        Some(Mutation::Batch(_)) => {}
        Some(Mutation::Item(f)) => {
            for it in out.iter_mut() {
                if let Some((_, m, p)) = asked.iter().find(|(id, _, _)| *id == it["id"]) {
                    if f(it, m, p) {
                        applied += 1;
                    }
                }
            }
        }
        None => {}
    }
    world.s().applied += applied;
    if batch {
        Value::Array(out).to_string()
    } else {
        out.into_iter().next().unwrap().to_string()
    }
}

pub struct Server {
    pub base: String,
    task: tokio::task::JoinHandle<()>,
}

impl Server {
    pub fn url(&self, chain_id: u64, key: &str) -> String {
        format!("{}/v1/rpc/{chain_id}/{key}", self.base)
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A JSON-RPC server over a world, at /v1/rpc/{chainId}/{key}, responding to batches as Willow does.
pub async fn serve(world: Arc<World>) -> Server {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        loop {
            let Ok((mut sock, _)) = listener.accept().await else {
                return;
            };
            let world = world.clone();
            tokio::spawn(async move {
                let mut buf = Vec::new();
                let mut tmp = [0u8; 65536];
                let (head_end, len) = loop {
                    let Ok(k) = sock.read(&mut tmp).await else {
                        return;
                    };
                    if k == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..k]);
                    if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
                        let head = String::from_utf8_lossy(&buf[..i]).to_ascii_lowercase();
                        let len = head
                            .lines()
                            .find_map(|l| {
                                l.strip_prefix("content-length:")
                                    .map(|x| x.trim().parse::<usize>().unwrap_or(0))
                            })
                            .unwrap_or(0);
                        break (i + 4, len);
                    }
                };
                while buf.len() < head_end + len {
                    let Ok(k) = sock.read(&mut tmp).await else {
                        return;
                    };
                    if k == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..k]);
                }
                let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
                let path = head.split_whitespace().nth(1).unwrap_or("").to_string();
                let body = String::from_utf8_lossy(&buf[head_end..(head_end + len).min(buf.len())])
                    .to_string();
                let parts: Vec<&str> = path.trim_matches('/').split('/').collect();
                let file = world.s().files.get(&path).cloned();
                let raw = world.s().raw.clone().and_then(|r| r(&path));
                let mut headers = String::new();
                let (status, text) = match parts.as_slice() {
                    _ if raw.is_some() => {
                        let (code, hs, body) = raw.unwrap();
                        for (k, v) in hs {
                            headers.push_str(&format!("{k}: {v}\r\n"));
                        }
                        (
                            match code {
                                200 => "200 OK",
                                429 => "429 Too Many Requests",
                                500 => "500 Internal Server Error",
                                _ => "404 Not Found",
                            },
                            body,
                        )
                    }
                    _ if file.is_some() => {
                        let (code, body) = file.unwrap();
                        (
                            if code == 200 {
                                "200 OK"
                            } else {
                                "404 Not Found"
                            },
                            body,
                        )
                    }
                    ["v1", "rpc", c, k] => match c.parse::<u64>() {
                        Ok(c) => ("200 OK", respond_http(&world, c, k, &body)),
                        Err(_) => ("404 Not Found", String::new()),
                    },
                    _ => ("404 Not Found", String::new()),
                };
                let resp = format!(
                    "HTTP/1.1 {status}\r\ncontent-type: application/json\r\n{headers}content-length: {}\r\nconnection: close\r\n\r\n{text}",
                    text.len()
                );
                let _ = sock.write_all(resp.as_bytes()).await;
                let _ = sock.shutdown().await;
            });
        }
    });
    Server { base, task }
}

/// The mock anchor: verifies the stand-in signature, the block, the chain and the age, as
/// `Anchor::verify` does for real.
pub struct MockAnchor;

fn out(v: Value) -> String {
    v.to_string()
}

fn fail(kind: &str, message: &str) -> String {
    json!({"ok": false, "kind": kind, "message": message}).to_string()
}

impl AnchorApi for MockAnchor {
    fn restore(&mut self, _: &str, _: f64) -> String {
        out(json!({"ok": true, "period": 0}))
    }
    fn bootstrap(&mut self, _: &str, _: f64) -> String {
        out(json!({"ok": true, "period": 0}))
    }
    fn updates(&mut self, _: &str) -> String {
        out(json!({"ok": true, "period": 1}))
    }
    fn verify(
        &mut self,
        chain_id: u32,
        number: f64,
        hash: &str,
        response: &str,
        now: f64,
    ) -> String {
        let Ok(r) = serde_json::from_str::<Value>(response) else {
            return fail("malformed-input", "not JSON");
        };
        let r = &r["result"];
        let m = &r["mock"];
        if !m.is_object() {
            return fail("malformed-input", "not a mock anchor");
        }
        if anchor_sig(m) != r["sig"].as_str().unwrap_or("") {
            return fail("rejected", "the validators' signature does not verify");
        }
        let mn = m["number"].as_u64().unwrap_or(u64::MAX);
        if crate::hex::to_number(&r["blockNumber"]) != Some(mn)
            || m["chainId"].as_u64() != Some(chain_id as u64)
        {
            return fail("rejected", "the anchor is for another chain or block");
        }
        if mn as f64 != number {
            return fail(
                "rejected",
                &format!("the anchor is for block {mn}, not {number}"),
            );
        }
        let time = m["time"].as_u64().unwrap_or(0);
        let limit = if chain_id == 1 { 600.0 } else { 3.0 * 3600.0 };
        if m["level"] == "signed" && now - time as f64 > limit {
            return fail("stale-head", "the signed head is old");
        }
        let mut o = json!({"ok": true, "level": m["level"], "path": m["path"], "hash": m["hash"],
            "matches": m["hash"].as_str() == Some(hash.to_ascii_lowercase().as_str())});
        if chain_id == 1 {
            o["headTime"] = json!(time);
            o["head"] = json!(if m["path"] == "head" { mn } else { mn + 32 });
            o["finalized"] = json!(if m["level"] == "final" {
                mn
            } else {
                mn.saturating_sub(64)
            });
            o["signers"] = json!(500);
        } else {
            o["anchorBlock"] = json!(mn);
            o["l1Block"] = json!(1);
            if m["path"] == "sequencer" || m["path"] == "parlia" {
                o["anchorTime"] = json!(time);
            }
        }
        out(o)
    }
    fn saved(&self) -> String {
        "[]".into()
    }
    fn reset(&mut self) {}
    fn start(&mut self, _: &str, _: &str) -> String {
        out(json!({"ok": true}))
    }
    fn bootstrap_for(&mut self, _: &str, _: &str, _: f64) -> String {
        out(json!({"ok": true, "period": 0}))
    }
    fn updates_for(&mut self, _: &str, _: &str) -> String {
        out(json!({"ok": true, "period": 1}))
    }
    fn restore_for(&mut self, _: &str, _: &str, _: f64) -> String {
        out(json!({"ok": true}))
    }
    fn saved_for(&self, _: &str) -> String {
        "[]".into()
    }
    fn reset_for(&mut self, _: &str) {}
}

pub fn mock_anchor() -> AnchorFactory {
    Arc::new(|_, _| Box::new(MockAnchor))
}

pub struct Harness {
    pub world: Arc<World>,
    pub _server: Server,
    pub client: Verifier,
    pub events: Arc<Mutex<Vec<Event>>>,
}

impl Harness {
    pub fn last(&self) -> Event {
        self.events
            .lock()
            .unwrap()
            .last()
            .cloned()
            .expect("an event")
    }

    pub fn events(&self) -> Vec<Event> {
        self.events.lock().unwrap().clone()
    }

    pub async fn ask(&self, method: &str, params: Value) -> Result<Value, crate::Error> {
        self.client.request(method, params).await
    }
}

/// The client on a mock server over `world`, as the npm suite's harness makes it.
pub async fn harness_with(
    chain_id: u64,
    world: Arc<World>,
    key_: &str,
    f: impl FnOnce(Config) -> Config,
) -> Harness {
    let server = serve(world.clone()).await;
    let events = Arc::new(Mutex::new(vec![]));
    let ev = events.clone();
    let w = world.clone();
    let mut cfg = Config::new(chain_id)
        .url(server.url(chain_id, key_))
        .storage(StorageChoice::None)
        .checkpoint(Checkpoint {
            root: format!("0x{}", "11".repeat(32)),
            slot: 15_613_952,
        })
        .clock(move || w.now_sec() * 1000)
        .batch_window(Duration::from_millis(2))
        .waits(
            Some(Duration::from_millis(5)),
            Some(Duration::from_millis(5)),
        )
        .on_verification(move |e| ev.lock().unwrap().push(e.clone()));
    cfg.anchor_factory = Some(mock_anchor());
    cfg.unheld = true;
    let cfg = f(cfg);
    let client = Verifier::new(cfg).unwrap();
    Harness {
        world,
        _server: server,
        client,
        events,
    }
}

pub async fn harness(chain_id: u64, f: impl FnOnce(Config) -> Config) -> Harness {
    harness_with(chain_id, full_world(), "test-key", f).await
}

/// The npm suite's `flip`: one nibble near the end of a hex string changed.
pub fn flip(h: &str, from_end: usize) -> String {
    let i = h.len() - from_end;
    let c = &h[i..i + 1];
    format!(
        "{}{}{}",
        &h[..i],
        if c == "0" { "1" } else { "0" },
        &h[i + 1..]
    )
}

/// An error's code and kind, for comparing with the npm suite's expectations.
pub fn code_kind(e: &crate::Error) -> (i64, Option<String>) {
    (e.code(), e.kind().map(String::from))
}
