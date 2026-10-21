//! `willow-receipts-trie` — pure-Rust Ethereum receipts-trie reconstruction +
//! native keccak/RLP helpers.
//!
//! This is the **backend-agnostic oracle** for the receipts-root completeness
//! proof: it builds the canonical receipts MPT for keys `rlp(0..N)`, computes
//! every node's keccak hash, and binds the public node-hash chain to a claimed
//! `receipts_root` (`verify_binding`). It has **no Expander/GF2/wasm
//! dependency**, so both the Orion backend (`willow-gkr`) and the Binius64
//! backend reuse it unchanged. Validated against `alloy_trie` for N=1..512.
//!
//! `no_std + alloc` without the (default) `std` feature — the SP1 window
//! guest links it for the sharded-block digest-trie join.

#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod helpers;
mod native_trie;

pub use helpers::{
    circuit_leaf_len_bytes, native_keccak256, native_leaf_hash, rlp_list_header,
    rlp_long_len_bytes, rlp_string_header,
};
pub use native_trie::*;
