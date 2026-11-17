








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


pub fn pinned_chains_json() -> Value {
    PINNED_CHAINS
        .iter()
        .map(|&(id, name)| json!({"chainId": id, "name": name}))
        .collect()
}



#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {

    Rejected,

    MalformedInput,

    InvalidFilter,

    InvalidKeys,

    UnknownChain,

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


    pub fn json(&self) -> Value {
        json!({"ok": false, "kind": self.kind.as_str(), "message": self.message})
    }
}


#[derive(Deserialize)]
struct Response<T> {
    result: T,
}



pub fn result_of<T: DeserializeOwned>(body: &str, method: &str) -> Result<T, Failure> {
    serde_json::from_str::<Response<T>>(body)
        .map(|r| r.result)
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} response: {e}")))
}


pub struct Outcome {

    pub cutover: Option<u64>,
    pub verified: Verified,
    pub from_block: u64,
    pub to_block: u64,
    pub segments: usize,

    pub blocks: usize,

    pub leaves: usize,
}




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
                "the server returned blocks {}-{}, not the requested {from}-{to}",
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


            "proofs": {"sp1": count(ProofSystem::Sp1), "openvm": count(ProofSystem::OpenVm)},
            "openvmFrom": quantity(v.openvm_from),
            "cutover": quantity(self.cutover),
            "firstParent": hash_json(v.first_parent),
            "checkpoints": v.checkpoints.iter().copied().map(hash_json).collect::<Vec<_>>(),
            "lastProvenBlock": hash_json(*v.checkpoints.last().expect("verify returns a part")),
        })
    }



    pub fn compare(&self, response: &str) -> Result<Value, Failure> {
        let answer: Value = result_of(response, "eth_getLogs")?;
        let answer = answer.as_array().ok_or_else(|| {
            Failure::new(Kind::MalformedInput, "eth_getLogs response is not an array")
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


pub fn compare_json(checked: Option<&Outcome>, response: &str) -> Value {
    let Some(o) = checked else {
        return Failure::new(Kind::Internal, "compare called without a verified response").json();
    };
    match o.compare(response) {
        Ok(result) => json!({"ok": true, "result": result}),
        Err(f) => f.json(),
    }
}



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


#[wasm_bindgen(js_name = pinnedChains)]
pub fn pinned_chains() -> String {
    pinned_chains_json().to_string()
}





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




    pub fn keys(&self) -> String {
        keys_json(&self.trust).to_string()
    }









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



    pub fn compare(&self, response: &str) -> String {
        compare_json(self.checked.as_ref(), response).to_string()
    }
}



pub struct JsOpenVm(pub Option<js_sys::Function>);


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






#[cfg(feature = "test-cutover")]
#[wasm_bindgen]
impl Verifier {


    #[wasm_bindgen(js_name = setTestCutover)]
    pub fn set_test_cutover(&mut self, block: Option<u64>) {
        if let Ok(t) = &mut self.trust {
            t.openvm_from = block;
        }
        self.checked = None;
    }
}



