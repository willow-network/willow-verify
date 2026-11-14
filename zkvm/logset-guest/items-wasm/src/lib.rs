//! The browser verifier's transaction, receipt, block-receipts and block proofs
//! (Merkle-Patricia proofs against the block hash, no zk), and its fee and
//! fee-history proofs (`willow-fee-v1` over verified headers and bodies), with
//! [`logset_blockproof`], compiled to wasm32 and run in the page's Web Worker
//! and in `@willow-network/rpc` (web/client, interface I6 in its
//! `src/verifiers.ts`). The caller's own request decides what a proof must
//! show ([`Asked`]); the response is checked against it, never the other way
//! round. Like the log verifier (query-wasm), no method throws a verdict: each
//! returns JSON with `ok` and, when it is false, a `kind` (and, for a failed
//! verification, the `fault` blockproof names) the caller decides by.

use logset_blockproof::compare::{compare, Comparison};
use logset_blockproof::fault::{fault, Fault};
use logset_blockproof::fees::{self, FeeHistory, FeeHistoryRequest, FeeProof, Fees};
use logset_blockproof::proof::{decode_header, verify, Block, BlockProof, Item, Subject, Verified};
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
    /// What failed verification, when blockproof names it ([`Fault`]): on `rejected` failures.
    pub fault: Option<Fault>,
}

impl Failure {
    pub fn new(kind: Kind, message: impl Into<String>) -> Self {
        Failure {
            kind,
            message: message.into(),
            fault: None,
        }
    }

    /// A verification that failed: `rejected`, with the [`Fault`] the error carries.
    pub fn rejected(e: anyhow::Error) -> Self {
        Failure {
            kind: Kind::Rejected,
            message: format!("{e:#}"),
            fault: fault(&e),
        }
    }

    /// `{"ok": false, "kind", "message", "fault"?}`, as the page receives it.
    pub fn json(&self) -> Value {
        let mut out = json!({"ok": false, "kind": self.kind.as_str(), "message": self.message});
        if let Some(f) = self.fault {
            out["fault"] = json!(f.as_str());
        }
        out
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
        .map_err(|e| Failure::new(Kind::MalformedInput, format!("{method} response: {e}")))
}

/// Differences listed per comparison, and characters kept of each.
const DIFFER_SHOWN: usize = 50;
const DIFFER_CHARS: usize = 300;

/// A whole number: decimal, or `0x` hex.
fn parse_number(s: &str) -> Option<u64> {
    match s.strip_prefix("0x") {
        Some(h) if !h.is_empty() && h.len() <= 16 => u64::from_str_radix(h, 16).ok(),
        Some(_) => None,
        None if !s.is_empty() && s.len() <= 20 && s.bytes().all(|b| b.is_ascii_digit()) => {
            s.parse().ok()
        }
        None => None,
    }
}

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
    parse_number(s)
        .map(Block::Number)
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

/// What the page asks for: `transaction`, `receipt`, `blockReceipts` or `block`, and the text typed.
pub fn subject(kind: &str, target: &str) -> Result<Subject, Failure> {
    let bad = |m: String| Failure::new(Kind::InvalidRequest, m);
    match kind {
        "transaction" => parse_item(target).map(Subject::Transaction).map_err(bad),
        "receipt" => parse_item(target).map(Subject::Receipt).map_err(bad),
        "blockReceipts" => parse_block(target).map(Subject::BlockReceipts).map_err(bad),
        "block" => parse_block(target).map(Subject::Block).map_err(bad),
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
        Subject::Block(b) => ("willow_getBlockProof", json!([block_param(b)])),
    }
}

/// What a caller asked to be proven: an item of a block (or the block), a block's fees, or a fee
/// history. The response is verified against it.
#[derive(Clone, Debug, PartialEq)]
pub enum Asked {
    Items(Subject),
    /// `willow_getFeeProof` at a block (H): `eth_gasPrice`, `eth_maxPriorityFeePerGas` and
    /// `eth_blobBaseFee` by `willow-fee-v1`.
    Fees(Block),
    /// `willow_getFeeHistoryProof`: `eth_feeHistory`.
    FeeHistory(FeeHistoryRequest),
}

/// `kind` and `target` read as a request on chain `chain_id`: what it asks, and the JSON-RPC
/// method and params that ask Willow for it. Kinds: `transaction`, `receipt`, `blockReceipts`,
/// `block` and `fees` (target: the text typed), `feeHistory` (target: eth_feeHistory's params
/// as JSON text, `[blockCount, newestBlock, rewardPercentiles]`).
pub fn asked(
    chain_id: u64,
    kind: &str,
    target: &str,
) -> Result<(Asked, &'static str, Value), Failure> {
    let bad = |m: String| Failure::new(Kind::InvalidRequest, m);
    match kind {
        "fees" => {
            let b = parse_block(target).map_err(bad)?;
            Ok((
                Asked::Fees(b),
                "willow_getFeeProof",
                json!([block_param(b)]),
            ))
        }
        "feeHistory" => fee_history_request(chain_id, target),
        _ => {
            let s = subject(kind, target)?;
            let (method, params) = request(&s);
            Ok((Asked::Items(s), method, params))
        }
    }
}

/// `[blockCount, newestBlock, rewardPercentiles]` (JSON text) as a fee-history request, checked
/// by geth's rules (`FeeHistoryRequest::check`): a count of at least 1 (truncated to 1,024), the
/// newest block as a JSON string, at most 100 percentiles, each in [0, 100], increasing. With
/// percentiles on a chain with tips, one proof covers at most 16 blocks; the caller asks for more
/// in parts. The params sent are `[count as a 0x quantity, newestBlock as written, percentiles as
/// written]` (`[]` when left out).
fn fee_history_request(
    chain_id: u64,
    target: &str,
) -> Result<(Asked, &'static str, Value), Failure> {
    let bad = |m: String| Failure::new(Kind::InvalidRequest, m);
    let shape = "a fee history is asked as [blockCount, newestBlock, rewardPercentiles]";
    let p: Value = serde_json::from_str(target).map_err(|e| bad(format!("{shape}: {e}")))?;
    let list = p
        .as_array()
        .filter(|l| (2..=3).contains(&l.len()))
        .ok_or_else(|| bad(shape.into()))?;
    let count = match &list[0] {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => parse_number(s.trim()),
        _ => None,
    }
    .filter(|&c| c >= 1)
    .ok_or_else(|| {
        bad(format!(
            "blockCount {} is not a whole number of at least 1",
            list[0]
        ))
    })?;
    let newest_text = list[1]
        .as_str()
        .ok_or_else(|| bad(format!("{shape}: newestBlock is a JSON string")))?;
    let newest = parse_block(newest_text).map_err(bad)?;
    let percentiles = match list.get(2) {
        None | Some(Value::Null) => json!([]),
        Some(v) => v.clone(),
    };
    let reward_percentiles = percentiles
        .as_array()
        .ok_or_else(|| bad("rewardPercentiles is a list of numbers".into()))?
        .iter()
        .map(|x| {
            x.as_f64()
                .ok_or_else(|| bad(format!("reward percentile {x} is not a number")))
        })
        .collect::<Result<Vec<f64>, Failure>>()?;
    let req = FeeHistoryRequest {
        block_count: count,
        newest,
        reward_percentiles,
    };
    req.check().map_err(|e| bad(format!("{e:#}")))?;
    if req.with_bodies(chain_id) && req.count() > fees::MAX_HISTORY_WITH_REWARDS {
        return Err(bad(format!(
            "a fee-history proof with rewards covers at most {} blocks, not {}: ask for the rest in another proof",
            fees::MAX_HISTORY_WITH_REWARDS,
            req.count()
        )));
    }
    let params = json!([format!("0x{:x}", req.count()), list[1].clone(), percentiles]);
    Ok((Asked::FeeHistory(req), "willow_getFeeHistoryProof", params))
}

/// The block a proof names, as the caller anchors it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Named {
    pub number: u64,
    pub hash: [u8; 32],
    pub timestamp: u64,
    pub parent_hash: [u8; 32],
}

impl Named {
    fn json(&self) -> Value {
        json!({
            "number": format!("0x{:x}", self.number),
            "hash": format!("0x{}", hex::encode(self.hash)),
            "timestamp": self.timestamp,
            "parentHash": format!("0x{}", hex::encode(self.parent_hash)),
        })
    }

    fn of(v: &Verified) -> Named {
        Named {
            number: v.block_number,
            hash: v.block_hash,
            timestamp: v.header.timestamp,
            parent_hash: v.header.parent_hash.0,
        }
    }

    /// The newest header of a fee or fee-history proof that verified.
    fn newest(p: &FeeProof) -> Result<Named, Failure> {
        let internal = |m: String| Failure::new(Kind::Internal, m);
        let last = p
            .headers
            .last()
            .ok_or_else(|| internal("a verified proof without headers".into()))?;
        let rlp = hex::decode(last.strip_prefix("0x").unwrap_or(last))
            .map_err(|e| internal(format!("a verified header: {e}")))?;
        let h = decode_header(&rlp).map_err(|e| internal(format!("a verified header: {e:#}")))?;
        Ok(Named {
            number: h.number,
            hash: logset_blockproof::keccak(&rlp),
            timestamp: h.timestamp,
            parent_hash: h.parent_hash.0,
        })
    }
}

/// A verified proof and the standard JSON rendered from its proven bytes.
pub struct ItemOutcome {
    pub verified: Verified,
    pub method: &'static str,
    /// The response of `eth_getTransactionByHash`, `eth_getTransactionReceipt`,
    /// `eth_getBlockReceipts`, or `eth_getBlockByNumber(n, false)` (block proofs).
    pub answer: Rendered,
    /// Every transaction object of the block (block-receipts proofs).
    pub transactions: Option<Rendered>,
    /// The receipt's transaction object (receipt proofs).
    pub transaction: Option<Rendered>,
}

/// Check `response` (the proof method's response body, as the server sent it)
/// as the answer to `s` on chain `chain_id`.
pub fn check_items(chain_id: u64, s: &Subject, response: &str) -> Result<ItemOutcome, Failure> {
    let (method, _) = request(s);
    let p: BlockProof = result_of(response, method)?;
    let verified = verify(&p, chain_id, s).map_err(Failure::rejected)?;
    let answer = verified.answer().map_err(Failure::rejected)?;
    let transactions = match s {
        Subject::BlockReceipts(_) => Some(verified.transactions_json().map_err(Failure::rejected)?),
        _ => None,
    };
    let transaction = match s {
        Subject::Receipt(_) => Some(verified.transaction().map_err(Failure::rejected)?),
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

/// A comparison as the page and the client read it.
fn comparison_json(c: &Comparison) -> Value {
    let clip = |s: &String| -> String {
        if s.chars().count() <= DIFFER_CHARS {
            return s.clone();
        }
        let cut: String = s.chars().take(DIFFER_CHARS).collect();
        format!("{cut}…")
    };
    json!({
        "exact": c.is_exact(),
        "absent": false,
        "matched": c.matched,
        "differ": c.differ.iter().take(DIFFER_SHOWN).map(clip).collect::<Vec<_>>(),
        "differCount": c.differ.len(),
        "unproven": c.unproven.iter().take(DIFFER_SHOWN).collect::<Vec<_>>(),
        "omitted": c.omitted.iter().take(DIFFER_SHOWN).collect::<Vec<_>>(),
    })
}

/// Another endpoint that does not have what the proof shows (it responded `null`).
fn absent() -> Value {
    json!({"exact": false, "absent": true, "matched": 0,
        "differ": [], "differCount": 0, "unproven": [], "omitted": []})
}

impl ItemOutcome {
    pub fn report(&self) -> Value {
        let v = &self.verified;
        let mut r = json!({
            "method": self.method,
            "chainId": v.chain_id,
            "block": Named::of(v).json(),
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
        if self.transactions.is_some() || matches!(v.subject, Subject::Block(_)) {
            r["transactions"] = json!(v.transactions.len());
        }
        r
    }

    fn is_block(&self) -> bool {
        matches!(self.verified.subject, Subject::Block(_))
    }

    /// How another response compares with the proven JSON. `which`: `answer`
    /// (the response's `result` against the proven response) or `transactions`
    /// (the `transactions` of an `eth_getBlockByHash(h, true)` result against
    /// every proven transaction, for block-receipts and block proofs).
    pub fn compare(&self, which: &str, response: &str) -> Result<Value, Failure> {
        let full;
        let (proven, method) = match which {
            "answer" => (&self.answer, ""),
            "transactions" if self.is_block() => {
                let b = self.verified.block(true).map_err(Failure::rejected)?;
                full = Rendered {
                    json: b.json.get("transactions").cloned().unwrap_or(Value::Null),
                    unproven: b
                        .unproven
                        .iter()
                        .filter_map(|f| f.strip_prefix("transactions").map(str::to_string))
                        .collect(),
                };
                (&full, "eth_getBlockByHash")
            }
            "transactions" => match &self.transactions {
                Some(t) => (t, "eth_getBlockByHash"),
                None => return Err(Failure::new(Kind::Internal, "no proven transactions")),
            },
            other => return Err(Failure::new(Kind::Internal, format!("compare {other}"))),
        };
        let mut answer: Value = match method {
            "" => serde_json::from_str::<Response<Value>>(response)
                .map(|r| r.result)
                .map_err(|e| Failure::new(Kind::MalformedInput, format!("the response: {e}")))?,
            m => result_of(response, m)?,
        };
        if which == "transactions" && !answer.is_null() {
            answer = match answer.get_mut("transactions").map(Value::take) {
                Some(t @ Value::Array(_)) => t,
                _ => {
                    return Err(Failure::new(
                        Kind::MalformedInput,
                        "eth_getBlockByHash response has no transactions list",
                    ))
                }
            };
        }
        if answer.is_null() {
            return Ok(absent());
        }
        Ok(comparison_json(&compare(proven, &answer)))
    }
}

/// What a passing `verify` holds, for `block` and `compare`.
pub enum Checked {
    Items(Box<ItemOutcome>),
    Fees { fees: Fees, block: Named },
    FeeHistory { history: FeeHistory, block: Named },
}

/// Check `response` (the proof method's response body, as the server sent it) as the response
/// to `a` on chain `chain_id`.
pub fn check(chain_id: u64, a: &Asked, response: &str) -> Result<Checked, Failure> {
    match a {
        Asked::Items(s) => check_items(chain_id, s, response).map(|o| Checked::Items(Box::new(o))),
        Asked::Fees(b) => {
            let p: FeeProof = result_of(response, "willow_getFeeProof")?;
            let fees = fees::verify_fees(&p, chain_id, *b).map_err(Failure::rejected)?;
            Ok(Checked::Fees {
                block: Named::newest(&p)?,
                fees,
            })
        }
        Asked::FeeHistory(req) => {
            let p: FeeProof = result_of(response, "willow_getFeeHistoryProof")?;
            let history = fees::verify_fee_history(&p, chain_id, req).map_err(Failure::rejected)?;
            Ok(Checked::FeeHistory {
                block: Named::newest(&p)?,
                history,
            })
        }
    }
}

impl Checked {
    /// The report of a fee or fee-history proof: the block the caller anchors (H, or the newest).
    fn fee_report(method: &str, chain_id: u64, block: &Named) -> Value {
        json!({
            "method": method,
            "chainId": chain_id,
            "block": block.json(),
            "unproven": [],
            "formula": fees::FORMULA,
        })
    }

    /// Another endpoint's response compared with the proven one. Fees: `which` is the method
    /// (`eth_gasPrice`, `eth_maxPriorityFeePerGas` or `eth_blobBaseFee`), its result against
    /// the formula's value. Fee history: `answer`, an `eth_feeHistory` result.
    pub fn compare(&self, which: &str, response: &str) -> Result<Value, Failure> {
        match self {
            Checked::Items(o) => o.compare(which, response),
            Checked::Fees { fees, .. } => {
                let proven = fees.json();
                let value = match which {
                    "eth_gasPrice" | "eth_maxPriorityFeePerGas" | "eth_blobBaseFee" => {
                        proven[which].clone()
                    }
                    other => return Err(Failure::new(Kind::Internal, format!("compare {other}"))),
                };
                let answer: Value = result_of(response, which)?;
                if answer.is_null() && !value.is_null() {
                    return Ok(absent());
                }
                let mine = Rendered {
                    json: json!({ which: value }),
                    unproven: vec![],
                };
                Ok(comparison_json(&compare(&mine, &json!({ which: answer }))))
            }
            Checked::FeeHistory { history, .. } => {
                if which != "answer" {
                    return Err(Failure::new(Kind::Internal, format!("compare {which}")));
                }
                let mut answer: Value = result_of(response, "eth_feeHistory")?;
                if answer.is_null() {
                    return Ok(absent());
                }
                let mine = history.json();
                same_ratios(&mine, &mut answer);
                let mine = Rendered {
                    json: mine,
                    unproven: vec![],
                };
                Ok(comparison_json(&compare(&mine, &answer)))
            }
        }
    }
}

/// The gas-used ratios of another `eth_feeHistory` response that are ours to 12 significant
/// digits, written as ours: nodes divide with their own float steps (reth's blobGasUsedRatio is
/// geth's ± 1 ulp) and write `0` or `0.0`.
fn same_ratios(mine: &Value, theirs: &mut Value) {
    for key in ["gasUsedRatio", "blobGasUsedRatio"] {
        let (Some(a), Some(b)) = (
            mine[key].as_array(),
            theirs.get_mut(key).and_then(Value::as_array_mut),
        ) else {
            continue;
        };
        for (x, y) in a.iter().zip(b.iter_mut()) {
            if let (Some(p), Some(q)) = (x.as_f64(), y.as_f64()) {
                if (p - q).abs() <= 1e-12 * p.abs().max(q.abs()) {
                    *y = x.clone();
                }
            }
        }
    }
}

/// What `ItemVerifier.verify` returns: `{"ok": true, "report", "proven", "provenTransactions"?,
/// "provenTransaction"?}` or a [`Failure`]. A receipt's transaction lists the fields the proof
/// does not cover in `provenTransactionUnproven`.
pub fn item_verified_json(r: &Result<ItemOutcome, Failure>) -> Value {
    match r {
        Ok(o) => item_json(o),
        Err(f) => f.json(),
    }
}

fn item_json(o: &ItemOutcome) -> Value {
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

/// What `ItemVerifier.verify` returns for any kind: `{"ok": true, "report", "proven", ...}` or
/// a [`Failure`]. Fees: `proven` is `Fees::json()`; fee history: the `eth_feeHistory` result.
pub fn verified_json(chain_id: u64, r: &Result<Checked, Failure>) -> Value {
    match r {
        Ok(Checked::Items(o)) => item_json(o),
        Ok(Checked::Fees { fees, block }) => json!({
            "ok": true,
            "report": Checked::fee_report("willow_getFeeProof", chain_id, block),
            "proven": fees.json(),
        }),
        Ok(Checked::FeeHistory { history, block }) => json!({
            "ok": true,
            "report": Checked::fee_report("willow_getFeeHistoryProof", chain_id, block),
            "proven": history.json(),
        }),
        Err(f) => f.json(),
    }
}

/// The JS face (interface I6, web/client/src/verifiers.ts): `new ItemVerifier(chainId)`, then
/// `request(kind, target)`, `verify(response)`, `block(full)` and `compare(which, response)`.
/// Like `Verifier`, no method throws a verdict: each returns JSON with `ok` and, when it is
/// false, `kind`, `message` and, for a failed verification, `fault`.
#[wasm_bindgen]
pub struct ItemVerifier {
    chain_id: u64,
    asked: Option<Asked>,
    checked: Option<Checked>,
}

#[wasm_bindgen]
impl ItemVerifier {
    #[wasm_bindgen(constructor)]
    pub fn new(chain_id: u32) -> ItemVerifier {
        ItemVerifier {
            chain_id: chain_id.into(),
            asked: None,
            checked: None,
        }
    }

    /// What to ask Willow for `kind` (`transaction`, `receipt`, `blockReceipts`, `block`, `fees`,
    /// `feeHistory`) and the target (a transaction hash or `<block>:<index>`; a block number,
    /// hash or `latest`; for `feeHistory`, eth_feeHistory's params as JSON text):
    /// `{"ok": true, "method", "params"}`, or `{"ok": false, "kind": "invalid-request", ...}`
    /// (`internal` for an unknown kind). The next `verify` checks the response against this
    /// request.
    pub fn request(&mut self, kind: &str, target: &str) -> String {
        self.checked = None;
        self.asked = None;
        match asked(self.chain_id, kind, target) {
            Ok((a, method, params)) => {
                self.asked = Some(a);
                json!({"ok": true, "method": method, "params": params}).to_string()
            }
            Err(f) => f.json().to_string(),
        }
    }

    /// Checks `response` (the proof method's response body, as the server sent it) as the
    /// response to the last `request`.
    pub fn verify(&mut self, response: &str) -> String {
        self.checked = None;
        let Some(a) = &self.asked else {
            return Failure::new(Kind::Internal, "verify called before request")
                .json()
                .to_string();
        };
        let r = check(self.chain_id, a, response);
        let out = verified_json(self.chain_id, &r).to_string();
        self.checked = r.ok();
        out
    }

    /// The last passing `verify` of a block proof rendered as `eth_getBlockByNumber(n, full)`:
    /// `{"ok": true, "proven", "unproven"}`. With `full`, every transaction object, each sender
    /// recovered from its signature here, on demand. `internal` when the last `verify` was not
    /// a passing block proof.
    pub fn block(&self, full: bool) -> String {
        let Some(Checked::Items(o)) = &self.checked else {
            return Failure::new(
                Kind::Internal,
                "block called without a verified block proof",
            )
            .json()
            .to_string();
        };
        if !o.is_block() {
            return Failure::new(
                Kind::Internal,
                "block called without a verified block proof",
            )
            .json()
            .to_string();
        }
        let r = if full {
            o.verified.block(true).map_err(Failure::rejected)
        } else {
            Ok(o.answer.clone())
        };
        match r {
            Ok(r) => json!({"ok": true, "proven": r.json, "unproven": r.unproven}).to_string(),
            Err(f) => f.json().to_string(),
        }
    }

    /// Compares another response (its body, as the server sent it) with the proven JSON of the
    /// last `verify` that passed: `{"ok": true, "result"}` or a failure. `which`: `answer` or
    /// `transactions` for item and block proofs, `answer` for fee history, and for fees the
    /// method (`eth_gasPrice`, `eth_maxPriorityFeePerGas`, `eth_blobBaseFee`).
    pub fn compare(&self, which: &str, response: &str) -> String {
        let Some(c) = &self.checked else {
            return Failure::new(Kind::Internal, "compare called without a verified proof")
                .json()
                .to_string();
        };
        match c.compare(which, response) {
            Ok(result) => json!({"ok": true, "result": result}).to_string(),
            Err(f) => f.json().to_string(),
        }
    }
}

#[cfg(test)]
mod tests;
