//! The verifiers, native: the verify page's own Rust crates, called through the same JSON faces
//! the page and `@willow-network/rpc` call in wasm:
//! - items: `willow_items_verify_wasm::ItemVerifier` (blocks, fees, fee history, transactions,
//!   receipts, block receipts);
//! - state: `willow_state_verify_wasm::StateVerifier` (state, calls, estimates, traces);
//! - anchors: `anchor_verify::Anchor` (Ethereum's, Sepolia's and BNB's light clients);
//! - logs: the verify page's log verifier (`willow_logs_verify_wasm`) with SP1's verifier, and OpenVM proofs through
//!   the `willow-logs-verify-openvm` helper (OpenVM's verifier cannot share SP1's dependency
//!   graph), as the CLI does.
//!
//! Every method returns the module's JSON, parsed: `{"ok": true, ...}` or a failure
//! `{"ok": false, "kind", "message", ...}`. They run on tokio's blocking pool; a panic there is
//! the verifier failing to run, never a verdict.

use crate::error::Error;
use serde_json::{json, Value};
#[cfg(feature = "logs")]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// The light clients, as `anchor_verify::Anchor` exposes them.
pub(crate) trait AnchorApi: Send {
    fn restore(&mut self, saved: &str, now: f64) -> String;
    fn bootstrap(&mut self, response: &str, now: f64) -> String;
    fn updates(&mut self, response: &str) -> String;
    fn verify(
        &mut self,
        chain_id: u32,
        number: f64,
        hash: &str,
        response: &str,
        now: f64,
    ) -> String;
    fn saved(&self) -> String;
    fn reset(&mut self);
    fn start(&mut self, name: &str, start: &str) -> String;
    fn bootstrap_for(&mut self, name: &str, response: &str, now: f64) -> String;
    fn updates_for(&mut self, name: &str, response: &str) -> String;
    fn restore_for(&mut self, name: &str, saved: &str, now: f64) -> String;
    fn saved_for(&self, name: &str) -> String;
    fn reset_for(&mut self, name: &str);
}

impl AnchorApi for anchor_verify::Anchor {
    fn restore(&mut self, saved: &str, now: f64) -> String {
        anchor_verify::Anchor::restore(self, saved, now)
    }
    fn bootstrap(&mut self, response: &str, now: f64) -> String {
        anchor_verify::Anchor::bootstrap(self, response, now)
    }
    fn updates(&mut self, response: &str) -> String {
        anchor_verify::Anchor::updates(self, response)
    }
    fn verify(
        &mut self,
        chain_id: u32,
        number: f64,
        hash: &str,
        response: &str,
        now: f64,
    ) -> String {
        anchor_verify::Anchor::verify(self, chain_id, number, hash, response, now)
    }
    fn saved(&self) -> String {
        anchor_verify::Anchor::saved(self)
    }
    fn reset(&mut self) {
        anchor_verify::Anchor::reset(self)
    }
    fn start(&mut self, name: &str, start: &str) -> String {
        anchor_verify::Anchor::start(self, name, start)
    }
    fn bootstrap_for(&mut self, name: &str, response: &str, now: f64) -> String {
        anchor_verify::Anchor::bootstrap_for(self, name, response, now)
    }
    fn updates_for(&mut self, name: &str, response: &str) -> String {
        anchor_verify::Anchor::updates_for(self, name, response)
    }
    fn restore_for(&mut self, name: &str, saved: &str, now: f64) -> String {
        anchor_verify::Anchor::restore_for(self, name, saved, now)
    }
    fn saved_for(&self, name: &str) -> String {
        anchor_verify::Anchor::saved_for(self, name)
    }
    fn reset_for(&mut self, name: &str) {
        anchor_verify::Anchor::reset_for(self, name)
    }
}

/// Makes the light clients from Ethereum's checkpoint (root, slot).
pub(crate) type AnchorFactory = Arc<dyn Fn(&str, f64) -> Box<dyn AnchorApi> + Send + Sync>;

pub(crate) fn real_anchor() -> AnchorFactory {
    Arc::new(|root, slot| Box::new(anchor_verify::Anchor::new(root, slot)))
}

/// The light client that anchors chain `chain_id`: Ethereum's (`None`), Sepolia's or BNB's.
pub(crate) fn light_client_of(chain_id: u64) -> Option<&'static str> {
    if chain_id == 56 {
        Some("bnb")
    } else if SEPOLIA_CHAINS.contains(&chain_id) {
        Some("sepolia")
    } else {
        None
    }
}

/// The chains that settle on Sepolia (anchor-verify `l2::SEPOLIA_CHAINS`).
pub(crate) const SEPOLIA_CHAINS: [u64; 6] = [11155420, 84532, 1301, 763373, 421614, 46630];

/// A verifier that is not in this build or can't run here: the client can't verify with it (not
/// a verdict).
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "logs"), allow(dead_code))]
pub(crate) struct NoVerifier(pub String);

#[cfg_attr(not(feature = "logs"), allow(dead_code))]
pub(crate) enum Ran<T> {
    Done(T),
    NoVerifier(NoVerifier),
}

/// Where OpenVM log proofs are verified: the helper binary, or nowhere.
#[derive(Clone, Debug)]
#[cfg_attr(not(feature = "logs"), allow(dead_code))]
pub(crate) enum OpenVm {
    Helper(Option<std::path::PathBuf>),
    None,
}

pub(crate) struct Backend {
    anchor_factory: AnchorFactory,
    anchor: Mutex<Option<Box<dyn AnchorApi>>>,
    #[cfg(feature = "logs")]
    openvm: Mutex<Option<Helper>>,
    #[cfg_attr(not(feature = "logs"), allow(dead_code))]
    openvm_choice: OpenVm,
    /// The longest the OpenVM helper may take to start or to give one verdict.
    #[cfg_attr(not(feature = "logs"), allow(dead_code))]
    openvm_wait: std::time::Duration,
}

/// Runs `f` on the blocking pool; a panic is the verifier failing to run (`internal`); a run its
/// runtime dropped before it started (at shutdown) is no verdict (-32603).
pub(crate) async fn blocking<T, F>(f: F) -> Result<T, Error>
where
    T: Send + 'static,
    F: FnOnce() -> T + Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|e| {
        if !e.is_panic() {
            return Error::rpc(
                -32603,
                "a verifier was dropped with its runtime before it ran",
            );
        }
        let p = e.into_panic();
        let why = p
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| p.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| "a verifier panicked".into());
        Error::verification("internal", why, None)
    })
}

fn parse(s: String) -> Value {
    serde_json::from_str(&s).unwrap_or_else(|e| {
        json!({"ok": false, "kind": "internal", "message": format!("a verifier wrote no JSON: {e}")})
    })
}

/// An OpenVM proof's first bytes (`OVM2`).
#[cfg(feature = "logs")]
fn is_openvm(b: &[u8]) -> bool {
    b.starts_with(b"OVM2")
}

impl Backend {
    pub(crate) fn new(anchor_factory: AnchorFactory, openvm: OpenVm) -> Backend {
        Backend {
            anchor_factory,
            anchor: Mutex::new(None),
            #[cfg(feature = "logs")]
            openvm: Mutex::new(None),
            openvm_choice: openvm,
            openvm_wait: std::time::Duration::from_secs(120),
        }
    }

    /// The OpenVM helper's time limit: past it, it is stopped (unavailable at its start; a failed
    /// check once it has a proof).
    pub(crate) fn with_openvm_wait(mut self, d: std::time::Duration) -> Backend {
        self.openvm_wait = d;
        self
    }

    // ---------- items ----------

    pub(crate) fn item_request(chain_id: u64, kind: &str, target: &str) -> Value {
        let mut v = willow_items_verify_wasm::ItemVerifier::new(chain_id as u32);
        parse(v.request(kind, target))
    }

    /// `request`, `verify`, and for a block with `full`, `block(true)` as `rendered`.
    pub(crate) fn item_verify(
        chain_id: u64,
        kind: &str,
        target: &str,
        response: &str,
        full: bool,
    ) -> Value {
        let mut v = willow_items_verify_wasm::ItemVerifier::new(chain_id as u32);
        let req = parse(v.request(kind, target));
        if req["ok"] != true {
            return req;
        }
        let mut r = parse(v.verify(response));
        if r["ok"] != true || kind != "block" || !full {
            return r;
        }
        let rendered = parse(v.block(true));
        if rendered["ok"] != true {
            return rendered;
        }
        r["rendered"] = rendered;
        r
    }

    // ---------- state ----------

    pub(crate) fn state_request(chain_id: u64, kind: &str, target: &str) -> Value {
        let mut v = willow_state_verify_wasm::StateVerifier::new(chain_id as u32);
        parse(v.request(kind, target))
    }

    pub(crate) fn state_verify(chain_id: u64, kind: &str, target: &str, response: &str) -> Value {
        let mut v = willow_state_verify_wasm::StateVerifier::new(chain_id as u32);
        let req = parse(v.request(kind, target));
        if req["ok"] != true {
            return req;
        }
        parse(v.verify(response))
    }

    // ---------- anchors ----------

    /// The light clients, made on first use from the starts.
    fn with_anchor<T>(
        &self,
        starts: &crate::checkpoint::Starts,
        f: impl FnOnce(&mut dyn AnchorApi) -> T,
    ) -> T {
        let mut g = self.anchor.lock().unwrap_or_else(|p| p.into_inner());
        if g.is_none() {
            let mut a = (self.anchor_factory)(&starts.ethereum.root, starts.ethereum.slot as f64);
            a.start(
                "sepolia",
                &json!({"root": starts.sepolia.root, "slot": starts.sepolia.slot}).to_string(),
            );
            a.start(
                "bnb",
                &json!({"number": starts.bnb.number, "hash": starts.bnb.hash}).to_string(),
            );
            *g = Some(a);
        }
        f(g.as_mut().expect("made").as_mut())
    }

    pub(crate) fn anchor_restore(
        &self,
        starts: &crate::checkpoint::Starts,
        saved: &str,
        now: u64,
        lc: Option<&str>,
    ) -> Value {
        self.with_anchor(starts, |a| {
            parse(match lc {
                Some(n) => a.restore_for(n, saved, now as f64),
                None => a.restore(saved, now as f64),
            })
        })
    }

    pub(crate) fn anchor_bootstrap(
        &self,
        starts: &crate::checkpoint::Starts,
        response: &str,
        now: u64,
        lc: Option<&str>,
    ) -> Value {
        self.with_anchor(starts, |a| {
            parse(match lc {
                Some(n) => a.bootstrap_for(n, response, now as f64),
                None => a.bootstrap(response, now as f64),
            })
        })
    }

    pub(crate) fn anchor_updates(
        &self,
        starts: &crate::checkpoint::Starts,
        response: &str,
        lc: Option<&str>,
    ) -> Value {
        self.with_anchor(starts, |a| {
            parse(match lc {
                Some(n) => a.updates_for(n, response),
                None => a.updates(response),
            })
        })
    }

    /// `Anchor.verify`, and after a pass (with `want_saved`) what to keep of the light client that
    /// anchors the chain. A `need-bootstrap` resets the light client it names.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn anchor_verify(
        &self,
        starts: &crate::checkpoint::Starts,
        chain_id: u64,
        number: u64,
        hash: &str,
        response: &str,
        now: u64,
        want_saved: bool,
    ) -> (Value, Option<String>) {
        self.with_anchor(starts, |a| {
            let r = parse(a.verify(chain_id as u32, number as f64, hash, response, now as f64));
            if r["ok"] != true && r["kind"] == "need-bootstrap" {
                match r["lightClient"].as_str() {
                    Some(n) => a.reset_for(n),
                    None => a.reset(),
                }
            }
            let saved = if r["ok"] != true || !want_saved {
                None
            } else {
                Some(match light_client_of(chain_id) {
                    Some(lc) => a.saved_for(lc),
                    None => a.saved(),
                })
            };
            (r, saved)
        })
    }

    // ---------- logs ----------

    /// A `willow_getLogsProof` response body against `filter`, with the proofs by id.
    #[cfg(feature = "logs")]
    pub(crate) fn logs_verify(
        &self,
        chain_id: u64,
        response: &str,
        filter: &str,
        proofs: &HashMap<String, Vec<u8>>,
    ) -> Ran<Value> {
        use logset_query::BySystem;
        use willow_logs_verify::Sp1Check;
        use willow_logs_verify_wasm::{check_answer, trust, verified_json};

        let trust = trust(chain_id, "", "");
        let trust = match &trust {
            Ok(t) => t,
            Err(f) => return Ran::Done(f.json()),
        };
        let mut fetch = |id: &str| -> Result<Vec<u8>, String> {
            proofs
                .get(id)
                .cloned()
                .ok_or_else(|| format!("proof {id} was not fetched"))
        };
        let any_openvm = proofs.values().any(|p| is_openvm(p));
        // Only a check with OpenVM proofs waits for (and holds) the helper.
        let mut none = None;
        let mut guard = any_openvm.then(|| self.openvm.lock().unwrap_or_else(|p| p.into_inner()));
        let program = match (&self.openvm_choice, any_openvm) {
            (OpenVm::Helper(path), true) => Some(helper_program(path)),
            (OpenVm::None, true) => {
                return Ran::NoVerifier(NoVerifier(
                    "no openvm verifier module: this client was built without one".into(),
                ))
            }
            _ => None,
        };
        let mut openvm = OpenVmCheck {
            slot: guard.as_deref_mut().unwrap_or(&mut none),
            program,
            wait: self.openvm_wait,
            unavailable: None,
            other_keys: None,
        };
        let mut check = BySystem {
            sp1: Sp1Check::default(),
            openvm: &mut openvm,
        };
        let r = check_answer(trust, response, filter, &mut fetch, &mut check);
        if let Some(why) = openvm.other_keys.take() {
            return Ran::Done(json!({"ok": false, "kind": "internal", "message": why}));
        }
        if let Some(why) = openvm.unavailable.take() {
            // The helper didn't start (missing or broken): not a verdict.
            return Ran::NoVerifier(NoVerifier(format!("no openvm verifier module: {why}")));
        }
        Ran::Done(verified_json(&r))
    }
}

/// The OpenVM helper's binary name.
#[cfg(feature = "logs")]
const OPENVM_HELPER: &str = "willow-logs-verify-openvm";

/// The helper `path` names, else `willow-logs-verify-openvm` next to this executable, else on `PATH`.
#[cfg(feature = "logs")]
fn helper_program(path: &Option<std::path::PathBuf>) -> std::path::PathBuf {
    path.clone().unwrap_or_else(|| {
        std::env::current_exe()
            .ok()
            .and_then(|p| Some(p.parent()?.join(OPENVM_HELPER)))
            .filter(|p| p.is_file())
            .unwrap_or_else(|| OPENVM_HELPER.into())
    })
}

/// The OpenVM half of a log check. A helper that doesn't start is unavailable (not a verdict); one
/// that fails once it has a proof fails that check, as a throw from the page's OpenVM module does,
/// and is started afresh for the next.
#[cfg(feature = "logs")]
struct OpenVmCheck<'a> {
    slot: &'a mut Option<Helper>,
    program: Option<std::path::PathBuf>,
    wait: std::time::Duration,
    unavailable: Option<String>,
    other_keys: Option<String>,
}

#[cfg(feature = "logs")]
impl logset_query::ProofCheck for &mut OpenVmCheck<'_> {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        if !is_openvm(proof) {
            return Err("not an OpenVM proof".into());
        }
        let Some(program) = &self.program else {
            let e = "no OpenVM verifier".to_string();
            self.unavailable = Some(e.clone());
            return Err(e);
        };
        // A helper that exited while idle is started again, not asked (its next verdict would be lost).
        if self.slot.as_mut().is_some_and(|h| !h.alive()) {
            *self.slot = None;
        }
        if self.slot.is_none() {
            match Helper::start(program, self.wait) {
                Ok(h) => *self.slot = Some(h),
                Err(Start::Unavailable(e)) => {
                    self.unavailable = Some(e.clone());
                    return Err(e);
                }
                Err(Start::OtherKeys(e)) => {
                    self.other_keys = Some(e.clone());
                    return Err(e);
                }
            }
        }
        let h = self.slot.as_mut().expect("started");
        match h.ask(vkey, proof, public_values, self.wait) {
            Ok(verdict) => verdict,
            Err(e) => {
                *self.slot = None;
                Err(format!(
                    "the OpenVM verifier {} failed on a proof: {e}",
                    program.display()
                ))
            }
        }
    }
}

/// `willow-logs-verify-openvm serve`, over its line protocol
/// (`zkvm/logset-guest-openvm/verify/src/serve.rs`): `ready <keys>` first, then per proof
/// `<key> <public values> <proof>` in hex and `ok` or `fail <why>`.
#[cfg(feature = "logs")]
pub(crate) struct Helper {
    child: std::process::Child,
    /// Requests for the helper's stdin, written on a thread of its own (a helper that stops
    /// reading can't block the client: the verdict's wait times out and the helper is killed).
    requests: Option<std::sync::mpsc::Sender<String>>,
    /// The helper's stdout, line by line, from a thread of its own (so a read can time out).
    lines: std::sync::mpsc::Receiver<std::io::Result<String>>,
}

#[cfg(feature = "logs")]
enum Start {
    Unavailable(String),
    OtherKeys(String),
}

#[cfg(feature = "logs")]
impl Helper {
    fn start(program: &std::path::Path, wait: std::time::Duration) -> Result<Helper, Start> {
        use std::process::{Command, Stdio};
        use willow_logs_verify_openvm::pinned::{self, Kind};
        let shown = program.display();
        let mut child = Command::new(program)
            .arg("serve")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .map_err(|e| Start::Unavailable(format!("{shown}: {e}")))?;
        let mut stdin = child.stdin.take().expect("piped");
        let (requests, to_write) = std::sync::mpsc::channel::<String>();
        std::thread::spawn(move || {
            use std::io::Write;
            for r in to_write {
                if stdin
                    .write_all(r.as_bytes())
                    .and_then(|_| stdin.flush())
                    .is_err()
                {
                    break;
                }
            }
        });
        let stdout = child.stdout.take().expect("piped");
        let (tx, lines) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            use std::io::BufRead;
            for l in std::io::BufReader::new(stdout).lines() {
                let failed = l.is_err();
                if tx.send(l).is_err() || failed {
                    break;
                }
            }
        });
        let mut h = Helper {
            child,
            requests: Some(requests),
            lines,
        };
        let line = h
            .line(wait)
            .map_err(|e| Start::Unavailable(format!("{shown}: {e}")))?;
        // It must verify under exactly the keys this build trusts OpenVM parts with.
        let want: Vec<String> = Kind::ALL
            .iter()
            .map(|k| pinned::key(*k).map(hex::encode))
            .collect::<Result<_, _>>()
            .map_err(Start::Unavailable)?;
        match line.split_once(' ') {
            Some(("ready", keys)) if keys.split(' ').eq(want.iter().map(String::as_str)) => Ok(h),
            Some(("ready", keys)) => Err(Start::OtherKeys(format!(
                "the OpenVM verifier {shown} trusts other programs ({keys}) than this build ({}); \
                 use the willow-logs-verify-openvm built with it",
                want.join(" ")
            ))),
            Some(("error", why)) => Err(Start::Unavailable(format!("{shown}: {why}"))),
            _ => Err(Start::Unavailable(format!("{shown} said {line:?}"))),
        }
    }

    fn alive(&mut self) -> bool {
        matches!(self.child.try_wait(), Ok(None))
    }

    fn line(&mut self, wait: std::time::Duration) -> Result<String, String> {
        use std::sync::mpsc::RecvTimeoutError;
        match self.lines.recv_timeout(wait) {
            Ok(Ok(l)) => Ok(l.trim_end().to_string()),
            Ok(Err(e)) => Err(e.to_string()),
            Err(RecvTimeoutError::Disconnected) => Err("stopped".into()),
            Err(RecvTimeoutError::Timeout) => Err(format!("no response in {wait:?}")),
        }
    }

    /// The helper's verdict on one proof, or why it gave none.
    fn ask(
        &mut self,
        vkey: &[u8; 32],
        proof: &[u8],
        pv: &[u8],
        wait: std::time::Duration,
    ) -> Result<Result<(), String>, String> {
        let request = format!(
            "{} {} {}\n",
            hex::encode(vkey),
            hex::encode(pv),
            hex::encode(proof)
        );
        self.requests
            .as_ref()
            .ok_or("stopped")?
            .send(request)
            .map_err(|_| "stopped".to_string())?;
        let line = self.line(wait)?;
        match line.split_once(' ').unwrap_or((&line, "")) {
            ("ok", "") => Ok(Ok(())),
            ("fail", why) => Ok(Err(why.to_string())),
            _ => Err(format!("said {line:?}")),
        }
    }
}

#[cfg(feature = "logs")]
impl Drop for Helper {
    fn drop(&mut self) {
        // End of input ends `serve`; kill it only if it does not stop.
        drop(self.requests.take());
        if !matches!(self.child.try_wait(), Ok(Some(_))) {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if !matches!(self.child.try_wait(), Ok(Some(_))) {
                let _ = self.child.kill();
            }
        }
        let _ = self.child.wait();
    }
}
