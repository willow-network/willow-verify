//! A block's hash from Ethereum's own block-hash history (EIP-2935): the contract's account under
//! a verified execution state root, and its storage slot `number mod 8191` under the account.

use crate::lc::bytes_of;
use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_trie::proof::verify_proof;
use alloy_trie::Nibbles;
use serde_json::Value;

/// The history contract (EIP-2935), the same address on Ethereum and the L2s that run it.
pub const HISTORY_ADDRESS: Address =
    alloy_primitives::address!("0000F90827F1C53a10cb7A02335B175320002935");
/// Ethereum's ring: the hashes of the 8,191 blocks before the state's block.
pub const ETHEREUM_RING: u64 = 8191;
/// EIP-2935's runtime code (the EIP's, deployed on Ethereum by Prague).
pub const ETHEREUM_HISTORY_CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500";

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

/// A `0x` quantity (`eth_getProof` writes nonces, balances, keys and values this way).
pub(crate) fn quantity(v: &Value, name: &str) -> Result<U256, Failure> {
    let s = v
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| bad(format!("no {name}")))?;
    let h = s
        .strip_prefix("0x")
        .filter(|h| !h.is_empty() && h.len() <= 64)
        .ok_or_else(|| bad(format!("{name} is not a 0x number")))?;
    U256::from_str_radix(h, 16).map_err(|_| bad(format!("{name} is not hex")))
}

pub(crate) fn b256_of(v: &Value, name: &str) -> Result<B256, Failure> {
    let s = v
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| bad(format!("no {name}")))?;
    let b = bytes_of(s, name)?;
    B256::try_from(b.as_slice()).map_err(|_| bad(format!("{name} is not 32 bytes")))
}

pub(crate) fn nodes(v: &Value, name: &str) -> Result<Vec<Bytes>, Failure> {
    let list = v
        .get(name)
        .and_then(Value::as_array)
        .ok_or_else(|| bad(format!("no {name}")))?;
    if list.len() > 64 {
        return Err(bad(format!("{name} has {} nodes", list.len())));
    }
    list.iter()
        .map(|n| {
            n.as_str()
                .ok_or_else(|| bad(format!("{name} holds a non-string")))
                .and_then(|s| bytes_of(s, name))
                .map(Bytes::from)
        })
        .collect()
}

/// Block `number`'s hash from an `eth_getProof` answer for the history contract at the block
/// whose state root is `state_root` and number `at`. `ring` is the chain's history length,
/// `code_hash` its contract's.
pub fn block_hash(
    state_root: [u8; 32],
    at: u64,
    number: u64,
    ring: u64,
    code_hash: B256,
    proof: &Value,
) -> Result<[u8; 32], Failure> {
    if number >= at || at - number > ring {
        return Err(rejected(format!(
            "block {number} is not among the {ring} blocks before {at}"
        )));
    }
    let address = proof
        .get("address")
        .and_then(Value::as_str)
        .ok_or_else(|| bad("no address"))?;
    if !address.eq_ignore_ascii_case(&HISTORY_ADDRESS.to_string()) {
        return Err(rejected("the proof is not of the history contract"));
    }
    let nonce = quantity(proof, "nonce")?;
    let balance = quantity(proof, "balance")?;
    let storage_root = b256_of(proof, "storageHash")?;
    let account_code = b256_of(proof, "codeHash")?;
    if account_code != code_hash {
        return Err(rejected("the history contract's code is not EIP-2935's"));
    }
    let nonce: u64 = nonce.try_into().map_err(|_| bad("nonce is over 64 bits"))?;
    // The account's trie value: rlp([nonce, balance, storageRoot, codeHash]).
    let mut fields = Vec::new();
    alloy_rlp::Encodable::encode(&nonce, &mut fields);
    alloy_rlp::Encodable::encode(&balance, &mut fields);
    alloy_rlp::Encodable::encode(&storage_root, &mut fields);
    alloy_rlp::Encodable::encode(&code_hash, &mut fields);
    let mut account = Vec::with_capacity(fields.len() + 3);
    alloy_rlp::Header {
        list: true,
        payload_length: fields.len(),
    }
    .encode(&mut account);
    account.extend(fields);
    verify_proof(
        B256::from(state_root),
        Nibbles::unpack(keccak256(HISTORY_ADDRESS)),
        Some(account),
        &nodes(proof, "accountProof")?,
    )
    .map_err(|_| rejected("the history contract's account proof does not verify"))?;

    let storage = proof
        .get("storageProof")
        .and_then(Value::as_array)
        .filter(|s| s.len() == 1)
        .ok_or_else(|| bad("storageProof is not one slot"))?;
    let slot = U256::from(number % ring);
    if quantity(&storage[0], "key")? != slot {
        return Err(rejected("the storage proof is for another slot"));
    }
    let value = quantity(&storage[0], "value")?;
    if value.is_zero() {
        return Err(rejected(format!(
            "the history holds no hash for block {number}"
        )));
    }
    verify_proof(
        storage_root,
        Nibbles::unpack(keccak256(B256::from(slot))),
        Some(alloy_rlp::encode(value)),
        &nodes(&storage[0], "proof")?,
    )
    .map_err(|_| {
        rejected(format!(
            "block {number}'s slot in the history does not verify"
        ))
    })?;
    Ok(value.to_be_bytes::<32>())
}

/// keccak256 of Ethereum's history contract code.
pub fn ethereum_code_hash() -> B256 {
    keccak256(hex::decode(ETHEREUM_HISTORY_CODE).expect("constant hex"))
}
