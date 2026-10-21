//! Verified calls on Ethereum: a contract's own code run over state proven against an execution
//! state root the light client verified (as Helios, the reference, runs `eth_call`). The witness
//! is `eth_getProof` per account plus its code; anything the call touches that is not in it fails
//! the call, so a contract upgrade or a server leaving something out cannot change an answer, only
//! refuse one.

use crate::history::{b256_of, nodes, quantity};
use crate::lc::{bytes_of, Execution};
use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_trie::proof::verify_proof;
use alloy_trie::Nibbles;
use revm::context::result::{ExecutionResult, Output};
use revm::context::{BlockEnv, CfgEnv, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::database_interface::DBErrorMarker;
use revm::primitives::eip4844::BLOB_BASE_FEE_UPDATE_FRACTION_PRAGUE;
use revm::primitives::hardfork::SpecId;
use revm::primitives::TxKind;
use revm::state::{AccountInfo, Bytecode};
use revm::{Context, Database, ExecuteEvm, MainBuilder, MainContext};
use serde_json::Value;
use std::collections::HashMap;

/// Ethereum's Prague and Osaka (Fusaka) activation times.
const PRAGUE_TIME: u64 = 1_746_612_311;
const OSAKA_TIME: u64 = 1_764_798_551;
/// EIP-7825's per-transaction gas cap: plenty for a view.
const CALL_GAS: u64 = 1 << 24;
/// Accounts a witness may carry.
const MAX_ACCOUNTS: usize = 64;
/// Shared trie nodes, and slots, one witness may carry (twice as many proof nodes in all).
const MAX_SHARED: usize = 8_192;
/// The longest trie node, in bytes.
const MAX_NODE: usize = 1_100;

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

struct Account {
    /// `None`: proven not to exist.
    info: Option<AccountInfo>,
    storage: HashMap<U256, U256>,
}

/// Accounts proven against one state root.
pub struct State {
    accounts: HashMap<Address, Account>,
}

/// The account's trie value: rlp([nonce, balance, storageRoot, codeHash]).
pub(crate) fn account_rlp(
    nonce: u64,
    balance: U256,
    storage_root: B256,
    code_hash: B256,
) -> Vec<u8> {
    let mut fields = Vec::new();
    alloy_rlp::Encodable::encode(&nonce, &mut fields);
    alloy_rlp::Encodable::encode(&balance, &mut fields);
    alloy_rlp::Encodable::encode(&storage_root, &mut fields);
    alloy_rlp::Encodable::encode(&code_hash, &mut fields);
    let mut out = Vec::with_capacity(fields.len() + 3);
    alloy_rlp::Header {
        list: true,
        payload_length: fields.len(),
    }
    .encode(&mut out);
    out.extend(fields);
    out
}

/// A witness with its trie nodes shared, `{"accounts": [...], "nodes": [...]}`, as the plain list
/// of accounts: each proof node written `"#i"` is `nodes[i]`.
fn expand(witness: &Value) -> Result<Value, Failure> {
    let pool = witness
        .get("nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("the state witness has no shared nodes"))?;
    // Bounded before anything is copied: a trie node is at most about 600 bytes, and no proof
    // here is longer than 64 nodes.
    if pool.len() > MAX_SHARED
        || pool
            .iter()
            .any(|n| n.as_str().is_none_or(|s| s.len() > 2 + 2 * MAX_NODE))
    {
        return Err(bad(
            "the state witness's shared nodes are too many or too large",
        ));
    }
    let node = |v: &Value| -> Result<Value, Failure> {
        match v.as_str().and_then(|s| s.strip_prefix('#')) {
            None => Ok(v.clone()),
            Some(i) => i
                .parse::<usize>()
                .ok()
                .and_then(|i| pool.get(i))
                .cloned()
                .ok_or_else(|| bad("a proof node refers to no shared node")),
        }
    };
    let path = |v: &Value| -> Result<Value, Failure> {
        v.as_array()
            .filter(|p| p.len() <= 64)
            .ok_or_else(|| bad("a proof is not a list of at most 64 nodes"))?
            .iter()
            .map(node)
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array)
    };
    let mut accounts = witness
        .get("accounts")
        .and_then(Value::as_array)
        .filter(|l| l.len() <= MAX_ACCOUNTS)
        .cloned()
        .ok_or_else(|| bad("the state witness is not a list of at most 64 accounts"))?;
    let slots: usize = accounts
        .iter()
        .map(|a| a["storageProof"].as_array().map_or(0, Vec::len))
        .sum();
    let len = |v: &Value| v.as_array().map_or(0, Vec::len);
    let refs: usize = accounts
        .iter()
        .map(|a| {
            len(&a["accountProof"])
                + a["storageProof"]
                    .as_array()
                    .map_or(0, |s| s.iter().map(|s| len(&s["proof"])).sum())
        })
        .sum();
    if slots > MAX_SHARED || refs > 2 * MAX_SHARED {
        return Err(bad("the state witness proves too many slots"));
    }
    for a in &mut accounts {
        if let Some(p) = a.get("accountProof") {
            a["accountProof"] = path(p)?;
        }
        if let Some(slots) = a.get_mut("storageProof").and_then(Value::as_array_mut) {
            for s in slots {
                if let Some(p) = s.get("proof") {
                    s["proof"] = path(p)?;
                }
            }
        }
    }
    Ok(Value::Array(accounts))
}

/// The inverse of [`expand`] for a server: `accounts` with every proof node seen more than once
/// moved to the shared list (proofs of many slots of one contract share most of their nodes).
pub fn share_nodes(accounts: &[Value]) -> Value {
    let proofs = |a: &Value| -> Vec<Value> {
        let mut out: Vec<Value> = a["accountProof"].as_array().cloned().unwrap_or_default();
        for s in a["storageProof"].as_array().into_iter().flatten() {
            out.extend(s["proof"].as_array().cloned().unwrap_or_default());
        }
        out
    };
    let mut seen: HashMap<String, usize> = HashMap::new();
    for a in accounts {
        for n in proofs(a) {
            if let Some(s) = n.as_str() {
                *seen.entry(s.to_string()).or_default() += 1;
            }
        }
    }
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut pool: Vec<Value> = Vec::new();
    let mut share = |v: &Value| -> Value {
        let s = v.as_str().unwrap_or_default();
        if seen.get(s).copied().unwrap_or(0) < 2 {
            return v.clone();
        }
        let i = *index.entry(s.to_string()).or_insert_with(|| {
            pool.push(v.clone());
            pool.len() - 1
        });
        Value::String(format!("#{i}"))
    };
    let mut out = accounts.to_vec();
    for a in &mut out {
        if let Some(p) = a.get_mut("accountProof").and_then(Value::as_array_mut) {
            for n in p.iter_mut() {
                *n = share(n);
            }
        }
        if let Some(slots) = a.get_mut("storageProof").and_then(Value::as_array_mut) {
            for s in slots {
                if let Some(p) = s.get_mut("proof").and_then(Value::as_array_mut) {
                    for n in p.iter_mut() {
                        *n = share(n);
                    }
                }
            }
        }
    }
    serde_json::json!({"accounts": out, "nodes": pool})
}

/// The accounts of `witness` (each an `eth_getProof` result plus its `code`), checked against
/// `state_root`.
pub fn prove_state(state_root: B256, witness: &Value) -> Result<State, Failure> {
    let expanded;
    let witness = match witness.get("accounts") {
        Some(_) => {
            expanded = expand(witness)?;
            &expanded
        }
        None => witness,
    };
    let list = witness
        .as_array()
        .filter(|l| l.len() <= MAX_ACCOUNTS)
        .ok_or_else(|| bad("the state witness is not a list of at most 64 accounts"))?;
    let mut accounts = HashMap::new();
    for a in list {
        let address: Address = a
            .get("address")
            .and_then(Value::as_str)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| bad("an account has no address"))?;
        let nonce: u64 = quantity(a, "nonce")?
            .try_into()
            .map_err(|_| bad("nonce is over 64 bits"))?;
        let balance = quantity(a, "balance")?;
        let storage_root = b256_of(a, "storageHash")?;
        let code_hash = b256_of(a, "codeHash")?;
        let proof = nodes(a, "accountProof")?;
        let key = Nibbles::unpack(keccak256(address));
        let rlp = account_rlp(nonce, balance, storage_root, code_hash);
        let exists = verify_proof(state_root, key, Some(rlp), &proof).is_ok();
        if !exists && verify_proof(state_root, key, None, &proof).is_err() {
            return Err(rejected(format!(
                "account {address}'s proof does not verify"
            )));
        }
        let code = bytes_of(
            a.get("code").and_then(Value::as_str).unwrap_or("0x"),
            "code",
        )?;
        if exists && keccak256(&code) != code_hash {
            return Err(rejected(format!(
                "account {address}'s code is not its code hash"
            )));
        }
        let mut storage = HashMap::new();
        for s in a
            .get("storageProof")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("an account has no storageProof"))?
        {
            let slot = quantity(s, "key")?;
            let value = quantity(s, "value")?;
            let expected = (!value.is_zero()).then(|| alloy_rlp::encode(value));
            let root = if exists {
                storage_root
            } else {
                alloy_trie::EMPTY_ROOT_HASH
            };
            verify_proof(
                root,
                Nibbles::unpack(keccak256(B256::from(slot))),
                expected,
                &nodes(s, "proof")?,
            )
            .map_err(|_| rejected(format!("slot {slot} of {address} does not verify")))?;
            storage.insert(slot, value);
        }
        let info = exists.then(|| AccountInfo {
            balance,
            nonce,
            code_hash,
            code: Some(Bytecode::new_raw(Bytes::from(code))),
        });
        if accounts
            .insert(address, Account { info, storage })
            .is_some()
        {
            return Err(bad(format!("account {address} twice")));
        }
    }
    Ok(State { accounts })
}

#[derive(Debug)]
struct Unproven(String);

impl std::fmt::Display for Unproven {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for Unproven {}
impl DBErrorMarker for Unproven {}

struct ProvenDb<'a>(&'a State);

/// Precompiles answer from code built into the EVM, not from state.
fn is_precompile(a: &Address) -> bool {
    let b = a.as_slice();
    b[..18].iter().all(|&x| x == 0) && {
        let n = u16::from_be_bytes([b[18], b[19]]);
        (1..=0x11).contains(&n) || n == 0x100
    }
}

impl Database for ProvenDb<'_> {
    type Error = Unproven;

    fn basic(&mut self, address: Address) -> Result<Option<AccountInfo>, Unproven> {
        if is_precompile(&address) {
            return Ok(Some(AccountInfo::default()));
        }
        self.0
            .accounts
            .get(&address)
            .map(|a| a.info.clone())
            .ok_or_else(|| Unproven(format!("account {address}")))
    }

    fn code_by_hash(&mut self, code_hash: B256) -> Result<Bytecode, Unproven> {
        self.0
            .accounts
            .values()
            .filter_map(|a| a.info.as_ref())
            .find(|i| i.code_hash == code_hash)
            .and_then(|i| i.code.clone())
            .ok_or_else(|| Unproven(format!("code {code_hash}")))
    }

    fn storage(&mut self, address: Address, index: U256) -> Result<U256, Unproven> {
        if is_precompile(&address) {
            return Ok(U256::ZERO);
        }
        let a = self
            .0
            .accounts
            .get(&address)
            .ok_or_else(|| Unproven(format!("account {address}")))?;
        if a.info.is_none() {
            return Ok(U256::ZERO);
        }
        a.storage
            .get(&index)
            .copied()
            .ok_or_else(|| Unproven(format!("slot {index} of {address}")))
    }

    fn block_hash(&mut self, number: u64) -> Result<B256, Unproven> {
        Err(Unproven(format!("the hash of block {number}")))
    }
}

/// `data` sent to `to` at the block `block` (whose state `state` is), from the zero address: the
/// returned bytes, or a failure when the call reverts or needs anything not proven.
pub fn call(state: &State, block: &Execution, to: Address, data: Bytes) -> Result<Bytes, Failure> {
    if block.timestamp < PRAGUE_TIME {
        return Err(rejected("calls are run only at blocks from Prague on"));
    }
    let spec = if block.timestamp >= OSAKA_TIME {
        SpecId::OSAKA
    } else {
        SpecId::PRAGUE
    };
    let mut cfg = CfgEnv::default();
    cfg.spec = spec;
    cfg.chain_id = 1;
    cfg.disable_nonce_check = true;
    cfg.disable_balance_check = true;
    cfg.disable_base_fee = true;
    cfg.disable_block_gas_limit = true;
    cfg.disable_eip3607 = true;
    let block_env = BlockEnv {
        number: U256::from(block.number),
        beneficiary: Address::from(block.fee_recipient),
        timestamp: U256::from(block.timestamp),
        gas_limit: block.gas_limit,
        basefee: block.base_fee.try_into().unwrap_or(u64::MAX),
        difficulty: U256::ZERO,
        prevrandao: Some(B256::from(block.prev_randao)),
        // The views called here never read the blob base fee; the EVM only needs one set.
        blob_excess_gas_and_price: Some(BlobExcessGasAndPrice::new(
            0,
            BLOB_BASE_FEE_UPDATE_FRACTION_PRAGUE,
        )),
    };
    let tx = TxEnv {
        caller: Address::ZERO,
        gas_limit: CALL_GAS,
        gas_price: 0,
        kind: TxKind::Call(to),
        data,
        ..Default::default()
    };
    let mut evm = Context::mainnet()
        .with_db(ProvenDb(state))
        .with_block(block_env)
        .with_cfg(cfg)
        .build_mainnet();
    let out = evm.transact(tx).map_err(|e| {
        rejected(format!(
            "the call to {to} needed what Willow did not prove: {e}"
        ))
    })?;
    match out.result {
        ExecutionResult::Success {
            output: Output::Call(b),
            ..
        } => Ok(b),
        ExecutionResult::Revert { .. } => Err(rejected(format!("the call to {to} reverted"))),
        other => Err(rejected(format!("the call to {to} failed: {other:?}"))),
    }
}

/// A 32-byte word of a call's output.
pub fn word(out: &Bytes, i: usize) -> Result<[u8; 32], Failure> {
    out.get(32 * i..32 * (i + 1))
        .and_then(|w| w.try_into().ok())
        .ok_or_else(|| rejected("a call returned too little"))
}

/// The selector of `signature`, e.g. `isGameClaimValid(address)`.
pub fn selector(signature: &str) -> [u8; 4] {
    keccak256(signature.as_bytes())[..4].try_into().unwrap()
}

/// `selector(signature) ‖ args`, each a 32-byte word.
pub fn calldata(signature: &str, args: &[[u8; 32]]) -> Bytes {
    let mut d = selector(signature).to_vec();
    for a in args {
        d.extend_from_slice(a);
    }
    Bytes::from(d)
}

pub fn address_word(a: Address) -> [u8; 32] {
    B256::left_padding_from(a.as_slice()).0
}
