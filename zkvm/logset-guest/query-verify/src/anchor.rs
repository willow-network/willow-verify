//! Block anchoring without trusting a node: `anchor_verify::Anchor`, the verify page's own
//! light clients, run natively. A block hash is block N of its chain when Willow's
//! `willow_getBlockAnchor [N]` response verifies against the validators of the chain's L1, from a
//! pinned start:
//! - Ethereum and the chains that settle on it: Ethereum's sync committee, from a checkpoint;
//! - the Sepolia testnets: Sepolia's sync committee, from a Sepolia checkpoint;
//! - BNB Smart Chain: its own validator set, from a pinned epoch block.
//!
//! The starts come from `--checkpoint` / `--sepolia-checkpoint` / `--bnb-start`, else the
//! `checkpoint.json` built into the binary, else (once that one is too old) Willow's published
//! `checkpoint.json`, an Ethereum one confirmed by independent beacon APIs. What the light clients
//! verify is kept in the cache directory (`~/.cache/willow-verify/`) and verified again from the
//! start on the next run, so nothing kept is trusted. Light-client data is asked for as the
//! anchor names it: Ethereum's on chain 1's route, Sepolia's and BNB's on the chain's own.

use crate::rpc::{error_of, Transport};
use serde_json::{json, Value};
use std::path::PathBuf;

/// The starts built into the binary (Ethereum's at the top level, as the page's checkpoint.json).
pub const SHIPPED: &str = include_str!("../checkpoint.json");
/// Where Willow publishes newer starts, beside the verify page.
pub const CHECKPOINT_URL: &str = "https://api.willow.tech/verify/checkpoint.json";
/// Beacon APIs that confirm a fetched Ethereum checkpoint (the page build's two).
pub const BEACON_APIS: [&str; 2] = [
    "https://ethereum-beacon-api.publicnode.com",
    "https://lodestar-mainnet.chainsafe.io",
];
const ETHEREUM_GENESIS_TIME: u64 = 1_606_824_023;
const SEPOLIA_GENESIS_TIME: u64 = 1_655_733_600;
const DAY: u64 = 86_400;
/// A shipped BNB start older than this gives way to a newer published one (the client's rule).
const BNB_REFRESH_SECS: u64 = DAY;
/// Light-client updates asked for at once (anchor-verify's limits).
const UPDATES_PER_ASK: u64 = 16;
const BNB_UPDATES_PER_ASK: u64 = 64;
/// Most periods or epochs walked in one catch-up: past it, the start is the problem.
const MAX_CATCH_UP: u64 = 4096;
/// Catch-up rounds per anchor, as the page and the client.
const CATCH_UP_ROUNDS: usize = 3;
/// A block newer than the signed head is asked for again, this many times.
const WAITING_RETRIES: usize = 3;
const ZERO_HASH: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

/// An anchor's level, lowest first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Level {
    Signed,
    Committed,
    Final,
}

impl Level {
    pub fn parse(s: &str) -> Option<Level> {
        match s {
            "signed" => Some(Level::Signed),
            "committed" => Some(Level::Committed),
            "final" => Some(Level::Final),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Level::Signed => "signed",
            Level::Committed => "committed",
            Level::Final => "final",
        }
    }
}

/// The light client that anchors a chain: `None` is Ethereum's.
pub fn light_client_of(chain_id: u64) -> Option<&'static str> {
    if chain_id == anchor_verify::bnb::CHAIN_ID {
        Some("bnb")
    } else if anchor_verify::l2::on_sepolia(chain_id) {
        Some("sepolia")
    } else {
        None
    }
}

/// Why a block was not anchored: `kind` is anchor-verify's (`rejected`, `unreachable`, ...) or
/// this module's (`refused`, `mismatch`, `below-min-level`, `stale-head`, `not-the-head`).
#[derive(Clone, Debug, PartialEq)]
pub struct Fail {
    pub kind: String,
    pub message: String,
}

impl Fail {
    pub fn new(kind: &str, message: impl Into<String>) -> Fail {
        Fail {
            kind: kind.into(),
            message: message.into(),
        }
    }

    fn of(r: &Value) -> Fail {
        Fail::new(
            r["kind"].as_str().unwrap_or("internal"),
            r["message"].as_str().unwrap_or("the anchor failed"),
        )
    }

    /// Whether the data was shown wrong (a forged or another block's anchor), not just short.
    pub fn is_verdict(&self) -> bool {
        matches!(
            self.kind.as_str(),
            "rejected" | "malformed-input" | "mismatch" | "not-the-head"
        )
    }
}

/// How anchoring runs.
#[derive(Clone, Debug)]
pub struct Options {
    pub checkpoint: Option<(String, u64)>,
    pub sepolia: Option<(String, u64)>,
    pub bnb: Option<(u64, String)>,
    pub checkpoint_url: String,
    pub beacon_apis: Vec<String>,
    pub cache_dir: Option<PathBuf>,
    pub min_level: Level,
    /// The newest block's age limit, seconds.
    pub max_head_age: u64,
    /// The time verification runs at (`--now`), else the clock.
    pub now: Option<u64>,
    /// Between asks for a block newer than the signed head, ms.
    pub waiting_ms: u64,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            checkpoint: None,
            sepolia: None,
            bnb: None,
            checkpoint_url: CHECKPOINT_URL.into(),
            beacon_apis: BEACON_APIS.iter().map(|s| s.to_string()).collect(),
            cache_dir: default_cache_dir(),
            min_level: Level::Signed,
            max_head_age: 600,
            now: None,
            waiting_ms: 4000,
        }
    }
}

/// `$XDG_CACHE_HOME/willow-verify`, else `~/.cache/willow-verify`.
pub fn default_cache_dir() -> Option<PathBuf> {
    if let Some(x) = std::env::var_os("XDG_CACHE_HOME").filter(|x| !x.is_empty()) {
        return Some(PathBuf::from(x).join("willow-verify"));
    }
    std::env::var_os("HOME")
        .filter(|h| !h.is_empty())
        .map(|h| PathBuf::from(h).join(".cache").join("willow-verify"))
}

/// `<root>:<slot>`: a finalized beacon block root and its slot.
pub fn parse_checkpoint(s: &str) -> Result<(String, u64), String> {
    let (root, slot) = s
        .rsplit_once(':')
        .ok_or_else(|| format!("{s:?} is not <root>:<slot>"))?;
    let root = root.trim().to_ascii_lowercase();
    if !is_hash(&root) {
        return Err(format!("{root:?} is not a 32-byte block root"));
    }
    let slot = slot
        .trim()
        .parse()
        .map_err(|_| format!("{slot:?} is not a slot"))?;
    Ok((root, slot))
}

/// `<number>:<hash>`: a BNB Smart Chain epoch block (its number a multiple of 1,000).
pub fn parse_bnb_start(s: &str) -> Result<(u64, String), String> {
    let (n, hash) = s
        .split_once(':')
        .ok_or_else(|| format!("{s:?} is not <number>:<hash>"))?;
    let n: u64 = n
        .trim()
        .parse()
        .map_err(|_| format!("{n:?} is not a block number"))?;
    let hash = hash.trim().to_ascii_lowercase();
    if !n.is_multiple_of(1000) || !is_hash(&hash) {
        return Err(format!(
            "{s:?}: a BNB start is an epoch block (a multiple of 1,000) and its hash"
        ));
    }
    Ok((n, hash))
}

fn is_hash(s: &str) -> bool {
    s.len() == 66 && s.starts_with("0x") && s[2..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A light client's start, where it came from, and why when that is notable.
#[derive(Clone, Debug)]
struct Start {
    json: Value,
    from: &'static str,
    note: Option<String>,
}

/// GET `url` as JSON.
fn get_json(http: &reqwest::blocking::Client, url: &str) -> Result<Value, String> {
    http.get(url)
        .header("accept", "application/json")
        .send()
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.json())
        .map_err(|e| e.without_url().to_string())
}

/// Willow's published starts, fetched at most once.
struct Published<'h> {
    http: Option<&'h reqwest::blocking::Client>,
    url: String,
    got: Option<Result<Value, String>>,
}

impl Published<'_> {
    fn get(&mut self) -> Result<Value, String> {
        if self.got.is_none() {
            self.got = Some(match self.http {
                Some(h) => get_json(h, &self.url),
                None => Err("no network here".into()),
            });
        }
        self.got.clone().expect("set")
    }
}

/// The start of light client `lc` at time `now`, by the rules in the module docs.
fn choose_start(
    lc: Option<&str>,
    o: &Options,
    now: u64,
    http: Option<&reqwest::blocking::Client>,
) -> Start {
    let shipped: Value = serde_json::from_str(SHIPPED).expect("checkpoint.json is JSON");
    let mut published = Published {
        http,
        url: o.checkpoint_url.clone(),
        got: None,
    };
    let slot_of = |v: &Value| v["slot"].as_u64().unwrap_or(0);
    match lc {
        None => {
            if let Some((root, slot)) = &o.checkpoint {
                return Start {
                    json: json!({"root": root, "slot": slot}),
                    from: "--checkpoint",
                    note: None,
                };
            }
            let s = json!({"root": shipped["root"], "slot": shipped["slot"]});
            let age = now.saturating_sub(ETHEREUM_GENESIS_TIME + 12 * slot_of(&s));
            if age + DAY < anchor_verify::lc::MAX_START_AGE_SECS {
                return Start {
                    json: s,
                    from: "shipped",
                    note: None,
                };
            }
            match published
                .get()
                .and_then(|c| confirmed_checkpoint(&c, slot_of(&s), o, http))
            {
                Ok(c) => Start {
                    json: c,
                    from: "checkpoint.json",
                    note: None,
                },
                Err(why) => Start {
                    json: s,
                    from: "shipped",
                    note: Some(format!(
                        "the shipped checkpoint is {} days old and no newer one was taken: {why}",
                        age / DAY
                    )),
                },
            }
        }
        Some("sepolia") => {
            if let Some((root, slot)) = &o.sepolia {
                return Start {
                    json: json!({"root": root, "slot": slot}),
                    from: "--sepolia-checkpoint",
                    note: None,
                };
            }
            let s = shipped["sepolia"].clone();
            let age = now.saturating_sub(SEPOLIA_GENESIS_TIME + 12 * slot_of(&s));
            if age + DAY < anchor_verify::lc::MAX_START_AGE_SECS {
                return Start {
                    json: s,
                    from: "shipped",
                    note: None,
                };
            }
            match published.get() {
                Ok(c)
                    if c["sepolia"]["root"].as_str().is_some_and(is_hash)
                        && slot_of(&c["sepolia"]) > slot_of(&s) =>
                {
                    Start {
                        json: json!({"root": c["sepolia"]["root"], "slot": c["sepolia"]["slot"]}),
                        from: "checkpoint.json",
                        note: None,
                    }
                }
                other => Start {
                    json: s,
                    from: "shipped",
                    note: Some(format!(
                        "the shipped Sepolia checkpoint is {} days old and no newer one was taken{}",
                        age / DAY,
                        other.err().map(|e| format!(": {e}")).unwrap_or_default()
                    )),
                },
            }
        }
        Some(_) => {
            if let Some((number, hash)) = &o.bnb {
                return Start {
                    json: json!({"number": number, "hash": hash}),
                    from: "--bnb-start",
                    note: None,
                };
            }
            let s = shipped["bnb"].clone();
            let age = now.saturating_sub(s["time"].as_u64().unwrap_or(0));
            let number = |v: &Value| v["number"].as_u64().unwrap_or(0);
            if age < BNB_REFRESH_SECS {
                return Start {
                    json: json!({"number": s["number"], "hash": s["hash"]}),
                    from: "shipped",
                    note: None,
                };
            }
            match published.get() {
                Ok(c)
                    if c["bnb"]["hash"].as_str().is_some_and(is_hash)
                        && number(&c["bnb"]).is_multiple_of(1000)
                        && number(&c["bnb"]) > number(&s) =>
                {
                    Start {
                        json: json!({"number": c["bnb"]["number"], "hash": c["bnb"]["hash"]}),
                        from: "checkpoint.json",
                        note: None,
                    }
                }
                _ => Start {
                    json: json!({"number": s["number"], "hash": s["hash"]}),
                    from: "shipped",
                    note: (age >= anchor_verify::bnb::MAX_START_AGE_SECS).then(|| {
                        format!(
                            "the shipped BNB start is {} days old and no newer one was published",
                            age / DAY
                        )
                    }),
                },
            }
        }
    }
}

/// A fetched Ethereum checkpoint, newer than `shipped_slot`, that the beacon APIs confirm: at
/// least one serves the same root at its slot, and none another.
fn confirmed_checkpoint(
    c: &Value,
    shipped_slot: u64,
    o: &Options,
    http: Option<&reqwest::blocking::Client>,
) -> Result<Value, String> {
    let root = c["root"]
        .as_str()
        .filter(|r| is_hash(r))
        .ok_or("checkpoint.json holds no checkpoint")?
        .to_ascii_lowercase();
    let slot = c["slot"].as_u64().ok_or("checkpoint.json holds no slot")?;
    if slot <= shipped_slot {
        return Err("checkpoint.json holds no newer checkpoint".into());
    }
    let http = http.ok_or("no network here")?;
    let mut confirmed = 0;
    for api in &o.beacon_apis {
        let Ok(v) = get_json(
            http,
            &format!("{}/eth/v1/beacon/headers/{slot}", api.trim_end_matches('/')),
        ) else {
            continue;
        };
        let theirs = (v["data"]["canonical"] != json!(false))
            .then(|| v["data"]["root"].as_str())
            .flatten()
            .map(str::to_ascii_lowercase);
        if theirs.as_deref() != Some(root.as_str()) {
            return Err(format!(
                "a beacon API does not confirm checkpoint {root} at slot {slot}"
            ));
        }
        confirmed += 1;
    }
    if confirmed == 0 {
        return Err("no beacon API could confirm the published checkpoint".into());
    }
    Ok(json!({"root": root, "slot": slot}))
}

/// The newest block, anchored.
#[derive(Clone, Debug)]
pub struct Head {
    pub number: u64,
    pub hash: String,
    /// The anchor's report.
    pub report: Value,
}

/// One chain's anchoring: its light client, kept between runs, and the head once asked for.
pub struct Anchoring<'t> {
    t: &'t dyn Transport,
    chain_id: u64,
    lc: Option<&'static str>,
    a: anchor_verify::Anchor,
    start: Start,
    opts: Options,
    now: u64,
    restored: bool,
    kept: Option<String>,
    head: Option<Head>,
    /// Light-client requests made, for the report.
    pub asked: usize,
}

impl<'t> Anchoring<'t> {
    /// `http` fetches Willow's published starts when the built-in one is too old (`None`: never).
    pub fn new(
        t: &'t dyn Transport,
        chain_id: u64,
        opts: Options,
        http: Option<&reqwest::blocking::Client>,
    ) -> Result<Anchoring<'t>, String> {
        u32::try_from(chain_id).map_err(|_| format!("chain {chain_id} is not anchored"))?;
        let now = opts.now.unwrap_or_else(now_secs);
        let lc = light_client_of(chain_id);
        let start = choose_start(lc, &opts, now, http);
        let (eth_root, eth_slot) = match lc {
            None => (
                start.json["root"].as_str().unwrap_or_default().to_string(),
                start.json["slot"].as_u64().unwrap_or(0),
            ),
            // Unused for these chains: any well-formed checkpoint.
            Some(_) => (ZERO_HASH.to_string(), 0),
        };
        let mut a = anchor_verify::Anchor::new(&eth_root, eth_slot as f64);
        if let Some(name) = lc {
            let r: Value = serde_json::from_str(&a.start(name, &start.json.to_string()))
                .map_err(|e| e.to_string())?;
            if r["ok"] != true {
                return Err(format!(
                    "the {name} start: {}",
                    r["message"].as_str().unwrap_or("refused")
                ));
            }
        }
        Ok(Anchoring {
            t,
            chain_id,
            lc,
            a,
            start,
            opts,
            now,
            restored: false,
            kept: None,
            head: None,
            asked: 0,
        })
    }

    /// Which light client, and where it started.
    pub fn start_json(&self) -> Value {
        let mut s = self.start.json.clone();
        s["lightClient"] = json!(self.lc.unwrap_or("ethereum"));
        s["from"] = json!(self.start.from);
        if let Some(n) = &self.start.note {
            s["note"] = json!(n);
        }
        s
    }

    fn cache_file(&self) -> Option<PathBuf> {
        let name = match self.lc {
            None => "light-client.json".to_string(),
            Some(n) => format!("light-client-{n}.json"),
        };
        self.opts.cache_dir.as_ref().map(|d| d.join(name))
    }

    /// What earlier runs kept, verified again from the start (a failure only means starting over).
    fn restore(&mut self) {
        if self.restored {
            return;
        }
        self.restored = true;
        let Some(saved) = self
            .cache_file()
            .and_then(|p| std::fs::read_to_string(p).ok())
        else {
            return;
        };
        let r = match self.lc {
            None => self.a.restore(&saved, self.now as f64),
            Some(n) => self.a.restore_for(n, &saved, self.now as f64),
        };
        if serde_json::from_str::<Value>(&r)
            .is_ok_and(|v| v["ok"] == true && !v["period"].is_null())
        {
            self.kept = Some(saved);
        }
    }

    /// Keeps what the light client verified, when it changed.
    fn save(&mut self) {
        let saved = match self.lc {
            None => self.a.saved(),
            Some(n) => self.a.saved_for(n),
        };
        if saved == "null" || self.kept.as_deref() == Some(saved.as_str()) {
            return;
        }
        if let Some(path) = self.cache_file() {
            let tmp = path.with_extension(format!("tmp{}", std::process::id()));
            let wrote = path
                .parent()
                .map_or(Ok(()), std::fs::create_dir_all)
                .and_then(|_| std::fs::write(&tmp, &saved))
                .and_then(|_| std::fs::rename(&tmp, &path));
            if wrote.is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
        }
        self.kept = Some(saved);
    }

    /// Light-client data on its route: Ethereum's on chain 1's, the others' on the chain's own.
    fn lc_body(&mut self, lc: Option<&str>, method: &str, params: Value) -> Result<String, Fail> {
        let route = if lc.is_some() { self.chain_id } else { 1 };
        self.asked += 1;
        let body = self
            .t
            .body(route, method, &params)
            .map_err(|e| Fail::new("refused", e))?;
        if let Some(e) = error_of(&body) {
            return Err(Fail::new("refused", format!("{method}: {e}")));
        }
        Ok(body)
    }

    fn bootstrap(&mut self, lc: Option<&str>) -> Result<(), Fail> {
        if lc != self.lc {
            return Err(Fail::new(
                "internal",
                format!("chain {} asked for light client {lc:?}", self.chain_id),
            ));
        }
        let root = match lc {
            Some("bnb") => self.start.json["hash"].clone(),
            _ => self.start.json["root"].clone(),
        };
        let body = self.lc_body(lc, "willow_lightClientBootstrap", json!([root]))?;
        let r = match lc {
            None => self.a.bootstrap(&body, self.now as f64),
            Some(n) => self.a.bootstrap_for(n, &body, self.now as f64),
        };
        ok_json(&r).map(|_| ())
    }

    fn updates(&mut self, lc: Option<&str>, from: u64, count: u64) -> Result<(), Fail> {
        if lc != self.lc {
            return Err(Fail::new(
                "internal",
                format!("chain {} asked for light client {lc:?}", self.chain_id),
            ));
        }
        if count == 0 || count > MAX_CATCH_UP {
            return Err(Fail::new(
                "unreachable",
                format!("the light client would walk {count} periods from {from}: start from a newer checkpoint"),
            ));
        }
        let per = if lc == Some("bnb") {
            BNB_UPDATES_PER_ASK
        } else {
            UPDATES_PER_ASK
        };
        let (mut from, mut left) = (from, count);
        while left > 0 {
            let k = left.min(per);
            let body = self.lc_body(lc, "willow_lightClientUpdates", json!([from, k]))?;
            let r = match lc {
                None => self.a.updates(&body),
                Some(n) => self.a.updates_for(n, &body),
            };
            let r = ok_json(&r)?;
            if r["period"].is_null() {
                return Err(Fail::new(
                    "unreachable",
                    format!("Willow sent no light-client update from {from}"),
                ));
            }
            from += k;
            left -= k;
        }
        Ok(())
    }

    /// `Anchor.verify` of an anchor response body, fetching the light-client data it asks for.
    fn checked(&mut self, number: u64, hash: &str, body: &str) -> Result<Value, Fail> {
        self.restore();
        let mut caught_up = false;
        for round in 0..=CATCH_UP_ROUNDS {
            let out = self.a.verify(
                self.chain_id as u32,
                number as f64,
                hash,
                body,
                self.now as f64,
            );
            let r: Value =
                serde_json::from_str(&out).map_err(|e| Fail::new("internal", e.to_string()))?;
            if r["ok"] == true {
                if caught_up || self.kept.is_none() {
                    self.save();
                }
                return Ok(r);
            }
            let kind = r["kind"].as_str().unwrap_or("internal");
            if kind != "need-bootstrap" && kind != "need-updates" {
                return Err(Fail::of(&r));
            }
            if round == CATCH_UP_ROUNDS {
                return Err(Fail::new(
                    "unreachable",
                    "Willow's light-client data did not reach the validators who signed this block",
                ));
            }
            let lc = r["lightClient"].as_str();
            if kind == "need-bootstrap" {
                self.bootstrap(lc)?;
            } else {
                let (Some(from), Some(count)) = (r["from"].as_u64(), r["count"].as_u64()) else {
                    return Err(Fail::new(
                        "internal",
                        format!("need-updates without periods: {r}"),
                    ));
                };
                self.updates(lc, from, count)?;
            }
            caught_up = true;
        }
        unreachable!("the loop returns")
    }

    /// `willow_getBlockAnchor [param]`'s body; a block newer than the signed head is asked for again.
    fn anchor_body(&mut self, param: Value) -> Result<String, Fail> {
        for attempt in 0.. {
            let body = self
                .t
                .body(self.chain_id, "willow_getBlockAnchor", &json!([param]))
                .map_err(|e| Fail::new("refused", e))?;
            let Some(e) = error_of(&body) else {
                return Ok(body);
            };
            let waiting = e["data"]["status"] == "waiting";
            if waiting && attempt < WAITING_RETRIES {
                std::thread::sleep(std::time::Duration::from_millis(self.opts.waiting_ms));
                continue;
            }
            return Err(Fail::new(
                "refused",
                format!("willow_getBlockAnchor [{param}]: {e}"),
            ));
        }
        unreachable!("the loop returns")
    }

    fn l1_name(&self) -> &'static str {
        match self.lc {
            None => "Ethereum",
            Some("sepolia") => "Sepolia",
            Some(_) => "BNB Smart Chain's validators",
        }
    }

    fn level_of(r: &Value) -> Result<Level, Fail> {
        r["level"]
            .as_str()
            .and_then(Level::parse)
            .ok_or_else(|| Fail::new("internal", format!("an anchor of no known level: {r}")))
    }

    fn at_min_level(&self, r: &Value, number: u64) -> Result<(), Fail> {
        let level = Self::level_of(r)?;
        if level < self.opts.min_level {
            return Err(Fail::new(
                "below-min-level",
                format!(
                    "block {number} is {} on {}, below --min-level {}",
                    level.as_str(),
                    self.l1_name(),
                    self.opts.min_level.as_str()
                ),
            ));
        }
        Ok(())
    }

    /// The report of a passing anchor: anchor-verify's fields, how it was reached, and the start.
    fn report(&self, r: &Value) -> Value {
        let mut out = json!({"by": "light client", "anchored": true});
        if let Value::Object(m) = r {
            for (k, v) in m {
                if k != "ok" && k != "matches" {
                    out[k] = v.clone();
                }
            }
        }
        out["start"] = self.start_json();
        out
    }

    /// The report of an anchor that failed.
    pub fn failed(&self, f: &Fail) -> Value {
        json!({
            "by": "light client",
            "anchored": false,
            "kind": f.kind,
            // true: the data was shown wrong; false: it could not be checked (no verdict).
            "disproven": f.is_verdict(),
            "message": f.message,
            "start": self.start_json(),
        })
    }

    /// Block `number` with hash `hash`, anchored at `--min-level` or above: the report, or why not.
    pub fn block(&mut self, number: u64, hash: &str) -> Result<Value, Fail> {
        let hash = hash.to_ascii_lowercase();
        if let Some(h) = &self.head {
            if h.number == number && h.hash == hash {
                return Ok(h.report.clone());
            }
        }
        for attempt in 0..2 {
            let body = self.anchor_body(json!(format!("0x{number:x}")))?;
            let r = match self.checked(number, &hash, &body) {
                // A stale signed head: a fresh anchor, once.
                Err(f) if attempt == 0 && f.kind == "stale-head" => continue,
                other => other?,
            };
            if r["matches"] != true {
                return Err(Fail::new(
                    "mismatch",
                    format!(
                        "block {number} of chain {} is {} on {}, not {hash}",
                        self.chain_id,
                        r["hash"].as_str().unwrap_or("another block"),
                        self.l1_name()
                    ),
                ));
            }
            self.at_min_level(&r, number)?;
            return Ok(self.report(&r));
        }
        unreachable!("the loop returns")
    }

    /// The newest block (`latest`), or Ethereum's finalized one (`finalized`), anchored at
    /// `--min-level` or above and at most `--max-head-age` old.
    pub fn head(&mut self, which: &str) -> Result<Head, Fail> {
        let c = self.chain_id;
        let bnb = c == anchor_verify::bnb::CHAIN_ID;
        let tag = match which {
            "latest" if c == 1 && self.opts.min_level > Level::Signed => "finalized",
            "latest" => "latest",
            "finalized" if c == 1 => "finalized",
            "finalized" if bnb => "latest",
            other => {
                return Err(Fail::new(
                    "unverifiable-tag",
                    format!("\"{other}\" on chain {c} can't be anchored: name a block"),
                ))
            }
        };
        if c != 1 && !bnb && self.opts.min_level > Level::Signed {
            return Err(Fail::new(
                "below-min-level",
                format!(
                    "chain {c}'s newest block is only ever signed, below --min-level {}: name a block",
                    self.opts.min_level.as_str()
                ),
            ));
        }
        let body = self.anchor_body(json!(tag))?;
        let n = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|v| {
                let s = v["result"]["blockNumber"].as_str()?.to_string();
                u64::from_str_radix(s.strip_prefix("0x")?, 16).ok()
            })
            .ok_or_else(|| {
                Fail::new(
                    "malformed-input",
                    format!("the anchor for \"{tag}\" names no block number"),
                )
            })?;
        let r = self.checked(n, ZERO_HASH, &body)?;
        let time = if c == 1 {
            if tag == "latest" && r["path"] != "head" {
                return Err(Fail::new(
                    "not-the-head",
                    format!(
                        "\"latest\" was anchored by path {}, not the signed head",
                        r["path"]
                    ),
                ));
            }
            if tag == "finalized" && (r["level"] != "final" || r["finalized"].as_u64() != Some(n)) {
                return Err(Fail::new(
                    "not-the-head",
                    format!(
                        "\"finalized\" named block {n}, but the light client's finalized block is {}",
                        r["finalized"]
                    ),
                ));
            }
            r["headTime"].as_u64()
        } else if r["path"] == "sequencer" || r["path"] == "parlia" {
            r["anchorTime"].as_u64()
        } else {
            None
        };
        let Some(time) = time else {
            return Err(Fail::new(
                "stale-head",
                format!("the anchor for \"{tag}\" on chain {c} gives no time to bound its age"),
            ));
        };
        let age = self.now.saturating_sub(time);
        if age > self.opts.max_head_age {
            return Err(Fail::new(
                "stale-head",
                format!(
                    "the newest block Willow anchored on chain {c} is {age} s old (--max-head-age {})",
                    self.opts.max_head_age
                ),
            ));
        }
        self.at_min_level(&r, n)?;
        let hash = r["hash"].as_str().unwrap_or_default().to_ascii_lowercase();
        let mut report = self.report(&r);
        report["tag"] = json!(which);
        report["age"] = json!(age);
        let head = Head {
            number: n,
            hash,
            report,
        };
        self.head = Some(head.clone());
        Ok(head)
    }
}

/// An anchor module result that must be `ok`.
fn ok_json(s: &str) -> Result<Value, Fail> {
    let r: Value = serde_json::from_str(s).map_err(|e| Fail::new("internal", e.to_string()))?;
    if r["ok"] == true {
        Ok(r)
    } else {
        Err(Fail::of(&r))
    }
}

#[cfg(test)]
mod tests;
