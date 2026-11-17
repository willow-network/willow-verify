








use serde::Deserialize;
use serde_json::{json, Value};
use state_verify::wire::estimate_algorithm;
use state_verify::{
    call_request, proof, verify_call, verify_estimate, verify_state, At, Failure, Kind, Named,
    MAX_PROOF_SLOTS,
};
use wasm_bindgen::prelude::*;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateKind {


    State,

    Call,

    Estimate,


    Trace,
}

impl StateKind {
    pub fn parse(s: &str) -> Option<StateKind> {
        match s {
            "state" => Some(StateKind::State),
            "call" => Some(StateKind::Call),
            "estimate" => Some(StateKind::Estimate),
            "trace" => Some(StateKind::Trace),
            _ => None,
        }
    }

    pub fn method(self) -> &'static str {
        match self {
            StateKind::State => "willow_getStateProof",
            StateKind::Call => "willow_getCallProof",
            StateKind::Estimate => "willow_getEstimateGasProof",
            StateKind::Trace => "willow_getTraceProof",
        }
    }
}

fn invalid(m: impl Into<String>) -> Failure {
    Failure::new(Kind::InvalidRequest, m)
}

fn malformed(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}

fn internal(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Internal, m)
}










pub fn request(chain_id: u64, kind: StateKind, target: &str) -> Result<Value, Failure> {
    let params: Value = serde_json::from_str(target)
        .map_err(|e| invalid(format!("the params are not JSON: {e}")))?;
    let list = params
        .as_array()
        .ok_or_else(|| invalid("expected a list of params"))?;
    match kind {
        StateKind::State => {
            if !(3..=4).contains(&list.len()) {
                return Err(invalid(
                    "expected [address, storageKeys, block, {\"code\": bool}]",
                ));
            }
            proof::address(&list[0]).map_err(invalid)?;
            let keys = list[1]
                .as_array()
                .ok_or_else(|| invalid("expected a list of storage keys"))?;
            if keys.len() > MAX_PROOF_SLOTS {
                return Err(invalid(format!(
                    "at most {MAX_PROOF_SLOTS} storage keys per state proof"
                )));
            }
            for k in keys {
                proof::slot(k).map_err(invalid)?;
            }
            At::parse(&list[2])?;
            match list.get(3) {
                None | Some(Value::Null) => {}
                Some(Value::Object(o)) if o.get("code").is_none_or(Value::is_boolean) => {}
                Some(o) => return Err(invalid(format!("expected {{\"code\": bool}}, not {o}"))),
            }
        }
        StateKind::Call | StateKind::Estimate => {
            let req = call_request(chain_id, &params)?;
            if kind == StateKind::Estimate && !state_verify::estimate::estimated_on(req.engine) {
                return Err(Failure::new(
                    Kind::Unsupported,
                    format!("estimates are not verified on chain {chain_id}"),
                ));
            }
            if kind == StateKind::Estimate {
                if let Some(why) = state_verify::estimate::not_searched(&req) {
                    return Err(Failure::new(Kind::Unsupported, why));
                }
            }
        }
        StateKind::Trace => {
            trace_verify::request::parse(chain_id, &params).map_err(|f| {
                let kind = match f.kind {
                    trace_verify::Kind::Unsupported => Kind::Unsupported,
                    trace_verify::Kind::InvalidRequest => Kind::InvalidRequest,
                    _ => Kind::Internal,
                };
                Failure::new(kind, f.message)
            })?;
        }
    }
    Ok(params)
}


#[derive(Deserialize)]
struct Response {
    result: Value,
}


#[derive(Clone, Debug, PartialEq)]
pub struct Verified {
    pub report: Value,


    pub proven: Value,
}

fn block_json(b: &Named) -> Value {
    json!({
        "number": format!("0x{:x}", b.number),
        "hash": format!("0x{}", hex::encode(b.hash.as_slice())),
        "timestamp": b.header.timestamp,
        "parentHash": format!("0x{}", hex::encode(b.header.parent_hash.as_slice())),
    })
}



pub fn verify(
    chain_id: u64,
    kind: StateKind,
    params: &Value,
    response: &str,
) -> Result<Verified, Failure> {
    let r: Response = serde_json::from_str(response)
        .map_err(|e| malformed(format!("{} response: {e}", kind.method())))?;
    let (block, proven) = match kind {
        StateKind::State => {
            let p = verify_state(chain_id, params, &r.result)?;
            (p.block, p.responses)
        }
        StateKind::Call => {
            let p = verify_call(chain_id, params, &r.result)?;
            (p.block, p.response)
        }
        StateKind::Estimate => {
            let p = verify_estimate(chain_id, params, &r.result)?;
            (p.block, p.response)
        }
        StateKind::Trace => return Err(internal("a trace proof is verified by verify_trace")),
    };
    let mut report = json!({
        "method": kind.method(),
        "chainId": chain_id,
        "block": block_json(&block),
    });
    if kind == StateKind::Estimate {
        report["algorithm"] = json!(estimate_algorithm(chain_id));
    }
    Ok(Verified { report, proven })
}




pub fn verify_trace(
    chain_id: u64,
    params: &Value,
    response: &str,
) -> Result<Verified, trace_verify::Failure> {
    let r: Response = serde_json::from_str(response).map_err(|e| {
        trace_verify::Failure::new(
            trace_verify::Kind::Malformed,
            format!("willow_getTraceProof response: {e}"),
        )
    })?;
    let p = trace_verify::verify_trace(chain_id, params, &r.result)?;
    let report = json!({
        "method": StateKind::Trace.method(),
        "chainId": chain_id,
        "block": {
            "number": format!("0x{:x}", p.number),
            "hash": format!("0x{}", hex::encode(p.hash.as_slice())),
            "timestamp": p.timestamp,
            "parentHash": format!("0x{}", hex::encode(p.parent_hash.as_slice())),
        },
        "traced": p.method,
        "algorithm": trace_verify::ALGORITHM,
    });
    Ok(Verified {
        report,
        proven: json!({"result": p.result}),
    })
}


fn differences(path: &str, a: &Value, b: &Value, room: usize, out: &mut Vec<String>) {
    if out.len() >= room || a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().chain(y.keys().filter(|k| !x.contains_key(*k))) {
                let (u, v) = (
                    x.get(k).unwrap_or(&Value::Null),
                    y.get(k).unwrap_or(&Value::Null),
                );
                differences(&format!("{path}.{k}"), u, v, room, out);
            }
        }
        (Value::Array(x), Value::Array(y)) if x.len() == y.len() => {
            for (i, (u, v)) in x.iter().zip(y).enumerate() {
                differences(&format!("{path}[{i}]"), u, v, room, out);
            }
        }
        _ => out.push(format!(
            "{path}: proven {}, the response {}",
            shown(a),
            shown(b)
        )),
    }
}


const SHOWN: usize = 200;

fn shown(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() <= SHOWN {
        return s;
    }
    let cut: String = s.chars().take(SHOWN).collect();
    format!("{cut}…")
}


enum Theirs<'a> {
    Result(&'a Value),
    Error(&'a Value),
}

fn theirs(body: &Value) -> Result<Theirs<'_>, Failure> {
    match (body.get("result"), body.get("error")) {
        (Some(r), _) => Ok(Theirs::Result(r)),
        (None, Some(e)) => Ok(Theirs::Error(e)),
        _ => Err(malformed("the response has neither a result nor an error")),
    }
}


#[derive(Clone, Copy)]
enum As {
    Number,
    Data,
}

fn same(how: As, a: &Value, b: &Value) -> bool {
    match how {
        As::Number => matches!(
            (proof::quantity(a), proof::quantity(b)),
            (Ok(x), Ok(y)) if x == y
        ),
        As::Data => matches!(
            (proof::data(a), proof::data(b)),
            (Ok(x), Ok(y)) if x == y
        ),
    }
}


fn value(path: &str, how: As, mine: &Value, other: Option<&Value>, differ: &mut Vec<String>) {
    match other {
        Some(o) if same(how, mine, o) => {}
        Some(o) => differ.push(format!(
            "{path}: proven {}, the response {}",
            shown(mine),
            shown(o)
        )),
        None => differ.push(format!(
            "{path}: proven {}, the response has none",
            shown(mine)
        )),
    }
}


fn list(path: &str, how: As, mine: &Value, other: Option<&Value>, differ: &mut Vec<String>) {
    let (Some(a), Some(b)) = (mine.as_array(), other.and_then(Value::as_array)) else {
        differ.push(format!(
            "{path}: proven a list, the response {}",
            other.map_or("none".into(), shown)
        ));
        return;
    };
    if a.len() != b.len() {
        differ.push(format!(
            "{path}: {} proven entries, the response has {}",
            a.len(),
            b.len()
        ));
        return;
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        value(&format!("{path}[{i}]"), how, x, Some(y), differ);
    }
}


fn account(mine: &Value, other: &Value, differ: &mut Vec<String>) {
    for (k, how) in [
        ("address", As::Data),
        ("balance", As::Number),
        ("nonce", As::Number),
        ("codeHash", As::Data),
        ("storageHash", As::Data),
    ] {
        value(k, how, &mine[k], other.get(k), differ);
    }
    list(
        "accountProof",
        As::Data,
        &mine["accountProof"],
        other.get("accountProof"),
        differ,
    );
    let (Some(a), Some(b)) = (
        mine["storageProof"].as_array(),
        other.get("storageProof").and_then(Value::as_array),
    ) else {
        differ.push("storageProof: the response has no list".into());
        return;
    };
    if a.len() != b.len() {
        differ.push(format!(
            "storageProof: {} proven entries, the response has {}",
            a.len(),
            b.len()
        ));
        return;
    }
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        let at = format!("storageProof[{i}]");
        value(
            &format!("{at}.key"),
            As::Number,
            &x["key"],
            y.get("key"),
            differ,
        );
        value(
            &format!("{at}.value"),
            As::Number,
            &x["value"],
            y.get("value"),
            differ,
        );
        list(
            &format!("{at}.proof"),
            As::Data,
            &x["proof"],
            y.get("proof"),
            differ,
        );
    }
}




fn outcome(method: &str, how: As, mine: &Value, other: &Theirs, differ: &mut Vec<String>) {
    match (mine.get("result"), mine.get("error"), other) {
        (Some(r), _, Theirs::Result(o)) => value(method, how, r, Some(o), differ),
        (Some(r), _, Theirs::Error(e)) => differ.push(format!(
            "{method}: proven the result {}, the response is the error {}",
            shown(r),
            shown(e)
        )),
        (None, Some(e), Theirs::Result(o)) => differ.push(format!(
            "{method}: proven the error {}, the response is the result {}",
            shown(e),
            shown(o)
        )),
        (None, Some(e), Theirs::Error(o)) => {
            if e.get("code") != o.get("code") {
                differ.push(format!(
                    "{method}: proven error code {}, the response's {}",
                    shown(&e["code"]),
                    o.get("code").map_or("none".into(), shown)
                ));
            }
            match (e.get("data"), o.get("data")) {
                (None, None) => {}
                (Some(d), Some(od)) if same(As::Data, d, od) => {}
                (d, od) => differ.push(format!(
                    "{method}: proven error data {}, the response's {}",
                    d.map_or("none".into(), shown),
                    od.map_or("none".into(), shown)
                )),
            }
        }
        _ => differ.push(format!("{method}: no proven response")),
    }
}



pub fn compare(
    kind: StateKind,
    proven: &Value,
    method: &str,
    response: &str,
) -> Result<Value, Failure> {
    let body: Value = serde_json::from_str(response)
        .map_err(|e| malformed(format!("the {method} response: {e}")))?;
    compared(kind, proven, method, &body)
}

fn compared<'a>(
    kind: StateKind,
    proven: &Value,
    method: &str,
    body: &'a Value,
) -> Result<Value, Failure> {
    let other = theirs(body)?;
    let mut differ = Vec::new();
    let result = |o: &Theirs<'a>, differ: &mut Vec<String>| -> Option<&'a Value> {
        match *o {
            Theirs::Result(r) => Some(r),
            Theirs::Error(e) => {
                differ.push(format!("{method}: the response is the error {}", shown(e)));
                None
            }
        }
    };
    match (kind, method) {
        (StateKind::State, "eth_getBalance" | "eth_getTransactionCount") => {
            if let Some(r) = result(&other, &mut differ) {
                value(method, As::Number, &proven[method], Some(r), &mut differ);
            }
        }
        (StateKind::State, "eth_getCode") => {
            if proven["eth_getCode"].is_null() {
                return Err(internal("the code was not asked for"));
            }
            if let Some(r) = result(&other, &mut differ) {
                value(method, As::Data, &proven[method], Some(r), &mut differ);
            }
        }
        (StateKind::State, "eth_getProof") => {
            if let Some(r) = result(&other, &mut differ) {
                account(&proven["eth_getProof"], r, &mut differ);
            }
        }
        (StateKind::State, m) if m.starts_with("eth_getStorageAt:") => {
            let mine = m["eth_getStorageAt:".len()..]
                .parse::<usize>()
                .ok()
                .and_then(|i| proven["eth_getStorageAt"].get(i))
                .ok_or_else(|| internal(format!("no proven storage key {m}")))?;
            if let Some(r) = result(&other, &mut differ) {
                value("eth_getStorageAt", As::Number, mine, Some(r), &mut differ);
            }
        }
        (StateKind::Call, "eth_call") => outcome(method, As::Data, proven, &other, &mut differ),
        (StateKind::Estimate, "eth_estimateGas") => {
            outcome(method, As::Number, proven, &other, &mut differ)
        }
        (StateKind::Trace, m) if trace_verify::request::PROVEN.contains(&m) => {
            if let Some(r) = result(&other, &mut differ) {
                differences(m, &proven["result"], r, 20, &mut differ);
            }
        }
        (k, m) => {
            return Err(internal(format!(
                "compare {m} after a {} proof",
                k.method()
            )))
        }
    }
    Ok(json!({"exact": differ.is_empty(), "differ": differ}))
}

fn json_of(r: Result<Value, Failure>) -> String {
    match r {
        Ok(v) => v.to_string(),
        Err(f) => f.json().to_string(),
    }
}




#[wasm_bindgen]
pub struct StateVerifier {
    chain_id: u64,
    asked: Option<(StateKind, Value)>,
    verified: Option<(StateKind, Value)>,
}

#[wasm_bindgen]
impl StateVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(chain_id: u32) -> StateVerifier {
        StateVerifier {
            chain_id: chain_id.into(),
            asked: None,
            verified: None,
        }
    }





    pub fn request(&mut self, kind: &str, target: &str) -> String {
        self.asked = None;
        self.verified = None;
        let Some(k) = StateKind::parse(kind) else {
            return internal(format!("unknown proof kind {kind}"))
                .json()
                .to_string();
        };
        json_of(request(self.chain_id, k, target).map(|params| {
            let out = json!({"ok": true, "method": k.method(), "params": params});
            self.asked = Some((k, params));
            out
        }))
    }



    pub fn verify(&mut self, response: &str) -> String {
        self.verified = None;
        let Some((k, params)) = &self.asked else {
            return internal("verify called before request").json().to_string();
        };
        let k = *k;
        if k == StateKind::Trace {
            return match verify_trace(self.chain_id, params, response) {
                Ok(v) => {
                    let out = json!({"ok": true, "report": v.report, "proven": v.proven});
                    self.verified = Some((k, v.proven));
                    out.to_string()
                }
                Err(f) => f.json().to_string(),
            };
        }
        match verify(self.chain_id, k, params, response) {
            Ok(v) => {
                let out = json!({"ok": true, "report": v.report, "proven": v.proven});
                self.verified = Some((k, v.proven));
                out.to_string()
            }
            Err(f) => f.json().to_string(),
        }
    }






    pub fn compare(&self, method: &str, response: &str) -> String {
        let Some((k, proven)) = &self.verified else {
            return internal("compare called without a verified proof")
                .json()
                .to_string();
        };
        json_of(compare(*k, proven, method, response).map(|r| json!({"ok": true, "result": r})))
    }
}



