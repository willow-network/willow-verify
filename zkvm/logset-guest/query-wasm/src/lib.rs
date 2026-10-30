//! The browser verifier (`web/verify`): [`logset_query::verify`] with
//! [`Sp1Check`], compiled to wasm32 and run in a Web Worker. The page fetches
//! the answer and the proofs; everything that decides "verified" is here, except
//! the verdict on each OpenVM proof, which comes from the page's OpenVM module
//! (`willow-logs-verify-openvm-wasm`, a second wasm module: OpenVM's verifier and
//! SP1's cannot share one dependency graph) through the `checkOpenVm` callback.
//! Which keys a proof is checked under, and whether its system is the right one
//! for its blocks, is decided here.

use logset_query::{
    compare, verify, BySystem, Keys, Log, LogFilter, LogsProof, ProofCheck, ProofPart, ProofSystem,
    Trust, Verified,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;
use willow_logs_verify::{
    openvm_keys, parse_vkey, pinned_chain, sp1_keys, trust_with, Sp1Check, PINNED_CHAINS,
};

/// Trust in both systems' pinned program keys on `chain_id`. A non-empty override
/// (hex keys separated by commas or whitespace) replaces that SP1 tier's keys; a
/// chain without pinned keys needs both, and then trusts no OpenVM program.
pub fn trust(chain_id: u64, segment_vkeys: &str, leaf_vkeys: &str) -> Result<Trust, Failure> {
    let keys = |s: &str| -> Result<Vec<[u8; 32]>, Failure> {
        s.split(|c: char| c == ',' || c.is_whitespace())
            .filter(|k| !k.is_empty())
            .map(parse_vkey)
            .collect::<Result<_, _>>()
            .map_err(|e| Failure::new(Kind::InvalidKeys, e))
    };
    let (seg, leaf) = (keys(segment_vkeys)?, keys(leaf_vkeys)?);
    trust_with(chain_id, seg, leaf).map_err(|e| Failure::new(Kind::UnknownChain, e))
}

/// The chains with pinned program keys: `[{"chainId", "name"}]` (one key pair per system serves all).
pub fn pinned_chains_json() -> Value {
    PINNED_CHAINS
        .iter()
        .map(|&(id, name)| json!({"chainId": id, "name": name}))
        .collect()
}

/// Why a check did not pass. The page chooses what to say by `kind` alone, never
/// by the message, which can carry text the server chose (proof ids, for one).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Willow's answer failed a check: a proof, a hash link, a record, the range.
    Rejected,
    /// Willow's answer could not be read: not JSON, nested too deeply, or not the expected shape.
    MalformedInput,
    /// The filter the page passed in could not be read (not about Willow's answer).
    InvalidFilter,
    /// The program keys entered under Advanced could not be read.
    InvalidKeys,
    /// The chain has no pinned program keys and not both tiers were entered.
    UnknownChain,
    /// The verifier was used out of order, a fault in the page. Never caused by an answer.
    Internal,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Rejected => "rejected",
            Kind::MalformedInput => "malformed-input",
            Kind::InvalidFilter => "invalid-filter",
            Kind::InvalidKeys => "invalid-keys",
            Kind::UnknownChain => "unknown-chain",
            Kind::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: Kind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Failure {
            kind,
            message: message.into(),
        }
    }

    /// `{"ok": false, "kind", "message"}`, as the page receives it.
    pub fn json(&self) -> Value {
        json!({"ok": false, "kind": self.kind.as_str(), "message": self.message})
    }
}

/// A JSON-RPC response body as the server sent it; the page turns `error` responses away first.
#[derive(Deserialize)]
struct Response<T> {
    result: T,
}

/// The `result` of a response body, parsed here so the page never re-serialises server data.
/// Anything unreadable, or nested past serde_json's 128 levels, is `malformed-input`.
pub fn result_of<T: DeserializeOwned>(body: &str, method: &str) -> Result<T, Failure> {
    serde_json::from_str::<Response<T>>(body)
        .map(|r| r.result)
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} answer: {e}")))
}

/// A checked answer and its shape.
pub struct Outcome {
    /// The chain's first OpenVM block, if the client knew it ([`Trust::openvm_from`]).
    pub cutover: Option<u64>,
    pub verified: Verified,
    pub from_block: u64,
    pub to_block: u64,
    pub segments: usize,
    /// One-block leaf parts.
    pub blocks: usize,
    /// Multi-block leaf parts (a fast chain's segment of `S` blocks each).
    pub leaves: usize,
}

/// Check `response` (a `willow_getLogsProof` response body) against `filter`, whose
/// `fromBlock`/`toBlock` must be resolved `0x` numbers: the range comes from
/// the client, never from the server's answer.
pub fn check_answer(
    trust: &Trust,
    response: &str,
    filter: &str,
    fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    check: &mut dyn ProofCheck,
) -> Result<Outcome, Failure> {
    let bad_filter = |m: String| Failure::new(Kind::InvalidFilter, m);
    let filter: Value =
        serde_json::from_str(filter).map_err(|e| bad_filter(format!("filter: {e}")))?;
    let block = |k: &str| -> Result<u64, Failure> {
        filter
            .get(k)
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|h| u64::from_str_radix(h, 16).ok())
            .ok_or_else(|| bad_filter(format!("filter {k} must be a resolved 0x block number")))
    };
    let (from, to) = (block("fromBlock")?, block("toBlock")?);
    let f = LogFilter::from_eth_filter(&filter, from, to).map_err(bad_filter)?;
    let p: LogsProof = result_of(response, "willow_getLogsProof")?;
    if (p.from_block, p.to_block) != (from, to) {
        return Err(Failure::new(
            Kind::Rejected,
            format!(
                "the server answered blocks {}-{}, not the requested {from}-{to}",
                p.from_block, p.to_block
            ),
        ));
    }
    let verified = verify(&p, &f, trust, fetch, check)
        .map_err(|e| Failure::new(Kind::Rejected, e.to_string()))?;
    let count = |f: fn(&ProofPart) -> bool| p.parts.iter().filter(|x| f(x)).count();
    Ok(Outcome {
        cutover: trust.openvm_from,
        verified,
        from_block: from,
        to_block: to,
        segments: count(|x| matches!(x, ProofPart::Segment(_))),
        blocks: count(|x| matches!(x, ProofPart::Block(_))),
        leaves: count(|x| matches!(x, ProofPart::Leaf(_))),
    })
}

pub fn log_json(l: &Log) -> Value {
    json!({
        "address": format!("0x{}", hex::encode(l.address)),
        "topics": l.topics.iter().map(|t| format!("0x{}", hex::encode(t))).collect::<Vec<_>>(),
        "data": format!("0x{}", hex::encode(&l.data)),
        "blockNumber": format!("0x{:x}", l.block),
        "transactionIndex": format!("0x{:x}", l.tx_index),
        "logIndex": format!("0x{:x}", l.log_index),
    })
}

fn hash_json((number, hash): (u64, [u8; 32])) -> Value {
    json!({"number": format!("0x{number:x}"), "hash": format!("0x{}", hex::encode(hash))})
}

fn quantity(n: Option<u64>) -> Value {
    n.map_or(Value::Null, |n| json!(format!("0x{n:x}")))
}

impl Outcome {
    pub fn report(&self) -> Value {
        let v = &self.verified;
        let count = |s| v.systems.iter().filter(|&&x| x == s).count();
        json!({
            "fromBlock": format!("0x{:x}", self.from_block),
            "toBlock": format!("0x{:x}", self.to_block),
            "logs": v.logs.len(),
            "parts": {"segments": self.segments, "blocks": self.blocks, "leaves": self.leaves},
            // Proofs per system; where the answer's OpenVM parts start; the chain's pinned
            // first OpenVM block, which the split was checked against (null: not pinned).
            "proofs": {"sp1": count(ProofSystem::Sp1), "openvm": count(ProofSystem::OpenVm)},
            "openvmFrom": quantity(v.openvm_from),
            "cutover": quantity(self.cutover),
            "firstParent": hash_json(v.first_parent),
            "checkpoints": v.checkpoints.iter().copied().map(hash_json).collect::<Vec<_>>(),
            "lastProvenBlock": hash_json(*v.checkpoints.last().expect("verify returns a part")),
        })
    }

    /// How a plain `eth_getLogs` answer (a response body whose `result` is a JSON array)
    /// compares with the proven logs.
    pub fn compare(&self, response: &str) -> Result<Value, Failure> {
        let answer: Value = result_of(response, "eth_getLogs")?;
        let answer = answer.as_array().ok_or_else(|| {
            Failure::new(Kind::MalformedInput, "eth_getLogs answer is not an array")
        })?;
        let c = compare(&self.verified.logs, answer);
        Ok(json!({
            "exact": c.is_exact(),
            "matched": c.matched,
            "missing": c.missing.iter().map(log_json).collect::<Vec<_>>(),
            "unproven": c.unproven,
        }))
    }
}

/// What `Verifier.verify` returns: `{"ok": true, "report", "logs"}` or a [`Failure`].
pub fn verified_json(r: &Result<Outcome, Failure>) -> Value {
    match r {
        Ok(o) => json!({
            "ok": true,
            "report": o.report(),
            "logs": o.verified.logs.iter().map(log_json).collect::<Vec<_>>(),
        }),
        Err(f) => f.json(),
    }
}

/// What `Verifier.compare` returns: `{"ok": true, "result"}` or a [`Failure`].
pub fn compare_json(checked: Option<&Outcome>, response: &str) -> Value {
    let Some(o) = checked else {
        return Failure::new(Kind::Internal, "compare called without a verified answer").json();
    };
    match o.compare(response) {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(f) => f.json(),
    }
}

/// What `Verifier.keys` returns: each system's program keys in force and pinned, and the
/// chain's pinned first OpenVM block (null: not pinned), or a [`Failure`].
pub fn keys_json(trust: &Result<Trust, Failure>) -> Value {
    let keys = |k: &Keys| {
        let hex = |v: &[[u8; 32]]| v.iter().map(hex::encode).collect::<Vec<_>>();
        json!({"segment": hex(&k.segment), "leaf": hex(&k.leaf)})
    };
    match trust {
        Ok(t) => json!({
            "ok": true,
            "chainId": t.chain_id,
            "chain": pinned_chain(t.chain_id),
            "sp1": keys(&t.sp1),
            "openvm": keys(&t.openvm),
            "openvmFrom": quantity(t.openvm_from),
            "pinned": {"sp1": keys(&sp1_keys()), "openvm": keys(&openvm_keys())},
        }),
        Err(f) => f.json(),
    }
}

/// The chains with pinned program keys, as JSON: `[{"chainId", "name"}]`.
#[wasm_bindgen(js_name = pinnedChains)]
pub fn pinned_chains() -> String {
    pinned_chains_json().to_string()
}

/// The JS face: `new Verifier(chainId, segmentKeys, leafKeys)` (the keys: SP1 overrides).
/// No method throws for a bad answer, bad keys or a bad filter: each returns JSON with `ok`
/// and, when it is false, `kind` and `message`. Anything thrown is the runtime itself
/// failing (a wasm trap, out of memory), never a verdict on the answer.
#[wasm_bindgen]
pub struct Verifier {
    trust: Result<Trust, Failure>,
    checked: Option<Outcome>,
}

#[wasm_bindgen]
impl Verifier {
    #[wasm_bindgen(constructor)]
    pub fn new(chain_id: u32, segment_vkeys: &str, leaf_vkeys: &str) -> Verifier {
        Verifier {
            trust: trust(chain_id.into(), segment_vkeys, leaf_vkeys),
            checked: None,
        }
    }

    /// The program keys in force: `{"ok": true, "chainId", "chain", "sp1", "openvm",
    /// "openvmFrom", "pinned"}` (each system's `{"segment", "leaf"}`), or
    /// `{"ok": false, "kind": "invalid-keys" | "unknown-chain", "message"}`.
    pub fn keys(&self) -> String {
        keys_json(&self.trust).to_string()
    }

    /// Checks `response` (the `willow_getLogsProof` response body, as the server sent it)
    /// against `filter`. `getProof(id)` returns the proof bytes (`Uint8Array`);
    /// `onChecked(id, error, system)` runs after each proof check (`error` is `undefined`
    /// when it passed; `system` is `"sp1"` or `"openvm"`). `checkOpenVm(key, proof,
    /// publicValues)` (each a `Uint8Array`) verifies one OpenVM proof in the OpenVM module
    /// and returns `undefined` when it holds, else the reason; without it, every OpenVM
    /// proof fails. Returns `{"ok": true, "report", "logs"}` (every proven log as
    /// `eth_getLogs` JSON, without block and transaction hashes) or a failure.
    pub fn verify(
        &mut self,
        response: &str,
        filter: &str,
        get_proof: &js_sys::Function,
        on_checked: &js_sys::Function,
        check_openvm: Option<js_sys::Function>,
    ) -> String {
        self.checked = None;
        let trust = match &self.trust {
            Ok(t) => t,
            Err(f) => return f.json().to_string(),
        };
        let current = std::cell::RefCell::new(String::new());
        let mut fetch = |id: &str| -> Result<Vec<u8>, String> {
            *current.borrow_mut() = id.to_string();
            let v = get_proof
                .call1(&JsValue::NULL, &JsValue::from_str(id))
                .map_err(|e| format!("proof {id}: {}", js_text(&e)))?;
            if !v.is_instance_of::<js_sys::Uint8Array>() {
                return Err(format!("proof {id} was not fetched"));
            }
            Ok(js_sys::Uint8Array::from(v).to_vec())
        };
        let mut check = Reporting {
            inner: BySystem {
                sp1: Sp1Check::default(),
                openvm: JsOpenVm(check_openvm),
            },
            current: &current,
            on_checked,
        };
        let r = check_answer(trust, response, filter, &mut fetch, &mut check);
        let out = verified_json(&r).to_string();
        self.checked = r.ok();
        out
    }

    /// Compares a plain `eth_getLogs` answer (its response body, as the server sent it) with
    /// the proven logs of the last `verify` that passed: `{"ok": true, "result"}` or a failure.
    pub fn compare(&self, response: &str) -> String {
        compare_json(self.checked.as_ref(), response).to_string()
    }
}

/// An OpenVM proof check in the page's OpenVM module: `checkOpenVm(key, proof, publicValues)`
/// returns `undefined` when the proof holds. Anything else, a throw included, is a failure.
pub struct JsOpenVm(pub Option<js_sys::Function>);

/// Why an OpenVM proof fails when the page gave no OpenVM check.
pub const NO_OPENVM: &str = "this verifier has no OpenVM module, so it cannot check OpenVM proofs";

impl ProofCheck for JsOpenVm {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        let f = self.0.as_ref().ok_or(NO_OPENVM)?;
        let bytes = |b: &[u8]| JsValue::from(js_sys::Uint8Array::from(b));
        let r = f
            .call3(
                &JsValue::NULL,
                &bytes(vkey),
                &bytes(proof),
                &bytes(public_values),
            )
            .map_err(|e| format!("OpenVM check: {}", js_text(&e)))?;
        if r.is_undefined() {
            Ok(())
        } else {
            Err(r
                .as_string()
                .unwrap_or_else(|| "OpenVM check failed".into()))
        }
    }
}

/// Each system's check, that tells the page after each one.
struct Reporting<'a> {
    inner: BySystem<Sp1Check, JsOpenVm>,
    current: &'a std::cell::RefCell<String>,
    on_checked: &'a js_sys::Function,
}

impl ProofCheck for Reporting<'_> {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        let r = self.inner.check(vkey, proof, public_values);
        let err = r
            .as_ref()
            .err()
            .map_or(JsValue::UNDEFINED, |e| JsValue::from_str(e));
        let system = ProofSystem::of(proof).map_or(JsValue::UNDEFINED, |s| s.as_str().into());
        let id = JsValue::from_str(&self.current.borrow());
        let _ = self.on_checked.call3(&JsValue::NULL, &id, &err, &system);
        r
    }
}

fn js_text(v: &JsValue) -> String {
    v.as_string()
        .or_else(|| {
            v.dyn_ref::<js_sys::Error>()
                .map(|e| String::from(e.message()))
        })
        .unwrap_or_else(|| format!("{v:?}"))
}

/// The page's test build only (feature `test-cutover`, which `web/verify/build.sh` turns on for
/// `CHECKPOINT=test` alone, and checks is absent from every other build): a test cutover in place
/// of the chain's pinned first OpenVM block, so `test/real-openvm.mjs` can verify real OpenVM
/// answers from before the pin end to end. Kept below everything else in this file, so the
/// production build's code (its panic locations included) is the same with or without it.
#[cfg(feature = "test-cutover")]
#[wasm_bindgen]
impl Verifier {
    /// Replaces the chain's first OpenVM block for this verifier (`undefined`: none, so an
    /// answer may switch once anywhere). No effect on a verifier whose keys were refused.
    #[wasm_bindgen(js_name = setTestCutover)]
    pub fn set_test_cutover(&mut self, block: Option<u64>) {
        if let Ok(t) = &mut self.trust {
            t.openvm_from = block;
        }
        self.checked = None;
    }
}

#[cfg(test)]
mod tests;
