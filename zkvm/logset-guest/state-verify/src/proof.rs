//! An `eth_getProof` answer checked against a verified block's state root (EIP-1186): the
//! account's inclusion, or its absence, then each storage slot against the account's storage root.

use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_trie::{proof::verify_proof, Nibbles, TrieAccount, EMPTY_ROOT_HASH, KECCAK_EMPTY};
use serde_json::{json, Value};

/// An account as proven at one state root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proven {
    pub address: Address,
    /// `false`: proven not to exist (every field zero, no code, no storage).
    pub exists: bool,
    pub nonce: u64,
    pub balance: U256,
    pub storage_root: B256,
    pub code_hash: B256,
    /// The slots asked for, in that order, each with its proven value.
    pub slots: Vec<(B256, U256)>,
}

impl Proven {
    /// The code hash of an account without code (absent or not).
    pub fn has_code(&self) -> bool {
        self.exists && self.code_hash != KECCAK_EMPTY
    }
}

pub fn quantity(v: &Value) -> Result<U256, String> {
    let s = v.as_str().ok_or_else(|| format!("not a quantity: {v}"))?;
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("not a quantity: {s}"))?;
    if digits.is_empty() || digits.len() > 64 {
        return Err(format!("not a quantity: {s}"));
    }
    U256::from_str_radix(digits, 16).map_err(|_| format!("not a quantity: {s}"))
}

pub fn data(v: &Value) -> Result<Bytes, String> {
    let s = v.as_str().ok_or_else(|| format!("not hex data: {v}"))?;
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("not hex data: {s}"))?;
    hex::decode(digits)
        .map(Bytes::from)
        .map_err(|_| format!("not hex data: {s}"))
}

pub fn address(v: &Value) -> Result<Address, String> {
    let b = data(v).map_err(|_| format!("not an address: {v}"))?;
    (b.len() == 20)
        .then(|| Address::from_slice(&b))
        .ok_or_else(|| format!("not an address: {v}"))
}

/// A storage key as clients write it: 32 bytes, or a shorter quantity (geth left-pads it).
pub fn slot(v: &Value) -> Result<B256, String> {
    let s = v
        .as_str()
        .ok_or_else(|| format!("not a storage key: {v}"))?;
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| format!("not a storage key: {s}"))?;
    if digits.len() > 64 {
        return Err(format!("not a storage key: {s}"));
    }
    U256::from_str_radix(if digits.is_empty() { "0" } else { digits }, 16)
        .map(B256::from)
        .map_err(|_| format!("not a storage key: {s}"))
}

fn hash(proof: &Value, key: &str) -> Result<B256, String> {
    let b = data(&proof[key]).map_err(|_| format!("no {key}"))?;
    (b.len() == 32)
        .then(|| B256::from_slice(&b))
        .ok_or_else(|| format!("{key} is not 32 bytes"))
}

fn nodes(v: &Value) -> Result<Vec<Bytes>, String> {
    v.as_array()
        .ok_or_else(|| "no proof nodes".to_string())?
        .iter()
        .map(data)
        .collect()
}

/// `proof` (an `eth_getProof` result for `address` and `slots`) checked against `state_root`.
pub fn verify(
    state_root: B256,
    address: Address,
    slots: &[B256],
    proof: &Value,
) -> Result<Proven, String> {
    check(state_root, address, slots, proof).map_err(|f| f.message)
}

/// [`verify`], saying what failed by its [`Kind`].
pub fn check(
    state_root: B256,
    address: Address,
    slots: &[B256],
    proof: &Value,
) -> Result<Proven, Failure> {
    let bad = |m: String| Failure::new(Kind::Malformed, m);
    if self::address(&proof["address"]).ok() != Some(address) {
        return Err(Failure::new(Kind::Account, "a proof of another account"));
    }
    let nonce = quantity(&proof["nonce"]).map_err(bad)?;
    let nonce: u64 = nonce
        .try_into()
        .map_err(|_| bad("nonce is over 64 bits".into()))?;
    let balance = quantity(&proof["balance"]).map_err(bad)?;
    // Geth names an absent account's code and storage by zero hashes, others by the empty ones.
    let storage_root = hash(proof, "storageHash").map_err(bad)?;
    let code_hash = hash(proof, "codeHash").map_err(bad)?;
    let account_proof = nodes(&proof["accountProof"]).map_err(bad)?;
    let key = Nibbles::unpack(keccak256(address));
    let account = TrieAccount {
        nonce,
        balance,
        storage_root,
        code_hash,
    };
    let exists = verify_proof(
        state_root,
        key,
        Some(alloy_rlp::encode(account)),
        &account_proof,
    )
    .is_ok();
    if !exists {
        verify_proof(state_root, key, None, &account_proof).map_err(|_| {
            Failure::new(
                Kind::Account,
                format!("the proof of account {address} does not verify"),
            )
        })?;
        if nonce != 0 || !balance.is_zero() {
            return Err(Failure::new(
                Kind::Account,
                format!("account {address} is absent but has a balance or nonce"),
            ));
        }
    }
    let root = if exists {
        storage_root
    } else {
        EMPTY_ROOT_HASH
    };
    let storage = proof["storageProof"]
        .as_array()
        .ok_or_else(|| bad("no storageProof".into()))?;
    if storage.len() != slots.len() {
        return Err(Failure::new(
            Kind::Slot,
            format!("{} storage proofs for {} slots", storage.len(), slots.len()),
        ));
    }
    let mut proven = Vec::with_capacity(slots.len());
    for (want, p) in slots.iter().zip(storage) {
        if slot(&p["key"]).map_err(bad)? != *want {
            return Err(Failure::new(Kind::Slot, "a proof of another slot"));
        }
        let value = quantity(&p["value"]).map_err(bad)?;
        let expected = (!value.is_zero()).then(|| alloy_rlp::encode(value));
        verify_proof(
            root,
            Nibbles::unpack(keccak256(want)),
            expected,
            &nodes(&p["proof"]).map_err(bad)?,
        )
        .map_err(|_| {
            Failure::new(
                Kind::Slot,
                format!("the proof of slot {want} of {address} does not verify"),
            )
        })?;
        proven.push((*want, value));
    }
    Ok(Proven {
        address,
        exists,
        nonce: if exists { nonce } else { 0 },
        balance: if exists { balance } else { U256::ZERO },
        storage_root: root,
        code_hash: if exists { code_hash } else { KECCAK_EMPTY },
        slots: proven,
    })
}

/// `answer` (an `eth_getMultiProof` result for `targets`: one `eth_getProof` result per target, in
/// their order) checked against `state_root`, each as [`verify`] checks it.
pub fn verify_many(
    state_root: B256,
    targets: &[(Address, Vec<B256>)],
    answer: &Value,
) -> Result<Vec<Proven>, String> {
    let proofs = answer.as_array().ok_or("not a list of proofs")?;
    if proofs.len() != targets.len() {
        return Err(format!(
            "{} proofs for {} accounts",
            proofs.len(),
            targets.len()
        ));
    }
    targets
        .iter()
        .zip(proofs)
        .map(|((address, slots), p)| verify(state_root, *address, slots, p))
        .collect()
}

/// `given` (a proof that verified as `p` against `state_root`) with each list of nodes cut to the
/// fewest, from its first, that still prove the same: where a key's walk leaves the trie at an
/// extension node, reth's `eth_getMultiProof` adds that node's child after it, while
/// `eth_getProof` (reth's and geth's) ends at the extension. A list no shorter part of proves it
/// is kept whole, so the answer verifies as `given` does.
pub fn shortest(state_root: B256, p: &Proven, given: &Value) -> Value {
    let mut out = given.clone();
    let account = p.exists.then(|| {
        alloy_rlp::encode(TrieAccount {
            nonce: p.nonce,
            balance: p.balance,
            storage_root: p.storage_root,
            code_hash: p.code_hash,
        })
    });
    if let Some(list) = out.get_mut("accountProof") {
        cut(
            list,
            state_root,
            Nibbles::unpack(keccak256(p.address)),
            account,
        );
    }
    for (i, (slot, value)) in p.slots.iter().enumerate() {
        let list = out
            .get_mut("storageProof")
            .and_then(|s| s.get_mut(i))
            .and_then(|s| s.get_mut("proof"));
        if let Some(list) = list {
            let value = (!value.is_zero()).then(|| alloy_rlp::encode(value));
            cut(
                list,
                p.storage_root,
                Nibbles::unpack(keccak256(slot)),
                value,
            );
        }
    }
    out
}

/// `list` (a proof's nodes) cut to the fewest from its first that prove `value` at `key`.
fn cut(list: &mut Value, root: B256, key: Nibbles, value: Option<Vec<u8>>) {
    let Ok(all) = nodes(list) else {
        return;
    };
    let fewest =
        (1..all.len()).find(|&n| verify_proof(root, key, value.clone(), &all[..n]).is_ok());
    if let (Some(n), Some(list)) = (fewest, list.as_array_mut()) {
        list.truncate(n);
    }
}

/// `code` is the code an account's proven code hash names.
pub fn code_matches(account: &Proven, code: &[u8]) -> bool {
    match account.has_code() {
        true => keccak256(code) == account.code_hash,
        false => code.is_empty(),
    }
}

/// The `eth_getProof` answer for a verified proof: its fields as proven (an absent account's
/// zero, with the empty code hash and storage root), its proofs as the provider gave them, and
/// the storage keys as the caller wrote them (as geth echoes them).
pub fn response(p: &Proven, given: &Value, keys: Vec<Value>) -> Value {
    let (code_hash, storage_hash) = match p.exists {
        true => (p.code_hash, p.storage_root),
        false => (KECCAK_EMPTY, EMPTY_ROOT_HASH),
    };
    let storage: Vec<Value> = p
        .slots
        .iter()
        .zip(keys)
        .zip(given["storageProof"].as_array().into_iter().flatten())
        .map(|(((_, value), key), sp)| {
            json!({"key": key, "value": format!("{value:#x}"), "proof": sp["proof"]})
        })
        .collect();
    json!({
        "address": hx(p.address.as_slice()),
        "accountProof": given["accountProof"],
        "balance": format!("{:#x}", p.balance),
        "codeHash": hx(code_hash.as_slice()),
        "nonce": format!("0x{:x}", p.nonce),
        "storageHash": hx(storage_hash.as_slice()),
        "storageProof": storage,
    })
}

pub(crate) fn hx(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}
