





















use crate::helpers::{native_keccak256, native_leaf_hash};


use alloc::vec;
use alloc::vec::Vec;



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeafSpec {

    pub hp: Vec<u8>,

    pub value_index: usize,

    pub hash: [u8; 32],
}



#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchSpec {

    pub children: [Option<[u8; 32]>; 16],

    pub hash: [u8; 32],
}





#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionSpec {

    pub hp: Vec<u8>,

    pub child: [u8; 32],

    pub hash: [u8; 32],
}



#[derive(Debug, Clone)]
pub struct TriePlan {
    pub root: [u8; 32],
    pub leaves: Vec<LeafSpec>,
    pub branches: Vec<BranchSpec>,

    pub extensions: Vec<ExtensionSpec>,
}


pub type LeafHasher<'a> = &'a dyn Fn(&[u8], &[u8]) -> [u8; 32];



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



fn index_nibbles(i: usize) -> Vec<u8> {
    rlp_index(i)
        .into_iter()
        .flat_map(|b| [b >> 4, b & 0x0f])
        .collect()
}



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



fn hp_encode(nibbles: &[u8], is_leaf: bool) -> Vec<u8> {
    rlp_short_string(&hp_bytes(nibbles, is_leaf))
}


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


pub fn empty_root() -> [u8; 32] {
    native_keccak256(&[0x80])
}




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





pub fn build_trie(values: &[Vec<u8>]) -> TriePlan {
    build_trie_with_leaf_hasher(values, &|hp, v| native_leaf_hash(hp, v))
}






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




























































































































































































































































































































































































































































































































