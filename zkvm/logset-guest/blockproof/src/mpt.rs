//! Merkle-Patricia proofs into a block's ordered tries (keys `rlp(index)`):
//! transactionsRoot and receiptsRoot.
//!
//! [`prove`] builds them with alloy-trie's `HashBuilder`. [`verify`] reads the
//! value at a key twice, with [`walk`] (a positional walk written for these
//! tries: every node hash-referenced, values only in leaves) and with
//! alloy-trie's `verify_proof`; the two must agree.

use alloy_primitives::{keccak256, Bytes, B256};
use alloy_trie::proof::{verify_proof, ProofRetainer};
use alloy_trie::root::adjust_index_for_rlp;
use alloy_trie::{HashBuilder, Nibbles};
use anyhow::{bail, ensure, Context, Result};
use logset_blockcheck::receipts::{rlp_item, rlp_list_items};

/// Most items a block's trie may hold here: `logset_blockcheck`'s bound.
pub const MAX_ITEMS: usize = logset_blockcheck::trie::MAX_VALUES;

/// The trie key of item `index`: `rlp(index)`.
pub fn key(index: usize) -> Vec<u8> {
    alloy_rlp::encode(index)
}

/// One item's proof: the trie nodes from the root down to its leaf.
pub type Path = Vec<Vec<u8>>;

/// The root of the ordered trie over `values`, and for each of `indices` the
/// nodes from the root down to that item's leaf.
pub fn prove<T: AsRef<[u8]>>(values: &[T], indices: &[usize]) -> Result<([u8; 32], Vec<Path>)> {
    ensure!(
        values.len() <= MAX_ITEMS,
        "{} trie values, at most {MAX_ITEMS} supported",
        values.len()
    );
    for &i in indices {
        ensure!(i < values.len(), "item {i} of a trie of {}", values.len());
    }
    let targets: Vec<Nibbles> = indices.iter().map(|&i| Nibbles::unpack(key(i))).collect();
    let mut hb = HashBuilder::default().with_proof_retainer(ProofRetainer::new(targets.clone()));
    for i in 0..values.len() {
        let index = adjust_index_for_rlp(i, values.len());
        hb.add_leaf(Nibbles::unpack(key(index)), values[index].as_ref());
    }
    let root = hb.root().0;
    let nodes = hb.take_proof_nodes();
    let proofs = targets
        .iter()
        .map(|t| {
            nodes
                .matching_nodes_sorted(t)
                .into_iter()
                .map(|(_, n)| n.to_vec())
                .collect()
        })
        .collect();
    Ok((root, proofs))
}

/// The value of item `index` under `root`, read from `proof` by both walkers.
pub fn verify(root: &[u8; 32], index: usize, proof: &[Vec<u8>]) -> Result<Vec<u8>> {
    ensure!(index < MAX_ITEMS, "item index {index} is past {MAX_ITEMS}");
    let k = key(index);
    let value = walk(root, &k, proof)?;
    let nodes: Vec<Bytes> = proof.iter().map(|n| Bytes::copy_from_slice(n)).collect();
    verify_proof(
        B256::from(*root),
        Nibbles::unpack(&k),
        Some(value.clone()),
        nodes.iter(),
    )
    .map_err(|e| anyhow::anyhow!("alloy-trie rejects the proof the walk accepted: {e}"))?;
    Ok(value)
}

/// The value at `key` under `root`: each node must hash to the reference its
/// parent holds (the root, for the first), branches and extensions must
/// follow the key, and the proof must end at the leaf for exactly `key`.
/// Inline (unhashed) nodes are refused: no transaction or receipt leaf is
/// short enough to be one.
pub fn walk(root: &[u8; 32], key: &[u8], proof: &[Vec<u8>]) -> Result<Vec<u8>> {
    let nibbles: Vec<u8> = key.iter().flat_map(|b| [b >> 4, b & 0x0f]).collect();
    let mut want = *root;
    let mut at = 0usize;
    for (n, node) in proof.iter().enumerate() {
        ensure!(
            keccak256(node).0 == want,
            "proof node {n} does not hash to the reference above it"
        );
        let (payload, total, is_list) =
            rlp_item(node).with_context(|| format!("proof node {n}"))?;
        ensure!(
            is_list && total == node.len(),
            "proof node {n} is not one RLP list"
        );
        let items = rlp_list_items(payload).with_context(|| format!("proof node {n}"))?;
        match items.len() {
            17 => {
                let next = *nibbles
                    .get(at)
                    .with_context(|| format!("proof node {n}: a branch below the key"))?;
                for (i, (child, list)) in items[..16].iter().enumerate() {
                    ensure!(
                        !list && (child.is_empty() || child.len() == 32),
                        "proof node {n}: branch child {i} is not a hash"
                    );
                }
                ensure!(
                    items[16].0.is_empty() && !items[16].1,
                    "proof node {n}: a branch with a value"
                );
                let child = items[next as usize].0;
                ensure!(
                    !child.is_empty(),
                    "proof node {n}: no child at nibble {next:x}: the key is not in the trie"
                );
                want = child.try_into().expect("32 bytes");
                at += 1;
            }
            2 => {
                let (path, list) = items[0];
                ensure!(!list && !path.is_empty(), "proof node {n}: bad path");
                let flag = path[0] >> 4;
                ensure!(flag <= 3, "proof node {n}: bad path flag {flag}");
                let mut seg = Vec::with_capacity(2 * path.len());
                if flag & 1 == 1 {
                    seg.push(path[0] & 0x0f);
                } else {
                    ensure!(path[0] & 0x0f == 0, "proof node {n}: bad path padding");
                }
                seg.extend(path[1..].iter().flat_map(|b| [b >> 4, b & 0x0f]));
                ensure!(
                    nibbles[at..].starts_with(&seg),
                    "proof node {n}: the path leaves the key: the key is not in the trie"
                );
                at += seg.len();
                let (child, list) = items[1];
                ensure!(!list, "proof node {n}: an inline child");
                if flag >= 2 {
                    ensure!(
                        at == nibbles.len(),
                        "proof node {n}: a leaf for another key"
                    );
                    ensure!(
                        n + 1 == proof.len(),
                        "{} proof nodes after the leaf",
                        proof.len() - n - 1
                    );
                    return Ok(child.to_vec());
                }
                ensure!(!seg.is_empty(), "proof node {n}: an empty extension");
                ensure!(
                    child.len() == 32,
                    "proof node {n}: extension child is not a hash"
                );
                want = child.try_into().expect("32 bytes");
            }
            k => bail!("proof node {n} has {k} items"),
        }
    }
    bail!("the proof ends before the item's leaf")
}
