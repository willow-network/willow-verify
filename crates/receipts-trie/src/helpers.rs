//! Pure-Rust native keccak + RLP leaf-hash helpers (backend-agnostic).
//!
//! Originally extracted from `willow-gkr`'s Expander-coupled GF2 keccak +
//! receipts-trie circuits (since removed) so `native_trie` (and a Binius
//! receipts-root prover) can use them without pulling the Expander/GF2
//! circuit stack.

use alloc::vec::Vec;
use tiny_keccak::Hasher;

/// Native Keccak-256, as a reference for circuit-output parity.
pub fn native_keccak256(msg: &[u8]) -> [u8; 32] {
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(msg);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}

/// Minimal big-endian length bytes for an RLP long string/list. The prefix byte
/// is `0xb7 + bytes.len()` for a string / `0xf7 + bytes.len()` for a list. Covers
/// 1-byte (56..255), be16 (≤ 0xffff), be24 (≤ 0xffffff), … — the canonical form
/// alloy emits.
pub fn rlp_long_len_bytes(len: usize) -> Vec<u8> {
    let be = len.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(be.len() - 1);
    be[first..].to_vec()
}

/// Length bytes for the in-circuit leaf header, floored at 2 bytes (be16).
/// This is the circuit's `leaf_header_widths` cap-based form, NOT canonical
/// RLP: for values < 256 B canonical RLP uses fewer header bytes (or the
/// short-string form entirely). Use only for circuit-witness assembly, where
/// the witnessed preimage must stay byte-identical to the circuit layout; on
/// the circuit's domain (receipt values ≥ 256 B) the two forms coincide.
/// Widens to be24 for `> 65535`-byte values.
pub fn circuit_leaf_len_bytes(len: usize) -> Vec<u8> {
    let mut lb = rlp_long_len_bytes(len);
    while lb.len() < 2 {
        lb.insert(0, 0);
    }
    lb
}

/// Canonical RLP header for a string of `bytes`: nothing for a single byte
/// < 0x80, `0x80 + len` below 56 bytes, else `0xb7 + n` followed by the
/// minimal big-endian length.
pub fn rlp_string_header(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return Vec::new();
    }
    if bytes.len() < 56 {
        return alloc::vec![0x80 + bytes.len() as u8];
    }
    let lb = rlp_long_len_bytes(bytes.len());
    let mut out = Vec::with_capacity(1 + lb.len());
    out.push(0xb7 + lb.len() as u8);
    out.extend_from_slice(&lb);
    out
}

/// Canonical RLP header for a list with `payload_len` payload bytes:
/// `0xc0 + len` below 56, else `0xf7 + n` followed by the minimal
/// big-endian length.
pub fn rlp_list_header(payload_len: usize) -> Vec<u8> {
    if payload_len < 56 {
        return alloc::vec![0xc0 + payload_len as u8];
    }
    let lb = rlp_long_len_bytes(payload_len);
    let mut out = Vec::with_capacity(1 + lb.len());
    out.push(0xf7 + lb.len() as u8);
    out.extend_from_slice(&lb);
    out
}

/// Native leaf-node hash: `keccak(RLP([hp_str, value]))` in canonical RLP for
/// every value length — validated against `alloy-trie` from 33-byte synthetic
/// values through > 65535-byte (be24) leaves. For receipt-shaped values
/// (≥ 256 B) the bytes are identical to the circuit's cap-based layout
/// ([`circuit_leaf_len_bytes`]), so circuit↔native parity holds on the
/// circuit's whole domain.
pub fn native_leaf_hash(hp_str: &[u8], value: &[u8]) -> [u8; 32] {
    let v_hdr = rlp_string_header(value);
    let payload = hp_str.len() + v_hdr.len() + value.len();
    let p_hdr = rlp_list_header(payload);
    let mut node = Vec::with_capacity(p_hdr.len() + payload);
    node.extend_from_slice(&p_hdr);
    node.extend_from_slice(hp_str);
    node.extend_from_slice(&v_hdr);
    node.extend_from_slice(value);
    native_keccak256(&node)
}
