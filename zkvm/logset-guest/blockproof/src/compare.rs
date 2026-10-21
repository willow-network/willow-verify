//! Another endpoint's answer (`eth_getTransactionByHash`,
//! `eth_getTransactionReceipt`, `eth_getBlockReceipts`, ...) against the JSON
//! rendered from a verified proof.

use crate::render::Rendered;
use serde_json::Value;

/// How an answer compares with the proven JSON.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    /// Proven values the answer reports the same way.
    pub matched: usize,
    /// Proven values the answer reports differently or leaves out:
    /// `path: proven …, answer …`.
    pub differ: Vec<String>,
    /// Fields of the answer the proof does not determine (passed-through
    /// fields, fields the proof lacks the items for, fields we don't serve).
    pub unproven: Vec<String>,
    /// Proven fields the answer leaves out that some nodes never report
    /// (`blockTimestamp`; a deposit's `nonce` and `depositReceiptVersion` on
    /// some OP Mainnet nodes).
    pub omitted: Vec<String>,
}

impl Comparison {
    /// Every proven value the answer reports is the proven one.
    pub fn is_exact(&self) -> bool {
        self.differ.is_empty()
    }
}

/// Compare `answer` with `proven`, field by field.
pub fn compare(proven: &Rendered, answer: &Value) -> Comparison {
    let mut c = Comparison::default();
    walk(&proven.json, answer, "", &proven.unproven, &mut c);
    c
}

/// `path` with every array index replaced by `[]`: the form of
/// [`Rendered::unproven`] entries.
fn pattern(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut skipping = false;
    for ch in path.chars() {
        match ch {
            '[' => {
                out.push('[');
                skipping = true;
            }
            ']' => {
                out.push(']');
                skipping = false;
            }
            _ if skipping => {}
            _ => out.push(ch),
        }
    }
    out
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// A proven field some nodes leave out: `blockTimestamp` anywhere, and a
/// deposit transaction's `nonce` and `depositReceiptVersion`.
fn may_omit(key: &str, ours: &serde_json::Map<String, Value>) -> bool {
    let deposit = ours.get("type").is_some_and(|t| t == "0x7e") && ours.contains_key("sourceHash");
    key == "blockTimestamp" || (deposit && matches!(key, "nonce" | "depositReceiptVersion"))
}

fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(x), Value::String(y)) if x.starts_with("0x") && y.starts_with("0x") => {
            x.eq_ignore_ascii_case(y)
        }
        _ => a == b,
    }
}

fn walk(ours: &Value, theirs: &Value, path: &str, unproven: &[String], c: &mut Comparison) {
    match (ours, theirs) {
        (Value::Object(a), Value::Object(b)) => {
            let mut keys: Vec<&String> = a.keys().chain(b.keys()).collect();
            keys.sort();
            keys.dedup();
            for k in keys {
                let at = join(path, k);
                if unproven.contains(&pattern(&at)) {
                    if b.contains_key(k) {
                        c.unproven.push(at);
                    }
                    continue;
                }
                match (a.get(k), b.get(k)) {
                    (Some(x), Some(y)) => {
                        // A log's blockTimestamp of 0x0 (Robinhood Chain's own RPC).
                        if k == "blockTimestamp" && y == "0x0" && path.contains("logs[") {
                            c.omitted.push(at);
                            continue;
                        }
                        walk(x, y, &at, unproven, c)
                    }
                    (Some(_), None) if may_omit(k, a) => c.omitted.push(at),
                    (Some(x), None) => c
                        .differ
                        .push(format!("{at}: proven {x}, the answer has none")),
                    (None, Some(_)) => c.unproven.push(at),
                    (None, None) => {}
                }
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                c.differ.push(format!(
                    "{path}: {} proven entries, the answer has {}",
                    a.len(),
                    b.len()
                ));
                return;
            }
            for (i, (x, y)) in a.iter().zip(b).enumerate() {
                walk(x, y, &format!("{path}[{i}]"), unproven, c);
            }
        }
        (a, b) if same(a, b) => c.matched += 1,
        (a, b) => c.differ.push(format!("{path}: proven {a}, the answer {b}")),
    }
}
