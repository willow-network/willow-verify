//! A block's transactions and receipts, as nodes report them and as proofs
//! against the block hash.
//!
//! - [`txs`], [`derive`], [`opfee`], [`render`]: the JSON a node reports
//!   (`eth_getTransactionByHash`, `eth_getTransactionReceipt`,
//!   `eth_getBlockReceipts`, ...) rendered from the block's consensus bytes
//!   (each transaction's and receipt's trie value) and its header. The log
//!   store's full-block tier serves from these; a verifier renders the same
//!   JSON from proven bytes.
//! - [`mpt`]: Merkle-Patricia proofs into a block's ordered tries
//!   (transactionsRoot, receiptsRoot), built with alloy-trie and checked by
//!   two walkers that must agree.
//! - [`proof`]: the proof a server sends for a transaction, a receipt or a
//!   block's receipts (the header, the items and their proofs), how it is
//!   built and how a client verifies it.
//! - [`compare`]: another endpoint's answer against the proven JSON.

pub mod compare;
pub mod derive;
pub mod mpt;
pub mod opfee;
pub mod proof;
pub mod render;
pub mod txs;

#[cfg(test)]
mod tests;

/// One transaction as stored: its transactions-trie value and its sender.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Tx {
    /// EIP-2718 bytes (a legacy transaction as its RLP list).
    pub value: Vec<u8>,
    /// Recovered from the signature, or the `from` the bytes carry (OP-stack
    /// deposits, Nitro types), or ArbOS's address (Nitro internal).
    pub sender: [u8; 20],
}

/// Receipt fields that are neither in the receipts trie nor derivable from the
/// block: passed through from the node as it reported them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReceiptExtras {
    /// Arbitrum Nitro: L2 gas charged for the transaction's L1 data
    /// (`gasUsedForL1`), priced from ArbOS state.
    pub gas_used_for_l1: Option<u64>,
    /// Arbitrum Nitro: sequenced through the Timeboost express lane
    /// (`timeboosted`), from the node's block metadata.
    pub timeboosted: Option<bool>,
    /// OP stack before Canyon: a deposit's `depositNonce`, which its receipt's
    /// trie value carries only from Canyon on.
    pub deposit_nonce: Option<u64>,
}

/// keccak-256: a transaction's hash, a block's hash.
pub fn keccak(b: &[u8]) -> [u8; 32] {
    alloy_primitives::keccak256(b).0
}

/// `0x`-prefixed hex quantity.
pub fn hex_u64(v: &serde_json::Value) -> Option<u64> {
    u64::from_str_radix(v.as_str()?.strip_prefix("0x")?, 16).ok()
}

/// `0x`-prefixed hex data of exactly `N` bytes.
pub fn hex_fixed<const N: usize>(v: &serde_json::Value) -> Option<[u8; N]> {
    let s = v.as_str()?;
    hex::decode(s.strip_prefix("0x").unwrap_or(s))
        .ok()?
        .try_into()
        .ok()
}
