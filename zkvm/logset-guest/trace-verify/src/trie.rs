



use crate::wire;
use crate::{Failure, Kind};
use alloy::trie::nodes::TrieNode;
use alloy::trie::{Nibbles, TrieAccount, EMPTY_ROOT_HASH};
use alloy_primitives::{keccak256, Address, Bytes, B256, U256};
use alloy_rlp::Decodable;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};


pub const MAX_NODE_BYTES: usize = 1_100;
pub const MAX_NODES: usize = 1 << 20;

pub const MAX_CODE_BYTES: usize = 1 << 17;
pub const MAX_CODES: usize = 1 << 16;


#[derive(Clone, Debug, Default)]
pub struct Trie {
    nodes: HashMap<B256, Bytes>,
    codes: HashMap<B256, Bytes>,
}


#[derive(Debug, PartialEq)]
pub enum Found<T> {

    Proven(Option<T>),

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




















    pub fn add_node(&mut self, node: Bytes) {
        self.nodes.insert(keccak256(&node), node);
    }

    pub fn add_code(&mut self, code: Bytes) {
        self.codes.insert(keccak256(&code), code);
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

                None => child.as_slice().to_vec(),
            };
        }
    }


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
