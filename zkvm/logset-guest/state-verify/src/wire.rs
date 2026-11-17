














use crate::call::{self, Ran};
use crate::estimate::Estimated;
use crate::proof::{self, hx};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;



pub const ESTIMATE_ALGORITHM: &str = "geth-v1.17.10-estimate";


pub const NITRO_ESTIMATE_ALGORITHM: &str = "nitro-v3.12.2-estimate";


pub fn estimate_algorithm(chain_id: u64) -> &'static str {
    match chain_id {
        willow_arb_exec::ROBINHOOD => NITRO_ESTIMATE_ALGORITHM,
        _ => ESTIMATE_ALGORITHM,
    }
}


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StateProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,

    pub account: Value,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Witness {


    pub accounts: Vec<Value>,

    pub nodes: Vec<String>,

    #[serde(default)]
    pub codes: BTreeMap<String, String>,
}


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    pub witness: Witness,

    pub result: Outcome,
}


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {

    Returned(String),

    Reverted(String),

    Halted(String),

    Invalid(String),
}

impl Default for Outcome {
    fn default() -> Self {
        Outcome::Returned("0x".into())
    }
}


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EstimateProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,

    pub witness: Witness,

    pub algorithm: String,
    pub estimate: Estimate,
}


#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Estimate {

    Gas(String),

    Reverted(String),

    Error(String),
}

impl Default for Estimate {
    fn default() -> Self {
        Estimate::Gas("0x0".into())
    }
}


fn canonical(data: &str) -> Option<String> {
    proof::data(&json!(data)).ok().map(|b| hx(&b))
}

impl Outcome {

    pub fn of(ran: &Ran) -> Option<Outcome> {
        Some(match ran {
            Ran::Returned(b) => Outcome::Returned(hx(b)),

            Ran::Reverted(b) | Ran::RevertedSaying(b, _) => Outcome::Reverted(hx(b)),
            Ran::Halted(m) => Outcome::Halted(m.clone()),
            Ran::Invalid(m) => Outcome::Invalid(m.clone()),
            Ran::Missing(_) | Ran::NoBlockHash(_) | Ran::Refused(_) | Ran::Internal(_) => {
                return None
            }
        })
    }


    pub fn normalized(&self) -> Option<Outcome> {
        Some(match self {
            Outcome::Returned(d) => Outcome::Returned(canonical(d)?),
            Outcome::Reverted(d) => Outcome::Reverted(canonical(d)?),
            other => other.clone(),
        })
    }




    pub fn response_of(ran: &Ran) -> Option<Value> {
        match ran {
            Ran::RevertedSaying(b, said) => Some(json!({"error": {"code": 3,
                "message": format!("execution reverted: {said}"), "data": hx(b)}})),
            r => Outcome::of(r).map(|o| o.response()),
        }
    }



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

    pub fn of(e: &Estimated) -> Option<Estimate> {
        Some(match e {
            Estimated::Gas(g) => Estimate::Gas(format!("0x{g:x}")),

            Estimated::Reverted(b) | Estimated::RevertedSaying(b, _) => Estimate::Reverted(hx(b)),
            Estimated::Error(m) => Estimate::Error(m.clone()),
            Estimated::Missing(_) | Estimated::NoBlockHash(_) | Estimated::Internal(_) => {
                return None
            }
        })
    }


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



    pub fn response_of(e: &Estimated) -> Option<Value> {
        match e {
            Estimated::RevertedSaying(b, said) => Some(json!({"error": {"code": 3,
                "message": format!("execution reverted: {said}"), "data": hx(b)}})),
            e => Estimate::of(e).map(|x| x.response()),
        }
    }


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
