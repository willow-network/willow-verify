//! Proofs of a block, its transactions and its receipts against its block hash.
//!
//! A proof carries the block's header (its RLP, whose keccak is the block
//! hash) and items of its two ordered tries: either every value of a trie
//! (`transactions`, `receipts`; the client rebuilds the root) or single items
//! with the nodes from the root to each (`transactionProofs`,
//! `receiptProofs`). A block proof (`willow_getBlockProof`) also carries the
//! block's withdrawals and uncles, which rebuild their header fields. What it
//! proves is decided by the client's own request ([`Subject`]), never by the
//! response: [`verify`] binds the block (number or hash) and the transaction
//! (hash or position) to the proven bytes, and the rendered JSON
//! ([`Verified::answer`]) is built only from those bytes and the header. The one thing left to the client is that the block hash is
//! canonical: a node or light client of its own confirms it.

use crate::block::{check_uncles, check_withdrawals, served_header};
use crate::fault::{check, reject, Fault, Tag};
use crate::mpt;
use crate::render::{block_millis, Context, Rendered, View};
use alloy::eips::eip4895::Withdrawal;
use alloy_rlp::Decodable;
use anyhow::{bail, ensure, Context as _, Result};
use logset_blockcheck::receipts::{deposits_allowed, rlp_item, rlp_list_items, DEPOSIT_TX_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A block, as a client names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Number(u64),
    Hash([u8; 32]),
    /// A tag (`latest`, ...) the server resolved: any block, reported back.
    Tag,
}

/// A transaction, as a client names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Hash([u8; 32]),
    At(Block, u64),
}

/// What a client asked to be proven.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {
    /// `willow_getTransactionProof`: the `eth_getTransactionByHash` object.
    Transaction(Item),
    /// `willow_getReceiptProof`: the `eth_getTransactionReceipt` object.
    Receipt(Item),
    /// `willow_getBlockReceiptsProof`: the `eth_getBlockReceipts` list (and
    /// every transaction object of the block).
    BlockReceipts(Block),
    /// `willow_getBlockProof`: the `eth_getBlockByNumber` / `ByHash` object,
    /// with transaction hashes or in full ([`Verified::block`]).
    Block(Block),
}

/// The wire form (JSON-RPC result): hex quantities and `0x` data.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,
    /// The header's RLP: its keccak256 is the block hash.
    pub header: String,
    /// The transaction a transaction or receipt proof is about.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<String>,
    /// Every transaction's trie value, in block order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<String>>,
    /// Every receipt's trie value, in block order.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transaction_proofs: Vec<ItemProof>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_proofs: Vec<ItemProof>,
    /// Block proofs: the block's withdrawals as one RLP list; absent when the
    /// header has no withdrawalsRoot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawals: Option<String>,
    /// Block proofs: the uncle headers' RLPs, in order; absent when the block
    /// has none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncles: Option<Vec<String>>,
    /// Block proofs: the fields the proof does not cover, left out of the
    /// rendering (only [`crate::block::UNPROVEN_UNCLES`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unproven: Vec<String>,
}

/// One trie item and the nodes from the root to its leaf, root first.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemProof {
    pub index: String,
    pub value: String,
    pub proof: Vec<String>,
}

fn qty(n: u64) -> String {
    format!("0x{n:x}")
}

fn hexs(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

pub(crate) fn parse_qty(s: &str, what: &str) -> Result<u64> {
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| reject(Fault::Malformed, format!("{what}: not a 0x quantity")))?;
    check!(
        !digits.is_empty() && digits.len() <= 16 && digits.bytes().all(|b| b.is_ascii_hexdigit()),
        Fault::Malformed,
        "{what}: not a quantity"
    );
    u64::from_str_radix(digits, 16).map_err(|e| reject(Fault::Malformed, format!("{what}: {e}")))
}

pub(crate) fn parse_bytes(s: &str, what: &str) -> Result<Vec<u8>> {
    let digits = s
        .strip_prefix("0x")
        .ok_or_else(|| reject(Fault::Malformed, format!("{what}: not 0x data")))?;
    hex::decode(digits).map_err(|_| reject(Fault::Malformed, format!("{what}: not hex")))
}

/// Which items a proof of transaction or receipt `index` carries.
fn items_for(
    chain_id: u64,
    txs: &[&[u8]],
    index: usize,
    receipt: bool,
) -> (Vec<usize>, Vec<usize>) {
    let deposit = deposits_allowed(chain_id);
    // Base from Denim: the BaseTime deposit (every object's `blockTimestampMs`).
    let millis = index != 1 && block_millis(chain_id, txs).is_some();
    if !receipt {
        // A deposit's nonce is its receipt's deposit nonce.
        let is_deposit = deposit && txs[index].first() == Some(&DEPOSIT_TX_TYPE);
        let mut t = vec![index];
        if millis {
            t.push(1);
            t.sort_unstable();
        }
        return (t, if is_deposit { vec![index] } else { vec![] });
    }
    // The transaction (hash, sender, recipient, prices), the previous receipt
    // (gasUsed), and on the OP stack the L1 info deposit (the L1 fee fields).
    let mut t = vec![index];
    if deposit && index > 0 {
        t.insert(0, 0);
    }
    if millis {
        t.push(1);
        t.sort_unstable();
    }
    let r = match index {
        0 => vec![0],
        i => vec![i - 1, i],
    };
    (t, r)
}

/// The proof of transaction (`receipt = false`) or receipt `index` of the
/// block with header RLP `header`, transactions `txs` and receipts `receipts`.
pub fn build_item(
    chain_id: u64,
    header: &[u8],
    txs: &[&[u8]],
    receipts: &[&[u8]],
    index: usize,
    receipt: bool,
) -> Result<BlockProof> {
    ensure!(
        txs.len() == receipts.len(),
        "{} transactions, {} receipts",
        txs.len(),
        receipts.len()
    );
    ensure!(index < txs.len(), "transaction {index} of {}", txs.len());
    let mut p = skeleton(chain_id, header)?;
    let (ti, ri) = items_for(chain_id, txs, index, receipt);
    p.transaction_index = Some(qty(index as u64));
    p.transaction_proofs = item_proofs(txs, &ti)?;
    p.receipt_proofs = item_proofs(receipts, &ri)?;
    Ok(p)
}

/// The proof of every transaction and receipt of a block.
pub fn build_block(
    chain_id: u64,
    header: &[u8],
    txs: &[&[u8]],
    receipts: &[&[u8]],
) -> Result<BlockProof> {
    ensure!(
        txs.len() == receipts.len(),
        "{} transactions, {} receipts",
        txs.len(),
        receipts.len()
    );
    let mut p = skeleton(chain_id, header)?;
    p.transactions = Some(txs.iter().map(|v| hexs(v)).collect());
    p.receipts = Some(receipts.iter().map(|v| hexs(v)).collect());
    Ok(p)
}

/// The proof of a block (`willow_getBlockProof`): its header, every
/// transaction, its withdrawals (`withdrawals`, their RLP list, when the
/// header has a withdrawalsRoot) and its uncle headers (`uncles`, in order;
/// `None` for a block whose uncle headers the server lacks, which leaves its
/// uncles and size unproven). On the OP stack it also proves each deposit's
/// receipt (`receipts`, all of the block's), whose bytes hold the nonce its
/// transaction object reports.
pub fn build_block_proof(
    chain_id: u64,
    header: &[u8],
    txs: &[&[u8]],
    receipts: Option<&[&[u8]]>,
    withdrawals: Option<&[u8]>,
    uncles: Option<&[&[u8]]>,
) -> Result<BlockProof> {
    let mut p = skeleton(chain_id, header)?;
    let h = decode_header(header)?;
    p.transactions = Some(txs.iter().map(|v| hexs(v)).collect());
    let deposits: Vec<usize> = (0..txs.len())
        .filter(|&i| deposits_allowed(chain_id) && txs[i].first() == Some(&DEPOSIT_TX_TYPE))
        .collect();
    if !deposits.is_empty() {
        let receipts = receipts.context("an OP-stack block proof needs the block's receipts")?;
        ensure!(
            receipts.len() == txs.len(),
            "{} transactions, {} receipts",
            txs.len(),
            receipts.len()
        );
        p.receipt_proofs = item_proofs(receipts, &deposits)?;
    }
    p.withdrawals = withdrawals.map(hexs);
    let empty = h.ommers_hash.0 == crate::block::empty_ommers();
    match uncles {
        Some(u) if !u.is_empty() => p.uncles = Some(u.iter().map(|v| hexs(v)).collect()),
        None if !empty => {
            p.unproven = crate::block::UNPROVEN_UNCLES
                .iter()
                .map(|s| s.to_string())
                .collect()
        }
        _ => {}
    }
    Ok(p)
}

fn skeleton(chain_id: u64, header: &[u8]) -> Result<BlockProof> {
    let h = decode_header(header)?;
    Ok(BlockProof {
        chain_id: qty(chain_id),
        block_number: qty(h.number),
        block_hash: hexs(&alloy_primitives::keccak256(header).0),
        header: hexs(header),
        ..Default::default()
    })
}

fn item_proofs(values: &[&[u8]], indices: &[usize]) -> Result<Vec<ItemProof>> {
    let (_, proofs) = mpt::prove(values, indices)?;
    Ok(indices
        .iter()
        .zip(proofs)
        .map(|(&i, nodes)| ItemProof {
            index: qty(i as u64),
            value: hexs(values[i]),
            proof: nodes.iter().map(|n| hexs(n)).collect(),
        })
        .collect())
}

/// The header from its RLP: alloy's decoder, which must re-encode to the very
/// bytes given (so no field hides from the hash), and a positional read of
/// the number and both roots, which must agree with it.
pub fn decode_header(rlp: &[u8]) -> Result<alloy::consensus::Header> {
    let mut buf = rlp;
    let h = alloy::consensus::Header::decode(&mut buf).context("header RLP")?;
    ensure!(
        buf.is_empty(),
        "header RLP has {} trailing bytes",
        buf.len()
    );
    ensure!(
        alloy_rlp::encode(&h) == rlp,
        "header RLP is not in canonical form"
    );
    let (payload, _, is_list) = rlp_item(rlp)?;
    ensure!(is_list, "header is not an RLP list");
    let f = rlp_list_items(payload)?;
    ensure!(f.len() > 8, "header has {} fields", f.len());
    let number = f[8]
        .0
        .iter()
        .try_fold(0u64, |a, b| a.checked_mul(256).map(|a| a | u64::from(*b)))
        .context("header number")?;
    ensure!(
        f[4].0 == h.transactions_root.as_slice()
            && f[5].0 == h.receipts_root.as_slice()
            && number == h.number,
        "the header's two readings disagree"
    );
    Ok(h)
}

/// A verified proof: the header, and the proven transaction and receipt bytes
/// by position.
#[derive(Clone, Debug)]
pub struct Verified {
    pub chain_id: u64,
    pub subject: Subject,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub header: alloy::consensus::Header,
    /// The proof's transaction (transaction and receipt proofs).
    pub index: Option<usize>,
    pub transactions: BTreeMap<usize, Vec<u8>>,
    pub receipts: BTreeMap<usize, Vec<u8>>,
    /// Every transaction and receipt of the block is here.
    pub complete: bool,
    /// The header's RLP.
    pub header_rlp: Vec<u8>,
    /// Block proofs: every transaction is here.
    pub all_transactions: bool,
    /// Block proofs: the withdrawals, when the header has a withdrawalsRoot.
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Block proofs: the uncles' RLPs and hashes; `None` when left unproven.
    pub uncles: Option<crate::block::Uncles>,
    /// Block proofs: the fields the proof leaves out.
    pub unproven: Vec<String>,
}

/// Check `p` as the answer to `subject` on chain `chain_id`: the header hashes
/// to the block hash, every item is under the header's roots (lists rebuild
/// them, item proofs walk to them), the block and transaction are the ones
/// asked for, and every item the subject needs is there. Each failure carries
/// its [`Fault`] ([`crate::fault::fault`]).
pub fn verify(p: &BlockProof, chain_id: u64, subject: &Subject) -> Result<Verified> {
    let got_chain = parse_qty(&p.chain_id, "chainId")?;
    check!(
        got_chain == chain_id,
        Fault::Chain,
        "the proof is for chain {got_chain}, not {chain_id}"
    );
    let header = parse_bytes(&p.header, "header")?;
    let h = decode_header(&header).tag(Fault::Header)?;
    let hash = alloy_primitives::keccak256(&header).0;
    check!(
        parse_bytes(&p.block_hash, "blockHash")? == hash,
        Fault::Header,
        "blockHash is not the keccak of the header"
    );
    check!(
        parse_qty(&p.block_number, "blockNumber")? == h.number,
        Fault::Header,
        "blockNumber is not the header's"
    );
    let block = match subject {
        Subject::Transaction(Item::At(b, _))
        | Subject::Receipt(Item::At(b, _))
        | Subject::BlockReceipts(b)
        | Subject::Block(b) => *b,
        _ => Block::Tag,
    };
    match block {
        Block::Number(n) => check!(
            h.number == n,
            Fault::Block,
            "the proof is for block {}, not {n}",
            h.number
        ),
        Block::Hash(want) => check!(
            hash == want,
            Fault::Block,
            "the proof is for block 0x{}, not 0x{}",
            hex::encode(hash),
            hex::encode(want)
        ),
        Block::Tag => {}
    }
    let is_block = matches!(subject, Subject::Block(_));
    if !is_block {
        check!(
            p.withdrawals.is_none() && p.uncles.is_none() && p.unproven.is_empty(),
            Fault::Malformed,
            "withdrawals, uncles or unproven fields in a proof that is not a block proof"
        );
    }
    let (transactions, all_txs) = items(
        p.transactions.as_deref(),
        &p.transaction_proofs,
        &h.transactions_root.0,
        "transaction",
    )
    .tag(Fault::Transactions)?;
    let (receipts, all_receipts) = items(
        p.receipts.as_deref(),
        &p.receipt_proofs,
        &h.receipts_root.0,
        "receipt",
    )
    .tag(Fault::Receipts)?;
    let complete = all_txs && all_receipts;
    if complete {
        check!(
            transactions.len() == receipts.len(),
            Fault::Receipts,
            "{} transactions but {} receipts",
            transactions.len(),
            receipts.len()
        );
    }
    let (mut withdrawals, mut uncles) = (None, None);
    let index = match subject {
        Subject::BlockReceipts(_) => {
            check!(
                complete,
                Fault::Missing,
                "a block's receipts are proven only by every transaction and receipt"
            );
            None
        }
        Subject::Block(_) => {
            check!(
                p.transaction_index.is_none(),
                Fault::Malformed,
                "a transactionIndex in a block proof"
            );
            check!(
                all_txs,
                Fault::Missing,
                "a block is proven only by every transaction"
            );
            let raw = p
                .withdrawals
                .as_deref()
                .map(|w| parse_bytes(w, "withdrawals"))
                .transpose()?;
            withdrawals = check_withdrawals(chain_id, &h, raw.as_deref())?;
            let raw = p
                .uncles
                .as_ref()
                .map(|u| {
                    u.iter()
                        .enumerate()
                        .map(|(i, v)| parse_bytes(v, &format!("uncle {i}")))
                        .collect::<Result<Vec<_>>>()
                })
                .transpose()?;
            uncles = check_uncles(&h, raw.as_deref(), &p.unproven)?;
            None
        }
        Subject::Transaction(item) | Subject::Receipt(item) => {
            let i = parse_qty(
                p.transaction_index
                    .as_deref()
                    .ok_or_else(|| reject(Fault::Missing, "no transactionIndex"))?,
                "transactionIndex",
            )?;
            let i = usize::try_from(i)
                .ok()
                .filter(|i| *i < mpt::MAX_ITEMS)
                .ok_or_else(|| reject(Fault::Malformed, "transactionIndex out of range"))?;
            let tx = transactions
                .get(&i)
                .ok_or_else(|| reject(Fault::Missing, format!("transaction {i} is not proven")))?;
            match item {
                Item::Hash(want) => check!(
                    alloy_primitives::keccak256(tx).0 == *want,
                    Fault::Item,
                    "the proven transaction {i} is 0x{}, not 0x{}",
                    hex::encode(alloy_primitives::keccak256(tx)),
                    hex::encode(want)
                ),
                Item::At(_, want) => {
                    check!(
                        i as u64 == *want,
                        Fault::Item,
                        "the proof is for transaction {i}, not {want}"
                    )
                }
            }
            if matches!(subject, Subject::Receipt(_)) {
                check!(
                    receipts.contains_key(&i),
                    Fault::Missing,
                    "receipt {i} is not proven"
                );
            }
            Some(i)
        }
    };
    Ok(Verified {
        chain_id,
        subject: *subject,
        block_number: h.number,
        block_hash: hash,
        header: h,
        index,
        transactions,
        receipts,
        complete,
        header_rlp: header,
        all_transactions: all_txs,
        withdrawals,
        uncles,
        unproven: if is_block { p.unproven.clone() } else { vec![] },
    })
}

/// A trie's proven values by position, and whether they are all of them.
fn items(
    all: Option<&[String]>,
    proofs: &[ItemProof],
    root: &[u8; 32],
    what: &str,
) -> Result<(BTreeMap<usize, Vec<u8>>, bool)> {
    let mut out = BTreeMap::new();
    if let Some(all) = all {
        check!(
            proofs.is_empty(),
            Fault::Malformed,
            "both every {what} and {what} proofs: one or the other"
        );
        let values = all
            .iter()
            .enumerate()
            .map(|(i, v)| parse_bytes(v, &format!("{what} {i}")))
            .collect::<Result<Vec<_>>>()?;
        check!(
            values.len() <= mpt::MAX_ITEMS,
            Fault::Malformed,
            "{} {what}s, more than any block holds",
            values.len()
        );
        let rebuilt = logset_blockcheck::trie::ordered_root(&values)?;
        ensure!(
            &rebuilt == root,
            "the {what}s rebuild root 0x{}, not the header's 0x{}",
            hex::encode(rebuilt),
            hex::encode(root)
        );
        return Ok((values.into_iter().enumerate().collect(), true));
    }
    for p in proofs {
        let i = parse_qty(&p.index, &format!("{what} proof index"))?;
        let i = usize::try_from(i)
            .ok()
            .filter(|i| *i < mpt::MAX_ITEMS)
            .ok_or_else(|| reject(Fault::Malformed, format!("{what} index {i} out of range")))?;
        let value = parse_bytes(&p.value, &format!("{what} {i}"))?;
        let nodes = p
            .proof
            .iter()
            .map(|n| parse_bytes(n, &format!("{what} {i} proof")))
            .collect::<Result<Vec<_>>>()?;
        let proven = mpt::verify(root, i, &nodes).with_context(|| format!("{what} {i}"))?;
        ensure!(
            proven == value,
            "{what} {i}: the value is not the proven one"
        );
        check!(
            out.insert(i, value).is_none(),
            Fault::Malformed,
            "{what} {i} twice"
        );
    }
    Ok((out, false))
}

impl Verified {
    /// The rendering context from the verified header.
    pub fn context(&self) -> Context {
        Context::from_rlp_header(self.chain_id, &self.header, self.block_hash)
    }

    /// The proven transactions and receipts, ready to render.
    pub fn view(&self) -> Result<View<'_>> {
        View::partial(self.context(), &self.transactions, &self.receipts)
    }

    /// The JSON the subject's standard method answers, rendered from the proven
    /// bytes, with the fields the proof does not determine. A block proof's is
    /// the block with transaction hashes ([`Verified::block`]).
    pub fn answer(&self) -> Result<Rendered> {
        if let Subject::Block(_) = self.subject {
            return self.block(false);
        }
        let v = self.view()?;
        match (self.subject, self.index) {
            (Subject::Transaction(_), Some(i)) => v.tx(i).context("no transaction"),
            (Subject::Receipt(_), Some(i)) => v.receipt(i).context("no receipt"),
            (Subject::BlockReceipts(_), _) => Ok(each(&v, |v, i| v.receipt(i))),
            _ => bail!("no transaction index"),
        }
    }

    /// `eth_getBlockByNumber(n, full)` (and `eth_getBlockByHash`) of a block
    /// proof, from the proven parts alone: the served header (the log store's
    /// own rendering) without the fields the proof leaves unproven, and with
    /// `full` every transaction object, each sender recovered from its bytes.
    pub fn block(&self, full: bool) -> Result<Rendered> {
        ensure!(
            matches!(self.subject, Subject::Block(_)) && self.all_transactions,
            "not a verified block proof"
        );
        let tx_values: Vec<Vec<u8>> = self.transactions.values().cloned().collect();
        let (uncle_rlps, uncle_hashes) = self.uncles.clone().unwrap_or_default();
        let size = logset_blockcheck::block::block_size(
            &self.header_rlp,
            &tx_values,
            &uncle_rlps,
            self.withdrawals.as_deref(),
        );
        let body = logset_blockcheck::VerifiedBody {
            tx_hashes: tx_values
                .iter()
                .map(|v| alloy_primitives::keccak256(v).0)
                .collect(),
            tx_values,
            withdrawals: self.withdrawals.clone(),
            uncles: uncle_hashes,
            size: size as u64,
        };
        let header = logset_blockcheck::header_from_rlp(&self.header_rlp)?;
        let mut json = served_header(self.chain_id, &header, &body)?;
        let o = json.as_object_mut().expect("an object");
        for f in &self.unproven {
            o.remove(f);
        }
        let mut unproven = self.unproven.clone();
        if full {
            let txs = each(&self.view()?, |v, i| v.tx(i));
            o.insert("transactions".into(), txs.json);
            unproven.extend(txs.unproven.into_iter().map(|f| format!("transactions{f}")));
        }
        unproven.sort();
        unproven.dedup();
        Ok(Rendered { json, unproven })
    }

    /// The proof's own transaction object, for transaction and receipt proofs.
    pub fn transaction(&self) -> Result<Rendered> {
        let i = self.index.context("no transaction index")?;
        self.view()?.tx(i).context("no transaction")
    }

    /// Every transaction object of the block (`eth_getBlockByNumber(n, true)`'s
    /// `transactions`), for a proof of every item.
    pub fn transactions_json(&self) -> Result<Rendered> {
        ensure!(self.complete, "not every transaction is proven");
        Ok(each(&self.view()?, |v, i| v.tx(i)))
    }
}

fn each(v: &View, one: impl Fn(&View, usize) -> Option<Rendered>) -> Rendered {
    let mut list = Vec::with_capacity(v.len());
    let mut unproven = Vec::new();
    for i in 0..v.len() {
        let r = one(v, i).unwrap_or(Rendered {
            json: Value::Null,
            unproven: vec![],
        });
        list.push(r.json);
        unproven.extend(r.unproven.into_iter().map(|f| format!("[].{f}")));
    }
    unproven.sort();
    unproven.dedup();
    Rendered {
        json: Value::Array(list),
        unproven,
    }
}
