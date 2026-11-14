//! The parent's state as a trace proof carries it: trie nodes by their hash and codes by theirs.
//! A lookup starts at the parent's state root and follows hashes only, so a node no hash names is
//! never read, and a node the proof lacks is a gap ([`Found::Gap`]), never an absence.

use crate::wire;
use crate::{Failure, Kind};
use alloy::trie::nodes::TrieNode;
use alloy::trie::{Nibbles, TrieAccount, EMPTY_ROOT_HASH};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// The longest trie node, in bytes (a full branch is 532), and the most nodes a proof may carry.
pub const MAX_NODE_BYTES: usize = 1_100;
pub const MAX_NODES: usize = 1 << 20;
/// The longest code (EIP-170's limit is 24,576 bytes), and the most codes a proof may carry.
pub const MAX_CODE_BYTES: usize = 1 << 17;
pub const MAX_CODES: usize = 1 << 16;

/// Trie nodes and codes by their hashes.
#[derive(Clone, Debug, Default)]
pub struct Trie {
    nodes: HashMap<B256, Bytes>,
    codes: HashMap<B256, Bytes>,
}

/// What a lookup in the trie came to.
#[derive(Debug, PartialEq)]
pub enum Found<T> {
    /// Proven present, or (`None`) proven absent.
    Proven(Option<T>),
    /// The proof lacks a node on the way.
    Gap,
}

fn limits(m: impl Into<String>) -> Failure {
    Failure::new(Kind::Limits, m)
}

fn hex_bytes(s: &str, what: &str) -> Result<Bytes, Failure> {
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| Failure::new(Kind::Malformed, format!("{what} is not 0x hex")))?;
    hex::decode(h)
        .map(Bytes::from)
        .map_err(|e| Failure::new(Kind::Malformed, format!("{what}: {e}")))
}

impl Trie {
    /// A proof's witness, within the limits.
    pub fn from_wire(w: &wire::Witness) -> Result<Self, Failure> {
        if w.nodes.len() > MAX_NODES {
            return Err(limits(format!(
                "{} nodes, more than {MAX_NODES}",
                w.nodes.len()
            )));
        }
        if w.codes.len() > MAX_CODES {
            return Err(limits(format!(
                "{} codes, more than {MAX_CODES}",
                w.codes.len()
            )));
        }
        let mut t = Self::default();
        for n in &w.nodes {
            if n.len() > 2 + 2 * MAX_NODE_BYTES {
                return Err(limits(format!("a node longer than {MAX_NODE_BYTES} bytes")));
            }
            t.add_node(hex_bytes(n, "a node")?);
        }
        for c in &w.codes {
            if c.len() > 2 + 2 * MAX_CODE_BYTES {
                return Err(limits(format!("a code longer than {MAX_CODE_BYTES} bytes")));
            }
            t.add_code(hex_bytes(c, "a code")?);
        }
        Ok(t)
    }

    /// The nodes and codes `reads` names, as a proof carries them (sorted by hash).
    pub fn to_wire(&self, nodes: &HashSet<B256>, codes: &HashSet<B256>) -> wire::Witness {
        let hx = |b: &Bytes| format!("0x{}", hex::encode(b));
        let mut n: Vec<&B256> = nodes
            .iter()
            .filter(|h| self.nodes.contains_key(*h))
            .collect();
        n.sort();
        let mut c: Vec<&B256> = codes
            .iter()
            .filter(|h| self.codes.contains_key(*h))
            .collect();
        c.sort();
        wire::Witness {
            nodes: n.into_iter().map(|h| hx(&self.nodes[h])).collect(),
            codes: c.into_iter().map(|h| hx(&self.codes[h])).collect(),
        }
    }

    pub fn add_node(&mut self, node: Bytes) {
        self.nodes.insert(keccak256(&node), node);
    }

    pub fn add_code(&mut self, code: Bytes) {
        self.codes.insert(keccak256(&code), code);
    }

    pub fn nodes(&self) -> usize {
        self.nodes.len()
    }

    pub fn code(&self, hash: &B256, read: &RefCell<HashSet<B256>>) -> Option<Bytes> {
        let code = self.codes.get(hash).cloned();
        if code.is_some() {
            read.borrow_mut().insert(*hash);
        }
        code
    }

    fn node(&self, hash: &B256, read: &RefCell<HashSet<B256>>) -> Option<&Bytes> {
        let node = self.nodes.get(hash);
        if node.is_some() {
            read.borrow_mut().insert(*hash);
        }
        node
    }

    /// The value at `key` (a keccak hash) in the trie under `root`; each node read noted in
    /// `read`.
    fn get(&self, root: B256, key: B256, read: &RefCell<HashSet<B256>>) -> Found<Vec<u8>> {
        if root == EMPTY_ROOT_HASH {
            return Found::Proven(None);
        }
        let Some(mut bytes) = self.node(&root, read).map(|b| b.to_vec()) else {
            return Found::Gap;
        };
        let path = Nibbles::unpack(key);
        let mut at = 0usize;
        loop {
            let Ok(node) = TrieNode::decode(&mut bytes.as_slice()) else {
                return Found::Gap;
            };
            let child = match node {
                TrieNode::EmptyRoot => return Found::Proven(None),
                TrieNode::Leaf(leaf) => {
                    let rest = path.slice(at..);
                    return Found::Proven((leaf.key == rest).then_some(leaf.value));
                }
                TrieNode::Extension(ext) => {
                    if !path.slice(at..).starts_with(&ext.key) {
                        return Found::Proven(None);
                    }
                    at += ext.key.len();
                    ext.child
                }
                TrieNode::Branch(branch) => {
                    let Some(nibble) = path.get(at) else {
                        return Found::Gap;
                    };
                    if !branch.state_mask.is_bit_set(nibble) {
                        return Found::Proven(None);
                    }
                    let index = (branch.state_mask.get() & ((1u16 << nibble) - 1)).count_ones();
                    at += 1;
                    match branch.stack.get(index as usize) {
                        Some(c) => c.clone(),
                        None => return Found::Gap,
                    }
                }
            };
            bytes = match child.as_hash() {
                Some(h) => match self.node(&h, read) {
                    Some(b) => b.to_vec(),
                    None => return Found::Gap,
                },
                // A node under 32 bytes is held in its parent, which its hash named.
                None => child.as_slice().to_vec(),
            };
        }
    }

    /// `address`'s account in the state trie under `root`.
    pub fn account(
        &self,
        root: B256,
        address: &Address,
        read: &RefCell<HashSet<B256>>,
    ) -> Found<TrieAccount> {
        match self.get(root, keccak256(address), read) {
            Found::Proven(Some(v)) => match TrieAccount::decode(&mut v.as_slice()) {
                Ok(a) => Found::Proven(Some(a)),
                Err(_) => Found::Gap,
            },
            Found::Proven(None) => Found::Proven(None),
            Found::Gap => Found::Gap,
        }
    }

    /// Slot `index` in the storage trie under `root`.
    pub fn slot(&self, root: B256, index: &U256, read: &RefCell<HashSet<B256>>) -> Found<U256> {
        match self.get(root, keccak256(B256::from(*index)), read) {
            Found::Proven(Some(v)) => match U256::decode(&mut v.as_slice()) {
                Ok(x) => Found::Proven(Some(x)),
                Err(_) => Found::Gap,
            },
            Found::Proven(None) => Found::Proven(Some(U256::ZERO)),
            Found::Gap => Found::Gap,
        }
    }
}
