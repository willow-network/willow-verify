//! The proof methods' results, as JSON-RPC sends them (I2 and I3). Every result names its block
//! (`chainId`, `blockNumber`, `blockHash`, `header`: the header's RLP, whose keccak is the block
//! hash); the client builds the standard response from the proven bytes alone.
//!
//! - `willow_getStateProof [address, storageKeys, block, {"code": bool}]` → [`StateProof`]
//!   (`testdata/state/`).
//! - `willow_getCallProof [call, block]` → [`CallProof`] (`testdata/call/call-*.json`).
//! - `willow_getEstimateGasProof [call, block]` → [`EstimateProof`]
//!   (`testdata/call/estimate-*.json`).
//!
//! Quantities are `0x` hex without leading zeros, data is `0x` hex, as in `eth_getProof`.

use crate::call::{self, Ran};
use crate::estimate::Estimated;
use crate::proof::{self, hx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

/// `willow_getEstimateGasProof`'s `algorithm`: the search the estimate is the output of.
pub const ESTIMATE_ALGORITHM: &str = "geth-v1.17.10-estimate";

/// `willow_getStateProof`: one account at one block.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    /// The `eth_getProof` object for the address and keys asked, as the standard method sends it.
    pub account: Value,
    /// The account's code: only when asked for (`{"code": true}`) and the account has code.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

/// The state a call reads, proven against the block's state root.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Witness {
    /// One `eth_getProof` object per account read, with the slots read of it. A proof node
    /// written `"#i"` is `nodes[i]`.
    pub accounts: Vec<Value>,
    /// Trie nodes more than one proof holds (the top of the state trie is in every account's).
    pub nodes: Vec<String>,
    /// Each code once, by its hash (`0x` + 64 hex digits, lowercase).
    #[serde(default)]
    pub codes: BTreeMap<String, String>,
}

/// `willow_getCallProof`: an `eth_call` on Ethereum and the state it read.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    pub witness: Witness,
    /// What the call came to; the client runs it and must come to the same.
    pub result: Outcome,
}

/// What a call came to, as the EVM ends it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    /// The bytes returned (a creation's: the code it deploys).
    Returned(String),
    /// The revert data.
    Reverted(String),
    /// Stopped by the EVM: geth's message for why (`out of gas`, `invalid opcode`, ...).
    Halted(String),
    /// Not run: geth's message for why the transaction is invalid at this block.
    Invalid(String),
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::Returned("0x".into())
    }
}

/// `willow_getEstimateGasProof`: an `eth_estimateGas` on Ethereum and the state its search read.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    /// Every read of every run of the search.
    pub witness: Witness,
    /// [`ESTIMATE_ALGORITHM`]; the client refuses a name it doesn't know.
    pub algorithm: String,
    pub estimate: Estimate,
}

/// What the search came to.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Estimate {
    /// The gas limit found (a quantity).
    Gas(String),
    /// The call reverts at the most gas it may have: the revert data.
    Reverted(String),
    /// Geth's error message (`gas required exceeds allowance (16777216)`, ...).
    Error(String),
}

impl Default for Estimate {
    fn default() -> Self {
        Estimate::Gas("0x0".into())
    }
}

/// `0x` data written as this crate writes it (lowercase), or `None` when it is not hex data.
fn canonical(data: &str) -> Option<String> {
    proof::data(&json!(data)).ok().map(|b| hx(&b))
}

impl Outcome {
    /// What a run came to, if it came to an outcome (not a read the witness lacks).
    pub fn of(ran: &Ran) -> Option<Outcome> {
        Some(match ran {
            Ran::Returned(b) => Outcome::Returned(hx(b)),
            Ran::Reverted(b) => Outcome::Reverted(hx(b)),
            Ran::Halted(m) => Outcome::Halted(m.clone()),
            Ran::Invalid(m) => Outcome::Invalid(m.clone()),
            Ran::Missing(_) | Ran::NoBlockHash(_) | Ran::Internal(_) => return None,
        })
    }

    /// The same outcome with its data written canonically; `None` when the data is not hex.
    pub fn normalized(&self) -> Option<Outcome> {
        Some(match self {
            Outcome::Returned(d) => Outcome::Returned(canonical(d)?),
            Outcome::Reverted(d) => Outcome::Reverted(canonical(d)?),
            other => other.clone(),
        })
    }

    /// The standard `eth_call` response, `{"result"}` or `{"error"}`, as Willow's state methods
    /// send it.
    pub fn response(&self) -> Value {
        match self {
            Outcome::Returned(d) => json!({"result": d}),
            Outcome::Reverted(d) => {
                let data = proof::data(&json!(d)).unwrap_or_default();
                json!({"error": {"code": 3, "message": call::revert_message(&data), "data": d}})
            }
            Outcome::Halted(m) => {
                json!({"error": {"code": -32000, "message": call::halt_message(m)}})
            }
            Outcome::Invalid(m) => json!({"error": {"code": -32000, "message": m}}),
        }
    }
}

impl Estimate {
    /// What a search came to, if it came to an estimate (not a read the witness lacks).
    pub fn of(e: &Estimated) -> Option<Estimate> {
        Some(match e {
            Estimated::Gas(g) => Estimate::Gas(format!("0x{g:x}")),
            Estimated::Reverted(b) => Estimate::Reverted(hx(b)),
            Estimated::Error(m) => Estimate::Error(m.clone()),
            Estimated::Missing(_) | Estimated::NoBlockHash(_) | Estimated::Internal(_) => {
                return None
            }
        })
    }

    /// The same estimate written canonically; `None` when a quantity or data is not hex.
    pub fn normalized(&self) -> Option<Estimate> {
        Some(match self {
            Estimate::Gas(g) => {
                let g = u64::try_from(proof::quantity(&json!(g)).ok()?).ok()?;
                Estimate::Gas(format!("0x{g:x}"))
            }
            Estimate::Reverted(d) => Estimate::Reverted(canonical(d)?),
            Estimate::Error(m) => Estimate::Error(m.clone()),
        })
    }

    /// The standard `eth_estimateGas` response, `{"result"}` or `{"error"}`, as geth sends it.
    pub fn response(&self) -> Value {
        match self {
            Estimate::Gas(g) => json!({"result": g}),
            Estimate::Reverted(d) => {
                let data = proof::data(&json!(d)).unwrap_or_default();
                json!({"error": {"code": 3, "message": call::revert_message(&data), "data": d}})
            }
            Estimate::Error(m) => json!({"error": {"code": -32000, "message": m}}),
        }
    }
}
