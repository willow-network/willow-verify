//! JSON-RPC over HTTP, batched: calls made within a window of each other go out as one batch per
//! URL (at most [`MAX_BATCH`] items), identical calls in flight share one item, and each item's
//! exact text is kept for the verifiers, which parse server data themselves.

use crate::error::Error;
use futures::future::{BoxFuture, FutureExt, Shared};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::oneshot;

/// The most items in one JSON-RPC batch (the server's `MAX_BATCH`).
pub const MAX_BATCH: usize = 100;
const RATE_LIMIT_WAITS_MS: [u64; 2] = [1000, 2000];

/// One item of a response: its exact text (passed to the verifiers as sent) and its parse.
#[derive(Clone, Debug)]
pub(crate) struct Reply {
    pub text: Arc<str>,
    pub value: Arc<Value>,
}

impl Reply {
    /// The item's error object, when it has one.
    pub fn error(&self) -> Option<&Value> {
        self.value.get("error").filter(|e| e.is_object())
    }

    /// The item's `result` (`Null` when absent).
    pub fn result(&self) -> &Value {
        self.value.get("result").unwrap_or(&Value::Null)
    }
}

pub(crate) type ReplyFuture = Shared<BoxFuture<'static, Result<Reply, Error>>>;

struct Entry {
    id: u64,
    key: String,
    method: String,
    params: Value,
    tx: Option<oneshot::Sender<Result<Reply, Error>>>,
    inner: std::sync::Weak<Inner>,
}

impl Entry {
    /// The key stops being in flight, so the next identical call is asked again.
    fn release(&mut self) {
        let Some(inner) = self.inner.upgrade() else {
            return;
        };
        self.inner = std::sync::Weak::new();
        let mut st = inner.state.lock().unwrap_or_else(|p| p.into_inner());
        if st
            .inflight
            .get(&self.key)
            .is_some_and(|(id, _)| *id == self.id)
        {
            st.inflight.remove(&self.key);
        }
    }
}

/// However an entry ends (settled, or dropped with a send or runtime that ended), it's released.
impl Drop for Entry {
    fn drop(&mut self) {
        self.release();
    }
}

#[derive(Default)]
struct State {
    queues: HashMap<String, Vec<Entry>>,
    /// Each URL's pending flush per runtime: a queue is flushed by the runtime of every call that
    /// waits on it, so one runtime ending doesn't strand another's calls.
    timers: HashMap<(String, tokio::runtime::Id), tokio::task::JoinHandle<()>>,
    inflight: HashMap<String, (u64, ReplyFuture)>,
    next_id: u64,
}

struct Inner {
    http: reqwest::Client,
    window: Duration,
    max_batch: usize,
    timeout: Duration,
    state: Mutex<State>,
    /// Tests: a pause after each response is handed over (to show what a caller sees then).
    #[cfg(test)]
    after_send: std::sync::atomic::AtomicU64,
}

#[derive(Clone)]
pub(crate) struct Batcher {
    inner: Arc<Inner>,
}

/// The elements of a top-level JSON array, each as its exact text, or `None` when `text` is not
/// one. Strings and nesting are tracked; each element is parsed afterwards.
pub(crate) fn split_array(text: &str) -> Option<Vec<&str>> {
    let b = text.as_bytes();
    let mut i = 0;
    let ws = |i: &mut usize| {
        while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\r' | b'\n') {
            *i += 1;
        }
    };
    ws(&mut i);
    if b.get(i) != Some(&b'[') {
        return None;
    }
    i += 1;
    let mut out = Vec::new();
    ws(&mut i);
    if b.get(i) == Some(&b']') {
        i += 1;
        ws(&mut i);
        return (i == b.len()).then_some(out);
    }
    loop {
        ws(&mut i);
        let start = i;
        let mut depth = 0usize;
        let mut in_string = false;
        while i < b.len() {
            let c = b[i];
            if in_string {
                if c == b'\\' {
                    i += 1;
                } else if c == b'"' {
                    in_string = false;
                }
            } else if c == b'"' {
                in_string = true;
            } else if c == b'{' || c == b'[' {
                depth += 1;
            } else if c == b'}' || c == b']' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
            } else if c == b',' && depth == 0 {
                break;
            }
            i += 1;
        }
        if i >= b.len() || in_string {
            return None;
        }
        out.push(text.get(start..i)?.trim_end());
        if b[i] == b',' {
            i += 1;
            continue;
        }
        i += 1;
        ws(&mut i);
        return (i == b.len()).then_some(out);
    }
}

pub(crate) fn host_of(url: &str) -> String {
    reqwest::Url::parse(url)
        .ok()
        .and_then(|u| {
            u.host_str()
                .map(|h| u.port().map_or(h.to_string(), |p| format!("{h}:{p}")))
        })
        .unwrap_or_else(|| url.to_string())
}

impl Batcher {
    pub(crate) fn new(
        http: reqwest::Client,
        window: Duration,
        max_batch: usize,
        timeout: Duration,
    ) -> Batcher {
        Batcher {
            inner: Arc::new(Inner {
                http,
                window,
                max_batch: max_batch.clamp(1, MAX_BATCH),
                timeout,
                state: Mutex::new(State::default()),
                #[cfg(test)]
                after_send: std::sync::atomic::AtomicU64::new(0),
            }),
        }
    }

    #[cfg(test)]
    pub(crate) fn pause_after_send(&self, d: Duration) {
        self.inner
            .after_send
            .store(d.as_millis() as u64, std::sync::atomic::Ordering::SeqCst);
    }

    #[cfg(test)]
    fn sent(&self) {
        let ms = self
            .inner
            .after_send
            .load(std::sync::atomic::Ordering::SeqCst);
        if ms > 0 {
            std::thread::sleep(Duration::from_millis(ms));
        }
    }

    /// `method(params)` on `url`, in the next batch to `url`. The call is queued now, when this
    /// is called, not when the future is first polled, so calls made together share a batch.
    pub(crate) fn call(&self, url: &str, method: &str, params: Value) -> ReplyFuture {
        // Off a tokio runtime (a request future moved to another executor) nothing can be
        // spawned: an error, before any lock is taken.
        let Ok(rt) = tokio::runtime::Handle::try_current().map(|h| h.id()) else {
            let e = Error::rpc(
                -32603,
                "willow-rpc runs on a tokio runtime, and none is running here",
            );
            return futures::future::ready(Err(e)).boxed().shared();
        };
        let key = format!("{url}\n{method}\n{params}");
        let mut st = self.inner.state.lock().unwrap();
        let (f, full) = match st.inflight.get(&key) {
            Some((_, f)) => (f.clone(), None),
            None => {
                let (tx, rx) = oneshot::channel();
                st.next_id += 1;
                let id = st.next_id;
                let entry = Entry {
                    id,
                    key: key.clone(),
                    method: method.to_string(),
                    params,
                    tx: Some(tx),
                    inner: Arc::downgrade(&self.inner),
                };
                let q = st.queues.entry(url.to_string()).or_default();
                q.push(entry);
                let full = (q.len() >= self.inner.max_batch)
                    .then(|| st.queues.remove(url).unwrap_or_default());
                let f: ReplyFuture = async move {
                    rx.await.unwrap_or_else(|_| {
                        Err(Error::rpc(
                            -32603,
                            "the request was dropped before a response",
                        ))
                    })
                }
                .boxed()
                .shared();
                st.inflight.insert(key, (id, f.clone()));
                (f, full)
            }
        };
        // A queue waits on a flush on this call's runtime too, also when the call joins a request
        // already queued; flushes left by runtimes that ended are dropped.
        let queued = st.queues.get(url).is_some_and(|q| !q.is_empty());
        let k = (url.to_string(), rt);
        if queued && st.timers.get(&k).is_none_or(|t| t.is_finished()) {
            st.timers.retain(|_, t| !t.is_finished());
            let me = self.clone();
            let u = url.to_string();
            let t = tokio::spawn(async move {
                tokio::time::sleep(me.inner.window).await;
                me.flush(&u, rt);
            });
            st.timers.insert(k, t);
        }
        drop(st);
        if let Some(batch) = full {
            self.spawn_send(url.to_string(), batch);
        }
        f
    }

    fn flush(&self, url: &str, rt: tokio::runtime::Id) {
        let q = {
            let mut st = self.inner.state.lock().unwrap();
            st.timers.remove(&(url.to_string(), rt));
            st.queues.remove(url).unwrap_or_default()
        };
        let max = self.inner.max_batch;
        let mut q = q;
        while !q.is_empty() {
            let rest = q.split_off(q.len().min(max));
            self.spawn_send(url.to_string(), q);
            q = rest;
        }
    }

    fn spawn_send(&self, url: String, batch: Vec<Entry>) {
        let me = self.clone();
        tokio::spawn(async move { me.send(&url, batch).await });
    }

    fn settle(&self, mut e: Entry, r: Result<Reply, Error>) {
        // Released before the response, so a caller that has it and asks again is asked anew.
        e.release();
        if let Some(tx) = e.tx.take() {
            let _ = tx.send(r);
            #[cfg(test)]
            self.sent();
        }
    }

    async fn send(&self, url: &str, batch: Vec<Entry>) {
        let body: Vec<Value> = batch
            .iter()
            .enumerate()
            .map(|(i, e)| json!({"jsonrpc": "2.0", "id": i + 1, "method": e.method, "params": e.params}))
            .collect();
        let body = Value::Array(body).to_string();
        let n = batch.len();
        let (text, status) = match self.post(url, body).await {
            Ok(x) => x,
            Err(e) => {
                for x in batch {
                    self.settle(x, Err(e.clone()));
                }
                return;
            }
        };
        let Some(items) = split_array(&text) else {
            let v: Option<Value> = serde_json::from_str(&text).ok();
            let e = match v
                .as_ref()
                .and_then(|v| v.get("error"))
                .filter(|e| e.is_object())
            {
                Some(err) => Error::from_object(err),
                None if !(200..300).contains(&status) => {
                    Error::rpc(-32603, format!("{} responded HTTP {status}", host_of(url)))
                }
                None => Error::verification(
                    "batch",
                    format!("the response to a batch of {n} is not a JSON-RPC batch"),
                    None,
                ),
            };
            for x in batch {
                self.settle(x, Err(e.clone()));
            }
            return;
        };
        let mut by_id: HashMap<u64, Reply> = HashMap::new();
        let mut twice: HashSet<u64> = HashSet::new();
        for t in items {
            let Ok(value) = serde_json::from_str::<Value>(t) else {
                continue;
            };
            let Some(id) = value.get("id").and_then(json_id) else {
                continue;
            };
            if by_id.contains_key(&id) {
                twice.insert(id);
            }
            by_id.insert(
                id,
                Reply {
                    text: t.into(),
                    value: Arc::new(value),
                },
            );
        }
        for (i, e) in batch.into_iter().enumerate() {
            let id = i as u64 + 1;
            let method = e.method.clone();
            let r = if twice.contains(&id) {
                Err(Error::verification(
                    "batch",
                    format!("the batch response holds item {id} twice"),
                    Some(&method),
                ))
            } else {
                match by_id.remove(&id) {
                    None => Err(Error::verification(
                        "batch",
                        format!("the batch response has no item {id} ({method})"),
                        Some(&method),
                    )),
                    Some(r) if !has_outcome(&r.value) => Err(Error::verification(
                        "batch",
                        format!("item {id} has neither a result nor an error"),
                        Some(&method),
                    )),
                    Some(r) => Ok(r),
                }
            };
            self.settle(e, r);
        }
    }

    /// One request on its own (not batched): for responses too large to share a batch, such as
    /// proofs of megabytes.
    pub(crate) async fn single(
        &self,
        url: &str,
        method: &str,
        params: Value,
    ) -> Result<Reply, Error> {
        let body =
            json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params}).to_string();
        let (text, status) = self.post(url, body).await?;
        let value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(_) if !(200..300).contains(&status) => {
                return Err(Error::rpc(
                    -32603,
                    format!("{} responded HTTP {status}", host_of(url)),
                ))
            }
            Err(_) => {
                return Err(Error::verification(
                    "batch",
                    format!("the response to {method} is not JSON"),
                    Some(method),
                ))
            }
        };
        if !value.is_object() || !has_outcome(&value) {
            return Err(Error::verification(
                "batch",
                format!("the response to {method} has neither a result nor an error"),
                Some(method),
            ));
        }
        Ok(Reply {
            text: text.into(),
            value: Arc::new(value),
        })
    }

    async fn post(&self, url: &str, body: String) -> Result<(String, u16), Error> {
        let mut attempt = 0;
        loop {
            let res = self
                .inner
                .http
                .post(url)
                .header("content-type", "application/json")
                .header("accept", "application/json")
                .timeout(self.inner.timeout)
                .body(body.clone())
                .send()
                .await
                .map_err(|e| {
                    Error::rpc(
                        -32603,
                        format!("{} could not be reached: {}", host_of(url), e.without_url()),
                    )
                })?;
            let status = res.status().as_u16();
            if status == 429 && attempt < RATE_LIMIT_WAITS_MS.len() {
                let after = res
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.trim().parse::<u64>().ok())
                    .filter(|a| (1..=10).contains(a));
                let _ = res.bytes().await;
                let wait = after.map_or(RATE_LIMIT_WAITS_MS[attempt], |a| a * 1000);
                tokio::time::sleep(Duration::from_millis(wait)).await;
                attempt += 1;
                continue;
            }
            let text = res.text().await.map_err(|e| {
                Error::rpc(
                    -32603,
                    format!("{} could not be read: {}", host_of(url), e.without_url()),
                )
            })?;
            return Ok((text, status));
        }
    }
}

/// A batch item's id: the client sends numbers.
fn json_id(v: &Value) -> Option<u64> {
    v.as_u64().filter(|_| v.is_number())
}

/// An item with a `result`, or an `error` object.
fn has_outcome(v: &Value) -> bool {
    v.get("result").is_some() || v.get("error").is_some_and(Value::is_object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_array_keeps_each_items_text() {
        let t = r#" [ {"id":1,"result":"a,]"} , {"id":2,"result":[1,{"x":"\"}"}]} ] "#;
        let v = split_array(t).unwrap();
        assert_eq!(
            v,
            vec![
                r#"{"id":1,"result":"a,]"}"#,
                r#"{"id":2,"result":[1,{"x":"\"}"}]}"#
            ]
        );
        assert_eq!(split_array("[]").unwrap(), Vec::<&str>::new());
        assert!(split_array(r#"{"id":1}"#).is_none());
        assert!(split_array(r#"[{"id":1}"#).is_none());
        assert!(split_array(r#"[{"id":1}] x"#).is_none());
    }
}
