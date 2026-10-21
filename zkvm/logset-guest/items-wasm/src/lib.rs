//! The browser verifier's transaction, receipt and block-receipts proofs
//! (Merkle-Patricia proofs against the block hash, no zk) with
//! [`logset_blockproof`], compiled to wasm32 and run in the page's Web Worker.
//! The page's own request decides what a proof must show ([`Subject`]); the
//! answer is checked against it, never the other way round. Like the log
//! verifier (query-wasm), no method throws a verdict: each returns JSON with
//! `ok` and, when it is false, a `kind` the page decides by.

use logset_blockproof::compare::compare;
use logset_blockproof::proof::{verify, Block, BlockProof, Item, Subject, Verified};
use logset_blockproof::render::Rendered;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

/// Why a check did not pass. The page chooses what to say by `kind` alone, never
/// by the message, which can carry text the server chose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Willow's answer failed a check: the header, a trie proof, the block or transaction asked for.
    Rejected,
    /// An answer could not be read: not JSON, nested too deeply, or not the expected shape.
    MalformedInput,
    /// The transaction or block typed in could not be read (not about Willow's answer).
    InvalidRequest,
    /// The verifier was used out of order, a fault in the page. Never caused by an answer.
    Internal,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Rejected => "rejected",
            Kind::MalformedInput => "malformed-input",
            Kind::InvalidRequest => "invalid-request",
            Kind::Internal => "internal",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    pub kind: Kind,
    pub message: String,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Failure {
            kind,
            message: message.into(),
        }
    }

    /// `{"ok": false, "kind", "message"}`, as the page receives it.
    pub fn json(&self) -> Value {
        json!({"ok": false, "kind": self.kind.as_str(), "message": self.message})
    }
}

/// A JSON-RPC response body as the server sent it; the page turns `error` responses away first.
#[derive(Deserialize)]
struct Response<T> {
    result: T,
}

/// The `result` of a response body, parsed here so the page never re-serialises server data.
/// Anything unreadable, or nested past serde_json's 128 levels, is `malformed-input`.
pub fn result_of<T: DeserializeOwned>(body: &str, method: &str) -> Result<T, Failure> {
    serde_json::from_str::<Response<T>>(body)
        .map(|r| r.result)
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} answer: {e}")))
}

/// Differences listed per comparison, and characters kept of each.
const DIFFER_SHOWN: usize = 50;
const DIFFER_CHARS: usize = 300;

/// A block: a number (decimal or `0x` hex), a 32-byte hash, or `latest`.
pub fn parse_block(s: &str) -> Result<Block, String> {
    let s = s.trim();
    if s == "latest" {
        return Ok(Block::Tag);
    }
    if let Some(h) = s.strip_prefix("0x").filter(|h| h.len() == 64) {
        let mut out = [0u8; 32];
        hex::decode_to_slice(h, &mut out).map_err(|_| format!("{s} is not a block hash"))?;
        return Ok(Block::Hash(out));
    }
    let n = match s.strip_prefix("0x") {
        Some(h) if !h.is_empty() && h.len() <= 16 => u64::from_str_radix(h, 16).ok(),
        Some(_) => None,
        None if !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse().ok()
        }
        None => None,
    };
    n.map(Block::Number)
        .ok_or_else(|| format!("\"{s}\" is not a block number, a block hash or \"latest\""))
}

/// A transaction: its hash, or `<block>:<index>`.
pub fn parse_item(s: &str) -> Result<Item, String> {
    let s = s.trim();
    if let Some((block, index)) = s.rsplit_once(':') {
        let index = index.trim();
        let i = match index.strip_prefix("0x") {
            Some(h) if !h.is_empty() && h.len() <= 8 => u64::from_str_radix(h, 16).ok(),
            Some(_) => None,
            None if !index.is_empty() && index.len() <= 8 => index.parse().ok(),
            None => None,
        }
        .ok_or_else(|| format!("\"{index}\" is not a transaction index"))?;
        return Ok(Item::At(parse_block(block)?, i));
    }
    match parse_block(s) {
        Ok(Block::Hash(h)) => Ok(Item::Hash(h)),
        _ => Err(format!(
            "\"{s}\" is not a transaction hash or <block>:<index> (for example 26119978:5)"
        )),
    }
}

/// What the page asks for: `transaction`, `receipt` or `blockReceipts`, and the text typed.
pub fn subject(kind: &str, target: &str) -> Result<Subject, Failure> {
    let bad = |m: String| Failure::new(Kind::InvalidRequest, m);
    match kind {
        "transaction" => parse_item(target).map(Subject::Transaction).map_err(bad),
        "receipt" => parse_item(target).map(Subject::Receipt).map_err(bad),
        "blockReceipts" => parse_block(target).map(Subject::BlockReceipts).map_err(bad),
        other => Err(Failure::new(
            Kind::Internal,
            format!("unknown proof kind {other}"),
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

/// The JSON-RPC method and params that ask Willow for a proof of `s`.
pub fn request(s: &Subject) -> (&'static str, Value) {
    match *s {
        Subject::Transaction(i) => ("willow_getTransactionProof", item_params(i)),
        Subject::Receipt(i) => ("willow_getReceiptProof", item_params(i)),
        Subject::BlockReceipts(b) => ("willow_getBlockReceiptsProof", json!([block_param(b)])),
    }
}

/// A verified proof and the standard JSON rendered from its proven bytes.
pub struct ItemOutcome {
    pub verified: Verified,
    pub method: &'static str,
    /// The answer of `eth_getTransactionByHash`, `eth_getTransactionReceipt` or
    /// `eth_getBlockReceipts`.
    pub answer: Rendered,
    /// Every transaction object of the block (block proofs).
    pub transactions: Option<Rendered>,
    /// The receipt's transaction object (receipt proofs).
    pub transaction: Option<Rendered>,
}

/// Check `response` (the proof method's response body, as the server sent it)
/// as the answer to `s` on chain `chain_id`.
pub fn check_items(chain_id: u64, s: &Subject, response: &str) -> Result<ItemOutcome, Failure> {
    let (method, _) = request(s);
    let p: BlockProof = result_of(response, method)?;
    let rejected = |e: anyhow::Error| Failure::new(Kind::Rejected, format!("{e:#}"));
    let verified = verify(&p, chain_id, s).map_err(rejected)?;
    let answer = verified.answer().map_err(rejected)?;
    let transactions = match s {
        Subject::BlockReceipts(_) => Some(verified.transactions_json().map_err(rejected)?),
        _ => None,
    };
    let transaction = match s {
        Subject::Receipt(_) => Some(verified.transaction().map_err(rejected)?),
        _ => None,
    };
    Ok(ItemOutcome {
        verified,
        method,
        answer,
        transactions,
        transaction,
    })
}

impl ItemOutcome {
    pub fn report(&self) -> Value {
        let v = &self.verified;
        let mut r = json!({
            "method": self.method,
            "chainId": v.chain_id,
            "block": {
                "number": format!("0x{:x}", v.block_number),
                "hash": format!("0x{}", hex::encode(v.block_hash)),
                "timestamp": v.header.timestamp,
            },
            "provenTransactions": v.transactions.len(),
            "provenReceipts": v.receipts.len(),
            "unproven": self.answer.unproven,
        });
        if let Some(i) = v.index {
            r["transactionIndex"] = json!(format!("0x{i:x}"));
            r["transactionHash"] = json!(v
                .transactions
                .get(&i)
                .map(|tx| format!("0x{}", hex::encode(logset_blockproof::keccak(tx)))));
        }
        if self.transactions.is_some() {
            r["transactions"] = json!(v.transactions.len());
        }
        r
    }

    /// How another answer compares with the proven JSON. `which`: `answer`
    /// (the response's `result` against the proven answer) or `transactions`
    /// (the `transactions` of an `eth_getBlockByHash(h, true)` result against
    /// every proven transaction).
    pub fn compare(&self, which: &str, response: &str) -> Result<Value, Failure> {
        let (proven, method) = match which {
            "answer" => (&self.answer, "the answer"),
            "transactions" => match &self.transactions {
                Some(t) => (t, "eth_getBlockByHash"),
                None => return Err(Failure::new(Kind::Internal, "no proven transactions")),
            },
            other => return Err(Failure::new(Kind::Internal, format!("compare {other}"))),
        };
        let mut answer: Value = result_of(response, method)?;
        if which == "transactions" && !answer.is_null() {
            answer = match answer.get_mut("transactions").map(Value::take) {
                Some(t @ Value::Array(_)) => t,
                _ => {
                    return Err(Failure::new(
                        Kind::MalformedInput,
                        "eth_getBlockByHash answer has no transactions list",
                    ))
                }
            };
        }
        if answer.is_null() {
            return Ok(json!({"exact": false, "absent": true, "matched": 0,
                "differ": [], "differCount": 0, "unproven": [], "omitted": []}));
        }
        let c = compare(proven, &answer);
        let clip = |s: &String| -> String {
            if s.chars().count() <= DIFFER_CHARS {
                return s.clone();
            }
            let cut: String = s.chars().take(DIFFER_CHARS).collect();
            format!("{cut}…")
        };
        Ok(json!({
            "exact": c.is_exact(),
            "absent": false,
            "matched": c.matched,
            "differ": c.differ.iter().take(DIFFER_SHOWN).map(clip).collect::<Vec<_>>(),
            "differCount": c.differ.len(),
            "unproven": c.unproven.iter().take(DIFFER_SHOWN).collect::<Vec<_>>(),
            "omitted": c.omitted.iter().take(DIFFER_SHOWN).collect::<Vec<_>>(),
        }))
    }
}

/// What `ItemVerifier.verify` returns: `{"ok": true, "report", "proven", "provenTransactions"?,
/// "provenTransaction"?}` or a [`Failure`]. A receipt's transaction lists the fields the proof
/// does not cover in `provenTransactionUnproven`.
pub fn item_verified_json(r: &Result<ItemOutcome, Failure>) -> Value {
    match r {
        Ok(o) => {
            let mut out = json!({"ok": true, "report": o.report(), "proven": o.answer.json});
            if let Some(t) = &o.transactions {
                out["provenTransactions"] = t.json.clone();
            }
            if let Some(t) = &o.transaction {
                out["provenTransaction"] = t.json.clone();
                out["provenTransactionUnproven"] = json!(t.unproven);
            }
            out
        }
        Err(f) => f.json(),
    }
}

/// The JS face for transaction and receipt proofs: `new ItemVerifier(chainId)`,
/// then `request(kind, text)`, `verify(response)` and `compare(which, response)`.
/// Like `Verifier`, no method throws a verdict: each returns JSON with `ok` and,
/// when it is false, `kind` and `message`.
#[wasm_bindgen]
pub struct ItemVerifier {
    chain_id: u64,
    subject: Option<Subject>,
    checked: Option<ItemOutcome>,
}

#[wasm_bindgen]
impl ItemVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(chain_id: u32) -> ItemVerifier {
        ItemVerifier {
            chain_id: chain_id.into(),
            subject: None,
            checked: None,
        }
    }

    /// What to ask Willow for `kind` (`transaction`, `receipt`, `blockReceipts`) and the text
    /// typed (a transaction hash or `<block>:<index>`; a block number, hash or `latest`):
    /// `{"ok": true, "method", "params"}`, or `{"ok": false, "kind": "invalid-request", ...}`.
    /// The next `verify` checks the answer against this request.
    pub fn request(&mut self, kind: &str, target: &str) -> String {
        self.checked = None;
        self.subject = None;
        match subject(kind, target) {
            Ok(s) => {
                let (method, params) = request(&s);
                self.subject = Some(s);
                json!({"ok": true, "method": method, "params": params}).to_string()
            }
            Err(f) => f.json().to_string(),
        }
    }

    /// Checks `response` (the proof method's response body, as the server sent it) as the
    /// answer to the last `request`.
    pub fn verify(&mut self, response: &str) -> String {
        self.checked = None;
        let Some(s) = self.subject else {
            return Failure::new(Kind::Internal, "verify called before request")
                .json()
                .to_string();
        };
        let r = check_items(self.chain_id, &s, response);
        let out = item_verified_json(&r).to_string();
        self.checked = r.ok();
        out
    }

    /// Compares another answer (its response body, as the server sent it) with the proven
    /// JSON of the last `verify` that passed: `{"ok": true, "result"}` or a failure.
    pub fn compare(&self, which: &str, response: &str) -> String {
        let Some(o) = &self.checked else {
            return Failure::new(Kind::Internal, "compare called without a verified proof")
                .json()
                .to_string();
        };
        match o.compare(which, response) {
            Ok(result) => json!({"ok": true, "result": result}).to_string(),
            Err(f) => f.json().to_string(),
        }
    }
}

#[cfg(test)]
mod tests;
