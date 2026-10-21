//! Native receipts-trie reconstruction (topology backbone + oracle).
//!
//! Builds the full Ethereum receipts trie for keys `rlp(0..N)` and returns
//! the root plus the node DAG (leaf + branch specs) the circuit layer
//! proves. It is the byte-exact oracle the in-circuit node proofs check
//! against (validated here vs `alloy_trie::ordered_trie_root_encoded`).
//!
//! ## Scope
//!
//! [`build_trie`] is a general recursive MPT builder covering **N up to
//! 65536** (validated vs `alloy_trie` for N=1..512): `index ≤ 127` keys are
//! 1-byte rlp (2 nibbles, no extension nodes); `128..=255` are 2-byte
//! (`0x81 XX`, 4 nibbles) and `256+` are 3-byte (`0x82 XX XX`, 6 nibbles),
//! whose shared multi-nibble prefixes introduce **extension nodes**.
//! Receipts always carry the 256-byte bloom, so every node ≥ 32 B and is
//! hash-referenced (the `<32 B` inline case never triggers).
//!
//! [`build_trie`], [`trie_topology`], and [`verify_binding`] all cover the
//! full range (validated for N=1..512). Proving extension nodes in a
//! circuit needs an extension-mode arm and a
//! witnessed-HP-length cap in the node circuit.

use crate::helpers::{native_keccak256, native_leaf_hash};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;

/// A leaf the trie contains: its RLP-encoded HP key and which receipt
/// value it holds (index into the input `values`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafSpec {
    /// RLP-encoded hex-prefix key (1 byte for ≤1-nibble paths, 3 for 2-nibble).
    pub hp: Vec<u8>,
    /// Index into the input `values` slice.
    pub value_index: usize,
    /// The leaf node's Keccak hash (what a parent branch references).
    pub hash: [u8; 32],
}

/// A branch node: which of its 16 slots hold child node hashes (the value
/// slot is always empty for a receipts trie).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSpec {
    /// `children[s] = Some(child_hash)` for occupied slot `s`.
    pub children: [Option<[u8; 32]>; 16],
    /// This branch node's Keccak hash.
    pub hash: [u8; 32],
}

/// An extension node: a shared-prefix HP key pointing at a single child node
/// (always a branch, always hash-referenced — receipts never inline). Appears
/// for N > 128, where variable-length `rlp(index)` keys share multi-nibble
/// prefixes (e.g. the `[8,1]` of every `0x81 XX` key).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSpec {
    /// RLP-encoded hex-prefix key (extension flag) of the shared nibble run.
    pub hp: Vec<u8>,
    /// The child node's Keccak hash.
    pub child: [u8; 32],
    /// This extension node's Keccak hash.
    pub hash: [u8; 32],
}

/// The reconstructed trie: root hash + the leaf, branch, and extension nodes
/// that compose it. The circuit layer proves every node and binds their hashes.
#[derive(Debug, Clone)]
pub struct TriePlan {
    pub root: [u8; 32],
    pub leaves: Vec<LeafSpec>,
    pub branches: Vec<BranchSpec>,
    /// Extension nodes (empty for N ≤ 128).
    pub extensions: Vec<ExtensionSpec>,
}

/// A leaf hasher: `(rlp_hp_key, value) -> keccak(leaf node bytes)`.
pub type LeafHasher<'a> = &'a dyn Fn(&[u8], &[u8]) -> [u8; 32];

/// RLP-encode integer key `i`: `rlp(0)=0x80`; `rlp(1..=127)=byte`;
/// `rlp(128..=255)=0x81||byte`; `rlp(256..=65535)=0x82||be16`.
fn rlp_index(i: usize) -> Vec<u8> {
    if i == 0 {
        vec![0x80]
    } else if i <= 0x7f {
        vec![i as u8]
    } else if i <= 0xff {
        vec![0x81, i as u8]
    } else if i <= 0xffff {
        vec![0x82, (i >> 8) as u8, (i & 0xff) as u8]
    } else {
        panic!("receipts-trie index {i} exceeds the supported range (<= 65535)")
    }
}

/// Nibble path of `rlp(i)` (each byte → high then low nibble). 2 nibbles for
/// `i ≤ 127`, 4 for `128..=255`, 6 for `256..=65535`.
fn index_nibbles(i: usize) -> Vec<u8> {
    rlp_index(i)
        .into_iter()
        .flat_map(|b| [b >> 4, b & 0x0f])
        .collect()
}

/// Raw hex-prefix bytes for a nibble path. `is_leaf` picks the leaf flag
/// (2/3) vs the extension flag (0/1); `+1` marks an odd-length path.
fn hp_bytes(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
    let flag = if is_leaf { 2u8 } else { 0u8 };
    let odd = nibbles.len() % 2 == 1;
    let mut out = Vec::with_capacity(nibbles.len() / 2 + 1);
    let start = if odd {
        out.push(((flag + 1) << 4) | nibbles[0]);
        1
    } else {
        out.push(flag << 4);
        0
    };
    let mut k = start;
    while k + 1 < nibbles.len() {
        out.push((nibbles[k] << 4) | nibbles[k + 1]);
        k += 2;
    }
    out
}

/// RLP-encode a short (`< 56` byte) string: a lone byte `< 0x80` self-encodes,
/// else `0x80+len` prefix. Sufficient for HP strings (≤ ~4 bytes).
fn rlp_short_string(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        bytes.to_vec()
    } else {
        let mut out = Vec::with_capacity(bytes.len() + 1);
        out.push(0x80 + bytes.len() as u8);
        out.extend_from_slice(bytes);
        out
    }
}

/// RLP-encoded HP string for a leaf/extension node — the `hp` field stored in
/// the specs, ready to splice into the node's RLP list.
fn hp_encode(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
    rlp_short_string(&hp_bytes(nibbles, is_leaf))
}

/// Longest common nibble prefix across all entries' remaining paths.
fn longest_common_prefix(entries: &[(Vec<u8>, usize)]) -> Vec<u8> {
    let first = entries[0].0.clone();
    let mut len = first.len();
    for (nibs, _) in &entries[1..] {
        len = len.min(nibs.len());
        let mut k = 0;
        while k < len && nibs[k] == first[k] {
            k += 1;
        }
        len = k;
        if len == 0 {
            return Vec::new();
        }
    }
    first[..len].to_vec()
}

/// Empty-trie root: `keccak(rlp("")) = keccak(0x80)` (== alloy `EMPTY_ROOT_HASH`).
pub fn empty_root() -> [u8; 32] {
    native_keccak256(&[0x80])
}

/// Native 17-way branch-node hash for arbitrary occupancy, covering both
/// RLP long-list header forms (1-byte length ≤255, else 2-byte). The
/// value slot (17th) is the empty string.
pub fn native_branch_hash(children: &[Option<[u8; 32]>; 16]) -> [u8; 32] {
    let payload: usize = children
        .iter()
        .map(|c| if c.is_some() { 33 } else { 1 })
        .sum::<usize>()
        + 1;
    let mut node = Vec::with_capacity(3 + payload);
    if payload <= 55 {
        node.push(0xc0 + payload as u8);
    } else if payload <= 255 {
        node.push(0xf8);
        node.push(payload as u8);
    } else {
        node.push(0xf9);
        node.extend_from_slice(&(payload as u16).to_be_bytes());
    }
    for c in children {
        match c {
            Some(h) => {
                node.push(0xa0);
                node.extend_from_slice(h);
            }
            None => node.push(0x80),
        }
    }
    node.push(0x80); // empty value slot
    native_keccak256(&node)
}

/// Native extension-node hash: `keccak(RLP([hp_str, 0xa0 ‖ child_hash]))`.
/// The child is always a branch (≥ 32 B) so it is hash-referenced (`0xa0`).
pub fn native_extension_hash(hp_str: &[u8], child_hash: &[u8; 32]) -> [u8; 32] {
    let payload = hp_str.len() + 33; // hp (already RLP'd) + 0xa0 + 32-byte hash
    let mut node = Vec::with_capacity(2 + payload);
    if payload <= 55 {
        node.push(0xc0 + payload as u8);
    } else {
        node.push(0xf8);
        node.push(payload as u8);
    }
    node.extend_from_slice(hp_str);
    node.push(0xa0);
    node.extend_from_slice(child_hash);
    native_keccak256(&node)
}

/// Recursively build the MPT node for `entries` (each `(remaining_nibbles,
/// value_index)`), appending leaf/branch/extension specs and returning the
/// node's hash. Standard Ethereum MPT: one entry → leaf; a shared multi-nibble
/// prefix → extension over a branch; else → branch on the next nibble.
fn build_node(
    entries: &[(Vec<u8>, usize)],
    values: &[Vec<u8>],
    leaf_hash: LeafHasher,
    leaves: &mut Vec<LeafSpec>,
    branches: &mut Vec<BranchSpec>,
    extensions: &mut Vec<ExtensionSpec>,
) -> [u8; 32] {
    debug_assert!(!entries.is_empty());
    if entries.len() == 1 {
        let (nibs, vi) = &entries[0];
        let hp = hp_encode(nibs, true);
        let hash = leaf_hash(&hp, &values[*vi]);
        leaves.push(LeafSpec {
            hp,
            value_index: *vi,
            hash,
        });
        return hash;
    }
    let lcp = longest_common_prefix(entries);
    if !lcp.is_empty() {
        // Shared prefix → extension node over a branch on the diverging nibble.
        let stripped: Vec<(Vec<u8>, usize)> = entries
            .iter()
            .map(|(nibs, vi)| (nibs[lcp.len()..].to_vec(), *vi))
            .collect();
        let child = build_branch(&stripped, values, leaf_hash, leaves, branches, extensions);
        let hp = hp_encode(&lcp, false);
        let hash = native_extension_hash(&hp, &child);
        extensions.push(ExtensionSpec { hp, child, hash });
        return hash;
    }
    build_branch(entries, values, leaf_hash, leaves, branches, extensions)
}

/// Build a branch: distribute `entries` by their next nibble (they differ
/// there — any shared prefix was already consumed), recurse, append the spec.
/// No receipts key is a prefix of another, so the value slot is always empty.
fn build_branch(
    entries: &[(Vec<u8>, usize)],
    values: &[Vec<u8>],
    leaf_hash: LeafHasher,
    leaves: &mut Vec<LeafSpec>,
    branches: &mut Vec<BranchSpec>,
    extensions: &mut Vec<ExtensionSpec>,
) -> [u8; 32] {
    let mut slots: [Vec<(Vec<u8>, usize)>; 16] = Default::default();
    for (nibs, vi) in entries {
        debug_assert!(
            !nibs.is_empty(),
            "no receipts-trie key terminates at a branch point"
        );
        slots[nibs[0] as usize].push((nibs[1..].to_vec(), *vi));
    }
    let mut children: [Option<[u8; 32]>; 16] = Default::default();
    for (s, slot) in slots.iter().enumerate() {
        if !slot.is_empty() {
            children[s] = Some(build_node(
                slot, values, leaf_hash, leaves, branches, extensions,
            ));
        }
    }
    let hash = native_branch_hash(&children);
    branches.push(BranchSpec { children, hash });
    hash
}

/// Reconstruct the receipts trie for `values` (keys `rlp(0..len)`). Returns
/// the full [`TriePlan`]; `plan.root` must equal `ordered_trie_root_encoded`.
/// Supports N up to 65536 — extension nodes appear for N > 128 (variable-length
/// `rlp(index)` keys).
pub fn build_trie(values: &[Vec<u8>]) -> TriePlan {
    build_trie_with_leaf_hasher(values, &|hp, v| native_leaf_hash(hp, v))
}

/// [`build_trie`] with a caller-supplied leaf hasher, for circuit-parity twins
/// whose leaf byte layout is not canonical RLP (the binius witness layer's
/// cap-based headers). Branch and extension nodes hash identically in every
/// layout — only the leaf form differs — so parameterizing the leaf hash is
/// sufficient for a self-consistent plan.
pub fn build_trie_with_leaf_hasher(values: &[Vec<u8>], leaf_hash: LeafHasher) -> TriePlan {
    assert!(values.len() <= 0x10000, "build_trie covers N <= 65536");
    if values.is_empty() {
        return TriePlan {
            root: empty_root(),
            leaves: vec![],
            branches: vec![],
            extensions: vec![],
        };
    }
    let entries: Vec<(Vec<u8>, usize)> = (0..values.len()).map(|i| (index_nibbles(i), i)).collect();
    let mut leaves = Vec::new();
    let mut branches = Vec::new();
    let mut extensions = Vec::new();
    let root = build_node(
        &entries,
        values,
        leaf_hash,
        &mut leaves,
        &mut branches,
        &mut extensions,
    );
    TriePlan {
        root,
        leaves,
        branches,
        extensions,
    }
}

// ===========================================================================
// Verifier-side topology + binding.
//
// The trie shape is fully determined by N (keys are rlp(0..N)), so a
// verifier reconstructs the expected structure from N alone — no values,
// no prover-supplied plan to trust — then checks the proofs' public hashes
// wire up correctly to `receipts_root`. This is the soundness glue between
// the (data-parallel) leaf and branch proofs: the leaf proof attests each
// leaf hash from a hidden receipt, the branch proof attests each node hash
// from its children, and `verify_binding` checks the child references + the
// root match the canonical N-topology.
// ===========================================================================

/// A child slot's referent in the reconstructed topology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChildRef {
    /// Index into [`TrieTopology::leaves`].
    Leaf(usize),
    /// Index into [`TrieTopology::branches`].
    Branch(usize),
    /// Index into [`TrieTopology::extensions`].
    Extension(usize),
}

/// A leaf's position-determined shape (no value/hash).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopoLeaf {
    pub hp: Vec<u8>,
    pub value_index: usize,
}

/// A branch's wiring: which slots reference which child nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopoBranch {
    pub children: [Option<ChildRef>; 16],
}

/// An extension's position-determined shape: its canonical HP key and the
/// (always-branch) child node it references.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopoExt {
    pub hp: Vec<u8>,
    pub child: ChildRef,
}

/// The trie structure derived from N alone. Node vecs are in the same order
/// [`build_trie`] emits them, so a `TriePlan`'s hashes index directly here.
#[derive(Debug, Clone)]
pub struct TrieTopology {
    pub leaves: Vec<TopoLeaf>,
    pub branches: Vec<TopoBranch>,
    pub extensions: Vec<TopoExt>,
    pub root: ChildRef,
}

/// Reconstruct the trie structure for `rlp(0..n)` keys from N alone (no
/// values) — mirrors [`build_trie`]'s recursion and node ordering exactly, so
/// a `TriePlan`'s hashes index directly into this topology. Supports N up to
/// 65536 (extension nodes for N > 128).
pub fn trie_topology(n: usize) -> TrieTopology {
    assert!(
        (1..=0x10000).contains(&n),
        "trie_topology covers 1 <= n <= 65536"
    );
    let entries: Vec<(Vec<u8>, usize)> = (0..n).map(|i| (index_nibbles(i), i)).collect();
    let mut leaves = Vec::new();
    let mut branches = Vec::new();
    let mut extensions = Vec::new();
    let root = topo_node(&entries, &mut leaves, &mut branches, &mut extensions);
    TrieTopology {
        leaves,
        branches,
        extensions,
        root,
    }
}

/// Mirror of [`build_node`] without hashes — returns the node's [`ChildRef`].
fn topo_node(
    entries: &[(Vec<u8>, usize)],
    leaves: &mut Vec<TopoLeaf>,
    branches: &mut Vec<TopoBranch>,
    extensions: &mut Vec<TopoExt>,
) -> ChildRef {
    if entries.len() == 1 {
        let (nibs, vi) = &entries[0];
        let li = leaves.len();
        leaves.push(TopoLeaf {
            hp: hp_encode(nibs, true),
            value_index: *vi,
        });
        return ChildRef::Leaf(li);
    }
    let lcp = longest_common_prefix(entries);
    if !lcp.is_empty() {
        let stripped: Vec<(Vec<u8>, usize)> = entries
            .iter()
            .map(|(nibs, vi)| (nibs[lcp.len()..].to_vec(), *vi))
            .collect();
        let child = topo_branch(&stripped, leaves, branches, extensions);
        let ei = extensions.len();
        extensions.push(TopoExt {
            hp: hp_encode(&lcp, false),
            child,
        });
        return ChildRef::Extension(ei);
    }
    topo_branch(entries, leaves, branches, extensions)
}

/// Mirror of [`build_branch`] without hashes.
fn topo_branch(
    entries: &[(Vec<u8>, usize)],
    leaves: &mut Vec<TopoLeaf>,
    branches: &mut Vec<TopoBranch>,
    extensions: &mut Vec<TopoExt>,
) -> ChildRef {
    let mut slots: [Vec<(Vec<u8>, usize)>; 16] = Default::default();
    for (nibs, vi) in entries {
        slots[nibs[0] as usize].push((nibs[1..].to_vec(), *vi));
    }
    let mut children: [Option<ChildRef>; 16] = Default::default();
    for (s, slot) in slots.iter().enumerate() {
        if !slot.is_empty() {
            children[s] = Some(topo_node(slot, leaves, branches, extensions));
        }
    }
    let bi = branches.len();
    branches.push(TopoBranch { children });
    ChildRef::Branch(bi)
}

/// Verify that proof-public hashes form the canonical N-trie up to
/// `expected_root`. `leaf_hashes[i]` is leaf `i`'s proven hash;
/// `branch_specs[j]` carries branch `j`'s proven (occupancy, child hashes,
/// node hash). Returns `Err` on any wiring/root/count mismatch.
pub fn verify_binding(
    n: usize,
    expected_root: [u8; 32],
    leaf_hashes: &[[u8; 32]],
    leaf_hps: &[Vec<u8>],
    branch_specs: &[BranchSpec],
    extension_specs: &[ExtensionSpec],
) -> Result<(), String> {
    if n == 0 {
        return if expected_root == empty_root()
            && leaf_hashes.is_empty()
            && branch_specs.is_empty()
            && extension_specs.is_empty()
        {
            Ok(())
        } else {
            Err("empty block: expected empty root and no nodes".into())
        };
    }
    let topo = trie_topology(n);
    if leaf_hashes.len() != topo.leaves.len() {
        return Err(format!(
            "leaf count {} != topology {}",
            leaf_hashes.len(),
            topo.leaves.len()
        ));
    }
    // Pin each leaf's (prover-committed, public) hex-prefix key to the
    // canonical hp the verifier derives from N alone. Without this, a leaf's
    // path is bound only indirectly via collision-resistance of the
    // authenticated root; this makes it a direct check — load-bearing for the
    // split-prover / browser model.
    if leaf_hps.len() != topo.leaves.len() {
        return Err(format!(
            "leaf hp count {} != topology {}",
            leaf_hps.len(),
            topo.leaves.len()
        ));
    }
    for (i, tl) in topo.leaves.iter().enumerate() {
        if leaf_hps[i] != tl.hp {
            return Err(format!(
                "leaf {i} hp does not match canonical rlp(index) path"
            ));
        }
    }
    if branch_specs.len() != topo.branches.len() {
        return Err(format!(
            "branch count {} != topology {}",
            branch_specs.len(),
            topo.branches.len()
        ));
    }
    if extension_specs.len() != topo.extensions.len() {
        return Err(format!(
            "extension count {} != topology {}",
            extension_specs.len(),
            topo.extensions.len()
        ));
    }
    // Resolve a child reference to the proven hash of the node it points at.
    let child_hash = |c: ChildRef| -> [u8; 32] {
        match c {
            ChildRef::Leaf(li) => leaf_hashes[li],
            ChildRef::Branch(bi) => branch_specs[bi].hash,
            ChildRef::Extension(ei) => extension_specs[ei].hash,
        }
    };
    // Extensions: pin the (public) hp to the canonical prefix and the child to
    // the node the topology says it points at (same direct-binding rationale).
    for (ei, te) in topo.extensions.iter().enumerate() {
        if extension_specs[ei].hp != te.hp {
            return Err(format!("extension {ei} hp does not match canonical path"));
        }
        if extension_specs[ei].child != child_hash(te.child) {
            return Err(format!("extension {ei} child does not match topology"));
        }
    }
    for (j, tb) in topo.branches.iter().enumerate() {
        for s in 0..16 {
            let expected_child = tb.children[s].map(&child_hash);
            if branch_specs[j].children[s] != expected_child {
                return Err(format!("branch {j} slot {s} child does not match topology"));
            }
        }
    }
    if child_hash(topo.root) != expected_root {
        return Err("root hash does not match expected receipts_root".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom};
    use alloy::eips::eip2718::Encodable2718;
    use alloy::primitives::{Address, Bloom, Log as AlloyLog, LogData, B256};
    use alloy_trie::root::ordered_trie_root_encoded;

    fn receipt(seed: u8, n_logs: usize) -> Vec<u8> {
        let mut logs = Vec::new();
        for k in 0..n_logs {
            let mut from = [0u8; 32];
            from[12..].copy_from_slice(&[0xAAu8.wrapping_add(k as u8); 20]);
            logs.push(AlloyLog {
                address: Address::from([seed; 20]),
                data: LogData::new_unchecked(
                    vec![B256::from([0x11u8; 32]), B256::from(from)],
                    vec![0u8; 32].into(),
                ),
            });
        }
        ReceiptEnvelope::Legacy(ReceiptWithBloom {
            receipt: Receipt {
                status: true.into(),
                cumulative_gas_used: 50_000,
                logs,
            },
            logs_bloom: Bloom::ZERO,
        })
        .encoded_2718()
    }

    fn alloy_root(values: &[Vec<u8>]) -> [u8; 32] {
        let mut out = [0u8; 32];
        out.copy_from_slice(ordered_trie_root_encoded(values).as_slice());
        out
    }

    /// Short (non-receipt-shaped) values must still produce canonical roots.
    /// The 2026-08-27 repro: 40-70 B synthetic values disagreed with alloy at
    /// every N because the leaf header was floored at two length bytes, which
    /// is canonical only for >= 256 B values. Sweeps the RLP header regimes —
    /// short-string (< 56), one length byte (56..=255), two (256+) — across
    /// leaf-only and extension-bearing tree shapes. Values stay >= 33 B so
    /// every node is >= 32 B and hash-referenced (the builder's domain).
    #[test]
    fn short_value_roots_match_alloy() {
        for &vlen in &[33usize, 40, 55, 56, 57, 70, 127, 128, 255, 256, 259] {
            for &n in &[1usize, 3, 16, 130] {
                let values: Vec<Vec<u8>> = (0..n)
                    .map(|i| {
                        (0..vlen)
                            .map(|j| (j as u8).wrapping_mul(7).wrapping_add(i as u8))
                            .collect()
                    })
                    .collect();
                let plan = build_trie(&values);
                assert_eq!(
                    plan.root,
                    alloy_root(&values),
                    "native root disagreed with alloy at value_len={vlen}, N={n}"
                );
            }
        }
    }

    #[test]
    fn empty_root_matches_alloy() {
        assert_eq!(empty_root(), alloy_root(&[]));
    }

    #[test]
    fn root_matches_alloy_across_n() {
        // 1..=128: the all-2-nibble regime (no extension nodes). 129+: 2-byte
        // rlp keys (0x81 XX) introduce extension nodes; 256+: 3-byte keys
        // (0x82 XX XX) deepen them. Covers the boundaries + busy-block sizes.
        for &n in &[
            1usize, 2, 4, 16, 64, 127, 128, 129, 130, 160, 200, 255, 256, 257, 300, 400, 512,
        ] {
            let values: Vec<Vec<u8>> = (0..n).map(|i| receipt(i as u8, 1 + (i % 3))).collect();
            let plan = build_trie(&values);
            assert_eq!(
                plan.root,
                alloy_root(&values),
                "native trie root disagreed with alloy at N={n}"
            );
            // Every receipt is represented by exactly one leaf.
            assert_eq!(plan.leaves.len(), n, "leaf count must equal N at N={n}");
            // N<=128 (all 1-byte keys) never produces an extension node.
            // Above 128 it's non-monotonic — a 2-byte-key group yields an
            // extension only when its members share their next nibble (e.g.
            // N=130: 128,129 share [8]) but a branch when they diverge (e.g.
            // N=160: 128..159 span 3rd-nibbles 8 and 9) — so the root-match
            // and leaf-count are the structural oracle, not extension presence.
            if n <= 128 {
                assert!(
                    plan.extensions.is_empty(),
                    "no extensions expected at N={n}"
                );
            }
        }
    }

    /// One receipt whose RLP exceeds 65535 bytes via a single big-data log, used
    /// to drive the trie-leaf RLP long-string forms above the 2-byte length cap.
    fn whale_receipt(seed: u8, data_len: usize) -> Vec<u8> {
        let log = AlloyLog {
            address: Address::from([seed; 20]),
            data: LogData::new_unchecked(
                vec![B256::from([0x11u8; 32])],
                vec![0xABu8; data_len].into(),
            ),
        };
        ReceiptEnvelope::Legacy(ReceiptWithBloom {
            receipt: Receipt {
                status: true.into(),
                cumulative_gas_used: 50_000,
                logs: vec![log],
            },
            logs_bloom: Bloom::ZERO,
        })
        .encoded_2718()
    }

    /// REGRESSION (the >65535-byte receipt): sweep a single receipt's RLP
    /// length across the 65535 leaf boundary. The leaf value-length header
    /// crosses from the 2-byte `0xb9‖be16` to the 3-byte `0xba‖be24` RLP
    /// long-string form at 65536; the outer list payload crosses 0xffff a few
    /// bytes earlier (value ≤ 0xffff but payload > 0xffff ⇒ `0xfa‖be24` outer
    /// header while the value header is still be16). Both must match alloy.
    /// Before the be24 fix, willow's `as u16` headers wrap and the root diverges.
    #[test]
    fn root_matches_alloy_across_the_65535_leaf_boundary() {
        for data_len in (65_150usize..=65_320)
            .step_by(5)
            .chain([60_000usize, 80_000, 131_000])
        {
            let v = whale_receipt(0x42, data_len);
            let values = vec![v.clone()];
            let plan = build_trie(&values);
            assert_eq!(
                plan.root,
                alloy_root(&values),
                "native root disagreed with alloy at receipt_len={} (data_len={data_len})",
                v.len()
            );
        }
    }

    #[test]
    fn branch_hash_handles_both_header_forms() {
        // ≤7 occupied → 1-byte length header; ≥8 occupied → 2-byte. Build a
        // trie that exercises a densely-occupied root branch (N=128 fills
        // root slots 0–8 = 9 children → 2-byte header form) and confirm the
        // root still matches alloy (already covered above, but assert the
        // payload-form boundary explicitly via a 9-child branch).
        let mut children: [Option<[u8; 32]>; 16] = Default::default();
        for c in children.iter_mut().take(9) {
            *c = Some([0x11u8; 32]);
        }
        // 9 occupied → payload = 9*33 + 7*1 + 1 = 305 > 255 → 0xf9 form.
        let h = native_branch_hash(&children);
        // Recompute independently to guard the header-form arithmetic.
        let mut expect = vec![0xf9u8];
        expect.extend_from_slice(&305u16.to_be_bytes());
        for c in &children {
            match c {
                Some(x) => {
                    expect.push(0xa0);
                    expect.extend_from_slice(x);
                }
                None => expect.push(0x80),
            }
        }
        expect.push(0x80);
        assert_eq!(h, native_keccak256(&expect));
    }

    #[test]
    fn topology_matches_build_trie_structure() {
        // The verifier's N-derived topology must match the prover's plan
        // structurally (same node counts, order, leaf HPs, extension HPs) —
        // including the extension-node regime above N=128.
        for &n in &[1usize, 2, 4, 16, 64, 128, 130, 200, 256, 400, 512] {
            let values: Vec<Vec<u8>> = (0..n).map(|i| receipt(i as u8, 1 + (i % 3))).collect();
            let plan = build_trie(&values);
            let topo = trie_topology(n);
            assert_eq!(plan.leaves.len(), topo.leaves.len(), "leaf count N={n}");
            assert_eq!(
                plan.branches.len(),
                topo.branches.len(),
                "branch count N={n}"
            );
            assert_eq!(
                plan.extensions.len(),
                topo.extensions.len(),
                "extension count N={n}"
            );
            for (l, tl) in plan.leaves.iter().zip(&topo.leaves) {
                assert_eq!(l.hp, tl.hp, "leaf hp N={n}");
                assert_eq!(l.value_index, tl.value_index, "leaf value_index N={n}");
            }
            for (e, te) in plan.extensions.iter().zip(&topo.extensions) {
                assert_eq!(e.hp, te.hp, "extension hp N={n}");
            }
        }
    }

    #[test]
    fn verify_binding_accepts_correct_rejects_tampered() {
        for &n in &[1usize, 2, 4, 16, 64, 128, 130, 200, 256, 400, 512] {
            let values: Vec<Vec<u8>> = (0..n).map(|i| receipt(i as u8, 1 + (i % 3))).collect();
            let plan = build_trie(&values);
            let leaf_hashes: Vec<[u8; 32]> = plan.leaves.iter().map(|l| l.hash).collect();
            let leaf_hps: Vec<Vec<u8>> = plan.leaves.iter().map(|l| l.hp.clone()).collect();
            let exts = plan.extensions.clone();

            assert!(
                verify_binding(n, plan.root, &leaf_hashes, &leaf_hps, &plan.branches, &exts)
                    .is_ok(),
                "correct plan must bind at N={n}"
            );

            let mut bad_root = plan.root;
            bad_root[0] ^= 1;
            assert!(
                verify_binding(n, bad_root, &leaf_hashes, &leaf_hps, &plan.branches, &exts)
                    .is_err(),
                "tampered root must fail at N={n}"
            );

            // A tampered leaf hash breaks the branch that references it.
            if n >= 2 {
                let mut bad_leaves = leaf_hashes.clone();
                bad_leaves[0][0] ^= 1;
                assert!(
                    verify_binding(n, plan.root, &bad_leaves, &leaf_hps, &plan.branches, &exts)
                        .is_err(),
                    "tampered leaf hash must fail at N={n}"
                );
            }

            // A mis-keyed leaf hp (wrong rlp(index) path) must be rejected
            // by the direct hp check, even with otherwise-correct hashes.
            let mut bad_hps = leaf_hps.clone();
            let last = bad_hps.len() - 1;
            bad_hps[last] = vec![0xff; bad_hps[last].len().max(1)];
            assert!(
                verify_binding(n, plan.root, &leaf_hashes, &bad_hps, &plan.branches, &exts)
                    .is_err(),
                "mis-keyed leaf hp must fail at N={n}"
            );

            // A tampered extension child must fail the extension wiring check.
            if !exts.is_empty() {
                let mut bad_exts = exts.clone();
                bad_exts[0].child[0] ^= 1;
                assert!(
                    verify_binding(
                        n,
                        plan.root,
                        &leaf_hashes,
                        &leaf_hps,
                        &plan.branches,
                        &bad_exts
                    )
                    .is_err(),
                    "tampered extension child must fail at N={n}"
                );
            }
        }
    }

    #[test]
    fn verify_binding_empty_block() {
        assert!(verify_binding(0, empty_root(), &[], &[], &[], &[]).is_ok());
        assert!(verify_binding(0, [1u8; 32], &[], &[], &[], &[]).is_err());
    }
}
