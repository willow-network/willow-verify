//! The light clients anchor every chain. Heads per chain from
//! `willow_getBlockAnchor ["latest"]` (and `["finalized"]` on Ethereum), specific blocks from
//! `[n]`, each verified by `anchor_verify::Anchor` and kept until its level expires.

use crate::checkpoint::Starts;
use crate::config::Level;
use crate::error::Error;
use crate::hex::{is_hash_str, quantity, to_number, ZERO_HASH};
use crate::http::Reply;
use crate::lru::Lru;
use crate::storage::{Storage, LIGHT_CLIENT_KEY};
use crate::verifiers::{blocking, light_client_of, Backend};
use futures::future::{BoxFuture, FutureExt, Shared};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// A block hash anchored, as the client keeps it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Anchored {
    pub chain_id: u64,
    pub number: u64,
    pub hash: String,
    /// `None`: checked against a node the user trusts ([`crate::Anchoring::Rpc`]).
    pub level: Option<Level>,
    pub path: String,
    /// The signed head's (Ethereum) or signed block's timestamp, seconds.
    pub time: Option<u64>,
    /// When the anchor stops counting (ms); `u64::MAX` for final ones.
    pub expires: u64,
}

/// Light-client updates asked for at once (anchor-verify `MAX_UPDATES`; BNB: one per epoch).
const UPDATES_PER_ASK: u64 = 16;
const BNB_UPDATES_PER_ASK: u64 = 64;
/// BNB's signed level: one validator's seal, counted at most 10 minutes (anchor-verify bnb.rs).
const BNB_SIGNED_SECS: u64 = 600;
/// Catch-up rounds per anchor, as the page's confirmOnEthereum.
const CATCH_UP_ROUNDS: usize = 3;
const WAITING_MS: u64 = 4000;
const ETHEREUM_SIGNED_SECS: u64 = 600;
const L2_SIGNED_SECS: u64 = 3 * 3600;

fn key_of(lc: Option<&str>) -> String {
    match lc {
        Some(n) => format!("{LIGHT_CLIENT_KEY}:{n}"),
        None => LIGHT_CLIENT_KEY.to_string(),
    }
}

/// The anchor's failure as the client's error: a verdict, or why the block can't be anchored.
fn anchor_failure(r: &Value, method: Option<&str>) -> Error {
    let kind = r["kind"].as_str().unwrap_or("internal");
    let message = r["message"].as_str().unwrap_or("").to_string();
    match kind {
        "rejected" => Error::verification("anchor-rejected", message, method),
        "malformed-input" => Error::verification("anchor-malformed", message, method),
        "internal" => Error::verification("internal", message, method),
        k => Error::unverifiable(&format!("anchor-{k}"), message, method),
    }
}

fn level_of(r: &Value) -> Option<Level> {
    r["level"].as_str().and_then(Level::parse)
}

pub(crate) type Rpc =
    Arc<dyn Fn(u64, &str, Value) -> BoxFuture<'static, Result<Reply, Error>> + Send + Sync>;
pub(crate) type StartsFn = Arc<dyn Fn() -> BoxFuture<'static, Result<Starts, Error>> + Send + Sync>;
type AnchoredFuture = Shared<BoxFuture<'static, Result<Anchored, Error>>>;

pub(crate) struct Options {
    pub backend: Arc<Backend>,
    /// JSON-RPC on chain `chainId`'s route (1: the light client's).
    pub rpc: Rpc,
    pub storage: Option<Arc<dyn Storage>>,
    pub starts: StartsFn,
    pub min_level: Level,
    pub head_max_age_ms: Option<u64>,
    pub max_head_age_sec: u64,
    pub now_ms: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub waiting_ms: Option<u64>,
    pub anchor_wait_ms: u64,
}

struct Head {
    value: Anchored,
    at: u64,
    started: u64,
}

pub(crate) struct Anchors {
    o: Options,
    restored: tokio::sync::OnceCell<()>,
    /// What each light client last kept (by key_of): it changes only when that client catches up.
    kept: Mutex<HashMap<String, String>>,
    /// One verify (and its catch-up) at a time: catch-up resets and advances the light client.
    lock: tokio::sync::Mutex<()>,
    cache: Mutex<Lru<Anchored>>,
    by_number: Mutex<Lru<String>>,
    heads: Mutex<HashMap<String, Head>>,
    inflight: Mutex<HashMap<String, (u64, AnchoredFuture)>>,
    fetches: std::sync::atomic::AtomicU64,
    /// Tests: a pause after each fetch's result is handed over.
    #[cfg(test)]
    after_send: std::sync::atomic::AtomicU64,
}

/// An anchor fetch's key leaves the in-flight map however the fetch ends: done, or dropped with
/// a runtime that ended (a key left behind would hold a failed fetch for every later caller).
struct Settled {
    anchors: Arc<Anchors>,
    key: String,
    id: u64,
}

impl Drop for Settled {
    fn drop(&mut self) {
        let mut inflight = self
            .anchors
            .inflight
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        if inflight
            .get(&self.key)
            .is_some_and(|(id, _)| *id == self.id)
        {
            inflight.remove(&self.key);
        }
    }
}

pub(crate) fn at_least(l: Level, min: Level) -> bool {
    l >= min
}

impl Anchors {
    pub(crate) fn new(o: Options) -> Arc<Anchors> {
        Arc::new(Anchors {
            o,
            restored: tokio::sync::OnceCell::new(),
            kept: Mutex::new(HashMap::new()),
            lock: tokio::sync::Mutex::new(()),
            cache: Mutex::new(Lru::new(8192, usize::MAX)),
            by_number: Mutex::new(Lru::new(8192, usize::MAX)),
            heads: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
            fetches: std::sync::atomic::AtomicU64::new(0),
            #[cfg(test)]
            after_send: std::sync::atomic::AtomicU64::new(0),
        })
    }

    fn now_ms(&self) -> u64 {
        (self.o.now_ms)()
    }

    fn now_sec(&self) -> u64 {
        self.now_ms() / 1000
    }

    fn head_max_age_ms(&self, chain_id: u64) -> u64 {
        self.o
            .head_max_age_ms
            .unwrap_or(if chain_id == 1 { 12_000 } else { 2_000 })
    }

    async fn restore(&self) {
        self.restored
            .get_or_init(|| async {
                let Some(storage) = self.o.storage.clone() else {
                    return;
                };
                let Ok(starts) = (self.o.starts)().await else {
                    return;
                };
                for lc in [None, Some("sepolia"), Some("bnb")] {
                    let Some(saved) = storage.get(&key_of(lc)) else {
                        continue;
                    };
                    // Kept data is verified again from the start; a failure only means starting over.
                    let backend = self.o.backend.clone();
                    let starts = starts.clone();
                    let now = self.now_sec();
                    let kept = saved.clone();
                    let r =
                        blocking(move || backend.anchor_restore(&starts, &saved, now, lc)).await;
                    // What was restored is what storage holds: written again only once it changes.
                    if r.is_ok_and(|r| r["ok"] == true) {
                        self.kept.lock().unwrap().insert(key_of(lc), kept);
                    }
                }
            })
            .await;
    }

    fn refused(reply: &Reply, what: &str, method: Option<&str>) -> Error {
        let e = reply.error().cloned().unwrap_or(Value::Null);
        let message = match &e["message"] {
            Value::String(s) => s.clone(),
            Value::Null => String::new(),
            v => v.to_string(),
        };
        Error::unverifiable(
            "anchor-refused",
            format!(
                "{what}: {message} ({})",
                match &e["code"] {
                    Value::Null => "undefined".to_string(),
                    Value::String(s) => s.clone(),
                    v => v.to_string(),
                }
            ),
            method,
        )
    }

    /// The light client's bootstrap: Ethereum's on chain 1's route, another's on the verified
    /// chain's own route (where Willow serves that light client's data).
    async fn bootstrap(
        &self,
        chain_id: u64,
        lc: Option<&'static str>,
        method: Option<&str>,
    ) -> Result<(), Error> {
        let starts = (self.o.starts)().await?;
        let root = match lc {
            Some("sepolia") => starts.sepolia.root.clone(),
            Some(_) => starts.bnb.hash.clone(),
            None => starts.ethereum.root.clone(),
        };
        let route = if lc.is_some() { chain_id } else { 1 };
        let reply = (self.o.rpc)(route, "willow_lightClientBootstrap", json!([root])).await?;
        if reply.error().is_some() {
            return Err(Self::refused(&reply, "willow_lightClientBootstrap", method));
        }
        let backend = self.o.backend.clone();
        let text = reply.text.clone();
        let now = self.now_sec();
        let r = blocking(move || backend.anchor_bootstrap(&starts, &text, now, lc))
            .await
            .map_err(|e| e.with_method(method))?;
        if r["ok"] != true {
            return Err(anchor_failure(&r, method));
        }
        Ok(())
    }

    async fn updates(
        &self,
        chain_id: u64,
        lc: Option<&'static str>,
        from: Option<u64>,
        count: Option<u64>,
        method: Option<&str>,
    ) -> Result<(), Error> {
        let (Some(mut from), Some(count)) = (from, count) else {
            return Err(Error::verification(
                "internal",
                format!("the light client asked for periods {from:?} ({count:?})"),
                method,
            ));
        };
        if count < 1 || from >= 1 << 53 || count >= 1 << 53 {
            return Err(Error::verification(
                "internal",
                format!("the light client asked for periods {from} ({count})"),
                method,
            ));
        }
        let starts = (self.o.starts)().await?;
        let route = if lc.is_some() { chain_id } else { 1 };
        let per = if lc == Some("bnb") {
            BNB_UPDATES_PER_ASK
        } else {
            UPDATES_PER_ASK
        };
        let mut left = count;
        while left > 0 {
            let k = left.min(per);
            let reply = (self.o.rpc)(route, "willow_lightClientUpdates", json!([from, k])).await?;
            if reply.error().is_some() {
                return Err(Self::refused(&reply, "willow_lightClientUpdates", method));
            }
            let backend = self.o.backend.clone();
            let text = reply.text.clone();
            let s = starts.clone();
            let r = blocking(move || backend.anchor_updates(&s, &text, lc)).await?;
            if r["ok"] != true {
                return Err(anchor_failure(&r, method));
            }
            if r["period"].is_null() {
                return Err(Error::unverifiable(
                    "anchor-unreachable",
                    format!("Willow sent no light-client update for period {from}"),
                    method,
                ));
            }
            from += k;
            left -= k;
        }
        Ok(())
    }

    /// `Anchor.verify` of a `willow_getBlockAnchor` response body, fetching light-client data as
    /// it asks.
    pub(crate) async fn verify(
        &self,
        chain_id: u64,
        number: u64,
        hash: &str,
        body: Arc<str>,
        method: Option<&str>,
    ) -> Result<Value, Error> {
        let _one = self.lock.lock().await;
        self.restore().await;
        let mut caught_up = false;
        let key = key_of(light_client_of(chain_id));
        for round in 0.. {
            let want_saved = self.o.storage.is_some()
                && (caught_up || !self.kept.lock().unwrap().contains_key(&key));
            let starts = (self.o.starts)().await?;
            let backend = self.o.backend.clone();
            let (b, h, now) = (body.clone(), hash.to_string(), self.now_sec());
            let (r, saved) = blocking(move || {
                backend.anchor_verify(&starts, chain_id, number, &h, &b, now, want_saved)
            })
            .await
            .map_err(|e| e.with_method(method))?;
            if r["ok"] == true {
                if let Some(saved) = saved.filter(|s| s != "null") {
                    let changed = self.kept.lock().unwrap().get(&key) != Some(&saved);
                    if changed {
                        self.kept.lock().unwrap().insert(key.clone(), saved.clone());
                        if let Some(s) = &self.o.storage {
                            s.set(&key, &saved);
                        }
                    }
                }
                return Ok(r);
            }
            let kind = r["kind"].as_str().unwrap_or("");
            if kind != "need-bootstrap" && kind != "need-updates" {
                return Err(anchor_failure(&r, method));
            }
            if round >= CATCH_UP_ROUNDS {
                return Err(Error::unverifiable(
                    "anchor-unreachable",
                    "Willow's light-client data did not reach the validators who signed this block",
                    method,
                ));
            }
            let lc = match r["lightClient"].as_str() {
                Some("sepolia") => Some("sepolia"),
                Some("bnb") => Some("bnb"),
                Some(other) => {
                    return Err(Error::verification(
                        "internal",
                        format!("the anchor asked for light client {other}"),
                        method,
                    ))
                }
                None => None,
            };
            if kind == "need-bootstrap" {
                self.bootstrap(chain_id, lc, method).await?;
            } else {
                self.updates(
                    chain_id,
                    lc,
                    r["from"].as_u64(),
                    r["count"].as_u64(),
                    method,
                )
                .await?;
            }
            caught_up = true;
        }
        unreachable!()
    }

    /// A `willow_getBlockAnchor` response. A block newer than the signed head (the server says
    /// `waiting`: the light client signs block N only in slot N+1) is asked for again until
    /// `anchor_wait` has passed, then it is unverifiable (`anchor-waiting`, -32099). The first
    /// ask is queued now, so it shares a batch with whatever is asked next.
    pub(crate) fn body(
        self: &Arc<Self>,
        chain_id: u64,
        param: Value,
        method: Option<String>,
    ) -> Shared<BoxFuture<'static, Result<Reply, Error>>> {
        let first = (self.o.rpc)(chain_id, "willow_getBlockAnchor", json!([param.clone()]));
        let me = self.clone();
        let started = Instant::now();
        async move {
            let mut next = Some(first);
            loop {
                let reply = match next.take() {
                    Some(f) => f.await?,
                    None => (me.o.rpc)(chain_id, "willow_getBlockAnchor", json!([param.clone()])).await?,
                };
                let Some(e) = reply.error() else {
                    return Ok(reply);
                };
                let data = e.get("data").filter(|d| d.is_object());
                if data.and_then(|d| d.get("status")) != Some(&json!("waiting")) {
                    return Ok(reply);
                }
                let bound = me.o.anchor_wait_ms;
                let elapsed = started.elapsed().as_millis() as u64;
                if elapsed >= bound {
                    let shown = match &param {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    };
                    let message = e["message"].as_str().unwrap_or("").to_string();
                    return Err(Error::unverifiable(
                        "anchor-waiting",
                        format!("block {shown} of chain {chain_id} was not anchored within {bound} ms: {message}"),
                        method.as_deref(),
                    ));
                }
                let left = bound - elapsed;
                let after = data
                    .and_then(|d| d.get("retryAfterSeconds"))
                    .and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok())))
                    .filter(|a| *a > 0.0 && *a <= 60.0);
                let every = me
                    .o
                    .waiting_ms
                    .unwrap_or_else(|| after.map_or(WAITING_MS, |a| (a * 1000.0) as u64));
                tokio::time::sleep(Duration::from_millis(every.min(left))).await;
            }
        }
        .boxed()
        .shared()
    }

    fn expiry(&self, chain_id: u64, r: &Value) -> u64 {
        let now = self.now_ms();
        let soon = now.saturating_add(self.head_max_age_ms(chain_id));
        let until = |t: u64, secs: u64| t.saturating_add(secs).saturating_mul(1000);
        match level_of(r) {
            Some(Level::Final) => return u64::MAX,
            Some(Level::Committed) => return soon,
            _ => {}
        }
        if chain_id == 1 {
            return r["headTime"]
                .as_u64()
                .map_or(soon, |t| until(t, ETHEREUM_SIGNED_SECS));
        }
        if r["path"] == "parlia" {
            return r["anchorTime"]
                .as_u64()
                .map_or(soon, |t| until(t, BNB_SIGNED_SECS));
        }
        r["anchorTime"]
            .as_u64()
            .map_or(soon, |t| until(t, L2_SIGNED_SECS))
    }

    fn keep(&self, a: &Anchored) {
        let h = a.hash.to_ascii_lowercase();
        self.cache
            .lock()
            .unwrap()
            .set(&format!("{}:{}:{h}", a.chain_id, a.number), a.clone(), 0);
        self.by_number
            .lock()
            .unwrap()
            .set(&format!("{}:{}", a.chain_id, a.number), h, 0);
    }

    fn head_value(&self, chain_id: u64) -> Option<Anchored> {
        self.heads
            .lock()
            .unwrap()
            .get(&format!("{chain_id}:latest"))
            .map(|h| h.value.clone())
    }

    fn cached(&self, chain_id: u64, number: u64) -> Option<Anchored> {
        let hash = self
            .by_number
            .lock()
            .unwrap()
            .get(&format!("{chain_id}:{number}"))?;
        self.cache
            .lock()
            .unwrap()
            .get(&format!("{chain_id}:{number}:{hash}"))
    }

    /// The anchored hash of block `number`, when an anchor of it still counts (the head's included).
    pub(crate) fn hash_of(&self, chain_id: u64, number: u64) -> Option<String> {
        let now = self.now_ms();
        if let Some(h) = self.head_value(chain_id) {
            if h.number == number && h.expires > now {
                return Some(h.hash);
            }
        }
        self.cached(chain_id, number)
            .filter(|a| a.expires > now)
            .map(|a| a.hash)
    }

    /// Whether block `number` has an anchor that still counts (any hash): no anchor need be
    /// fetched ahead.
    pub(crate) fn known(&self, chain_id: u64, number: u64) -> bool {
        if self
            .head_value(chain_id)
            .is_some_and(|h| h.number == number)
        {
            return true;
        }
        self.cached(chain_id, number)
            .is_some_and(|a| a.expires > self.now_ms())
    }

    /// The newest block (`latest`), or on Ethereum the finalized one, anchored at `min_level` or above.
    pub(crate) async fn head(
        self: &Arc<Self>,
        chain_id: u64,
        which: &'static str,
        force: bool,
        method: Option<&str>,
    ) -> Result<Anchored, Error> {
        let min = self.o.min_level;
        // Ethereum's newest block at "final" is its finalized one; an L2 has no tag for a newer
        // block above "signed".
        let tag: &'static str = if which == "latest" && chain_id == 1 && min != Level::Signed {
            "finalized"
        } else {
            which
        };
        if which == "latest" && chain_id != 1 && chain_id != 56 && min != Level::Signed {
            return Err(Error::unverifiable(
                "anchor-level",
                format!("chain {chain_id} has no newest block anchored at {min}; name a block"),
                method,
            ));
        }
        let key = format!("{chain_id}:{which}");
        let now = self.now_ms();
        {
            let heads = self.heads.lock().unwrap();
            if let Some(h) = heads.get(&key) {
                if !force
                    && now.saturating_sub(h.at) < self.head_max_age_ms(chain_id)
                    && h.value.expires > now
                {
                    return Ok(h.value.clone());
                }
                // An eth_blockNumber burst shares one refresh.
                if force && now.saturating_sub(h.started) < 500 && h.value.expires > now {
                    return Ok(h.value.clone());
                }
            }
        }
        let me = self.clone();
        let m = method.map(str::to_string);
        self.fetch_once(key, move || async move {
            me.fetch_head(chain_id, which, tag, m.as_deref()).await
        })
        .await
    }

    /// The one fetch for `key`: joined while in flight, else started. Spawned, so it runs to its
    /// end whoever waits (a caller that gives up mid catch-up must not leave the light client's
    /// lock with a future nobody polls), and spawned outside the lock (a closing runtime drops the
    /// task at once, and its guard takes the lock).
    fn fetch_once<F, Fut>(self: &Arc<Self>, key: String, fetch: F) -> AnchoredFuture
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Anchored, Error>> + Send + 'static,
    {
        let (f, id, tx) = {
            let mut inflight = self.inflight.lock().unwrap();
            if let Some((_, f)) = inflight.get(&key) {
                return f.clone();
            }
            let id = self
                .fetches
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let (tx, rx) = tokio::sync::oneshot::channel();
            let f: AnchoredFuture = async move {
                // Ended with its runtime: not a verdict on the server.
                rx.await.unwrap_or_else(|_| {
                    Err(Error::rpc(
                        -32603,
                        "the anchor fetch ended before a response",
                    ))
                })
            }
            .boxed()
            .shared();
            inflight.insert(key.clone(), (id, f.clone()));
            (f, id, tx)
        };
        let done = Settled {
            anchors: self.clone(),
            key,
            id,
        };
        let run = fetch();
        tokio::spawn(async move {
            let done = done;
            let r = std::panic::AssertUnwindSafe(run)
                .catch_unwind()
                .await
                .unwrap_or_else(|p| {
                    let why = p
                        .downcast_ref::<String>()
                        .cloned()
                        .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                        .unwrap_or_else(|| "an anchor fetch panicked".into());
                    Err(Error::verification("internal", why, None))
                });
            #[cfg(test)]
            let me = done.anchors.clone();
            // Out of flight before the response, so a caller that has it and asks again is asked anew.
            drop(done);
            let _ = tx.send(r);
            #[cfg(test)]
            me.sent();
        });
        f
    }

    #[cfg(test)]
    pub(crate) fn pause_after_send(&self, d: Duration) {
        self.after_send
            .store(d.as_millis() as u64, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    fn sent(&self) {
        let ms = self.after_send.load(std::sync::atomic::Ordering::SeqCst);
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }

    /// Tests: how many futures share the fetch in flight for `key` (the map's own included).
    #[cfg(test)]
    pub(crate) fn sharing(&self, key: &str) -> usize {
        self.inflight
            .lock()
            .unwrap()
            .get(key)
            .and_then(|(_, f)| f.strong_count())
            .unwrap_or(0)
    }

    async fn fetch_head(
        self: &Arc<Self>,
        chain_id: u64,
        which: &str,
        tag: &str,
        method: Option<&str>,
    ) -> Result<Anchored, Error> {
        let started = self.now_ms();
        let reply = self
            .body(chain_id, json!(tag), method.map(str::to_string))
            .await?;
        if reply.error().is_some() {
            return Err(Self::refused(
                &reply,
                &format!("willow_getBlockAnchor [\"{tag}\"]"),
                method,
            ));
        }
        let n = reply
            .result()
            .get("blockNumber")
            .and_then(to_number)
            .ok_or_else(|| {
                Error::verification(
                    "anchor-malformed",
                    format!("the anchor for \"{tag}\" names no block number"),
                    method,
                )
            })?;
        let r = self
            .verify(chain_id, n, ZERO_HASH, reply.text.clone(), method)
            .await?;
        let path = r["path"].as_str().unwrap_or("").to_string();
        let level = level_of(&r).ok_or_else(|| {
            Error::verification("anchor-malformed", "the anchor names no level", method)
        })?;
        let time = if chain_id == 1 {
            // Ethereum's newest block is the signed head itself.
            if tag == "latest" && path != "head" {
                return Err(Error::verification(
                    "anchor-head",
                    format!("\"latest\" was anchored by path {path}, not the signed head"),
                    method,
                ));
            }
            // "finalized" is the light client's finalized block, at level final.
            if tag == "finalized" && (level != Level::Final || r["finalized"].as_u64() != Some(n)) {
                return Err(Error::verification(
                    "anchor-head",
                    format!(
                        "\"finalized\" named block {n}, but the light client's finalized block is {}",
                        match &r["finalized"] {
                            Value::Null => "undefined".to_string(),
                            v => v.to_string(),
                        }
                    ),
                    method,
                ));
            }
            r["headTime"].as_u64()
        } else if path == "sequencer" || path == "parlia" {
            // An L2's newest block is the newest its sequencer signed, aged by its own timestamp;
            // BNB's is its newest finalized block, aged the same way.
            r["anchorTime"].as_u64()
        } else {
            None
        };
        let Some(time) = time else {
            return Err(Error::unverifiable(
                "anchor-stale-head",
                format!(
                    "the anchor for \"{tag}\" on chain {chain_id} gives no time to bound its age"
                ),
                method,
            ));
        };
        let age = i128::from(self.now_sec()) - i128::from(time);
        if age > i128::from(self.o.max_head_age_sec) {
            return Err(Error::unverifiable(
                "anchor-stale-head",
                format!("the newest block Willow anchored on chain {chain_id} is {age} s old"),
                method,
            ));
        }
        if !at_least(level, self.o.min_level) {
            return Err(Error::unverifiable(
                "anchor-level",
                format!("the newest block is {level}, below {}", self.o.min_level),
                method,
            ));
        }
        let value = Anchored {
            chain_id,
            number: n,
            hash: r["hash"].as_str().unwrap_or("").to_ascii_lowercase(),
            level: Some(level),
            path,
            time: Some(time),
            expires: self.expiry(chain_id, &r),
        };
        self.keep(&value);
        self.heads.lock().unwrap().insert(
            format!("{chain_id}:{which}"),
            Head {
                value: value.clone(),
                at: self.now_ms(),
                started,
            },
        );
        Ok(value)
    }

    /// Block `number` with `hash`, anchored at `min_level` or above: the cached head or anchor,
    /// else `willow_getBlockAnchor [n]` (`prefetched` when it was asked for with the proof).
    pub(crate) async fn block(
        self: &Arc<Self>,
        chain_id: u64,
        number: u64,
        hash: &str,
        prefetched: Option<Shared<BoxFuture<'static, Result<Reply, Error>>>>,
        method: Option<&str>,
    ) -> Result<Anchored, Error> {
        let h = hash.to_ascii_lowercase();
        let now = self.now_ms();
        if let Some(head) = self.head_value(chain_id) {
            if head.number == number && head.hash == h && head.expires > now {
                return Ok(head);
            }
        }
        let key = format!("{chain_id}:{number}:{h}");
        if let Some(c) = self.cache.lock().unwrap().get(&key) {
            if c.expires > now {
                return Ok(c);
            }
        }
        let me = self.clone();
        let m = method.map(str::to_string);
        self.fetch_once(key, move || async move {
            me.fetch_block(chain_id, number, &h, prefetched, m.as_deref())
                .await
        })
        .await
    }

    async fn fetch_block(
        self: &Arc<Self>,
        chain_id: u64,
        number: u64,
        hash: &str,
        prefetched: Option<Shared<BoxFuture<'static, Result<Reply, Error>>>>,
        method: Option<&str>,
    ) -> Result<Anchored, Error> {
        let mut prefetched = prefetched;
        for attempt in 0.. {
            let reply = match prefetched.take() {
                Some(p) if attempt == 0 => p.await?,
                _ => {
                    self.body(
                        chain_id,
                        json!(quantity(number)),
                        method.map(str::to_string),
                    )
                    .await?
                }
            };
            if reply.error().is_some() {
                return Err(Self::refused(
                    &reply,
                    &format!("willow_getBlockAnchor [{number}]"),
                    method,
                ));
            }
            let r = match self
                .verify(chain_id, number, hash, reply.text.clone(), method)
                .await
            {
                Ok(r) => r,
                // A stale signed head: refresh this chain's head, and ask once more.
                Err(e)
                    if attempt == 0
                        && e.kind() == Some("anchor-stale-head")
                        && matches!(e, Error::Unverifiable(_)) =>
                {
                    self.heads
                        .lock()
                        .unwrap()
                        .remove(&format!("{chain_id}:latest"));
                    continue;
                }
                Err(e) => return Err(e),
            };
            let proven = r["hash"].as_str().unwrap_or("").to_string();
            if r["matches"] != true {
                return Err(Error::verification(
                    "anchor-mismatch",
                    format!(
                        "block {number} of chain {chain_id} is {proven} on Ethereum, not {hash}"
                    ),
                    method,
                ));
            }
            let level = level_of(&r).ok_or_else(|| {
                Error::verification("anchor-malformed", "the anchor names no level", method)
            })?;
            if !at_least(level, self.o.min_level) {
                return Err(Error::unverifiable(
                    "anchor-level",
                    format!("block {number} is {level}, below {}", self.o.min_level),
                    method,
                ));
            }
            let a = Anchored {
                chain_id,
                number,
                hash: hash.to_string(),
                level: Some(level),
                path: r["path"].as_str().unwrap_or("").to_string(),
                time: r["anchorTime"].as_u64().or_else(|| r["headTime"].as_u64()),
                expires: self.expiry(chain_id, &r),
            };
            self.keep(&a);
            return Ok(a);
        }
        unreachable!()
    }

    /// A verified block's parent is in the same anchored history: its hash counts at the
    /// child's level.
    pub(crate) fn parent(&self, child: &Anchored, parent_hash: &str) {
        if child.number == 0 || !is_hash_str(parent_hash) || child.level.is_none() {
            return;
        }
        let ph = parent_hash.to_ascii_lowercase();
        let key = format!("{}:{}:{ph}", child.chain_id, child.number - 1);
        if let Some(c) = self.cache.lock().unwrap().get(&key) {
            if c.expires >= child.expires {
                return;
            }
        }
        self.keep(&Anchored {
            number: child.number - 1,
            hash: ph,
            ..child.clone()
        });
    }
}
