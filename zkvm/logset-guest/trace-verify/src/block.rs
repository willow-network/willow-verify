//! The block a trace proof names, checked: its header is `blockHash`'s, its parent's header (and
//! each header before it) is the one its successor links to, its transactions and receipts are
//! its transactionsRoot's and receiptsRoot's, one receipt per transaction; each transaction read
//! (its sender recovered once it is to run, [`Block::recover_through`]), each receipt with what
//! its run must come to.

use crate::wire::TraceProof;
use crate::{malformed, Failure, Kind, ETHEREUM, UNICHAIN};
use alloy::consensus::Header;
use alloy_primitives::{keccak256, Address, Bytes, Log, B256};
use logset_blockproof::txs::{self, Decoded};
use std::collections::BTreeMap;

/// The most transactions (and receipts) one block may have here.
pub const MAX_TXS: usize = 1 << 16;
/// The most headers before the parent a proof may carry (BLOCKHASH reads 256 back at most).
pub const MAX_ANCESTORS: usize = 256;

/// One transaction of the block, with its hash, and its sender once recovered.
#[derive(Clone, Debug)]
pub struct Tx {
    pub raw: Bytes,
    pub hash: B256,
    pub sender: Option<Address>,
    pub decoded: Decoded,
}

/// What a receipt records of its transaction's run.
#[derive(Clone, Debug)]
pub struct Receipt {
    pub cumulative_gas: u64,
    /// Its status, where it records one (from Byzantium).
    pub success: Option<bool>,
    pub logs: Vec<Log>,
    /// A deposit's (OP stack): its `depositNonce` and `depositReceiptVersion`.
    pub deposit: Option<(Option<u64>, Option<u64>)>,
}

/// A block to run again: its header, its parent's state root, the block hashes proven, its
/// transactions and receipts.
#[derive(Clone, Debug)]
pub struct Block {
    pub chain_id: u64,
    pub number: u64,
    pub hash: B256,
    pub header: Header,
    pub header_rlp: Bytes,
    pub parent_rlp: Bytes,
    pub parent_root: B256,
    /// Block hashes proven by the headers, by number: the parent's, its parent's, and those the
    /// ancestors name.
    hashes: BTreeMap<u64, B256>,
    pub ancestors: Vec<Bytes>,
    pub txs: Vec<Tx>,
    pub receipts: Vec<Receipt>,
}

fn hex_bytes(s: &str, what: &str) -> Result<Bytes, Failure> {
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| malformed(format!("{what} is not 0x hex")))?;
    hex::decode(h)
        .map(Bytes::from)
        .map_err(|e| malformed(format!("{what}: {e}")))
}

fn header_of(rlp: &[u8], what: &str) -> Result<Header, Failure> {
    logset_blockproof::proof::decode_header(rlp)
        .map_err(|e| Failure::new(Kind::Header, format!("{what}: {e:#}")))
}

/// The block's transactions and receipts, and the headers it links to, as raw bytes.
#[derive(Clone, Debug, Default)]
pub struct Parts {
    pub header: Bytes,
    pub parent: Bytes,
    pub ancestors: Vec<Bytes>,
    pub txs: Vec<Bytes>,
    pub receipts: Vec<Bytes>,
}

impl Block {
    /// The block a proof names, checked.
    pub fn from_proof(chain_id: u64, p: &TraceProof) -> Result<Block, Failure> {
        if p.ancestors.len() > MAX_ANCESTORS {
            return Err(Failure::new(
                Kind::Limits,
                format!("{} ancestors, more than {MAX_ANCESTORS}", p.ancestors.len()),
            ));
        }
        if p.transactions.len() > MAX_TXS || p.receipts.len() > MAX_TXS {
            return Err(Failure::new(
                Kind::Limits,
                format!("more than {MAX_TXS} transactions or receipts"),
            ));
        }
        let chain = crate::request::quantity(&serde_json::json!(p.chain_id)).map_err(malformed)?;
        if chain != chain_id {
            return Err(Failure::new(
                Kind::WrongChain,
                format!("the proof is for chain {chain}, not {chain_id}"),
            ));
        }
        let parts = Parts {
            header: hex_bytes(&p.header, "the header")?,
            parent: hex_bytes(&p.parent, "the parent's header")?,
            ancestors: p
                .ancestors
                .iter()
                .map(|a| hex_bytes(a, "an ancestor"))
                .collect::<Result<_, _>>()?,
            txs: p
                .transactions
                .iter()
                .map(|t| hex_bytes(t, "a transaction"))
                .collect::<Result<_, _>>()?,
            receipts: p
                .receipts
                .iter()
                .map(|r| hex_bytes(r, "a receipt"))
                .collect::<Result<_, _>>()?,
        };
        let block = Block::new(chain_id, parts, None)?;
        let said = hex_bytes(&p.block_hash, "blockHash")?;
        if said.as_ref() != block.hash.as_slice() {
            return Err(Failure::new(
                Kind::Header,
                "blockHash is not the keccak of the header",
            ));
        }
        let number =
            crate::request::quantity(&serde_json::json!(p.block_number)).map_err(malformed)?;
        if number != block.number {
            return Err(Failure::new(
                Kind::Header,
                "blockNumber is not the header's",
            ));
        }
        Ok(block)
    }

    /// The block of `parts` on chain `chain_id`, checked; with `senders` (the server's, which
    /// its store recovered) each transaction's sender, else none recovered yet.
    pub fn new(
        chain_id: u64,
        parts: Parts,
        senders: Option<Vec<Address>>,
    ) -> Result<Block, Failure> {
        if chain_id != ETHEREUM && chain_id != UNICHAIN {
            return Err(Failure::new(
                Kind::Unsupported,
                format!("traces are proven on Ethereum and Unichain, not chain {chain_id}"),
            ));
        }
        let header = header_of(&parts.header, "the header")?;
        let hash = keccak256(&parts.header);
        let number = header.number;
        if number == 0 {
            return Err(Failure::new(Kind::InvalidRequest, "genesis is not traced"));
        }
        let parent = header_of(&parts.parent, "the parent's header")?;
        if keccak256(&parts.parent) != header.parent_hash || parent.number + 1 != number {
            return Err(Failure::new(
                Kind::Parent,
                "the parent's header is not the one the block links to",
            ));
        }
        let mut hashes = BTreeMap::new();
        hashes.insert(number - 1, header.parent_hash);
        if number >= 2 {
            hashes.insert(number - 2, parent.parent_hash);
        }
        let mut next = (number.saturating_sub(2), parent.parent_hash);
        for (i, a) in parts.ancestors.iter().enumerate() {
            let h = header_of(a, &format!("ancestor {i}"))?;
            if keccak256(a) != next.1 || h.number != next.0 || h.number == 0 {
                return Err(Failure::new(
                    Kind::Parent,
                    format!(
                        "ancestor {i} is not block {}, its successor's parent",
                        next.0
                    ),
                ));
            }
            hashes.insert(h.number - 1, h.parent_hash);
            next = (h.number - 1, h.parent_hash);
        }
        let values = |v: &[Bytes]| v.iter().map(|b| b.to_vec()).collect::<Vec<_>>();
        let tx_values = values(&parts.txs);
        let root = logset_blockcheck::trie::ordered_root(&tx_values).map_err(|e| {
            Failure::new(Kind::Transactions, format!("the transactions' root: {e:#}"))
        })?;
        if root != header.transactions_root.0 {
            return Err(Failure::new(
                Kind::Transactions,
                "the transactions are not the block's (its transactionsRoot)",
            ));
        }
        let receipt_values = values(&parts.receipts);
        let root = logset_blockcheck::receipts::receipts_root(&receipt_values)
            .map_err(|e| Failure::new(Kind::Receipts, format!("the receipts' root: {e:#}")))?;
        if root != header.receipts_root.0 || receipt_values.len() != tx_values.len() {
            return Err(Failure::new(
                Kind::Receipts,
                "the receipts are not the block's (its receiptsRoot), one per transaction",
            ));
        }
        if senders.as_ref().is_some_and(|s| s.len() != tx_values.len()) {
            return Err(Failure::new(
                Kind::Internal,
                "a sender for each transaction",
            ));
        }
        let mut txs = Vec::with_capacity(parts.txs.len());
        for (i, raw) in parts.txs.iter().enumerate() {
            let bad = |e: String| Failure::new(Kind::Transactions, format!("transaction {i}: {e}"));
            let decoded = txs::decode(raw, chain_id).map_err(|e| bad(format!("{e:#}")))?;
            let sender = senders.as_ref().map(|s| s[i]);
            txs.push(Tx {
                raw: raw.clone(),
                hash: keccak256(raw),
                sender,
                decoded,
            });
        }
        let mut receipts = Vec::with_capacity(parts.receipts.len());
        for (i, value) in parts.receipts.iter().enumerate() {
            let r = logset_blockproof::render::decode_receipt(value, chain_id)
                .map_err(|e| Failure::new(Kind::Receipts, format!("receipt {i}: {e:#}")))?;
            let logs = r
                .logs
                .iter()
                .map(|l| {
                    Log::new_unchecked(
                        Address::from(l.address),
                        l.topics.iter().map(|t| B256::from(*t)).collect(),
                        Bytes::from(l.data.clone()),
                    )
                })
                .collect();
            let deposit = (chain_id == UNICHAIN
                && r.ty == logset_blockcheck::receipts::DEPOSIT_TX_TYPE)
                .then_some((r.deposit_nonce, r.deposit_receipt_version));
            receipts.push(Receipt {
                cumulative_gas: r.cumulative_gas_used,
                success: r.root.is_none().then_some(r.status),
                logs,
                deposit,
            });
        }
        Ok(Block {
            chain_id,
            number,
            hash,
            parent_root: parent.state_root,
            header,
            header_rlp: parts.header,
            parent_rlp: parts.parent,
            hashes,
            ancestors: parts.ancestors,
            txs,
            receipts,
        })
    }

    /// Each sender through transaction `last` (the transactions a run reads), recovered from its
    /// signature (or the `from` a deposit carries) where not known.
    pub fn recover_through(&mut self, last: usize) -> Result<(), Failure> {
        for (i, t) in self.txs.iter_mut().enumerate().take(last.saturating_add(1)) {
            if t.sender.is_none() {
                let from = txs::sender(&t.decoded).map_err(|e| {
                    Failure::new(Kind::Transactions, format!("transaction {i}: {e:#}"))
                })?;
                t.sender = Some(Address::from(from));
            }
        }
        Ok(())
    }

    /// Block `n`'s hash, where the headers prove it.
    pub fn hash_of(&self, n: u64) -> Option<B256> {
        self.hashes.get(&n).copied()
    }
}
