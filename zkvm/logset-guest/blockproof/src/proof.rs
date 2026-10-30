//! Proofs of a block's transactions and receipts against its block hash.
//!
//! A proof carries the block's header (its RLP, whose keccak is the block
//! hash) and items of its two ordered tries: either every value of a trie
//! (`transactions`, `receipts`; the client rebuilds the root) or single items
//! with the nodes from the root to each (`transactionProofs`,
//! `receiptProofs`). What it proves is decided by the client's own request
//! ([`Subject`]), never by the answer: [`verify`] binds the block (number or
//! hash) and the transaction (hash or position) to the proven bytes, and the
//! rendered JSON ([`Verified::answer`]) is built only from those bytes and the
//! header. The one thing left to the client is that the block hash is
//! canonical: a node or light client of its own confirms it.

use crate::mpt;
use crate::render::{block_millis, Context, Rendered, View};
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

fn parse_qty(s: &str, what: &str) -> Result<u64> {
    let digits = s
        .strip_prefix("0x")
        .with_context(|| format!("{what}: not a 0x quantity"))?;
    ensure!(
        !digits.is_empty() && digits.len() <= 16 && digits.bytes().all(|b| b.is_ascii_hexdigit()),
        "{what}: not a quantity"
    );
    Ok(u64::from_str_radix(digits, 16)?)
}

fn parse_bytes(s: &str, what: &str) -> Result<Vec<u8>> {
    let digits = s
        .strip_prefix("0x")
        .with_context(|| format!("{what}: not 0x data"))?;
    hex::decode(digits).with_context(|| format!("{what}: not hex"))
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
}

/// Check `p` as the answer to `subject` on chain `chain_id`: the header hashes
/// to the block hash, every item is under the header's roots (lists rebuild
/// them, item proofs walk to them), the block and transaction are the ones
/// asked for, and every item the subject needs is there.
pub fn verify(p: &BlockProof, chain_id: u64, subject: &Subject) -> Result<Verified> {
    let got_chain = parse_qty(&p.chain_id, "chainId")?;
    ensure!(
        got_chain == chain_id,
        "the proof is for chain {got_chain}, not {chain_id}"
    );
    let header = parse_bytes(&p.header, "header")?;
    let h = decode_header(&header)?;
    let hash = alloy_primitives::keccak256(&header).0;
    ensure!(
        parse_bytes(&p.block_hash, "blockHash")? == hash,
        "blockHash is not the keccak of the header"
    );
    ensure!(
        parse_qty(&p.block_number, "blockNumber")? == h.number,
        "blockNumber is not the header's"
    );
    let block = match subject {
        Subject::Transaction(Item::At(b, _))
        | Subject::Receipt(Item::At(b, _))
        | Subject::BlockReceipts(b) => *b,
        _ => Block::Tag,
    };
    match block {
        Block::Number(n) => ensure!(
            h.number == n,
            "the proof is for block {}, not {n}",
            h.number
        ),
        Block::Hash(want) => ensure!(
            hash == want,
            "the proof is for block 0x{}, not 0x{}",
            hex::encode(hash),
            hex::encode(want)
        ),
        Block::Tag => {}
    }
    let (transactions, all_txs) = items(
        p.transactions.as_deref(),
        &p.transaction_proofs,
        &h.transactions_root.0,
        "transaction",
    )?;
    let (receipts, all_receipts) = items(
        p.receipts.as_deref(),
        &p.receipt_proofs,
        &h.receipts_root.0,
        "receipt",
    )?;
    let complete = all_txs && all_receipts;
    if complete {
        ensure!(
            transactions.len() == receipts.len(),
            "{} transactions but {} receipts",
            transactions.len(),
            receipts.len()
        );
    }
    let index = match subject {
        Subject::BlockReceipts(_) => {
            ensure!(
                complete,
                "a block's receipts are proven only by every transaction and receipt"
            );
            None
        }
        Subject::Transaction(item) | Subject::Receipt(item) => {
            let i = parse_qty(
                p.transaction_index
                    .as_deref()
                    .context("no transactionIndex")?,
                "transactionIndex",
            )?;
            let i = usize::try_from(i)
                .ok()
                .filter(|i| *i < mpt::MAX_ITEMS)
                .context("transactionIndex out of range")?;
            let tx = transactions
                .get(&i)
                .with_context(|| format!("transaction {i} is not proven"))?;
            match item {
                Item::Hash(want) => ensure!(
                    alloy_primitives::keccak256(tx).0 == *want,
                    "the proven transaction {i} is 0x{}, not 0x{}",
                    hex::encode(alloy_primitives::keccak256(tx)),
                    hex::encode(want)
                ),
                Item::At(_, want) => {
                    ensure!(
                        i as u64 == *want,
                        "the proof is for transaction {i}, not {want}"
                    )
                }
            }
            if matches!(subject, Subject::Receipt(_)) {
                ensure!(receipts.contains_key(&i), "receipt {i} is not proven");
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
        ensure!(
            proofs.is_empty(),
            "both every {what} and {what} proofs: one or the other"
        );
        let values = all
            .iter()
            .enumerate()
            .map(|(i, v)| parse_bytes(v, &format!("{what} {i}")))
            .collect::<Result<Vec<_>>>()?;
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
            .with_context(|| format!("{what} index {i} out of range"))?;
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
        ensure!(out.insert(i, value).is_none(), "{what} {i} twice");
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
    /// bytes, with the fields the proof does not determine.
    pub fn answer(&self) -> Result<Rendered> {
        let v = self.view()?;
        match (self.subject, self.index) {
            (Subject::Transaction(_), Some(i)) => v.tx(i).context("no transaction"),
            (Subject::Receipt(_), Some(i)) => v.receipt(i).context("no receipt"),
            (Subject::BlockReceipts(_), _) => Ok(each(&v, |v, i| v.receipt(i))),
            _ => bail!("no transaction index"),
        }
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
