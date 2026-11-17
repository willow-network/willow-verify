



use crate::call::Witness;
use crate::history::{History, HISTORY_ADDRESS};
use crate::proof;
use crate::wire;
use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, B256};
use alloy_trie::KECCAK_EMPTY;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};


pub const MAX_ACCOUNTS: usize = 256;
pub const MAX_SLOTS: usize = 8_192;

pub const MAX_NODE_BYTES: usize = 1_100;
pub const MAX_PROOF_NODES: usize = 64;

pub const MAX_SHARED_NODES: usize = 65_536;

fn limits(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Limits, m)
}

fn bad(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}


fn short(node: &Value) -> Result<&str, Failure> {
    let s = node
        .as_str()
        .ok_or_else(|| bad("a proof node is not a string"))?;
    match s.len() <= 2 + 2 * MAX_NODE_BYTES {
        true => Ok(s),
        false => Err(limits(format!(
            "a proof node is longer than {MAX_NODE_BYTES} bytes"
        ))),
    }
}



pub fn expand(w: &wire::Witness) -> Result<Vec<Value>, Failure> {
    if w.accounts.len() > MAX_ACCOUNTS {
        return Err(limits(format!(
            "{} accounts, more than {MAX_ACCOUNTS}",
            w.accounts.len()
        )));
    }
    if w.nodes.len() > MAX_SHARED_NODES {
        return Err(limits(format!(
            "{} shared nodes, more than {MAX_SHARED_NODES}",
            w.nodes.len()
        )));
    }
    for n in &w.nodes {
        if n.len() > 2 + 2 * MAX_NODE_BYTES {
            return Err(limits(format!(
                "a shared node is longer than {MAX_NODE_BYTES} bytes"
            )));
        }
    }
    let slots: usize = w
        .accounts
        .iter()
        .map(|a| a["storageProof"].as_array().map_or(0, Vec::len))
        .sum();
    if slots > MAX_SLOTS {
        return Err(limits(format!("{slots} slots, more than {MAX_SLOTS}")));
    }
    let path = |v: &Value| -> Result<Value, Failure> {
        let list = v.as_array().ok_or_else(|| bad("a proof is not a list"))?;
        if list.len() > MAX_PROOF_NODES {
            return Err(limits(format!(
                "a proof of {} nodes, more than {MAX_PROOF_NODES}",
                list.len()
            )));
        }
        list.iter()
            .map(|n| {
                let s = short(n)?;
                match s.strip_prefix('#') {
                    None => Ok(Value::String(s.to_string())),
                    Some(i) => i
                        .parse::<usize>()
                        .ok()
                        .and_then(|i| w.nodes.get(i))
                        .map(|n| Value::String(n.clone()))
                        .ok_or_else(|| bad(format!("proof node {s} refers to no shared node"))),
                }
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    };
    let mut out = Vec::with_capacity(w.accounts.len());
    for a in &w.accounts {
        let mut a = a.clone();
        if !a.is_object() {
            return Err(bad("an account is not an object"));
        }
        a["accountProof"] = path(&a["accountProof"])?;
        if let Some(storage) = a.get_mut("storageProof").and_then(Value::as_array_mut) {
            for s in storage {
                if !s.is_object() {
                    return Err(bad("a storage proof is not an object"));
                }
                s["proof"] = path(&s["proof"])?;
            }
        }
        out.push(a);
    }
    Ok(out)
}









pub fn verify_on(
    history: History,
    state_root: B256,
    w: &wire::Witness,
    own_code: &HashSet<Address>,
) -> Result<Witness, Failure> {
    let accounts = expand(w)?;
    if w.codes.len() > MAX_ACCOUNTS {
        return Err(limits(format!(
            "{} codes, more than {MAX_ACCOUNTS}",
            w.codes.len()
        )));
    }
    let mut codes: HashMap<B256, Bytes> = HashMap::new();
    for (h, c) in &w.codes {
        let hash = proof::data(&json!(h))
            .ok()
            .filter(|b| b.len() == 32)
            .map(|b| B256::from_slice(&b))
            .ok_or_else(|| bad(format!("code hash {h} is not 32 bytes")))?;
        let code = proof::data(&json!(c)).map_err(bad)?;
        if keccak256(&code) != hash {
            return Err(Failure::new(
                Kind::Code,
                format!("the code given for {hash} does not hash to it"),
            ));
        }
        codes.insert(hash, code);
    }
    let pinned = history.code_hash();
    let mut seen = HashSet::new();
    let mut out = Witness::default();
    for a in &accounts {
        let address = proof::address(&a["address"]).map_err(bad)?;
        if !seen.insert(address) {
            return Err(bad(format!("account {address} twice")));
        }
        if address == HISTORY_ADDRESS {

            let named = proof::data(&a["codeHash"]).ok().filter(|b| b.len() == 32);
            if let Some(h) = named.map(|b| B256::from_slice(&b)) {
                if h != pinned && h != KECCAK_EMPTY && h != B256::ZERO {
                    return Err(Failure::new(
                        Kind::History,
                        format!(
                            "the history account's code hash is {h}, not this chain's EIP-2935's"
                        ),
                    ));
                }
            }
        }
        let storage = a["storageProof"]
            .as_array()
            .ok_or_else(|| bad(format!("account {address} has no storageProof")))?;
        let mut keys = Vec::with_capacity(storage.len());
        let mut unique = HashSet::new();
        for s in storage {
            let k = proof::slot(&s["key"]).map_err(bad)?;
            if !unique.insert(k) {
                return Err(bad(format!("slot {k} of {address} twice")));
            }
            keys.push(k);
        }
        let p = proof::check(state_root, address, &keys, a)?;
        if address == HISTORY_ADDRESS && p.exists && p.code_hash != pinned {
            return Err(Failure::new(
                Kind::History,
                "the history account's code is not EIP-2935's",
            ));
        }
        let code = match p.has_code() {
            true => match codes.get(&p.code_hash) {
                Some(c) => c.clone(),

                None if own_code.contains(&address) => {
                    out.add_without_code(p);
                    continue;
                }
                None => {
                    return Err(Failure::new(
                        Kind::Code,
                        format!("the code of {address} ({}) is left out", p.code_hash),
                    ))
                }
            },
            false => Bytes::new(),
        };
        out.add(p, code);
    }
    Ok(out)
}




































































































































































