//! `--tx`, `--receipt`, `--block-receipts`: a transaction, a receipt or a
//! block's receipts proven against the block hash (Merkle-Patricia proofs, no
//! zk), with `logset_blockproof`.
//!
//! 1. Asks the Willow RPC for `willow_getTransactionProof`,
//!    `willow_getReceiptProof` or `willow_getBlockReceiptsProof`.
//! 2. Verifies it as the answer to the request given here, never to the
//!    server's: the header hashes to the block hash, every item is under the
//!    header's roots, and the block and transaction are the ones asked for.
//!    Renders the standard JSON from the proven bytes.
//! 3. `--compare-rpc`: that endpoint's `eth_getTransactionByHash`,
//!    `eth_getTransactionReceipt`, or `eth_getBlockReceipts` and the block's
//!    transactions (`eth_getBlockByHash(h, true)`), field by field.
//! 4. `--anchor-rpc`: the block hash is canonical there.

use crate::{call, pinned_chain};
use logset_blockproof::compare::{compare, Comparison};
use logset_blockproof::proof::{verify, Block, BlockProof, Item, Subject};
use logset_blockproof::render::Rendered;
use serde_json::{json, Value};

/// What to prove, from the command line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ask {
    Tx(Item),
    Receipt(Item),
    BlockReceipts(Block),
}

/// A block: a number (decimal or 0x hex), a 32-byte hash, or `latest`.
pub fn parse_block(s: &str) -> Result<Block, String> {
    let s = s.trim();
    if s == "latest" {
        return Ok(Block::Tag);
    }
    if let Some(h) = s.strip_prefix("0x").filter(|h| h.len() == 64) {
        let mut out = [0u8; 32];
        hex::decode_to_slice(h, &mut out).map_err(|e| format!("block hash {s}: {e}"))?;
        return Ok(Block::Hash(out));
    }
    let n = match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16),
        None => s.parse(),
    };
    n.map(Block::Number)
        .map_err(|_| format!("{s:?} is not a block number, hash or \"latest\""))
}

/// A transaction: its hash, or `<block>:<index>`.
pub fn parse_item(s: &str) -> Result<Item, String> {
    if let Some((block, index)) = s.rsplit_once(':') {
        let index = match index.strip_prefix("0x") {
            Some(h) => u64::from_str_radix(h, 16),
            None => index.parse(),
        }
        .map_err(|_| format!("{index:?} is not a transaction index"))?;
        return Ok(Item::At(parse_block(block)?, index));
    }
    match parse_block(s)? {
        Block::Hash(h) => Ok(Item::Hash(h)),
        _ => Err(format!(
            "{s:?}: give a transaction hash, or <block>:<index>"
        )),
    }
}

fn block_param(b: Block) -> Value {
    match b {
        Block::Number(n) => json!(format!("0x{n:x}")),
        Block::Hash(h) => json!(format!("0x{}", hex::encode(h))),
        Block::Tag => json!("latest"),
    }
}

fn item_params(i: Item) -> Value {
    match i {
        Item::Hash(h) => json!([format!("0x{}", hex::encode(h))]),
        Item::At(b, index) => json!([block_param(b), format!("0x{index:x}")]),
    }
}

fn comparison_json(c: &Comparison) -> Value {
    json!({
        "matched": c.matched,
        "differ": c.differ,
        "unproven": c.unproven,
        "omitted": c.omitted,
    })
}

/// Fetch, verify, compare; the report, and whether everything checked out.
pub fn run(
    http: &reqwest::blocking::Client,
    rpc: &str,
    chain_id: u64,
    ask: Ask,
    compare_rpc: Option<&str>,
    anchor_rpc: Option<&str>,
    print: bool,
) -> Result<(Value, bool), String> {
    let (method, params, subject) = match ask {
        Ask::Tx(i) => (
            "willow_getTransactionProof",
            item_params(i),
            Subject::Transaction(i),
        ),
        Ask::Receipt(i) => (
            "willow_getReceiptProof",
            item_params(i),
            Subject::Receipt(i),
        ),
        Ask::BlockReceipts(b) => (
            "willow_getBlockReceiptsProof",
            json!([block_param(b)]),
            Subject::BlockReceipts(b),
        ),
    };
    let raw = call(http, rpc, method, params)?;
    let size = raw.to_string().len();
    let proof: BlockProof = serde_json::from_value(raw).map_err(|e| format!("{method}: {e}"))?;
    let t = std::time::Instant::now();
    let v = verify(&proof, chain_id, &subject).map_err(|e| format!("not verified: {e:#}"))?;
    let proven = v.answer().map_err(|e| format!("not verified: {e:#}"))?;
    let txs = match subject {
        Subject::BlockReceipts(_) => Some(
            v.transactions_json()
                .map_err(|e| format!("not verified: {e:#}"))?,
        ),
        _ => None,
    };
    let hash = format!("0x{}", hex::encode(v.block_hash));
    let tx_hash = v
        .index
        .and_then(|i| v.transactions.get(&i))
        .map(|tx| format!("0x{}", hex::encode(logset_blockproof::keccak(tx))));
    let mut report = json!({
        "verified": true,
        "method": method,
        "chainId": chain_id,
        "chain": pinned_chain(chain_id),
        "block": {"number": format!("0x{:x}", v.block_number), "hash": hash},
        "proofBytes": size,
        "verifySeconds": t.elapsed().as_secs_f64(),
        "unproven": proven.unproven,
    });
    if let Some(i) = v.index {
        report["transactionIndex"] = json!(format!("0x{i:x}"));
        report["transactionHash"] = json!(tx_hash);
    }
    if let Subject::BlockReceipts(_) = subject {
        report["transactions"] = json!(v.transactions.len());
    }
    let mut ok = true;
    report["block"]["canonical"] = json!("not checked: pass --anchor-rpc <a node you trust>");
    if let Some(url) = anchor_rpc {
        let header = call(
            http,
            url,
            "eth_getBlockByNumber",
            json!([format!("0x{:x}", v.block_number), false]),
        )?;
        let canonical = header.get("hash").and_then(Value::as_str) == Some(hash.as_str());
        ok &= canonical;
        report["block"]["canonical"] = json!(canonical);
    }
    if let Some(url) = compare_rpc {
        let mut answers: Vec<(&str, Value, &Rendered)> = Vec::new();
        match subject {
            Subject::Transaction(_) => answers.push((
                "eth_getTransactionByHash",
                call(http, url, "eth_getTransactionByHash", json!([tx_hash]))?,
                &proven,
            )),
            Subject::Receipt(_) => answers.push((
                "eth_getTransactionReceipt",
                call(http, url, "eth_getTransactionReceipt", json!([tx_hash]))?,
                &proven,
            )),
            Subject::BlockReceipts(_) => {
                answers.push((
                    "eth_getBlockReceipts",
                    call(http, url, "eth_getBlockReceipts", json!([hash]))?,
                    &proven,
                ));
                let block = call(http, url, "eth_getBlockByHash", json!([hash, true]))?;
                answers.push((
                    "eth_getBlockByHash.transactions",
                    block.get("transactions").cloned().unwrap_or(Value::Null),
                    txs.as_ref().expect("a block proof's transactions"),
                ));
            }
        }
        let mut out = serde_json::Map::new();
        for (what, answer, proven) in answers {
            let c = compare(proven, &answer);
            ok &= c.is_exact();
            out.insert(what.into(), comparison_json(&c));
        }
        report["compare"] = Value::Object(out);
    }
    if print {
        report["proven"] = proven.json.clone();
        if let Some(t) = &txs {
            report["provenTransactions"] = t.json.clone();
        }
    }
    report["verified"] = json!(ok);
    Ok((report, ok))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_and_transactions_parse_as_given() {
        assert_eq!(parse_block("26119978"), Ok(Block::Number(26_119_978)));
        assert_eq!(parse_block("0x18e8f2a"), Ok(Block::Number(0x18e8f2a)));
        assert_eq!(parse_block("latest"), Ok(Block::Tag));
        let h = format!("0x{}", "ab".repeat(32));
        assert_eq!(parse_block(&h), Ok(Block::Hash([0xab; 32])));
        assert!(parse_block("finalized").is_err());
        assert_eq!(parse_item(&h), Ok(Item::Hash([0xab; 32])));
        assert_eq!(
            parse_item("0x18e8f2a:5"),
            Ok(Item::At(Block::Number(0x18e8f2a), 5))
        );
        assert_eq!(
            parse_item(&format!("{h}:0x10")),
            Ok(Item::At(Block::Hash([0xab; 32]), 16))
        );
        assert!(parse_item("26119978").is_err());
        assert!(parse_item("1:x").is_err());
    }
}
