












#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

mod helpers;
mod native_trie;

pub use helpers::{
    native_keccak256, native_leaf_hash, rlp_list_header,
    rlp_long_len_bytes, rlp_string_header,
};
pub use native_trie::*;
