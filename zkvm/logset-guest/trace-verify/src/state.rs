//! A block's pre-state as its run reads it: the [`Trie`] under the parent's state root, and block
//! hashes from the hashes given. What the trie lacks is noted in [`Missing`] and read as empty;
//! a run that noted anything decides nothing (the server proves what it lacked and runs again,
//! the client fails the proof). Every node, code and block hash read is kept ([`Pre::reads`]), so
//! the server's proof holds exactly what the final run read.

use crate::trie::{Found, Trie};
use crate::Reads;
use alloy::trie::{TrieAccount, KECCAK_EMPTY};
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::database_interface::DBErrorMarker;
use revm::state::{AccountInfo, Bytecode};
use revm::DatabaseRef;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

/// What a run read that the pre-state does not hold.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Missing {
    /// Accounts (or their code), each with the slots read of it.
    pub accounts: BTreeMap<Address, BTreeSet<B256>>,
    /// Slots read of accounts the trie holds.
    pub slots: BTreeMap<Address, BTreeSet<B256>>,
}

impl Missing {
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty() && self.slots.is_empty()
    }
}

/// What the EVM asked for that can't be read as empty: a block hash not proven, or code by a hash
/// no account read has.
#[derive(Debug)]
pub enum Unreadable {
    BlockHash(u64),
    Code(B256),
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockHash(n) => write!(f, "the hash of block {n}"),
            Self::Code(h) => write!(f, "code {h}"),
        }
    }
}
impl std::error::Error for Unreadable {}
impl DBErrorMarker for Unreadable {}

/// The parent's state under `root` in a [`Trie`], block hashes from `hash_of`.
pub struct Pre<'a> {
    trie: &'a Trie,
    root: B256,
    hash_of: &'a dyn Fn(u64) -> Option<B256>,
    missing: RefCell<Missing>,
    /// Accounts looked up, and code hashes by an account that has them.
    accounts: RefCell<HashMap<Address, Option<TrieAccount>>>,
    owners: RefCell<HashMap<B256, Address>>,
    nodes_read: RefCell<HashSet<B256>>,
    codes_read: RefCell<HashSet<B256>>,
    hashes_read: RefCell<BTreeSet<u64>>,
}

impl std::fmt::Debug for Pre<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Pre").field("root", &self.root).finish()
    }
}

/// Code as the EVM runs it (its jump table found once).
pub fn bytecode(code: Bytes) -> Bytecode {
    Bytecode::new_raw_checked(code.clone()).unwrap_or_else(|_| Bytecode::new_legacy(code))
}

impl<'a> Pre<'a> {
    pub fn new(trie: &'a Trie, root: B256, hash_of: &'a dyn Fn(u64) -> Option<B256>) -> Self {
        Self {
            trie,
            root,
            hash_of,
            missing: RefCell::default(),
            accounts: RefCell::default(),
            owners: RefCell::default(),
            nodes_read: RefCell::default(),
            codes_read: RefCell::default(),
            hashes_read: RefCell::default(),
        }
    }

    /// What the run so far lacked, if anything (and it is cleared).
    pub fn take_missing(&self) -> Option<Missing> {
        let empty = self.missing.borrow().is_empty();
        (!empty).then(|| self.missing.take())
    }

    pub fn has_missing(&self) -> bool {
        !self.missing.borrow().is_empty()
    }

    /// Every node, code and block hash read so far.
    pub fn reads(&self) -> Reads {
        Reads {
            nodes: self.nodes_read.borrow().iter().copied().collect(),
            codes: self.codes_read.borrow().iter().copied().collect(),
            block_hashes: self.hashes_read.borrow().clone(),
        }
    }

    /// The nodes and codes read, for [`Trie::to_wire`].
    pub fn read_sets(&self) -> (HashSet<B256>, HashSet<B256>) {
        (
            self.nodes_read.borrow().clone(),
            self.codes_read.borrow().clone(),
        )
    }

    fn trie_account(&self, address: &Address) -> Option<Option<TrieAccount>> {
        if let Some(a) = self.accounts.borrow().get(address) {
            return Some(*a);
        }
        match self.trie.account(self.root, address, &self.nodes_read) {
            Found::Proven(a) => {
                self.accounts.borrow_mut().insert(*address, a);
                Some(a)
            }
            Found::Gap => None,
        }
    }

    fn note_account(&self, address: Address) {
        self.missing
            .borrow_mut()
            .accounts
            .entry(address)
            .or_default();
    }

    /// `address`'s account: `None` when absent, or (noted) when the trie lacks it.
    pub fn account(&self, address: Address) -> Option<TrieAccount> {
        match self.trie_account(&address) {
            Some(a) => {
                if let Some(a) = &a {
                    if a.code_hash != KECCAK_EMPTY {
                        self.owners.borrow_mut().insert(a.code_hash, address);
                    }
                }
                a
            }
            None => {
                self.note_account(address);
                None
            }
        }
    }

    /// Code by its hash: `None` only for a hash no account read has (a lacking code of an
    /// account read is noted and read as empty).
    pub fn code(&self, code_hash: B256) -> Option<Bytes> {
        if code_hash == KECCAK_EMPTY {
            return Some(Bytes::new());
        }
        if let Some(c) = self.trie.code(&code_hash, &self.codes_read) {
            return Some(c);
        }
        let owner = self.owners.borrow().get(&code_hash).copied();
        owner.map(|a| {
            self.note_account(a);
            Bytes::new()
        })
    }

    /// Slot `index` of `address`: zero when absent, or (noted) when the trie lacks it.
    pub fn storage(&self, address: Address, index: U256) -> U256 {
        let key = B256::from(index);
        match self.trie_account(&address) {
            Some(None) => U256::ZERO,
            Some(Some(a)) => match self.trie.slot(a.storage_root, &index, &self.nodes_read) {
                Found::Proven(v) => v.unwrap_or_default(),
                Found::Gap => {
                    self.missing
                        .borrow_mut()
                        .slots
                        .entry(address)
                        .or_default()
                        .insert(key);
                    U256::ZERO
                }
            },
            None => {
                self.missing
                    .borrow_mut()
                    .accounts
                    .entry(address)
                    .or_default()
                    .insert(key);
                U256::ZERO
            }
        }
    }

    /// Block `number`'s hash, if proven.
    pub fn block_hash(&self, number: u64) -> Option<B256> {
        let h = (self.hash_of)(number)?;
        self.hashes_read.borrow_mut().insert(number);
        Some(h)
    }
}

impl DatabaseRef for Pre<'_> {
    type Error = Unreadable;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Unreadable> {
        Ok(self.account(address).map(|a| AccountInfo {
            balance: a.balance,
            nonce: a.nonce,
            code_hash: a.code_hash,
            code: None,
        }))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Unreadable> {
        self.code(code_hash)
            .map(bytecode)
            .ok_or(Unreadable::Code(code_hash))
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Unreadable> {
        Ok(self.storage(address, index))
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Unreadable> {
        self.block_hash(number).ok_or(Unreadable::BlockHash(number))
    }
}

impl willow_op_exec::State for Pre<'_> {
    fn basic(&self, address: Address) -> Option<willow_op_exec::Account> {
        self.account(address).map(|a| willow_op_exec::Account {
            balance: a.balance,
            nonce: a.nonce,
            code_hash: a.code_hash,
            code: None,
        })
    }

    fn storage(&self, address: Address, slot: U256) -> U256 {
        Pre::storage(self, address, slot)
    }

    fn code(&self, hash: B256) -> Option<Bytes> {
        Pre::code(self, hash)
    }

    fn block_hash(&self, number: u64) -> Option<B256> {
        Pre::block_hash(self, number)
    }
}
