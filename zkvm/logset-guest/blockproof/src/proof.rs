













use crate::block::{check_uncles, check_withdrawals, served_header};
use crate::fault::{check, reject, Fault, Tag};
use crate::mpt;
use crate::render::{Context, Rendered, View};
use alloy::eips::eip4895::Withdrawal;
use anyhow::{bail, ensure, Context as _, Result};
use logset_blockcheck::receipts::{rlp_item, rlp_list_items};



use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Block {
    Number(u64),
    Hash([u8; 32]),

    Tag,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Hash([u8; 32]),
    At(Block, u64),
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Subject {

    Transaction(Item),

    Receipt(Item),


    BlockReceipts(Block),


    Block(Block),
}


#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockProof {
    pub chain_id: String,
    pub block_number: String,
    pub block_hash: String,

    pub header: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transaction_index: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub transactions: Option<Vec<String>>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub receipts: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub transaction_proofs: Vec<ItemProof>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub receipt_proofs: Vec<ItemProof>,


    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub withdrawals: Option<String>,


    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uncles: Option<Vec<String>>,


    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unproven: Vec<String>,
}


#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemProof {
    pub index: String,
    pub value: String,
    pub proof: Vec<String>,
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







































































































































































pub fn decode_header(rlp: &[u8]) -> Result<alloy::consensus::Header> {


    let h = logset_blockcheck::header::alloy_header(rlp).context("header RLP")?;
    let trailing = logset_blockcheck::header::trailing_items(rlp).context("header RLP")?;
    ensure!(
        logset_blockcheck::header::encode_with_trailing(&h, &trailing)? == rlp,
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



#[derive(Clone, Debug)]
pub struct Verified {
    pub chain_id: u64,
    pub subject: Subject,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub header: alloy::consensus::Header,

    pub index: Option<usize>,
    pub transactions: BTreeMap<usize, Vec<u8>>,
    pub receipts: BTreeMap<usize, Vec<u8>>,

    pub complete: bool,

    pub header_rlp: Vec<u8>,

    pub all_transactions: bool,

    pub withdrawals: Option<Vec<Withdrawal>>,

    pub uncles: Option<crate::block::Uncles>,

    pub unproven: Vec<String>,
}






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

    pub fn context(&self) -> Context {
        Context::from_rlp_header(self.chain_id, &self.header, self.block_hash)
    }


    pub fn view(&self) -> Result<View<'_>> {
        View::partial(self.context(), &self.transactions, &self.receipts)
    }




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


    pub fn transaction(&self) -> Result<Rendered> {
        let i = self.index.context("no transaction index")?;
        self.view()?.tx(i).context("no transaction")
    }



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
