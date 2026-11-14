//! A call proof's witness ([`wire::Witness`]): read within the server's limits, every account,
//! slot and code proven against the block's state root ([`verify`]); and, for the server, the
//! witness of exactly what a call read ([`Builder`]).

use crate::call::{Reads, Witness};
use crate::history::{self, HISTORY_ADDRESS};
use crate::proof::{self, hx, Proven};
use crate::wire;
use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, B256};
use alloy_trie::KECCAK_EMPTY;
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap, HashSet};

/// Accounts and slots one call may read (the server's own limits, state/mod.rs).
pub const MAX_ACCOUNTS: usize = 256;
pub const MAX_SLOTS: usize = 8_192;
/// The longest trie node, in bytes, and the most nodes one proof may have.
pub const MAX_NODE_BYTES: usize = 1_100;
pub const MAX_PROOF_NODES: usize = 64;
/// Shared nodes one witness may carry.
pub const MAX_SHARED_NODES: usize = 65_536;

fn limits(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Limits, m)
}

fn bad(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Malformed, m)
}

/// A node no longer than [`MAX_NODE_BYTES`] (it is decoded when its proof is verified).
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

/// The witness's accounts as plain `eth_getProof` objects: every `"#i"` replaced by `nodes[i]`,
/// within the limits.
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

/// `w` proven against `state_root`: each account (present or absent) and each of its slots, and
/// each account's code against its code hash. An account no run reads is proven and ignored.
pub fn verify(state_root: B256, w: &wire::Witness) -> Result<Witness, Failure> {
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
    let pinned = history::code_hash();
    let mut seen = HashSet::new();
    let mut out = Witness::default();
    for a in &accounts {
        let address = proof::address(&a["address"]).map_err(bad)?;
        if !seen.insert(address) {
            return Err(bad(format!("account {address} twice")));
        }
        if address == HISTORY_ADDRESS {
            // Checked before its proof: a history account with other code is never EIP-2935's.
            let named = proof::data(&a["codeHash"]).ok().filter(|b| b.len() == 32);
            if let Some(h) = named.map(|b| B256::from_slice(&b)) {
                if h != pinned && h != KECCAK_EMPTY && h != B256::ZERO {
                    return Err(Failure::new(
                        Kind::History,
                        format!("the history account's code hash is {h}, not EIP-2935's"),
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
            true => codes.get(&p.code_hash).cloned().ok_or_else(|| {
                Failure::new(
                    Kind::Code,
                    format!("the code of {address} ({}) is left out", p.code_hash),
                )
            })?,
            false => Bytes::new(),
        };
        out.add(p, code);
    }
    Ok(out)
}

/// What a server proves a call's reads with: verified provider proofs and each account's code,
/// pruned to the reads of the final run ([`Builder::build`]).
#[derive(Clone, Debug, Default)]
pub struct Builder {
    /// Each account's fields and account proof, and its storage proofs by slot.
    accounts: BTreeMap<Address, (Value, BTreeMap<B256, Value>)>,
    codes: BTreeMap<B256, Bytes>,
}

impl Builder {
    /// `p`, verified from the provider's `eth_getProof` answer `given`, with the code its code
    /// hash names. Proofs of one account in several parts (slots asked for in chunks) merge.
    pub fn add(&mut self, p: &Proven, given: &Value, code: &Bytes) {
        let keys = p
            .slots
            .iter()
            .map(|(k, _)| json!(hx(k.as_slice())))
            .collect();
        let mut answer = proof::response(p, given, keys);
        let storage: Vec<Value> = answer["storageProof"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        answer["storageProof"] = json!([]);
        let entry = self
            .accounts
            .entry(p.address)
            .or_insert_with(|| (answer, BTreeMap::new()));
        for ((k, _), s) in p.slots.iter().zip(storage) {
            entry.1.insert(*k, s);
        }
        if p.has_code() {
            self.codes.insert(p.code_hash, code.clone());
        }
    }

    /// The witness of `reads`: each account read with the slots read of it, the EIP-2935 slots of
    /// the block hashes read, each code once and the nodes shared. Fails when something read was
    /// never added.
    pub fn build(&self, reads: &Reads) -> Result<wire::Witness, String> {
        let mut want = reads.accounts.clone();
        if !reads.block_hashes.is_empty() {
            let slots = want.entry(HISTORY_ADDRESS).or_default();
            slots.extend(reads.block_hashes.iter().map(|n| history::slot(*n)));
        }
        let mut accounts = Vec::with_capacity(want.len());
        let mut codes = BTreeMap::new();
        for (address, slots) in &want {
            let (answer, storage) = self
                .accounts
                .get(address)
                .ok_or_else(|| format!("no proof of {address}, which the call read"))?;
            let mut a = answer.clone();
            let mut proofs = Vec::with_capacity(slots.len());
            for k in slots {
                proofs.push(
                    storage
                        .get(k)
                        .cloned()
                        .ok_or_else(|| format!("no proof of slot {k} of {address}"))?,
                );
            }
            a["storageProof"] = Value::Array(proofs);
            let code_hash = proof::data(&a["codeHash"])
                .ok()
                .filter(|b| b.len() == 32)
                .map(|b| B256::from_slice(&b))
                .unwrap_or(KECCAK_EMPTY);
            if code_hash != KECCAK_EMPTY {
                let code = self
                    .codes
                    .get(&code_hash)
                    .ok_or_else(|| format!("no code of {address}"))?;
                codes.insert(hx(code_hash.as_slice()), hx(code));
            }
            accounts.push(a);
        }
        let (accounts, nodes) = share(&accounts);
        Ok(wire::Witness {
            accounts,
            nodes,
            codes,
        })
    }
}

/// Every proof node seen more than once moved to a shared list, first seen first, and written
/// `"#i"` in its proofs (as anchor-verify's `share_nodes`).
pub fn share(accounts: &[Value]) -> (Vec<Value>, Vec<String>) {
    let proofs = |a: &Value| -> Vec<String> {
        let mut out: Vec<String> = a["accountProof"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|n| n.as_str().map(str::to_string))
            .collect();
        for s in a["storageProof"].as_array().into_iter().flatten() {
            out.extend(
                s["proof"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|n| n.as_str().map(str::to_string)),
            );
        }
        out
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    for a in accounts {
        for n in proofs(a) {
            *seen.entry(n).or_default() += 1;
        }
    }
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut pool: Vec<String> = Vec::new();
    let mut shared = |v: &mut Value| {
        let Some(s) = v.as_str() else { return };
        if seen.get(s).copied().unwrap_or(0) < 2 {
            return;
        }
        let i = *index.entry(s.to_string()).or_insert_with(|| {
            pool.push(s.to_string());
            pool.len() - 1
        });
        *v = Value::String(format!("#{i}"));
    };
    let mut out = accounts.to_vec();
    for a in &mut out {
        if let Some(p) = a.get_mut("accountProof").and_then(Value::as_array_mut) {
            p.iter_mut().for_each(&mut shared);
        }
        if let Some(slots) = a.get_mut("storageProof").and_then(Value::as_array_mut) {
            for s in slots {
                if let Some(p) = s.get_mut("proof").and_then(Value::as_array_mut) {
                    p.iter_mut().for_each(&mut shared);
                }
            }
        }
    }
    (out, pool)
}
