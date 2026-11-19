//! The verifying client: `request` as EIP-1193, every response verified. A
//! port of `@willow-network/rpc`'s client.ts: the same method table, refusals, error kinds,
//! batching, caches and events.

use crate::anchor::{self, Anchored, Anchors};
use crate::checkpoint::{self, Starts};
use crate::config::{
    AnchoredBy, Anchoring, BlockRef, Config, Event, Flag, Level, OpenVmVerifier, Status,
    StorageChoice, Unverifiable,
};
use crate::error::Error;
use crate::hex::{
    is_address, is_hash, keccak_hex, quantity, quantity_u128, same_hex, shown, slot_key,
    str_number, to_bytes, to_number,
};
use crate::http::{Batcher, Reply, MAX_BATCH};
use crate::lru::Lru;
use crate::storage::{FileStorage, Storage};
#[cfg(feature = "logs")]
use crate::verifiers::Ran;
use crate::verifiers::{blocking, AnchorFactory, Backend, OpenVm};
use futures::future::{join_all, BoxFuture, FutureExt, Shared};
use serde_json::{json, Map, Value};
use std::collections::{BTreeSet, HashMap};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

/// This crate's version, as `web3_clientVersion` reports it.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const ORIGIN: &str = "https://api.willow.tech";
/// eth_getLogs over at most this many blocks goes by block-receipts proofs.
pub const LOGS_BY_RECEIPTS: u64 = 8;
const FEE_HISTORY_MAX: u64 = 1024;
/// Blocks per fee-history proof with rewards (`MAX_HISTORY_WITH_REWARDS`).
pub const FEE_HISTORY_WITH_REWARDS: u64 = 16;
const EMPTY_CODE_HASH: &str = "0xc5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470";
const NOT_FOUND_TX: &str = "not found in this store: transaction ";
const FILTER_IDLE_MS: u64 = 5 * 60 * 1000;
const NITRO: [u64; 4] = [42161, 4663, 421614, 46630];
const PROOF_DOWNLOADS: usize = 4;
/// The chains whose traces are proven (trace-verify): Ethereum and Unichain.
const TRACED: [u64; 2] = [1, 130];
/// The chains whose calls' traces are proven (debug_traceCall, trace_call): Ethereum.
const CALL_TRACED: [u64; 1] = [1];
const BLOCK_TRACES: [&str; 4] = [
    "debug_traceBlockByNumber",
    "debug_traceBlockByHash",
    "trace_block",
    "trace_replayBlockTransactions",
];
const TX_TRACES: [&str; 3] = [
    "debug_traceTransaction",
    "trace_transaction",
    "trace_replayTransaction",
];
/// Whether this release refuses the fee methods, gas estimates and Robinhood Chain's calls as
/// unverifiable (`no-verifier`). It doesn't: it verifies them.
pub(crate) const HELD: bool = false;

/// The most blocks one trace_filter spans.
pub const TRACE_FILTER_BLOCKS: u64 = 16;
/// The most blocks one `eth_getFilterChanges` on a block filter returns; the next poll goes on
/// from there. Above the blocks any chain makes in a filter's 5 idle minutes, so it bounds only a
/// newest block the client doesn't verify (`flag`, `Anchoring::None`).
pub const BLOCK_FILTER_MAX: u64 = 2000;

/// A request's outcome inside the client: an error, or (under `flag`) the server's value.
enum Fail {
    Err(Error),
    Fallback(Value),
}

impl From<Error> for Fail {
    fn from(e: Error) -> Fail {
        Fail::Err(e)
    }
}

type R<T> = Result<T, Fail>;

/// A block the proofs must name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Resolved {
    number: Option<u64>,
    hash: Option<String>,
}

impl Resolved {
    fn num(n: u64) -> Resolved {
        Resolved {
            number: Some(n),
            hash: None,
        }
    }
    fn hash(h: String) -> Resolved {
        Resolved {
            number: None,
            hash: Some(h),
        }
    }
}

struct CallState {
    status: Status,
    block: Option<BlockRef>,
    level: Option<Level>,
    flags: Vec<Flag>,
    unproven: BTreeSet<String>,
    /// The error raised is the client's own verified outcome (a revert), not a refusal.
    proven_error: bool,
    /// For a filter served here, the server request whose value `flag` may return instead.
    stand_in: Option<(String, Value)>,
}

/// The app's request, as the client tracks it for its event.
#[derive(Clone)]
struct Call {
    method: String,
    params: Vec<Value>,
    raw: Value,
    started: Instant,
    st: Arc<Mutex<CallState>>,
}

impl Call {
    fn p(&self, i: usize) -> &Value {
        self.params.get(i).unwrap_or(&Value::Null)
    }

    fn m(&self) -> Option<&str> {
        Some(&self.method)
    }

    fn anchored(&self, a: &Anchored) {
        let mut s = self.st.lock().unwrap();
        if s.block.as_ref().is_none_or(|b| a.number >= b.number) {
            s.block = Some(BlockRef {
                number: a.number,
                hash: a.hash.clone(),
            });
        }
        if let Some(l) = a.level {
            if s.level.is_none_or(|have| l < have) {
                s.level = Some(l);
            }
        }
    }

    fn flag(&self, kind: &str, detail: Option<String>) {
        let mut s = self.st.lock().unwrap();
        s.flags.push(Flag {
            kind: kind.into(),
            detail,
        });
        s.status = Status::Unverifiable;
    }

    fn leaves(&self, fields: &Value) {
        if let Some(a) = fields.as_array() {
            let mut s = self.st.lock().unwrap();
            for f in a.iter().filter_map(Value::as_str) {
                s.unproven.insert(f.to_string());
            }
        }
    }

    fn leaves_list(&self, fields: &[String]) {
        let mut s = self.st.lock().unwrap();
        for f in fields {
            s.unproven.insert(f.clone());
        }
    }

    fn set_status(&self, st: Status) {
        self.st.lock().unwrap().status = st;
    }

    fn set_stand_in(&self, method: &str, params: Value) {
        self.st.lock().unwrap().stand_in = Some((method.to_string(), params));
    }

    /// The server request standing for this one under `flag`: the request itself, a filter's
    /// eth_getLogs, or none (a filter served here with nothing on the server to stand for it).
    fn server_request(&self) -> Option<(String, Value)> {
        if let Some(x) = &self.st.lock().unwrap().stand_in {
            return Some(x.clone());
        }
        if LOCAL_FILTERS.contains(&self.method.as_str()) {
            return None;
        }
        let params = if self.raw.is_null() {
            json!([])
        } else {
            self.raw.clone()
        };
        Some((self.method.clone(), params))
    }
}

/// The filter methods served here, from the verified methods; the server knows none of their ids.
const LOCAL_FILTERS: [&str; 5] = [
    "eth_newFilter",
    "eth_newBlockFilter",
    "eth_getFilterChanges",
    "eth_getFilterLogs",
    "eth_uninstallFilter",
];

#[derive(Clone)]
struct BlockEntry {
    text: Arc<str>,
    proven: Value,
    full: Option<Value>,
    unproven: Vec<String>,
    full_unproven: Option<Vec<String>>,
    number: u64,
    hash: String,
}

#[derive(Clone)]
enum StateRead {
    Error(Value),
    Proven {
        proven: Value,
        report: Value,
        keys: Vec<String>,
    },
}

struct StateGroup {
    address: String,
    at: Resolved,
    keys: Vec<(String, String)>,
    code: bool,
    method: String,
    waiters: Vec<oneshot::Sender<Result<StateRead, Error>>>,
}

struct Filter {
    kind: &'static str,
    filter: Value,
    next: u64,
    touched: u64,
}

type Memo = Shared<BoxFuture<'static, Result<Value, Error>>>;
type ChainCheck = Shared<BoxFuture<'static, Result<(), Error>>>;

/// A memoized run's key leaves the memo however the run ends (done, or dropped with its runtime),
/// and only if the entry there is still this run's.
struct MemoDone {
    client: Arc<Client>,
    key: String,
    id: u64,
}

impl Drop for MemoDone {
    fn drop(&mut self) {
        let mut memo = self.client.memo.lock().unwrap_or_else(|p| p.into_inner());
        if memo.get(&self.key).is_some_and(|(i, _)| *i == self.id) {
            memo.remove(&self.key);
        }
    }
}

/// The one `eth_chainId` check, its task, and whether that task got to its end.
struct ChainCheckSlot {
    fut: ChainCheck,
    task: tokio::task::AbortHandle,
    settled: Arc<std::sync::atomic::AtomicBool>,
}

pub(crate) struct Client {
    chain_id: u64,
    url: String,
    flag_mode: bool,
    cfg: Config,
    now_ms: Arc<dyn Fn() -> u64 + Send + Sync>,
    batcher: Batcher,
    backend: Arc<Backend>,
    anchors: Arc<Anchors>,
    chain_check: Mutex<Option<ChainCheckSlot>>,
    memo: Mutex<HashMap<String, (u64, Memo)>>,
    memo_ids: AtomicU64,
    results: Mutex<Lru<Value>>,
    blocks: Mutex<Lru<BlockEntry>>,
    codes: Mutex<Lru<String>>,
    code_of: Mutex<Lru<String>>,
    sent: Mutex<HashMap<String, u128>>,
    state_queue: Mutex<HashMap<String, StateGroup>>,
    filters: Mutex<HashMap<String, Filter>>,
    filter_ids: AtomicU64,
}

/// `url` with its `/rpc/{chainId}` part naming chain `id`.
fn with_chain(url: &str, id: u64) -> Option<String> {
    let mut u = reqwest::Url::parse(url).ok()?;
    let path = u.path().to_string();
    // The last `/rpc/<digits>` that ends the path or a segment, as the npm client's
    // `^(.*\/rpc\/)(\d+)(\/.*)?$` finds it.
    let at = path
        .rmatch_indices("/rpc/")
        .map(|(i, m)| i + m.len())
        .find(|&at| {
            let rest = &path[at..];
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            digits > 0 && (rest.len() == digits || rest.as_bytes()[digits] == b'/')
        })?;
    let rest = &path[at..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    let tail = if rest.len() == digits {
        "/"
    } else {
        &rest[digits..]
    };
    u.set_path(&format!("{}{id}{tail}", &path[..at]));
    Some(u.to_string())
}

fn is_tag(x: &Value) -> bool {
    matches!(x, Value::Null) || x == "latest" || x == "pending"
}

fn empty_object(x: &Value) -> bool {
    match x {
        Value::Null => true,
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// A filter's address list, lowercased (empty: any).
fn address_list(x: &Value) -> BTreeSet<String> {
    let list: Vec<&Value> = match x {
        Value::Null => vec![],
        Value::Array(a) => a.iter().collect(),
        v => vec![v],
    };
    list.iter()
        .filter_map(|a| a.as_str())
        .map(str::to_ascii_lowercase)
        .collect()
}

/// `trace_filter`'s traces of the blocks' `trace_block` results, as the log RPC filters them
/// (trace-verify's filter, alloy's `TraceFilterMatcher`): matched by mode, `after` skipped, at
/// most `count`.
pub(crate) fn trace_filtered(filter: &Value, blocks: &[Value]) -> Vec<Value> {
    let from_set = address_list(&filter["fromAddress"]);
    let to_set = address_list(&filter["toAddress"]);
    let any = |s: &BTreeSet<String>, a: &Value| {
        s.is_empty()
            || a.as_str()
                .is_some_and(|a| s.contains(&a.to_ascii_lowercase()))
    };
    let union = filter["mode"] != "intersection";
    let all: Vec<Value> = blocks
        .iter()
        .flat_map(|b| b.as_array().cloned().unwrap_or_default())
        .filter(|t| {
            let a = &t["action"];
            let (from, to) = match t["type"].as_str() {
                Some("call") => (any(&from_set, &a["from"]), any(&to_set, &a["to"])),
                Some("create") => (
                    any(&from_set, &a["from"]),
                    if t["result"]["address"].is_string() {
                        any(&to_set, &t["result"]["address"])
                    } else {
                        to_set.is_empty()
                    },
                ),
                Some("suicide") => (
                    any(&from_set, &a["address"]),
                    any(&to_set, &a["refundAddress"]),
                ),
                Some("reward") => (from_set.is_empty(), any(&to_set, &a["author"])),
                _ => return false,
            };
            if !union {
                return from && to;
            }
            if from_set.is_empty() {
                return to;
            }
            if to_set.is_empty() {
                return from;
            }
            from || to
        })
        .collect();
    let mut out = all;
    if let Some(after) = to_number(&filter["after"]) {
        if after as usize >= out.len() {
            return vec![];
        }
        out = out.split_off(after as usize);
    }
    if let Some(count) = to_number(&filter["count"]) {
        out.truncate(count as usize);
    }
    out
}

/// Whether a proven log matches an `eth_getLogs` filter's address and topics.
fn matches(log: &Value, filter: &Value) -> bool {
    let a = &filter["address"];
    if !a.is_null() {
        let list: Vec<&Value> = match a {
            Value::Array(x) => x.iter().collect(),
            v => vec![v],
        };
        if !list.is_empty() && !list.iter().any(|x| same_hex(x, &log["address"])) {
            return false;
        }
    }
    if let Some(topics) = filter["topics"].as_array() {
        let have = log["topics"].as_array().map_or(0, Vec::len);
        if topics.len() > have {
            return false;
        }
        for (i, want) in topics.iter().enumerate() {
            if want.is_null() {
                continue;
            }
            let any: Vec<&Value> = match want {
                Value::Array(x) => x.iter().collect(),
                v => vec![v],
            };
            if !any.is_empty()
                && !any
                    .iter()
                    .any(|t| t.is_null() || same_hex(t, &log["topics"][i]))
            {
                return false;
            }
        }
    }
    true
}

fn rpc_error(e: &Value) -> Error {
    Error::from_object(e)
}

impl Client {
    pub(crate) fn new(o: Config) -> Result<Arc<Client>, Error> {
        let bad = |m: &str| Error::rpc(-32602, m);
        // The verifiers take a chain id as a u32 (as in wasm); a larger one would be another chain.
        if o.chain_id == 0 || o.chain_id > u64::from(u32::MAX) {
            return Err(bad("chainId must be a positive integer of at most 32 bits"));
        }
        let chain_id = o.chain_id;
        let url = o.url.clone().unwrap_or_else(|| {
            let key: String = url_component(o.key.as_deref().unwrap_or("public"));
            format!("{ORIGIN}/v1/rpc/{chain_id}/{key}")
        });
        let lc_url = match &o.light_client_url {
            Some(u) => u.clone(),
            None if chain_id == 1 => url.clone(),
            None => with_chain(&url, 1).ok_or_else(|| {
                bad("light_client_url is needed: url does not name a chain as /rpc/{chainId}/")
            })?,
        };
        let http = match &o.http {
            Some(h) => h.clone(),
            None => reqwest::Client::builder()
                .timeout(o.timeout)
                .gzip(true)
                .user_agent(concat!("willow-rpc/", env!("CARGO_PKG_VERSION")))
                .build()
                .map_err(|e| Error::rpc(-32603, format!("no HTTP client: {e}")))?,
        };
        let now_ms: Arc<dyn Fn() -> u64 + Send + Sync> = o.now_ms.clone().unwrap_or_else(|| {
            Arc::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0)
            })
        });
        let batcher = Batcher::new(http.clone(), o.batch_window, MAX_BATCH, o.timeout);
        #[cfg(test)]
        let factory: AnchorFactory = o
            .anchor_factory
            .clone()
            .unwrap_or_else(crate::verifiers::real_anchor);
        #[cfg(not(test))]
        let factory: AnchorFactory = crate::verifiers::real_anchor();
        let openvm = match &o.openvm {
            OpenVmVerifier::Default => OpenVm::Helper(None),
            OpenVmVerifier::Path(p) => OpenVm::Helper(Some(p.clone())),
            OpenVmVerifier::None => OpenVm::None,
        };
        let backend = Arc::new(Backend::new(factory, openvm).with_openvm_wait(o.timeout));
        let storage: Option<Arc<dyn Storage>> = match &o.storage {
            StorageChoice::None => None,
            StorageChoice::Custom(s) => Some(s.clone()),
            StorageChoice::Default => FileStorage::default_dir()
                .map(|d| Arc::new(FileStorage::new(d)) as Arc<dyn Storage>),
        };
        // Every light client's start, chosen once (a failure is tried again on the next request).
        let starts_cell: Arc<tokio::sync::OnceCell<Starts>> =
            Arc::new(tokio::sync::OnceCell::new());
        let starts: anchor::StartsFn = {
            let http = http.clone();
            let o = o.clone();
            let now_ms = now_ms.clone();
            Arc::new(move || {
                let cell = starts_cell.clone();
                let http = http.clone();
                let o = o.clone();
                let now_ms = now_ms.clone();
                async move {
                    cell.get_or_try_init(|| async {
                        let apis: Vec<String> = o.beacon_apis.clone().unwrap_or_else(|| {
                            checkpoint::BEACON_APIS
                                .iter()
                                .map(|s| s.to_string())
                                .collect()
                        });
                        checkpoint::choose(&checkpoint::Choice {
                            http: &http,
                            checkpoint: o.checkpoint.as_ref(),
                            sepolia: o.sepolia_checkpoint.as_ref(),
                            bnb: o.bnb_start.as_ref(),
                            checkpoint_url: o
                                .checkpoint_url
                                .as_deref()
                                .unwrap_or(checkpoint::CHECKPOINT_URL),
                            beacon_apis: &apis,
                            now_sec: now_ms() / 1000,
                        })
                        .await
                    })
                    .await
                    .cloned()
                }
                .boxed()
            })
        };
        let rpc: anchor::Rpc = {
            let batcher = batcher.clone();
            let url = url.clone();
            let lc_url = lc_url.clone();
            Arc::new(move |c: u64, method: &str, params: Value| {
                let target = if c == 1 {
                    Some(lc_url.clone())
                } else if c == chain_id {
                    Some(url.clone())
                } else {
                    with_chain(&url, c)
                };
                match target {
                    Some(t) => batcher.call(&t, method, params).boxed(),
                    None => {
                        let e = Error::rpc(
                            -32603,
                            format!("no route for chain {c}: url does not name a chain"),
                        );
                        async move { Err(e) }.boxed()
                    }
                }
            })
        };
        let anchors = Anchors::new(anchor::Options {
            backend: backend.clone(),
            rpc,
            storage,
            starts,
            min_level: o.min_level,
            head_max_age_ms: o.head_max_age.map(|d| d.as_millis() as u64),
            max_head_age_sec: o.max_head_age.as_secs(),
            now_ms: now_ms.clone(),
            waiting_ms: o.wait_anchor.map(|d| d.as_millis() as u64),
            anchor_wait_ms: o.anchor_wait.as_millis() as u64,
        });
        Ok(Arc::new(Client {
            chain_id,
            url,
            flag_mode: o.unverifiable == Unverifiable::Flag,
            cfg: o,
            now_ms,
            batcher,
            backend,
            anchors,
            chain_check: Mutex::new(None),
            memo: Mutex::new(HashMap::new()),
            memo_ids: AtomicU64::new(0),
            results: Mutex::new(Lru::new(1024, 16 << 20)),
            blocks: Mutex::new(Lru::new(256, 64 << 20)),
            codes: Mutex::new(Lru::new(512, 16 << 20)),
            code_of: Mutex::new(Lru::new(4096, usize::MAX)),
            sent: Mutex::new(HashMap::new()),
            state_queue: Mutex::new(HashMap::new()),
            filters: Mutex::new(HashMap::new()),
            filter_ids: AtomicU64::new(0),
        }))
    }

    pub(crate) fn chain_id(&self) -> u64 {
        self.chain_id
    }

    #[cfg(test)]
    pub(crate) fn anchors(&self) -> &Arc<Anchors> {
        &self.anchors
    }

    /// Tests: the most futures sharing any memoized run in flight (the memo's own included).
    #[cfg(test)]
    pub(crate) fn memo_sharing(&self) -> usize {
        self.memo
            .lock()
            .unwrap()
            .values()
            .filter_map(|(_, f)| f.strong_count())
            .max()
            .unwrap_or(0)
    }

    fn now(&self) -> u64 {
        (self.now_ms)()
    }

    fn anchored_by(&self) -> AnchoredBy {
        match self.cfg.anchoring {
            Anchoring::LightClient => AnchoredBy::LightClient,
            Anchoring::Rpc(_) => AnchoredBy::Rpc,
            Anchoring::None => AnchoredBy::None,
        }
    }

    pub(crate) async fn request(
        self: &Arc<Self>,
        method: &str,
        params: Value,
    ) -> Result<Value, Error> {
        // Spawning off a tokio runtime panics, and would with the client's locks held.
        if tokio::runtime::Handle::try_current().is_err() {
            return Err(Error::rpc(
                -32603,
                "willow-rpc runs on a tokio runtime, and none is running here",
            ));
        }
        let raw = params.clone();
        let list = match params {
            Value::Array(a) => a,
            _ => vec![],
        };
        let call = Call {
            method: method.to_string(),
            params: list,
            raw,
            started: Instant::now(),
            st: Arc::new(Mutex::new(CallState {
                status: Status::Verified,
                block: None,
                level: None,
                flags: vec![],
                unproven: BTreeSet::new(),
                proven_error: false,
                stand_in: None,
            })),
        };
        let checked = self.chain_checked();
        let r: Result<Value, Error> = async {
            let value = match self.dispatch(&call).await {
                Ok(v) => v,
                Err(Fail::Fallback(v)) => v,
                Err(Fail::Err(e)) => return Err(e),
            };
            checked.await?;
            Ok(value)
        }
        .await;
        match r {
            Ok(value) => {
                {
                    let mut s = call.st.lock().unwrap();
                    if value.is_null() && s.status == Status::Verified && s.block.is_none() {
                        s.status = Status::Absent;
                    }
                }
                self.emit(&call);
                Ok(value)
            }
            Err(e) => {
                {
                    let mut s = call.st.lock().unwrap();
                    match &e {
                        Error::Verification(d) | Error::Unverifiable(d) => {
                            s.status = Status::Failed;
                            s.flags.insert(
                                0,
                                Flag {
                                    kind: d.kind.clone(),
                                    detail: d.detail.clone(),
                                },
                            );
                        }
                        Error::Rpc { .. } => {
                            if s.status == Status::Verified && !s.proven_error {
                                s.status = Status::Refused;
                            }
                        }
                    }
                }
                self.emit(&call);
                Err(e)
            }
        }
    }

    fn emit(&self, call: &Call) {
        let Some(f) = &self.cfg.on_verification else {
            return;
        };
        let s = call.st.lock().unwrap();
        let e = Event {
            method: call.method.clone(),
            status: s.status,
            chain_id: self.chain_id,
            block: s.block.clone(),
            level: s.level,
            anchored_by: self.anchored_by(),
            kind: s.flags.first().map(|f| f.kind.clone()),
            detail: s.flags.first().and_then(|f| f.detail.clone()),
            flags: s.flags.clone(),
            unproven: s.unproven.iter().cloned().collect(),
            elapsed: call.started.elapsed(),
        };
        drop(s);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(&e)));
    }

    /// One eth_chainId at start: a wrong endpoint fails every request.
    fn chain_checked(self: &Arc<Self>) -> ChainCheck {
        let mut g = self.chain_check.lock().unwrap();
        // A check whose task ended without settling (its runtime ended) is asked again.
        if let Some(c) = &*g {
            if !c.task.is_finished() || c.settled.load(Ordering::SeqCst) {
                return c.fut.clone();
            }
        }
        let reply = self.batcher.call(&self.url, "eth_chainId", json!([]));
        let me = self.clone();
        let settled = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let done = settled.clone();
        // Spawned, so a failure that isn't a verdict is let go at once, waited for or not.
        let task = tokio::spawn(async move {
            let r: Result<(), Error> = async {
                let r = reply.await?;
                if let Some(e) = r.error() {
                    return Err(rpc_error(e));
                }
                if to_number(r.result()) != Some(me.chain_id) {
                    return Err(Error::verification(
                        "wrong-chain",
                        format!(
                            "the endpoint serves chain {}, not {}",
                            match r.result() {
                                Value::String(s) => s.clone(),
                                v => v.to_string(),
                            },
                            me.chain_id
                        ),
                        None,
                    ));
                }
                Ok(())
            }
            .await;
            if let Err(e) = &r {
                if !matches!(e, Error::Verification(_)) {
                    let mut g = me.chain_check.lock().unwrap();
                    if g.as_ref().is_some_and(|c| Arc::ptr_eq(&c.settled, &done)) {
                        *g = None;
                    }
                }
            }
            done.store(true, Ordering::SeqCst);
            r
        });
        let abort = task.abort_handle();
        let fut = async move {
            task.await.unwrap_or_else(|e| {
                Err(Error::rpc(
                    -32603,
                    format!("the chain check did not run: {e}"),
                ))
            })
        }
        .boxed()
        .shared();
        *g = Some(ChainCheckSlot {
            fut: fut.clone(),
            task: abort,
            settled,
        });
        fut
    }

    /// Why this release doesn't verify `call`, if it's one of the methods it holds back.
    fn held_back(&self, call: &Call) -> Option<String> {
        // Tests run the full verifiers too (the repo has them).
        #[cfg(test)]
        let held = HELD && !self.cfg.unheld;
        #[cfg(not(test))]
        let held = HELD;
        let m = call.method.as_str();
        let what = match m {
            "eth_gasPrice" | "eth_maxPriorityFeePerGas" | "eth_blobBaseFee" | "eth_feeHistory" => {
                "its fee formula"
            }
            "eth_estimateGas" => "its gas estimate",
            "eth_call" if self.chain_id == 4663 => "Robinhood Chain's calls",
            _ => return None,
        };
        held.then(|| {
            format!(
                "{m} is not verified by this release of willow-rpc: {what} is held back from crates.io \
                 for now (the verify page at https://api.willow.tech/verify/ verifies it)"
            )
        })
    }

    async fn dispatch(self: &Arc<Self>, call: &Call) -> R<Value> {
        let local = |v: Value| -> R<Value> {
            call.set_status(Status::Local);
            Ok(v)
        };
        if let Some(why) = self.held_back(call) {
            return self
                .policy(call, "no-verifier", why, self.standard(call))
                .await;
        }
        match call.method.as_str() {
            "eth_chainId" => local(json!(quantity(self.chain_id))),
            "net_version" => local(json!(self.chain_id.to_string())),
            "net_listening" => local(json!(true)),
            "net_peerCount" => local(json!("0x0")),
            "web3_clientVersion" => local(json!(format!("willow-rpc/{VERSION}"))),
            "eth_syncing" => local(json!(false)),
            "eth_accounts" => local(json!([])),
            "eth_blockNumber" => {
                let h = self.head(call, true).await?;
                Ok(json!(quantity(h.number.expect("a head has a number"))))
            }
            "eth_getBlockByNumber" => {
                let at = self.resolve(call, call.p(0)).await?;
                self.get_block(call, at, call.p(1) == &json!(true)).await
            }
            "eth_getBlockByHash" => {
                let h = self.hash_param(call.p(0))?;
                self.get_block(call, Resolved::hash(h), call.p(1) == &json!(true))
                    .await
            }
            "eth_getBlockTransactionCountByNumber" => {
                let at = self.resolve(call, call.p(0)).await?;
                self.tx_count(call, at).await
            }
            "eth_getBlockTransactionCountByHash" => {
                let h = self.hash_param(call.p(0))?;
                self.tx_count(call, Resolved::hash(h)).await
            }
            "eth_getBalance" | "eth_getTransactionCount" | "eth_getCode" | "eth_getStorageAt" => {
                self.state(call).await
            }
            "eth_getProof" => self.get_proof(call).await,
            "eth_call" => self.evm(call, "call").await,
            "eth_estimateGas" => self.evm(call, "estimate").await,
            "eth_gasPrice" | "eth_maxPriorityFeePerGas" | "eth_blobBaseFee" => {
                self.fees(call).await
            }
            "eth_feeHistory" => self.fee_history(call).await,
            "eth_sendRawTransaction" => self.send(call).await,
            "eth_getTransactionByHash" => {
                let h = self.hash_param(call.p(0))?;
                self.item(call, "transaction", &h, true).await
            }
            "eth_getTransactionReceipt" => {
                let h = self.hash_param(call.p(0))?;
                self.item(call, "receipt", &h, true).await
            }
            "eth_getTransactionByBlockHashAndIndex" => {
                let h = self.hash_param(call.p(0))?;
                let i = self.index_param(call.p(1))?;
                self.item(call, "transaction", &format!("{h}:{i}"), false)
                    .await
            }
            "eth_getTransactionByBlockNumberAndIndex" => {
                let at = self.resolve(call, call.p(0)).await?;
                let i = self.index_param(call.p(1))?;
                self.item(call, "transaction", &format!("{}:{i}", target(&at)), false)
                    .await
            }
            "eth_getBlockReceipts" => {
                let at = self.resolve(call, call.p(0)).await?;
                self.item(call, "blockReceipts", &target(&at), false).await
            }
            "eth_getLogs" => self
                .get_logs(call, call.p(0).clone())
                .await
                .map(Value::Array),
            m if TX_TRACES.contains(&m) || BLOCK_TRACES.contains(&m) => self.trace(call).await,
            "trace_filter" => self.trace_filter(call).await,
            "debug_traceCall" | "trace_call" => self.call_trace(call).await,
            "eth_newFilter"
            | "eth_newBlockFilter"
            | "eth_getFilterChanges"
            | "eth_getFilterLogs"
            | "eth_uninstallFilter" => self.filter(call).await,
            _ => self.forward(call).await,
        }
    }

    // ---------- errors and the unverifiable policy ----------

    /// A value that can't be verified: raise, or (flag) return the server's standard response,
    /// marked.
    async fn policy<F>(&self, call: &Call, kind: &str, detail: String, fallback: F) -> R<Value>
    where
        F: std::future::Future<Output = R<Value>>,
    {
        // Under `flag`, the server's value, when there is a server request to ask it of.
        if !self.flag_mode || call.server_request().is_none() {
            return Err(Error::unverifiable(kind, detail, call.m()).into());
        }
        let v = fallback.await?;
        call.flag(kind, Some(detail));
        Err(Fail::Fallback(v))
    }

    fn raw_params(call: &Call) -> Value {
        if call.raw.is_null() {
            json!([])
        } else {
            call.raw.clone()
        }
    }

    /// The app's own request, sent as it is (for a log filter, its eth_getLogs).
    async fn standard(&self, call: &Call) -> R<Value> {
        let Some((method, params)) = call.server_request() else {
            return Err(
                Error::rpc(-32603, format!("{} has no server request", call.method)).into(),
            );
        };
        let r = self.batcher.call(&self.url, &method, params).await?;
        if let Some(e) = r.error() {
            return Err(rpc_error(e).into());
        }
        Ok(r.result().clone())
    }

    /// A method with no proof: its errors pass through; a value is unverifiable.
    async fn forward(&self, call: &Call) -> R<Value> {
        let r = self
            .batcher
            .call(&self.url, &call.method, Self::raw_params(call))
            .await?;
        if let Some(e) = r.error() {
            call.set_status(Status::Refused);
            return Err(rpc_error(e).into());
        }
        let v = r.result().clone();
        self.policy(
            call,
            "method",
            format!("{} has no proof", call.method),
            async move { Ok(v) },
        )
        .await
    }

    /// A proof method's refusal: not served for this key or path (-32601) is unverifiable; the
    /// rest pass through.
    async fn refused<T>(&self, call: &Call, e: &Value) -> R<T> {
        if e.get("code").and_then(Value::as_i64) == Some(-32601)
            || e.get("code").and_then(Value::as_f64) == Some(-32601.0)
        {
            let message = match &e["message"] {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                v => v.to_string(),
            };
            return match self
                .policy(call, "not-on-plan", message, self.standard(call))
                .await
            {
                Err(f) => Err(f),
                Ok(_) => unreachable!("policy never returns a value"),
            };
        }
        call.set_status(Status::Refused);
        Err(rpc_error(e).into())
    }

    // ---------- blocks and anchors ----------

    fn hash_param(&self, x: &Value) -> R<String> {
        if !is_hash(x) {
            return Err(Error::rpc(
                -32602,
                format!("expected a 32-byte hash, not {}", shown(x, 80)),
            )
            .into());
        }
        Ok(x.as_str().unwrap().to_ascii_lowercase())
    }

    fn index_param(&self, x: &Value) -> R<String> {
        to_number(x).map(quantity).ok_or_else(|| {
            Error::rpc(
                -32602,
                format!("expected a transaction index, not {}", shown(x, 80)),
            )
            .into()
        })
    }

    /// The chain's newest block: anchored, checked with the trusted node, or the
    /// server's (no anchoring); under `flag`, the server's number when it can't be anchored.
    async fn head(self: &Arc<Self>, call: &Call, force: bool) -> R<Resolved> {
        match &self.cfg.anchoring {
            Anchoring::LightClient => {}
            Anchoring::Rpc(u) => return self.trusted_tag(call, u.clone(), "latest").await,
            Anchoring::None => return self.server_number(call).await,
        }
        match self
            .anchors
            .head(self.chain_id, "latest", force, call.m())
            .await
        {
            Ok(h) => {
                call.anchored(&h);
                Ok(Resolved {
                    number: Some(h.number),
                    hash: Some(h.hash),
                })
            }
            Err(e) => {
                if !(matches!(e, Error::Unverifiable(_)) && self.flag_mode) {
                    return Err(e.into());
                }
                let r = self.server_number(call).await?;
                let d = e.data().expect("unverifiable");
                call.flag(&d.kind, d.detail.clone());
                Ok(r)
            }
        }
    }

    async fn server_number(&self, _call: &Call) -> R<Resolved> {
        let r = self
            .batcher
            .call(&self.url, "eth_blockNumber", json!([]))
            .await?;
        if let Some(e) = r.error() {
            return Err(rpc_error(e).into());
        }
        let n = to_number(r.result())
            .ok_or_else(|| Error::rpc(-32603, "eth_blockNumber returned no number"))?;
        Ok(Resolved::num(n))
    }

    /// A tag's block on the node the user trusts ([`Anchoring::Rpc`]).
    async fn trusted_tag(&self, call: &Call, url: String, tag: &str) -> R<Resolved> {
        let r = self
            .batcher
            .call(&url, "eth_getBlockByNumber", json!([tag, false]))
            .await
            .map_err(|e| {
                Error::unverifiable(
                    "anchor-refused",
                    format!("the trusted node: {}", e.message()),
                    call.m(),
                )
            })?;
        if let Some(e) = r.error() {
            return Err(Error::unverifiable(
                "anchor-refused",
                format!("the trusted node: {}", rpc_error(e).message()),
                call.m(),
            )
            .into());
        }
        let n = to_number(&r.result()["number"]);
        let h = r.result()["hash"]
            .as_str()
            .filter(|h| crate::hex::is_hash_str(h));
        match (n, h) {
            (Some(n), Some(h)) => {
                let h = h.to_ascii_lowercase();
                self.mark_block(call, n, &h);
                Ok(Resolved {
                    number: Some(n),
                    hash: Some(h),
                })
            }
            _ => Err(Error::unverifiable(
                "anchor-refused",
                format!("the trusted node named no \"{tag}\" block"),
                call.m(),
            )
            .into()),
        }
    }

    fn mark_block(&self, call: &Call, number: u64, hash: &str) {
        let mut s = call.st.lock().unwrap();
        if s.block.as_ref().is_none_or(|b| number >= b.number) {
            s.block = Some(BlockRef {
                number,
                hash: hash.to_string(),
            });
        }
    }

    /// A block parameter, as the proofs must name it.
    fn resolve<'a>(
        self: &'a Arc<Self>,
        call: &'a Call,
        b: &'a Value,
    ) -> BoxFuture<'a, R<Resolved>> {
        async move {
            if is_tag(b) {
                return self.head(call, false).await;
            }
            if b == "earliest" {
                return Ok(Resolved::num(0));
            }
            if b == "finalized" || b == "safe" {
                let tag = b.as_str().unwrap();
                match &self.cfg.anchoring {
                    Anchoring::Rpc(u) => return self.trusted_tag(call, u.clone(), tag).await,
                    Anchoring::None => return self.server_tag(call, tag, false).await,
                    Anchoring::LightClient => {}
                }
                if self.chain_id == 1 {
                    if tag == "safe" {
                        return Err(Error::rpc(
                            -32000,
                            "not found in this store: the \"safe\" block tag",
                        )
                        .into());
                    }
                    let f = self.anchors.head(1, "finalized", false, call.m()).await?;
                    call.anchored(&f);
                    return Ok(Resolved {
                        number: Some(f.number),
                        hash: Some(f.hash),
                    });
                }
                // An L2's tags: the block's contents verify, the tag's meaning does not.
                if !self.flag_mode {
                    return Err(Error::unverifiable(
                        "tag",
                        format!("\"{tag}\" on chain {} can't be verified", self.chain_id),
                        call.m(),
                    )
                    .into());
                }
                return self.server_tag(call, tag, true).await;
            }
            if is_hash(b) {
                return Ok(Resolved::hash(b.as_str().unwrap().to_ascii_lowercase()));
            }
            if let Value::Object(o) = b {
                if o.contains_key("blockHash") && !o.contains_key("blockNumber") {
                    return Ok(Resolved::hash(self.hash_param(&o["blockHash"])?));
                }
                if o.contains_key("blockNumber") && !o.contains_key("blockHash") {
                    return self.resolve(call, &o["blockNumber"]).await;
                }
            }
            let n = match b {
                Value::String(s) => str_number(s),
                _ => None,
            };
            let Some(n) = n else {
                return Err(
                    Error::rpc(-32602, format!("bad block number {}", shown(b, 80))).into(),
                );
            };
            if self.cfg.anchoring != Anchoring::LightClient {
                return Ok(Resolved::num(n));
            }
            // A number already anchored goes out by its hash, so no other block can take its place.
            Ok(Resolved {
                number: Some(n),
                hash: self.anchors.hash_of(self.chain_id, n),
            })
        }
        .boxed()
    }

    /// A tag as the server resolves it (`eth_getBlockByNumber [tag, false]`).
    async fn server_tag(&self, call: &Call, tag: &str, flag: bool) -> R<Resolved> {
        let r = self
            .batcher
            .call(&self.url, "eth_getBlockByNumber", json!([tag, false]))
            .await?;
        if let Some(e) = r.error() {
            return Err(rpc_error(e).into());
        }
        let n = to_number(&r.result()["number"]).ok_or_else(|| {
            Error::rpc(
                -32000,
                format!("not found in this store: the \"{tag}\" block tag"),
            )
        })?;
        if flag {
            call.flag(
                "tag",
                Some(format!("\"{tag}\" resolved by the server to block {n}")),
            );
        }
        Ok(Resolved::num(n))
    }

    /// Anchors a verified block; under `flag`, a block that can't be anchored is marked and kept.
    async fn anchor(
        self: &Arc<Self>,
        call: &Call,
        number: u64,
        hash: &str,
        prefetched: Option<Shared<BoxFuture<'static, Result<Reply, Error>>>>,
    ) -> R<Option<Anchored>> {
        let hash = hash.to_ascii_lowercase();
        match &self.cfg.anchoring {
            Anchoring::LightClient => {}
            Anchoring::Rpc(u) => {
                let r = self
                    .batcher
                    .call(u, "eth_getBlockByNumber", json!([quantity(number), false]))
                    .await
                    .map_err(|e| {
                        Error::unverifiable(
                            "anchor-refused",
                            format!("the trusted node: {}", e.message()),
                            call.m(),
                        )
                    })?;
                if let Some(e) = r.error() {
                    return Err(Error::unverifiable(
                        "anchor-refused",
                        format!("the trusted node: {}", rpc_error(e).message()),
                        call.m(),
                    )
                    .into());
                }
                // A node that hasn't got the block can't vouch for it either way.
                let Some(theirs) = r.result()["hash"].as_str().map(str::to_ascii_lowercase) else {
                    return Err(Error::unverifiable(
                        "anchor-refused",
                        format!("the trusted node has no block {number}"),
                        call.m(),
                    )
                    .into());
                };
                if theirs != hash {
                    return Err(Error::verification(
                        "anchor-mismatch",
                        format!("block {number} of chain {} is {theirs} on the trusted node, not {hash}", self.chain_id),
                        call.m(),
                    )
                    .into());
                }
                self.mark_block(call, number, &hash);
                return Ok(Some(Anchored {
                    chain_id: self.chain_id,
                    number,
                    hash,
                    level: None,
                    path: "rpc".into(),
                    time: None,
                    expires: 0,
                }));
            }
            Anchoring::None => {
                self.mark_block(call, number, &hash);
                return Ok(None);
            }
        }
        match self
            .anchors
            .block(self.chain_id, number, &hash, prefetched, call.m())
            .await
        {
            Ok(a) => {
                call.anchored(&a);
                Ok(Some(a))
            }
            Err(e) => {
                if !(matches!(e, Error::Unverifiable(_)) && self.flag_mode) {
                    return Err(e.into());
                }
                let d = e.data().expect("unverifiable");
                call.flag(&d.kind, d.detail.clone());
                self.mark_block_if_none(call, number, &hash);
                Ok(None)
            }
        }
    }

    fn mark_block_if_none(&self, call: &Call, number: u64, hash: &str) {
        let mut s = call.st.lock().unwrap();
        if s.block.is_none() {
            s.block = Some(BlockRef {
                number,
                hash: hash.to_string(),
            });
        }
    }

    /// The anchor of block `n` asked for with its proof, when it is neither the head nor anchored.
    fn prefetch(
        &self,
        call: &Call,
        at: &Resolved,
    ) -> Option<Shared<BoxFuture<'static, Result<Reply, Error>>>> {
        if self.cfg.anchoring != Anchoring::LightClient {
            return None;
        }
        let n = at.number?;
        if at.hash.is_some() || self.anchors.known(self.chain_id, n) {
            return None;
        }
        Some(
            self.anchors
                .body(self.chain_id, json!(quantity(n)), Some(call.method.clone())),
        )
    }

    async fn item_request(&self, call: &Call, kind: &str, target: &str) -> R<(String, Value)> {
        let (c, k, t) = (self.chain_id, kind.to_string(), target.to_string());
        let r = blocking(move || Backend::item_request(c, &k, &t))
            .await
            .map_err(|e| e.with_method(call.m()))?;
        if r["ok"] == true {
            return Ok((
                r["method"].as_str().unwrap_or("").to_string(),
                r["params"].clone(),
            ));
        }
        let message = r["message"].as_str().unwrap_or("").to_string();
        if r["kind"] == "invalid-request" {
            return Err(Error::rpc(-32602, message).into());
        }
        Err(Error::verification("internal", message, call.m()).into())
    }

    async fn verify_item(
        &self,
        call: &Call,
        kind: &str,
        target: &str,
        text: Arc<str>,
        full: bool,
    ) -> R<Value> {
        let (c, k, t) = (self.chain_id, kind.to_string(), target.to_string());
        let r = blocking(move || Backend::item_verify(c, &k, &t, &text, full))
            .await
            .map_err(|e| e.with_method(call.m()))?;
        if r["ok"] != true {
            let message = r["message"].as_str().unwrap_or("").to_string();
            let fk = r["kind"].as_str().unwrap_or("");
            if fk == "internal" || fk == "invalid-request" {
                return Err(Error::verification("internal", message, call.m()).into());
            }
            let kind = match r["fault"].as_str() {
                Some(f) => f.to_string(),
                None if fk == "malformed-input" => "malformed".into(),
                None => "rejected".into(),
            };
            return Err(Error::verification(&kind, message, call.m()).into());
        }
        call.leaves(&r["report"]["unproven"]);
        Ok(r)
    }

    /// A proof by an items kind, requested, fetched and verified (not anchored); a refusal comes
    /// back as `Err(error object)`.
    async fn fetch_item(&self, call: &Call, kind: &str, target: &str) -> R<Result<Value, Value>> {
        let (method, params) = self.item_request(call, kind, target).await?;
        let reply = self.batcher.call(&self.url, &method, params).await?;
        if let Some(e) = reply.error() {
            return Ok(Err(e.clone()));
        }
        Ok(Ok(self
            .verify_item(call, kind, target, reply.text.clone(), false)
            .await?))
    }

    async fn prove_item(&self, call: &Call, kind: &str, target: &str) -> R<Value> {
        match self.fetch_item(call, kind, target).await? {
            Ok(v) => Ok(v),
            Err(e) => self.refused(call, &e).await,
        }
    }

    async fn block_reply(
        &self,
        call: &Call,
        at: &Resolved,
    ) -> R<Shared<BoxFuture<'static, Result<Reply, Error>>>> {
        let (method, params) = self.item_request(call, "block", &target(at)).await?;
        Ok(self.batcher.call(&self.url, &method, params))
    }

    /// Block `at`, verified and anchored (cached by hash), its parent's hash anchored with it.
    /// `reply`: its proof, already asked for; `prefetch: false`: no anchor asked for ahead.
    async fn block(
        self: &Arc<Self>,
        call: &Call,
        at: &Resolved,
        full: bool,
        reply: Option<Shared<BoxFuture<'static, Result<Reply, Error>>>>,
        prefetch: bool,
    ) -> R<Option<BlockEntry>> {
        let cached = at
            .hash
            .as_ref()
            .and_then(|h| self.blocks.lock().unwrap().get(h));
        let mut entry = match cached {
            Some(e) => {
                let a = self.anchor(call, e.number, &e.hash, None).await?;
                call.leaves_list(&e.unproven);
                if let Some(a) = a {
                    self.anchors
                        .parent(&a, e.proven["parentHash"].as_str().unwrap_or(""));
                }
                e
            }
            None => {
                let (prefetched, replied) = match reply {
                    Some(r) => (
                        if prefetch {
                            self.prefetch(call, at)
                        } else {
                            None
                        },
                        r,
                    ),
                    None => {
                        let (method, params) =
                            self.item_request(call, "block", &target(at)).await?;
                        // The anchor and the proof asked for in one turn, so they share a batch.
                        let prefetched = if prefetch {
                            self.prefetch(call, at)
                        } else {
                            None
                        };
                        (prefetched, self.batcher.call(&self.url, &method, params))
                    }
                };
                let reply = replied.await?;
                if let Some(e) = reply.error() {
                    return self.refused(call, e).await;
                }
                // A number past the server's newest block: null, as the standard method's.
                if reply.result().is_null() && at.hash.is_none() {
                    call.set_status(Status::Absent);
                    return Ok(None);
                }
                let v = self
                    .verify_item(call, "block", &target(at), reply.text.clone(), full)
                    .await?;
                let b = &v["report"]["block"];
                let entry = BlockEntry {
                    text: reply.text.clone(),
                    proven: v["proven"].clone(),
                    full: v.get("rendered").map(|r| r["proven"].clone()),
                    unproven: strings(&v["report"]["unproven"]),
                    full_unproven: v.get("rendered").map(|r| strings(&r["unproven"])),
                    number: to_number(&b["number"]).ok_or_else(|| {
                        Error::verification("malformed", "the proven block has no number", call.m())
                    })?,
                    hash: b["hash"].as_str().unwrap_or("").to_ascii_lowercase(),
                };
                let a = self
                    .anchor(call, entry.number, &entry.hash, prefetched)
                    .await?;
                if let Some(a) = a {
                    let ph = b["parentHash"]
                        .as_str()
                        .or_else(|| entry.proven["parentHash"].as_str())
                        .unwrap_or("");
                    self.anchors.parent(&a, ph);
                    self.blocks
                        .lock()
                        .unwrap()
                        .set(&entry.hash, entry.clone(), reply.text.len());
                }
                entry
            }
        };
        if full && entry.full.is_none() {
            let v = self
                .verify_item(call, "block", &entry.hash.clone(), entry.text.clone(), true)
                .await?;
            entry.full = v.get("rendered").map(|r| r["proven"].clone());
            entry.full_unproven = v.get("rendered").map(|r| strings(&r["unproven"]));
            let mut blocks = self.blocks.lock().unwrap();
            if blocks.has(&entry.hash) {
                blocks.set(&entry.hash, entry.clone(), entry.text.len());
            }
        }
        if full {
            if let Some(u) = &entry.full_unproven {
                call.leaves_list(u);
            }
        }
        Ok(Some(entry))
    }

    async fn get_block(self: &Arc<Self>, call: &Call, at: Resolved, full: bool) -> R<Value> {
        match self.block(call, &at, full, None, true).await? {
            None => Ok(Value::Null),
            Some(e) => Ok(if full {
                e.full.unwrap_or(Value::Null)
            } else {
                e.proven
            }),
        }
    }

    async fn tx_count(self: &Arc<Self>, call: &Call, at: Resolved) -> R<Value> {
        let Some(e) = self.block(call, &at, false, None, true).await? else {
            return Ok(Value::Null);
        };
        let txs = e.proven["transactions"].as_array().ok_or_else(|| {
            Error::verification(
                "malformed",
                "the proven block has no transaction list",
                call.m(),
            )
        })?;
        Ok(json!(quantity(txs.len() as u64)))
    }

    // ---------- state ----------

    /// Reads of one account at one block close together become one willow_getStateProof.
    fn state_read(
        self: &Arc<Self>,
        call: &Call,
        address: &str,
        at: &Resolved,
        key: Option<&str>,
        code: bool,
    ) -> oneshot::Receiver<Result<StateRead, Error>> {
        let k = format!(
            "{}@{}",
            address.to_ascii_lowercase(),
            at.hash
                .clone()
                .unwrap_or_else(|| format!("n{}", at.number.unwrap_or(0)))
        );
        let (tx, rx) = oneshot::channel();
        let mut q = self.state_queue.lock().unwrap();
        let g = q.entry(k.clone()).or_insert_with(|| StateGroup {
            address: address.to_string(),
            at: at.clone(),
            keys: vec![],
            code: false,
            method: call.method.clone(),
            waiters: vec![],
        });
        if let Some(key) = key {
            if let Some(norm) = slot_key(key) {
                if !g.keys.iter().any(|(n, _)| *n == norm) {
                    g.keys.push((norm, key.to_string()));
                }
            }
        }
        g.code |= code;
        g.waiters.push(tx);
        drop(q);
        // Every read schedules a flush and the first takes the group: a group whose flush ended
        // with its runtime is still sent by the next read's.
        let me = self.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(1)).await;
            me.flush_state(&k).await;
        });
        rx
    }

    async fn flush_state(self: &Arc<Self>, k: &str) {
        let Some(g) = self.state_queue.lock().unwrap().remove(k) else {
            return;
        };
        let keys: Vec<String> = g.keys.iter().map(|(_, orig)| orig.clone()).collect();
        let r = self
            .prove_state(&g.method, &g.address, &g.at, &keys, g.code)
            .await;
        for w in g.waiters {
            let _ = w.send(r.clone());
        }
    }

    async fn prove_state(
        &self,
        method: &str,
        address: &str,
        at: &Resolved,
        keys: &[String],
        code: bool,
    ) -> Result<StateRead, Error> {
        let target = json!([address, keys, block_param(at), {"code": code}]).to_string();
        let c = self.chain_id;
        let t = target.clone();
        let req = blocking(move || Backend::state_request(c, "state", &t)).await?;
        if req["ok"] != true {
            let message = req["message"].as_str().unwrap_or("").to_string();
            if req["kind"] == "invalid-request" {
                return Err(Error::rpc(-32602, message));
            }
            return Err(Error::verification(
                req["kind"].as_str().unwrap_or("internal"),
                message,
                Some(method),
            ));
        }
        let reply = self
            .batcher
            .call(
                &self.url,
                req["method"].as_str().unwrap_or(""),
                req["params"].clone(),
            )
            .await?;
        if let Some(e) = reply.error() {
            return Ok(StateRead::Error(e.clone()));
        }
        let text = reply.text.clone();
        let t = target.clone();
        let v = blocking(move || Backend::state_verify(c, "state", &t, &text)).await?;
        if v["ok"] != true {
            let kind = v["kind"].as_str().unwrap_or("internal");
            let kind = if kind == "invalid-request" {
                "internal"
            } else {
                kind
            };
            return Err(Error::verification(
                kind,
                v["message"].as_str().unwrap_or(""),
                Some(method),
            ));
        }
        let proven = v["proven"].clone();
        if let Some(code) = proven["eth_getCode"].as_str() {
            let h = proven["eth_getProof"]["codeHash"]
                .as_str()
                .unwrap_or("")
                .to_ascii_lowercase();
            self.codes
                .lock()
                .unwrap()
                .set(&h, code.to_string(), code.len());
            self.code_of
                .lock()
                .unwrap()
                .set(&address.to_ascii_lowercase(), h, 0);
        }
        Ok(StateRead::Proven {
            proven,
            report: v["report"].clone(),
            keys: keys.iter().filter_map(|k| slot_key(k)).collect(),
        })
    }

    /// A state read's outcome for one app request: refused, or verified and anchored.
    async fn state_of(
        self: &Arc<Self>,
        call: &Call,
        read: Result<StateRead, Error>,
    ) -> R<(Value, Vec<String>)> {
        // A merged proof's failure is each request's, by its own method.
        let r = read.map_err(|e| e.for_method(&call.method))?;
        match r {
            StateRead::Error(e) => self.refused(call, &e).await,
            StateRead::Proven {
                proven,
                report,
                keys,
            } => {
                let b = &report["block"];
                let n = to_number(&b["number"]).ok_or_else(|| {
                    Error::verification("malformed", "the proof names no block", call.m())
                })?;
                self.anchor(call, n, b["hash"].as_str().unwrap_or(""), None)
                    .await?;
                Ok((proven, keys))
            }
        }
    }

    async fn read(
        self: &Arc<Self>,
        call: &Call,
        address: &str,
        at: &Resolved,
        key: Option<&str>,
        code: bool,
    ) -> R<(Value, Vec<String>)> {
        let rx = self.state_read(call, address, at, key, code);
        // A flush that ended with another runtime: not a verdict on the server.
        let r = rx.await.unwrap_or_else(|_| {
            Err(Error::rpc(
                -32603,
                "the state read was dropped before a response",
            ))
        });
        self.state_of(call, r).await
    }

    async fn state(self: &Arc<Self>, call: &Call) -> R<Value> {
        let address = call.p(0).clone();
        if !is_address(&address) {
            return Err(Error::rpc(
                -32602,
                format!("expected an address, not {}", shown(&address, 80)),
            )
            .into());
        }
        let address = address.as_str().unwrap().to_string();
        let storage = call.method == "eth_getStorageAt";
        let block_arg = if storage { call.p(2) } else { call.p(1) };
        let slot =
            if storage {
                let k = call.p(1);
                Some(k.as_str().and_then(slot_key).ok_or_else(|| {
                    Error::rpc(-32602, format!("bad storage slot {}", shown(k, 80)))
                })?)
            } else {
                None
            };
        let at = self.resolve(call, block_arg).await?;
        match call.method.as_str() {
            "eth_getBalance" => {
                Ok(self.read(call, &address, &at, None, false).await?.0["eth_getBalance"].clone())
            }
            "eth_getTransactionCount" => {
                let nonce = self.read(call, &address, &at, None, false).await?.0
                    ["eth_getTransactionCount"]
                    .clone();
                if block_arg != "pending" {
                    return Ok(nonce);
                }
                // The pending nonce is never the server's: the proven one, or past this client's own sends.
                let own = self
                    .sent
                    .lock()
                    .unwrap()
                    .get(&address.to_ascii_lowercase())
                    .copied();
                let proven = nonce
                    .as_str()
                    .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok());
                match (own, proven) {
                    (Some(o), Some(p)) if o + 1 > p => Ok(json!(quantity_u128(o + 1))),
                    _ => Ok(nonce),
                }
            }
            "eth_getCode" => {
                let lower = address.to_ascii_lowercase();
                let hash = self.code_of.lock().unwrap().get(&lower);
                let cached = hash.and_then(|h| self.codes.lock().unwrap().get(&h));
                let (p, _) = self
                    .read(call, &address, &at, None, cached.is_none())
                    .await?;
                let code_hash = p["eth_getProof"]["codeHash"]
                    .as_str()
                    .unwrap_or("")
                    .to_ascii_lowercase();
                if code_hash == EMPTY_CODE_HASH {
                    return Ok(json!("0x"));
                }
                if let Some(c) = p["eth_getCode"].as_str().filter(|c| !c.is_empty()) {
                    return Ok(json!(c));
                }
                // Code by code hash, verified against it when it was sent.
                let kept = self.codes.lock().unwrap().get(&code_hash);
                if let Some(k) = kept {
                    if to_bytes(&k).is_some_and(|b| keccak_hex(&b) == code_hash) {
                        return Ok(json!(k));
                    }
                }
                let (again, _) = self.read(call, &address, &at, None, true).await?;
                match again["eth_getCode"].as_str().filter(|c| !c.is_empty()) {
                    Some(c) => Ok(json!(c)),
                    None => Err(Error::verification(
                        "code",
                        format!("the code of {address} was asked for and not proven"),
                        call.m(),
                    )
                    .into()),
                }
            }
            _ => {
                let key = call.p(1).as_str().unwrap_or("").to_string();
                let (proven, keys) = self.read(call, &address, &at, Some(&key), false).await?;
                let slot = slot.expect("eth_getStorageAt");
                let value = keys
                    .iter()
                    .position(|k| *k == slot)
                    .and_then(|i| proven["eth_getStorageAt"].get(i))
                    .filter(|v| v.is_string());
                value.cloned().ok_or_else(|| {
                    Error::verification("slot", format!("slot {key} is not in the proof"), call.m())
                        .into()
                })
            }
        }
    }

    async fn get_proof(self: &Arc<Self>, call: &Call) -> R<Value> {
        let address = call.p(0);
        let keys = call.p(1);
        let ok_keys = keys.as_array().is_some_and(|k| {
            k.iter().all(|k| {
                k.as_str().is_some_and(|s| {
                    s.starts_with("0x")
                        && s.len() <= 66
                        && s[2..].bytes().all(|b| b.is_ascii_hexdigit())
                })
            })
        });
        if !is_address(address) || !ok_keys {
            return Err(
                Error::rpc(-32602, "expected an address and a list of storage keys").into(),
            );
        }
        let address = address.as_str().unwrap().to_string();
        let keys: Vec<String> = keys
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap().to_string())
            .collect();
        let at = self.resolve(call, call.p(2)).await?;
        let r = self
            .prove_state(&call.method, &address, &at, &keys, false)
            .await;
        Ok(self.state_of(call, r).await?.0["eth_getProof"].clone())
    }

    // ---------- calls and estimates ----------

    async fn evm(self: &Arc<Self>, call: &Call, kind: &'static str) -> R<Value> {
        let tx = call.p(0);
        if !tx.is_object() {
            return Err(Error::rpc(-32602, "expected a call object").into());
        }
        let at = self.resolve(call, call.p(1)).await?;
        // The caller's state and block overrides go with the proof request and are laid over its
        // witness here: they are the caller's own, never proven. The verifier refuses a chain, a
        // call or overrides it doesn't run (`unsupported`): those are the server's response, by
        // the rules for methods without a proof.
        let mut rest: Vec<Value> = call.params.iter().skip(2).cloned().collect();
        while rest.last().is_some_and(empty_object) {
            rest.pop();
        }
        let mut params = vec![tx.clone(), block_param(&at)];
        params.extend(rest);
        let target = Value::Array(params).to_string();
        let r = self
            .proven_state(call, kind, &target, at.hash.as_deref())
            .await?;
        if r.get("forward").is_some() {
            return self.forward(call).await;
        }
        if let Some(e) = r.get("error") {
            return self.refused(call, e).await;
        }
        let v = &r["v"];
        let b = &v["report"]["block"];
        let n = to_number(&b["number"]).ok_or_else(|| {
            Error::verification("malformed", "the proof names no block", call.m())
        })?;
        self.anchor(call, n, b["hash"].as_str().unwrap_or(""), None)
            .await?;
        let out = &v["proven"];
        if let Some(result) = out.get("result") {
            return Ok(result.clone());
        }
        // A revert, halt or failed search, verified: the error the standard method sends.
        call.st.lock().unwrap().proven_error = true;
        let e = &out["error"];
        Err(Error::Rpc {
            code: e["code"].as_i64().unwrap_or(-32000),
            message: e["message"].as_str().unwrap_or("").to_string(),
            data: e.get("data").cloned(),
        }
        .into())
    }

    /// A state-module proof (`call`, `estimate`, `trace`) requested, fetched and verified (not
    /// anchored): `{"v": verified}`, `{"error": refusal}` or `{"forward": true}` (a request the
    /// verifier doesn't run). Results at a block named by hash are kept.
    async fn proven_state(
        self: &Arc<Self>,
        call: &Call,
        kind: &'static str,
        target: &str,
        hash: Option<&str>,
    ) -> R<Value> {
        let me = self.clone();
        let t = target.to_string();
        let method = call.method.clone();
        let run = move || -> BoxFuture<'static, Result<Value, Error>> {
            async move {
                let c = me.chain_id;
                let tt = t.clone();
                let req = blocking(move || Backend::state_request(c, kind, &tt))
                    .await
                    .map_err(|e| e.with_method(Some(&method)))?;
                if req["ok"] != true {
                    let message = req["message"].as_str().unwrap_or("").to_string();
                    return match req["kind"].as_str() {
                        Some("unsupported") => Ok(json!({"forward": true})),
                        Some("invalid-request") => Err(Error::rpc(-32602, message)),
                        k => Err(Error::verification(
                            k.unwrap_or("internal"),
                            message,
                            Some(&method),
                        )),
                    };
                }
                let reply = me
                    .batcher
                    .call(
                        &me.url,
                        req["method"].as_str().unwrap_or(""),
                        req["params"].clone(),
                    )
                    .await?;
                if let Some(e) = reply.error() {
                    return Ok(json!({"error": e}));
                }
                let text = reply.text.clone();
                let tt = t.clone();
                let v = blocking(move || Backend::state_verify(c, kind, &tt, &text))
                    .await
                    .map_err(|e| e.with_method(Some(&method)))?;
                if v["ok"] != true {
                    let k = v["kind"].as_str().unwrap_or("internal");
                    let k = if k == "invalid-request" {
                        "internal"
                    } else {
                        k
                    };
                    return Err(Error::verification(
                        k,
                        v["message"].as_str().unwrap_or(""),
                        Some(&method),
                    ));
                }
                Ok(json!({"v": v}))
            }
            .boxed()
        };
        let r = match hash {
            Some(h) => {
                let key = match kind {
                    "trace" => format!("trace:{target}"),
                    _ => format!("{kind}:{h}:{target}"),
                };
                self.once(key, run).await
            }
            None => run().await,
        };
        r.map_err(|e| Fail::Err(e.for_method(&call.method)))
    }

    /// `run` once per key, for results fixed by a block hash; a failure or a refusal is not kept.
    /// The run is spawned, so it goes on whoever waits for it (and on the runtime that started
    /// it); a panic in it is an error, and it leaves the key however it ends.
    pub(crate) async fn once(
        self: &Arc<Self>,
        key: String,
        run: impl FnOnce() -> BoxFuture<'static, Result<Value, Error>>,
    ) -> Result<Value, Error> {
        if let Some(v) = self.results.lock().unwrap().get(&key) {
            return Ok(v);
        }
        let (f, start) = {
            let mut memo = self.memo.lock().unwrap();
            match memo.get(&key) {
                Some((_, f)) => (f.clone(), None),
                None => {
                    let id = self.memo_ids.fetch_add(1, Ordering::Relaxed);
                    let (tx, rx) = oneshot::channel();
                    let f: Memo = async move {
                        // Ended with its runtime: not a verdict on the server.
                        rx.await.unwrap_or_else(|_| {
                            Err(Error::rpc(-32603, "the proof run ended before a response"))
                        })
                    }
                    .boxed()
                    .shared();
                    memo.insert(key.clone(), (id, f.clone()));
                    (f, Some((id, tx)))
                }
            }
        };
        if let Some((id, tx)) = start {
            let done = MemoDone {
                client: self.clone(),
                key: key.clone(),
                id,
            };
            let run = run();
            tokio::spawn(async move {
                let r = std::panic::AssertUnwindSafe(run)
                    .catch_unwind()
                    .await
                    .unwrap_or_else(|p| {
                        let why = p
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
                            .unwrap_or_else(|| "a proof run panicked".into());
                        Err(Error::verification("internal", why, None))
                    });
                if let Ok(v) = &r {
                    if v.get("error").is_none() {
                        done.client
                            .results
                            .lock()
                            .unwrap()
                            .set(&done.key, v.clone(), 1024);
                    }
                }
                // Out of flight before the response, as anchor fetches and batch entries are.
                drop(done);
                let _ = tx.send(r);
            });
        }
        f.await
    }

    // ---------- traces (willow_getTraceProof) ----------

    /// A trace method's response, from the client's own run of the block (trace-verify).
    async fn trace(self: &Arc<Self>, call: &Call) -> R<Value> {
        let Some(v) = self
            .proven_trace(call, &call.method, call.params.clone(), None)
            .await?
        else {
            return self.forward(call).await;
        };
        self.anchor_report(call, &v).await?;
        Ok(v["proven"]["result"].clone())
    }

    async fn anchor_report(self: &Arc<Self>, call: &Call, v: &Value) -> R<Option<Anchored>> {
        let b = &v["report"]["block"];
        let n = to_number(&b["number"]).ok_or_else(|| {
            Error::verification("malformed", "the proof names no block", call.m())
        })?;
        self.anchor(call, n, b["hash"].as_str().unwrap_or(""), None)
            .await
    }

    /// A call's trace (`debug_traceCall`, `trace_call`), from the client's own run of the call
    /// over the state the proof proves at the block the client resolved.
    async fn call_trace(self: &Arc<Self>, call: &Call) -> R<Value> {
        if !CALL_TRACED.contains(&self.chain_id) {
            return self.forward(call).await;
        }
        let mut p = call.params.clone();
        if !p.first().is_some_and(Value::is_object) {
            return Err(Error::rpc(-32602, "expected a call object").into());
        }
        let i = if call.method == "debug_traceCall" {
            1
        } else {
            2
        };
        let at = self.resolve(call, call.p(i)).await?;
        while p.len() <= i {
            p.push(Value::Null);
        }
        p[i] = block_param(&at);
        let Some(v) = self.proven_trace(call, &call.method, p, Some(at)).await? else {
            return self.forward(call).await;
        };
        self.anchor_report(call, &v).await?;
        Ok(v["proven"]["result"].clone())
    }

    /// `method(params)`'s trace proof, verified (not anchored); `None` when no proof covers it.
    async fn proven_trace(
        self: &Arc<Self>,
        call: &Call,
        method: &str,
        params: Vec<Value>,
        resolved: Option<Resolved>,
    ) -> R<Option<Value>> {
        if !TRACED.contains(&self.chain_id) {
            return Ok(None);
        }
        let mut p = params;
        let mut at = resolved;
        if BLOCK_TRACES.contains(&method) {
            let first = p.first().cloned().unwrap_or(Value::Null);
            let r = if method == "debug_traceBlockByHash" {
                Resolved::hash(self.hash_param(&first)?)
            } else {
                self.resolve(call, &first).await?
            };
            let v = if method == "debug_traceBlockByHash" {
                json!(r.hash.clone())
            } else {
                block_param(&r)
            };
            if p.is_empty() {
                p.push(v);
            } else {
                p[0] = v;
            }
            at = Some(r);
        } else if TX_TRACES.contains(&method) {
            let h = self.hash_param(p.first().unwrap_or(&Value::Null))?;
            if p.is_empty() {
                p.push(json!(h));
            } else {
                p[0] = json!(h);
            }
        }
        let target = json!([method, p]).to_string();
        let hash = at.as_ref().and_then(|a| a.hash.clone());
        let r = self
            .proven_state(call, "trace", &target, hash.as_deref())
            .await?;
        if r.get("forward").is_some() {
            return Ok(None);
        }
        if let Some(e) = r.get("error") {
            return self.refused(call, e).await;
        }
        Ok(Some(r["v"].clone()))
    }

    /// `trace_filter`, from each block's `trace_block` proof (one batch), filtered here as the
    /// server filters; the blocks linked by their proven parent hashes, so anchoring the newest
    /// anchors them all.
    async fn trace_filter(self: &Arc<Self>, call: &Call) -> R<Value> {
        if !TRACED.contains(&self.chain_id) {
            return self.forward(call).await;
        }
        let filter = call.p(0).clone();
        if !filter.is_object() {
            return Err(Error::rpc(-32602, "expected a trace filter object").into());
        }
        let from_arg = if filter["fromBlock"].is_null() {
            json!("earliest")
        } else {
            filter["fromBlock"].clone()
        };
        let to_arg = if filter["toBlock"].is_null() {
            json!("latest")
        } else {
            filter["toBlock"].clone()
        };
        let from = self.resolve(call, &from_arg).await?;
        let to = self.resolve(call, &to_arg).await?;
        let (Some(f), Some(t)) = (from.number, to.number) else {
            return Err(Error::rpc(-32602, "fromBlock and toBlock are numbers or tags").into());
        };
        if f > t {
            return Err(Error::rpc(
                -32602,
                "invalid parameters: fromBlock cannot be greater than toBlock",
            )
            .into());
        }
        if t - f >= TRACE_FILTER_BLOCKS {
            return Err(Error::rpc(
                -32602,
                format!("Block range too large; currently limited to {TRACE_FILTER_BLOCKS} blocks"),
            )
            .into());
        }
        let blocks = range_blocks(f, t, &from, &to);
        let top = blocks.len() - 1;
        let prefetched = self.prefetch(call, &blocks[top]);
        let verified = join_all(blocks.iter().map(|at| {
            let p = vec![match &at.hash {
                Some(h) => json!(h),
                None => json!(quantity(at.number.unwrap())),
            }];
            self.proven_trace(call, "trace_block", p, None)
        }))
        .await;
        let mut vs = Vec::new();
        for v in verified {
            match v? {
                Some(v) => vs.push(v),
                None => return self.forward(call).await,
            }
        }
        self.anchor_range(call, &vs, prefetched).await?;
        let results: Vec<Value> = vs.iter().map(|v| v["proven"]["result"].clone()).collect();
        Ok(Value::Array(trace_filtered(&filter, &results)))
    }

    /// Consecutive verified blocks (each report naming its block): when each one's parent hash is
    /// the block below it, anchoring the newest anchors them all; else each is anchored.
    async fn anchor_range(
        self: &Arc<Self>,
        call: &Call,
        vs: &[Value],
        prefetched: Option<Shared<BoxFuture<'static, Result<Reply, Error>>>>,
    ) -> R<()> {
        let named: Vec<(u64, String, Option<String>)> = vs
            .iter()
            .map(|v| {
                let b = &v["report"]["block"];
                Ok((
                    to_number(&b["number"]).ok_or_else(|| {
                        Error::verification("malformed", "the proof names no block", call.m())
                    })?,
                    b["hash"].as_str().unwrap_or("").to_ascii_lowercase(),
                    b["parentHash"].as_str().map(str::to_ascii_lowercase),
                ))
            })
            .collect::<Result<_, Error>>()?;
        let top = named.len() - 1;
        let linked = (0..top).all(|i| {
            named[i + 1].0 == named[i].0 + 1
                && named[i + 1].2.as_deref() == Some(named[i].1.as_str())
        });
        if linked {
            let a = self
                .anchor(call, named[top].0, &named[top].1, prefetched)
                .await?;
            if let Some(a) = a {
                for i in (1..=top).rev() {
                    let child = Anchored {
                        number: named[i].0,
                        hash: named[i].1.clone(),
                        ..a.clone()
                    };
                    self.anchors
                        .parent(&child, named[i].2.as_deref().unwrap_or(""));
                }
            }
        } else {
            let mut prefetched = prefetched;
            let futs: Vec<_> = named
                .iter()
                .enumerate()
                .map(|(i, (n, h, _))| {
                    let p = if i == top {
                        prefetched.take()
                    } else {
                        self.prefetch(call, &Resolved::num(*n))
                    };
                    self.anchor(call, *n, h, p)
                })
                .collect();
            for r in join_all(futs).await {
                r?;
            }
        }
        Ok(())
    }

    // ---------- fees ----------

    async fn fees(self: &Arc<Self>, call: &Call) -> R<Value> {
        let at = self.head(call, false).await?;
        let t = target(&at);
        // eth_gasPrice and eth_maxPriorityFeePerGas at one head share one proof; each call
        // handles a refusal itself.
        let r = match &at.hash {
            Some(h) => {
                let me = self.clone();
                let c = call.clone();
                let tt = t.clone();
                let run = move || -> BoxFuture<'static, Result<Value, Error>> {
                    async move {
                        match me.fetch_item(&c, "fees", &tt).await {
                            Ok(Ok(v)) => Ok(json!({"v": v})),
                            Ok(Err(e)) => Ok(json!({"error": e})),
                            Err(Fail::Err(e)) => Err(e),
                            Err(Fail::Fallback(_)) => Err(Error::verification(
                                "internal",
                                "a fee proof fell back",
                                None,
                            )),
                        }
                    }
                    .boxed()
                };
                self.once(format!("fees:{h}"), run)
                    .await
                    .map_err(|e| e.for_method(&call.method))?
            }
            None => match self.fetch_item(call, "fees", &t).await? {
                Ok(v) => json!({"v": v}),
                Err(e) => json!({"error": e}),
            },
        };
        if let Some(e) = r.get("error") {
            return self.refused(call, e).await;
        }
        let v = &r["v"];
        call.leaves(&v["report"]["unproven"]);
        self.anchor_report(call, v).await?;
        let f = &v["proven"];
        if f["formula"] != "willow-fee-v1" {
            return Err(Error::verification(
                "unknown-formula",
                format!(
                    "formula {}",
                    match &f["formula"] {
                        Value::String(s) => s.clone(),
                        Value::Null => "undefined".into(),
                        x => x.to_string(),
                    }
                ),
                call.m(),
            )
            .into());
        }
        match call.method.as_str() {
            "eth_gasPrice" => Ok(f["eth_gasPrice"].clone()),
            "eth_maxPriorityFeePerGas" => Ok(f["eth_maxPriorityFeePerGas"].clone()),
            _ => {
                if f["eth_blobBaseFee"].is_null() {
                    return Err(Error::rpc(
                        -32601,
                        format!(
                            "the method eth_blobBaseFee is not available on chain {}: its blocks carry no blob gas",
                            self.chain_id
                        ),
                    )
                    .into());
                }
                Ok(f["eth_blobBaseFee"].clone())
            }
        }
    }

    async fn fee_history(self: &Arc<Self>, call: &Call) -> R<Value> {
        let count0 = to_number(call.p(0)).ok_or_else(|| Error::rpc(-32602, "bad blockCount"))?;
        let pct = call.p(2);
        let percentiles = if pct.is_null() {
            json!([])
        } else {
            pct.clone()
        };
        if !percentiles
            .as_array()
            .is_some_and(|a| a.iter().all(Value::is_number))
        {
            return Err(Error::rpc(-32602, "bad rewardPercentiles").into());
        }
        if count0 == 0 {
            call.set_status(Status::Local);
            return Ok(json!({"oldestBlock": "0x0", "baseFeePerGas": [], "gasUsedRatio": []}));
        }
        let at = self.resolve(call, call.p(1)).await?;
        let Some(newest) = at.number else {
            return Err(Error::rpc(
                -32602,
                "eth_feeHistory names its newest block by number or tag",
            )
            .into());
        };
        let count = count0.min(FEE_HISTORY_MAX).min(newest + 1);
        // Nitro's rewards are zeros with no bodies, so only the other chains' proofs stop at 16 blocks.
        let with_rewards = !percentiles.as_array().unwrap().is_empty();
        let step = if with_rewards && !NITRO.contains(&self.chain_id) {
            FEE_HISTORY_WITH_REWARDS
        } else {
            count
        };
        let mut chunks = vec![];
        let (mut n, mut left) = (newest, count);
        while left > 0 {
            chunks.push((n, step.min(left)));
            if left <= step {
                break;
            }
            n -= step;
            left -= step;
        }
        let parts = join_all(chunks.iter().map(|&(newest, count)| {
            let percentiles = percentiles.clone();
            async move {
                let target = json!([quantity(count), quantity(newest), percentiles]).to_string();
                let v = self.prove_item(call, "feeHistory", &target).await?;
                self.anchor_report(call, &v).await?;
                let h = v["proven"].clone();
                let oldest = to_number(&h["oldestBlock"]);
                let last = to_number(&v["report"]["block"]["number"]);
                if oldest != Some(newest + 1 - count) || last != Some(newest) {
                    return Err(Fail::Err(Error::verification(
                        "wrong-range",
                        format!(
                            "a fee-history proof for blocks {}-{newest} covers {}-{}",
                            newest + 1 - count,
                            h["oldestBlock"].as_str().unwrap_or("?"),
                            v["report"]["block"]["number"].as_str().unwrap_or("?")
                        ),
                        call.m(),
                    )));
                }
                Ok(h)
            }
        }))
        .await;
        let mut parts: Vec<Value> = parts.into_iter().collect::<R<_>>()?;
        // Oldest first; each part's last base fee is the next part's first.
        parts.reverse();
        let mut out = Map::new();
        out.insert("oldestBlock".into(), parts[0]["oldestBlock"].clone());
        let join = |field: &str, next_one: bool| -> Option<Value> {
            if parts.iter().any(|p| !p[field].is_array()) {
                return None;
            }
            let last = parts.len() - 1;
            let mut all = vec![];
            for (i, p) in parts.iter().enumerate() {
                let a = p[field].as_array().unwrap();
                if next_one && i < last {
                    all.extend(a[..a.len().saturating_sub(1)].iter().cloned());
                } else {
                    all.extend(a.iter().cloned());
                }
            }
            Some(Value::Array(all))
        };
        if let Some(r) = join("reward", false) {
            out.insert("reward".into(), r);
        }
        if let Some(x) = join("baseFeePerGas", true) {
            out.insert("baseFeePerGas".into(), x);
        }
        if let Some(x) = join("gasUsedRatio", false) {
            out.insert("gasUsedRatio".into(), x);
        }
        if let Some(x) = join("baseFeePerBlobGas", true) {
            out.insert("baseFeePerBlobGas".into(), x);
        }
        if let Some(x) = join("blobGasUsedRatio", false) {
            out.insert("blobGasUsedRatio".into(), x);
        }
        Ok(Value::Object(out))
    }

    // ---------- transactions ----------

    async fn send(&self, call: &Call) -> R<Value> {
        let raw = call.p(0).clone();
        let bytes = raw
            .as_str()
            .and_then(to_bytes)
            .ok_or_else(|| Error::rpc(-32602, "expected a signed transaction's bytes"))?;
        let (hash, sender) = match crate::tx::decode_sent(&bytes) {
            Some(t) => (t.hash.clone(), Some(t)),
            None => {
                // A type this client does not decode: its hash is still that of its bytes.
                if bytes.first() == Some(&3) {
                    return Err(
                        Error::rpc(-32602, "a blob transaction this client cannot decode").into(),
                    );
                }
                (keccak_hex(&bytes), None)
            }
        };
        let r = self
            .batcher
            .call(&self.url, "eth_sendRawTransaction", json!([raw]))
            .await?;
        if let Some(e) = r.error() {
            // Broadcast can't be proven; the provider's error passes through, marked.
            let m = match &e["message"] {
                Value::String(s) => s.clone(),
                Value::Null => String::new(),
                v => v.to_string(),
            };
            call.flag("provider-error", Some(m));
            return Err(rpc_error(e).into());
        }
        if !same_hex(r.result(), &json!(hash)) {
            return Err(Error::verification(
                "tx-hash",
                format!(
                    "the server says {}, the bytes hash to {hash}",
                    match r.result() {
                        Value::String(s) => s.clone(),
                        v => v.to_string(),
                    }
                ),
                call.m(),
            )
            .into());
        }
        if let Some(s) = sender {
            if let Some(from) = s
                .from
                .filter(|_| s.chain_id.is_none_or(|c| c == self.chain_id))
            {
                self.record_sent(&from, u128::from(s.nonce));
            }
        }
        call.set_status(Status::Local);
        Ok(json!(hash))
    }

    /// A transaction this client sent: the highest nonce per sender, for the pending nonce.
    pub(crate) fn record_sent(&self, from: &str, nonce: u128) {
        let k = from.to_ascii_lowercase();
        let mut sent = self.sent.lock().unwrap();
        if sent.get(&k).is_none_or(|h| nonce > *h) {
            sent.insert(k, nonce);
        }
    }

    #[cfg(test)]
    pub(crate) fn sent_nonce(&self, from: &str) -> Option<u128> {
        self.sent
            .lock()
            .unwrap()
            .get(&from.to_ascii_lowercase())
            .copied()
    }

    async fn item(
        self: &Arc<Self>,
        call: &Call,
        kind: &str,
        target: &str,
        null_when_missing: bool,
    ) -> R<Value> {
        let (method, params) = self.item_request(call, kind, target).await?;
        let reply = self.batcher.call(&self.url, &method, params).await?;
        if let Some(e) = reply.error() {
            // An unknown transaction is an absence, as the standard methods return null for it.
            let code = e["code"].as_i64();
            let message = e["message"].as_str().unwrap_or("");
            if null_when_missing && code == Some(-32000) && message.starts_with(NOT_FOUND_TX) {
                call.set_status(Status::Absent);
                return Ok(Value::Null);
            }
            return self.refused(call, e).await;
        }
        if reply.result().is_null() {
            call.set_status(Status::Absent);
            return Ok(Value::Null);
        }
        let v = self
            .verify_item(call, kind, target, reply.text.clone(), false)
            .await?;
        self.anchor_report(call, &v).await?;
        Ok(v["proven"].clone())
    }

    // ---------- logs ----------

    async fn get_logs(self: &Arc<Self>, call: &Call, f: Value) -> R<Vec<Value>> {
        if !f.is_object() {
            return Err(Error::rpc(-32602, "expected a filter object").into());
        }
        if f.get("blockHash").is_some() {
            if f.get("fromBlock").is_some() || f.get("toBlock").is_some() {
                return Err(Error::rpc(
                    -32602,
                    "blockHash cannot be combined with fromBlock/toBlock",
                )
                .into());
            }
            let h = self.hash_param(&f["blockHash"])?;
            return self
                .logs_by_receipts(call, &f, vec![Resolved::hash(h)])
                .await;
        }
        let from_arg = f.get("fromBlock").cloned().unwrap_or(json!("latest"));
        let to_arg = f.get("toBlock").cloned().unwrap_or(json!("latest"));
        let from = self.resolve(call, &from_arg).await?;
        let to = self.resolve(call, &to_arg).await?;
        if from.number.is_none() || to.number.is_none() {
            return Err(Error::rpc(-32602, "fromBlock and toBlock are numbers or tags").into());
        }
        self.logs_range(call, &f, from, to).await
    }

    async fn logs_range(
        self: &Arc<Self>,
        call: &Call,
        filter: &Value,
        from: Resolved,
        to: Resolved,
    ) -> R<Vec<Value>> {
        let (f, t) = (from.number.unwrap(), to.number.unwrap());
        if f > t {
            return Err(Error::rpc(-32000, "invalid block range params").into());
        }
        if t - f < LOGS_BY_RECEIPTS {
            return self
                .logs_by_receipts(call, filter, range_blocks(f, t, &from, &to))
                .await;
        }
        self.logs_by_proof(call, filter, f, t).await
    }

    /// Logs of consecutive blocks from their block-receipts proofs, asked for at once.
    async fn logs_by_receipts(
        self: &Arc<Self>,
        call: &Call,
        filter: &Value,
        blocks: Vec<Resolved>,
    ) -> R<Vec<Value>> {
        let top = blocks.len() - 1;
        let prefetched = self.prefetch(call, &blocks[top]);
        let targets: Vec<String> = blocks.iter().map(target).collect();
        let verified = join_all(
            targets
                .iter()
                .map(|t| self.prove_item(call, "blockReceipts", t)),
        )
        .await;
        let vs: Vec<Value> = verified.into_iter().collect::<R<_>>()?;
        self.anchor_range(call, &vs, prefetched).await?;
        let mut out = vec![];
        for v in &vs {
            let receipts = v["proven"].as_array().ok_or_else(|| {
                Error::verification("malformed", "the proven receipts are not a list", call.m())
            })?;
            for r in receipts {
                if let Some(logs) = r["logs"].as_array() {
                    out.extend(logs.iter().filter(|l| matches(l, filter)).cloned());
                }
            }
        }
        Ok(out)
    }

    async fn logs_by_proof(
        self: &Arc<Self>,
        call: &Call,
        filter: &Value,
        from: u64,
        to: u64,
    ) -> R<Vec<Value>> {
        let mut f = Map::new();
        f.insert("fromBlock".into(), json!(quantity(from)));
        f.insert("toBlock".into(), json!(quantity(to)));
        if let Some(a) = filter.get("address") {
            f.insert("address".into(), a.clone());
        }
        if let Some(t) = filter.get("topics") {
            f.insert("topics".into(), t.clone());
        }
        let f = Value::Object(f);
        let deadline = self.now() + self.cfg.logs_timeout.as_millis() as u64;
        let reply = loop {
            let reply = self
                .batcher
                .call(&self.url, "willow_getLogsProof", json!([f]))
                .await?;
            let Some(e) = reply.error() else {
                break reply;
            };
            let d = e.get("data").filter(|d| d.is_object());
            let status = d.and_then(|d| d["status"].as_str());
            let code = e["code"].as_i64();
            if code == Some(-32000) && matches!(status, Some("queued" | "running" | "waiting")) {
                if self.now() >= deadline {
                    return self
                        .policy(
                            call,
                            "logs-timeout",
                            format!("Willow had not proven blocks {from}-{to} in time"),
                            self.standard(call),
                        )
                        .await
                        .map(|_| vec![]);
                }
                let d = d.unwrap();
                let ids: Vec<String> = match (&d["jobIds"], &d["jobId"]) {
                    (Value::Array(a), _) => a
                        .iter()
                        .filter_map(|x| x.as_str().map(String::from))
                        .collect(),
                    (_, Value::String(s)) => vec![s.clone()],
                    _ => vec![],
                };
                self.wait_for_jobs(call, &ids, to_number(&d["retryAfterSeconds"]), deadline)
                    .await?;
                continue;
            }
            if code == Some(-32601) || code == Some(-32005) {
                let m = e["message"].as_str().unwrap_or("").to_string();
                return self
                    .policy(
                        call,
                        "logs-range",
                        format!("{} blocks without a log proof: {m}", to - from + 1),
                        self.standard(call),
                    )
                    .await
                    .map(|_| vec![]);
            }
            call.set_status(Status::Refused);
            return Err(rpc_error(e).into());
        };
        let proof = reply.result();
        let Some(parts) = proof.get("parts").and_then(Value::as_array) else {
            return Err(Error::verification(
                "malformed",
                "willow_getLogsProof sent no parts",
                call.m(),
            )
            .into());
        };
        let mut ids: Vec<String> = vec![];
        for p in parts {
            if let Some(id) = p["proof"].as_str() {
                if !ids.iter().any(|x| x == id) {
                    ids.push(id.to_string());
                }
            }
        }
        let mut proofs: HashMap<String, Vec<u8>> = HashMap::new();
        for chunk in ids.chunks(PROOF_DOWNLOADS) {
            let got = join_all(chunk.iter().map(|id| async move {
                let r = self
                    .batcher
                    .single(&self.url, "willow_getProof", json!([id]))
                    .await?;
                if let Some(e) = r.error() {
                    return self.refused::<(String, Vec<u8>)>(call, e).await;
                }
                let b = r.result()["proof"]
                    .as_str()
                    .and_then(to_bytes)
                    .ok_or_else(|| {
                        Error::verification(
                            "malformed",
                            format!("willow_getProof sent no proof bytes for {id}"),
                            call.m(),
                        )
                    })?;
                Ok((id.clone(), b))
            }))
            .await;
            for g in got {
                let (id, b) = g?;
                proofs.insert(id, b);
            }
        }
        let v = self
            .logs_verify(call, reply.text.clone(), &f, proofs)
            .await?;
        if v["ok"] != true {
            let message = v["message"].as_str().unwrap_or("").to_string();
            return Err(match v["kind"].as_str() {
                Some("invalid-filter") => Error::rpc(-32602, message),
                Some("rejected") => Error::verification("logs", message, call.m()),
                Some("malformed-input") => Error::verification("malformed", message, call.m()),
                _ => Error::verification("internal", message, call.m()),
            }
            .into());
        }
        let last = &v["report"]["lastProvenBlock"];
        if to_number(&last["number"]) != Some(to) {
            return Err(Error::verification(
                "wrong-range",
                format!(
                    "the log proof ends at block {}, not {to}",
                    last["number"].as_str().unwrap_or("?")
                ),
                call.m(),
            )
            .into());
        }
        self.anchor(call, to, last["hash"].as_str().unwrap_or(""), None)
            .await?;
        call.leaves_list(&["[].blockHash".to_string(), "[].transactionHash".to_string()]);
        Ok(v["logs"].as_array().cloned().unwrap_or_default())
    }

    #[cfg(feature = "logs")]
    async fn logs_verify(
        &self,
        call: &Call,
        response: Arc<str>,
        f: &Value,
        proofs: HashMap<String, Vec<u8>>,
    ) -> R<Value> {
        let backend = self.backend.clone();
        let c = self.chain_id;
        let filter = f.to_string();
        let r = blocking(move || backend.logs_verify(c, &response, &filter, &proofs))
            .await
            .map_err(|e| e.with_method(call.m()))?;
        match r {
            Ran::Done(v) => Ok(v),
            Ran::NoVerifier(n) => {
                self.policy(call, "no-verifier", n.0, self.standard(call))
                    .await
            }
        }
    }

    #[cfg(not(feature = "logs"))]
    async fn logs_verify(
        &self,
        call: &Call,
        _response: Arc<str>,
        _f: &Value,
        _proofs: HashMap<String, Vec<u8>>,
    ) -> R<Value> {
        let _ = &self.backend;
        self.policy(
            call,
            "no-verifier",
            "no logs verifier module: this build has none (the `logs` feature)".into(),
            self.standard(call),
        )
        .await
    }

    async fn wait_for_jobs(
        &self,
        call: &Call,
        ids: &[String],
        retry_after: Option<u64>,
        deadline: u64,
    ) -> R<()> {
        let every = self
            .cfg
            .wait_jobs
            .unwrap_or_else(|| Duration::from_secs(retry_after.unwrap_or(15).clamp(2, 15)));
        let reask_at = self.now() + 60_000;
        loop {
            tokio::time::sleep(every).await;
            if ids.is_empty() || self.now() >= reask_at || self.now() >= deadline {
                return Ok(());
            }
            let jobs = join_all(ids.iter().map(|id| {
                self.batcher
                    .call(&self.url, "willow_getProofJob", json!([id]))
            }))
            .await;
            let status: Vec<Option<String>> = jobs
                .iter()
                .map(|j| match j {
                    Ok(r) if r.error().is_none() => r.result()["status"].as_str().map(String::from),
                    _ => None,
                })
                .collect();
            if let Some(i) = status.iter().position(|s| s.as_deref() == Some("failed")) {
                return self
                    .policy(
                        call,
                        "logs-job-failed",
                        format!("Willow's proof job {} failed", ids[i]),
                        self.standard(call),
                    )
                    .await
                    .map(|_| ());
            }
            if status.iter().all(|s| s.as_deref() == Some("done")) {
                return Ok(());
            }
        }
    }

    // ---------- local filters ----------

    async fn filter(self: &Arc<Self>, call: &Call) -> R<Value> {
        let now = self.now();
        self.filters
            .lock()
            .unwrap()
            .retain(|_, f| now.saturating_sub(f.touched) <= FILTER_IDLE_MS);
        let arg = call.p(0).clone();
        if call.method == "eth_newFilter" || call.method == "eth_newBlockFilter" {
            if call.method == "eth_newFilter" && !arg.is_object() && !arg.is_array() {
                return Err(Error::rpc(-32602, "expected a filter object").into());
            }
            let head = self.head(call, false).await?;
            let n = self.filter_ids.fetch_add(1, Ordering::Relaxed) + 1;
            let salt = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.subsec_nanos())
                .unwrap_or(0)
                & 0xffff) as u64;
            let id = quantity(n * 0x10000 + salt);
            // Computed before the lock: nothing the server says may panic while it's held.
            let next = head.number.unwrap_or(0).saturating_add(1);
            self.filters.lock().unwrap().insert(
                id.clone(),
                Filter {
                    kind: if call.method == "eth_newFilter" {
                        "logs"
                    } else {
                        "blocks"
                    },
                    filter: arg,
                    next,
                    touched: now,
                },
            );
            return Ok(json!(id));
        }
        if call.method == "eth_uninstallFilter" {
            call.set_status(Status::Local);
            let removed = arg
                .as_str()
                .is_some_and(|id| self.filters.lock().unwrap().remove(id).is_some());
            return Ok(json!(removed));
        }
        let (kind, filter, next) = {
            let mut fs = self.filters.lock().unwrap();
            let Some(f) = arg.as_str().and_then(|id| fs.get_mut(id)) else {
                return Err(Error::rpc(-32000, "filter not found").into());
            };
            f.touched = now;
            (f.kind, f.filter.clone(), f.next)
        };
        let id = arg.as_str().unwrap().to_string();
        if call.method == "eth_getFilterLogs" {
            if kind != "logs" {
                return Err(Error::rpc(-32000, "filter not found").into());
            }
            call.set_stand_in("eth_getLogs", json!([filter]));
            return self.get_logs(call, filter).await.map(Value::Array);
        }
        let head = self.head(call, false).await?;
        let top = head.number.unwrap_or(0);
        if top < next {
            return Ok(json!([]));
        }
        let set_next = |me: &Self, last: u64| {
            let next = last.saturating_add(1);
            if let Some(f) = me.filters.lock().unwrap().get_mut(&id) {
                f.next = next;
            }
        };
        if kind == "blocks" {
            // At most BLOCK_FILTER_MAX blocks a poll, oldest first; the next poll goes on from there.
            let last = top.min(next.saturating_add(BLOCK_FILTER_MAX - 1));
            let at = |n: u64| {
                if n == top && head.hash.is_some() {
                    head.clone()
                } else {
                    Resolved::num(n)
                }
            };
            // Every proof asked for at once (one batch); verified newest first, so each block's
            // parent hash anchors the one below it without an anchor of its own.
            let mut replies = HashMap::new();
            for n in (next..=last).rev() {
                replies.insert(n, self.block_reply(call, &at(n)).await?);
            }
            let mut out = vec![];
            for n in (next..=last).rev() {
                let b = self
                    .block(call, &at(n), false, replies.remove(&n), false)
                    .await?;
                let Some(b) = b else {
                    return Err(Error::verification(
                        "missing",
                        format!("block {n} of the range is absent"),
                        call.m(),
                    )
                    .into());
                };
                out.insert(0, json!(b.hash));
            }
            set_next(self, last);
            return Ok(Value::Array(out));
        }
        let lo = next.max(
            filter["fromBlock"]
                .as_str()
                .and_then(str_number)
                .unwrap_or(0),
        );
        let hi = filter["toBlock"]
            .as_str()
            .and_then(str_number)
            .map_or(top, |t| t.min(top));
        let logs = if lo > hi {
            vec![]
        } else {
            let mut range = Map::new();
            range.insert("fromBlock".into(), json!(quantity(lo)));
            range.insert("toBlock".into(), json!(quantity(hi)));
            for k in ["address", "topics"] {
                if let Some(v) = filter.get(k) {
                    range.insert(k.into(), v.clone());
                }
            }
            call.set_stand_in("eth_getLogs", json!([Value::Object(range)]));
            let got = self
                .logs_range(
                    call,
                    &filter,
                    Resolved::num(lo),
                    if hi == top {
                        head.clone()
                    } else {
                        Resolved::num(hi)
                    },
                )
                .await;
            match got {
                // The server's logs, flagged (`flag`): this range is still served.
                Err(Fail::Fallback(v)) => {
                    set_next(self, top);
                    return Err(Fail::Fallback(v));
                }
                r => r?,
            }
        };
        // Only once the logs are in: a failed poll is asked again from the same block.
        set_next(self, top);
        Ok(Value::Array(logs))
    }
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

/// A resolved block as a proof target text: its hash, or its number.
fn target(at: &Resolved) -> String {
    match (&at.hash, at.number) {
        (Some(h), _) => h.clone(),
        (None, Some(n)) => quantity(n),
        (None, None) => "latest".into(),
    }
}

/// A resolved block as a proof's block parameter: `{"blockHash"}`, or its number.
fn block_param(at: &Resolved) -> Value {
    match (&at.hash, at.number) {
        (Some(h), _) => json!({"blockHash": h}),
        (None, Some(n)) => json!(quantity(n)),
        (None, None) => json!("latest"),
    }
}

/// Blocks `f..=t`, the ends as resolved (by hash when known).
fn range_blocks(f: u64, t: u64, from: &Resolved, to: &Resolved) -> Vec<Resolved> {
    (f..=t)
        .map(|n| {
            if n == t && to.hash.is_some() {
                to.clone()
            } else if n == f && from.hash.is_some() {
                from.clone()
            } else {
                Resolved::num(n)
            }
        })
        .collect()
}

/// A key as a URL path segment.
fn url_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn with_chain_rewrites_the_rpc_path() {
        assert_eq!(
            with_chain("https://api.willow.tech/v1/rpc/8453/k", 1).unwrap(),
            "https://api.willow.tech/v1/rpc/1/k"
        );
        assert_eq!(
            with_chain("http://127.0.0.1:9/v1/rpc/10", 1).unwrap(),
            "http://127.0.0.1:9/v1/rpc/1/"
        );
        assert!(with_chain("https://example.com/rpc", 1).is_none());
        assert!(with_chain("https://example.com/rpc/x1/k", 1).is_none());
        assert_eq!(
            with_chain("https://example.com/rpc/abc/rpc/5/k", 1).unwrap(),
            "https://example.com/rpc/abc/rpc/1/k"
        );
    }

    #[test]
    fn keys_are_url_encoded_as_encode_uri_component() {
        assert_eq!(url_component("ab/c d"), "ab%2Fc%20d");
        assert_eq!(url_component("public"), "public");
    }
}
