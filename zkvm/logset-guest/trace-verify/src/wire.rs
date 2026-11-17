







use serde::{Deserialize, Serialize};


#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,


    pub parent: String,


    #[serde(default)]
    pub ancestors: Vec<String>,

    pub transactions: Vec<String>,

    pub receipts: Vec<String>,
    pub witness: Witness,

    pub algorithm: String,
}



#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Witness {
    pub nodes: Vec<String>,
    pub codes: Vec<String>,
}



#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallTraceProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    pub witness: state_verify::wire::Witness,

    pub algorithm: String,
}
