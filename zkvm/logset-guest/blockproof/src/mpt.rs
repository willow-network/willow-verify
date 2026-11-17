







use alloy_primitives::{keccak256, Bytes, B256};
use alloy_trie::proof::verify_proof;

use alloy_trie::Nibbles;
use anyhow::{bail, ensure, Context, Result};
use logset_blockcheck::receipts::{rlp_item, rlp_list_items};


pub const MAX_ITEMS: usize = logset_blockcheck::trie::MAX_VALUES;


pub fn key(index: usize) -> Vec<u8> {
    alloy_rlp::encode(index)
}





































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
