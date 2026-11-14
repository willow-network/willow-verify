//! `willow_getTraceProof [method, params]`'s result, as JSON-RPC sends it. It names its block
//! (`chainId`, `blockNumber`, `blockHash`, `header`: the header's RLP, whose keccak is the block
//! hash) like every proof method's result; the client builds the standard response from the
//! proven bytes alone, so the trace itself is never sent. A block's or a transaction's traces are
//! a [`TraceProof`]; a call's (`debug_traceCall`, `trace_call`) a [`CallTraceProof`].
//!
//! Quantities are `0x` hex without leading zeros, data is `0x` hex.

use serde::{Deserialize, Serialize};

/// A block's traces, proven: the block and everything its run reads.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TraceProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    /// The parent's header RLP: its keccak is the header's `parentHash`, its state root the state
    /// the block runs over.
    pub parent: String,
    /// The headers before the parent, newest first, back to the oldest block hash the run reads
    /// (BLOCKHASH): each the one its successor's `parentHash` names.
    #[serde(default)]
    pub ancestors: Vec<String>,
    /// Every transaction of the block, EIP-2718, in order (its transactionsRoot's values).
    pub transactions: Vec<String>,
    /// Every receipt of the block, EIP-2718, in order (its receiptsRoot's values).
    pub receipts: Vec<String>,
    pub witness: Witness,
    /// [`crate::ALGORITHM`]: how the block is run and traced.
    pub algorithm: String,
}

/// The parent's state the run reads: trie nodes, each found by its hash from the parent's state
/// root (account and storage tries alike), and codes, each by its hash.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Witness {
    pub nodes: Vec<String>,
    pub codes: Vec<String>,
}

/// A call's traces, proven: the call proof's block and witness (`willow_getCallProof`'s, the
/// state the call reads under the caller's overrides, which the client has and are never sent).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CallTraceProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    pub header: String,
    pub witness: state_verify::wire::Witness,
    /// [`crate::ALGORITHM`]: how the call is run and traced.
    pub algorithm: String,
}
