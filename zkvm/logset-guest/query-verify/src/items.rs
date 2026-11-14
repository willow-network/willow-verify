//! `--tx`, `--receipt`, `--block-receipts`, `--block`, `--fees` and `--fee-history`: Merkle-
//! Patricia proofs against the block hash (no zk), checked by the verify page's own items module
//! (`willow_items_verify_wasm`, natively).
//!
//! 1. Asks Willow for `willow_getTransactionProof`, `willow_getReceiptProof`,
//!    `willow_getBlockReceiptsProof`, `willow_getBlockProof`, `willow_getFeeProof` or
//!    `willow_getFeeHistoryProof`.
//! 2. Verifies it as the response to the request given here, never to the server's: the header
//!    hashes to the block hash, every item is under the header's roots, the block and transaction
//!    are the ones asked for; fees by the published formula `willow-fee-v1` over headers chained
//!    by their parent hashes. Renders the standard JSON from the proven bytes.
//! 3. Anchors the block (for fees and fee history, the newest; the others chain up to it).
//! 4. `--compare-rpc`: that endpoint's standard response, field by field.

use crate::check::{counted, problem, Ctx, Named};
use logset_blockproof::compare::{compare, Comparison};
use logset_blockproof::render::Rendered;
use serde_json::{json, Value};
use willow_items_verify_wasm as iw;

/// items-wasm's token for comparing the standard response itself (its `Checked::compare`).
const STANDARD: &str = "answer";

/// What to prove, from the command line.
#[derive(Clone, Debug, PartialEq)]
pub enum Ask {
    /// A transaction: its hash, or `<block>:<index>`.
    Tx(String),
    Receipt(String),
    BlockReceipts(String),
    Block {
        block: String,
        full: bool,
    },
    Fees {
        at: String,
    },
    FeeHistory(FeeHistoryArgs),
}

/// `--fee-history <count>,<newest>[,<p1>,<p2>…]`.
#[derive(Clone, Debug, PartialEq)]
pub struct FeeHistoryArgs {
    pub count: u64,
    pub newest: String,
    pub percentiles: Vec<f64>,
}

pub fn parse_fee_history(s: &str) -> Result<FeeHistoryArgs, String> {
    let shape =
        "--fee-history <count>,<newest block>[,<percentile>,…] (for example 4,latest,25,75)";
    let mut it = s.split(',').map(str::trim);
    let count = it
        .next()
        .and_then(crate::check::parse_number)
        .filter(|&c| c >= 1)
        .ok_or_else(|| format!("{shape}: the count is a whole number of at least 1"))?;
    let newest = it
        .next()
        .filter(|s| !s.is_empty())
        .ok_or_else(|| format!("{shape}: no newest block"))?
        .to_string();
    let percentiles = it
        .map(|p| {
            p.parse::<f64>()
                .map_err(|_| format!("{shape}: {p:?} is not a percentile"))
        })
        .collect::<Result<Vec<f64>, String>>()?;
    Ok(FeeHistoryArgs {
        count,
        newest,
        percentiles,
    })
}

/// A transaction target with its block resolved: `<hash>` as given, `<block>:<index>` with the
/// block named as the light client anchored it. `Err` inside: the report of a head not anchored.
fn item_target(ctx: &mut Ctx, s: &str, method: &str) -> Result<Result<String, Value>, String> {
    let s = s.trim();
    let Some((block, index)) = s.rsplit_once(':') else {
        return Ok(Ok(s.to_string()));
    };
    Ok(block_target(ctx, block, method)?.map(|b| format!("{b}:{index}")))
}

/// A block target resolved (see [`Ctx::resolve`]), as items-wasm reads it.
fn block_target(ctx: &mut Ctx, s: &str, method: &str) -> Result<Result<String, Value>, String> {
    match ctx.resolve(s) {
        Ok(n) => Ok(Ok(n.text())),
        Err(e) => e.report(ctx, method).map(Err),
    }
}

/// A clipped comparison as the reports show it (items-wasm's shape).
fn comparison_json(c: &Comparison) -> Value {
    json!({
        "exact": c.is_exact(),
        "absent": false,
        "matched": c.matched,
        "differ": c.differ.iter().take(50).collect::<Vec<_>>(),
        "differCount": c.differ.len(),
        "unproven": c.unproven.iter().take(50).collect::<Vec<_>>(),
        "omitted": c.omitted.iter().take(50).collect::<Vec<_>>(),
    })
}

/// The proof's failure report.
fn failed(ctx: &Ctx, method: &str, f: &iw::Failure, size: usize) -> Value {
    let mut r = ctx.base(method);
    r["proofBytes"] = json!(size);
    r["failure"] = f.json();
    r
}

/// Fetch, verify, anchor, compare: the report, and whether everything checked out.
pub fn run(ctx: &mut Ctx, ask: Ask) -> Result<(Value, bool), String> {
    let (kind, target) = match &ask {
        Ask::FeeHistory(h) => return fee_history(ctx, h),
        Ask::Tx(t) => (
            "transaction",
            item_target(ctx, t, "willow_getTransactionProof")?,
        ),
        Ask::Receipt(t) => ("receipt", item_target(ctx, t, "willow_getReceiptProof")?),
        Ask::BlockReceipts(b) => (
            "blockReceipts",
            block_target(ctx, b, "willow_getBlockReceiptsProof")?,
        ),
        Ask::Block { block, .. } => ("block", block_target(ctx, block, "willow_getBlockProof")?),
        Ask::Fees { at } => ("fees", block_target(ctx, at, "willow_getFeeProof")?),
    };
    let target = match target {
        Ok(t) => t,
        Err(report) => return Ok((report, false)),
    };
    let (asked, method, params) =
        iw::asked(ctx.chain_id, kind, &target).map_err(|f| f.message.clone())?;
    let body = ctx.willow(method, &params)?;
    if let Some(r) = ctx.not_proven(method, &body) {
        return Ok((r, false));
    }
    let t = std::time::Instant::now();
    let checked = iw::check(ctx.chain_id, &asked, &body);
    let verified = iw::verified_json(ctx.chain_id, &checked);
    let checked = match checked {
        Ok(c) => c,
        Err(f) => return Ok((failed(ctx, method, &f, body.len()), false)),
    };
    let mut report = verified["report"].clone();
    report["verified"] = json!(true);
    report["chain"] = json!(crate::check::chain_name(ctx.chain_id));
    report["proofBytes"] = json!(body.len());
    report["verifySeconds"] = json!(t.elapsed().as_secs_f64());
    let mut proven = verified["proven"].clone();
    let full = matches!(ask, Ask::Block { full: true, .. });
    if full {
        if let iw::Checked::Items(o) = &checked {
            match o.verified.block(true) {
                Ok(b) => {
                    proven = b.json;
                    report["unproven"] = json!(b.unproven);
                }
                Err(e) => {
                    return Ok((
                        failed(ctx, method, &iw::Failure::rejected(e), body.len()),
                        false,
                    ))
                }
            }
        }
    }
    let number = crate::check::parse_number(report["block"]["number"].as_str().unwrap_or(""))
        .ok_or("a verified report without a block number")?;
    let hash = report["block"]["hash"]
        .as_str()
        .ok_or("a verified report without a block hash")?
        .to_string();
    let mut ok = ctx.anchor_into(&mut report, number, &hash);
    if ctx.compare_rpc.is_some() {
        let tx_hash = report["transactionHash"].clone();
        let asks: Vec<(&str, &str, Value)> = match kind {
            "transaction" => vec![("eth_getTransactionByHash", STANDARD, json!([tx_hash]))],
            "receipt" => vec![("eth_getTransactionReceipt", STANDARD, json!([tx_hash]))],
            "blockReceipts" => vec![
                ("eth_getBlockReceipts", STANDARD, json!([hash])),
                ("eth_getBlockByHash", "transactions", json!([hash, true])),
            ],
            "block" if full => vec![
                ("eth_getBlockByHash", STANDARD, json!([hash, false])),
                ("eth_getBlockByHash", "transactions", json!([hash, true])),
            ],
            "block" => vec![("eth_getBlockByHash", STANDARD, json!([hash, false]))],
            _ => vec![
                ("eth_gasPrice", "eth_gasPrice", json!([])),
                (
                    "eth_maxPriorityFeePerGas",
                    "eth_maxPriorityFeePerGas",
                    json!([]),
                ),
                ("eth_blobBaseFee", "eth_blobBaseFee", json!([])),
            ],
        };
        let mut out = serde_json::Map::new();
        for (m, which, params) in asks {
            let key = if which == "transactions" {
                format!("{m}.transactions")
            } else {
                m.to_string()
            };
            let c = match ctx.other(m, &params) {
                Ok(theirs) => checked
                    .compare(which, &theirs)
                    .unwrap_or_else(|f| problem(f.json())),
                Err(e) => problem(json!(e)),
            };
            // The fee methods take no block: each node prices its own newest block by its own
            // rules, so they are shown, not counted (as on the verify page).
            if kind != "fees" && counted(&c) {
                ok &= c["exact"] == true;
            }
            out.insert(key, c);
        }
        if kind == "fees" {
            out.insert(
                "note".into(),
                json!("shown, not counted: the fee methods take no block, so another node prices its own newest block by its own rules"),
            );
        }
        report["compare"] = Value::Object(out);
    }
    if ctx.print {
        report["proven"] = proven;
        for k in ["provenTransactions", "provenTransaction"] {
            if !verified[k].is_null() {
                report[k] = verified[k].clone();
            }
        }
    }
    report["verified"] = json!(ok);
    Ok((report, ok))
}

/// Fee history in parts of at most 16 blocks when it carries rewards (one proof's limit), each
/// anchored at its newest block and checked to cover its range, joined oldest first.
fn fee_history(ctx: &mut Ctx, h: &FeeHistoryArgs) -> Result<(Value, bool), String> {
    let method = "willow_getFeeHistoryProof";
    let newest = match ctx.resolve(&h.newest) {
        Ok(n) => n,
        Err(e) => return e.report(ctx, method).map(|r| (r, false)),
    };
    // Parts are asked for by number: the anchored head's, or the one given.
    let newest_n =
        match &newest {
            Named::Number(n) => Some(*n),
            Named::Hash(h) => match &ctx.head_block {
                Some((n, hh)) if hh == h => Some(*n),
                _ => return Err(
                    "--fee-history names its newest block by number or tag, as eth_feeHistory does"
                        .into(),
                ),
            },
            Named::Latest => None,
        };
    let req = logset_blockproof::fees::FeeHistoryRequest {
        block_count: h.count,
        newest: logset_blockproof::proof::Block::Tag,
        reward_percentiles: h.percentiles.clone(),
    };
    let step = if req.with_bodies(ctx.chain_id) {
        logset_blockproof::fees::MAX_HISTORY_WITH_REWARDS
    } else {
        logset_blockproof::fees::MAX_HISTORY
    };
    let count = req.count();
    // Parts, newest first: (newest block, count). Without a number (no light client), one part.
    let parts: Vec<(Option<u64>, u64)> = match newest_n {
        Some(n) => {
            let count = count.min(n + 1);
            let mut v = Vec::new();
            let (mut top, mut left) = (n, count);
            while left > 0 {
                let k = left.min(step);
                v.push((Some(top), k));
                top = top.saturating_sub(k);
                left -= k;
            }
            v
        }
        None if count <= step => vec![(None, count)],
        None => {
            return Err(format!(
                "a fee history with rewards over {step} blocks needs the newest block's number: name it, or drop --anchor-rpc / --no-anchor"
            ))
        }
    };
    let t = std::time::Instant::now();
    let mut proven_parts = Vec::new();
    let mut anchors = Vec::new();
    let mut ok = true;
    let mut bytes = 0;
    let mut newest_block = Value::Null;
    for (top, k) in &parts {
        let top_text = top.map_or("latest".to_string(), |n| format!("0x{n:x}"));
        let target = json!([format!("0x{k:x}"), top_text, h.percentiles]).to_string();
        let (asked, m, params) =
            iw::asked(ctx.chain_id, "feeHistory", &target).map_err(|f| f.message.clone())?;
        let body = ctx.willow(m, &params)?;
        bytes += body.len();
        if let Some(r) = ctx.not_proven(m, &body) {
            return Ok((r, false));
        }
        let checked = iw::check(ctx.chain_id, &asked, &body);
        let v = iw::verified_json(ctx.chain_id, &checked);
        if let Err(f) = checked {
            return Ok((failed(ctx, method, &f, bytes), false));
        }
        let rep = &v["report"]["block"];
        let n = crate::check::parse_number(rep["number"].as_str().unwrap_or(""))
            .ok_or("a verified report without a block number")?;
        let oldest = crate::check::parse_number(v["proven"]["oldestBlock"].as_str().unwrap_or(""));
        let want_oldest = n + 1 - (*k).min(n + 1);
        if top.is_some_and(|t| t != n) || oldest != Some(want_oldest) {
            let mut r = ctx.base(method);
            r["failure"] = json!({"kind": "rejected", "message": format!(
                "a fee-history proof for {k} blocks up to {top_text} covers {} to {n}",
                v["proven"]["oldestBlock"])});
            return Ok((r, false));
        }
        let hash = rep["hash"].as_str().unwrap_or_default().to_string();
        let mut one = json!({"block": rep.clone()});
        ok &= ctx.anchor_into(&mut one, n, &hash);
        if newest_block.is_null() {
            newest_block = one["block"].clone();
        }
        anchors.push(one["anchor"].clone());
        proven_parts.push(v["proven"].clone());
    }
    proven_parts.reverse();
    anchors.reverse();
    let joined = join(&proven_parts);
    let mut report = json!({
        "verified": true,
        "method": method,
        "chainId": ctx.chain_id,
        "chain": crate::check::chain_name(ctx.chain_id),
        "block": newest_block,
        "formula": logset_blockproof::fees::FORMULA,
        "parts": parts.len(),
        "proofBytes": bytes,
        "verifySeconds": t.elapsed().as_secs_f64(),
        "unproven": [],
    });
    report["anchor"] = if anchors.len() == 1 {
        anchors[0].clone()
    } else {
        json!(anchors)
    };
    if let Some(head) = &ctx.head {
        if anchors.len() > 1 {
            report["head"] = head.clone();
        }
    }
    if ctx.compare_rpc.is_some() {
        let oldest = joined["oldestBlock"].clone();
        let newest_q = newest_block["number"].clone();
        let params = json!([
            format!("0x{:x}", count_of(&joined)),
            newest_q,
            h.percentiles
        ]);
        let theirs = ctx.other("eth_feeHistory", &params);
        let c = match theirs.and_then(|t| crate::rpc::result_of("eth_feeHistory", &t)) {
            Ok(Value::Null) => json!({"exact": false, "absent": true}),
            Ok(mut other) => {
                same_ratios(&joined, &mut other);
                let mine = Rendered {
                    json: joined.clone(),
                    unproven: vec![],
                };
                comparison_json(&compare(&mine, &other))
            }
            Err(e) => problem(json!(e)),
        };
        if counted(&c) {
            ok &= c["exact"] == true;
        }
        report["compare"] = json!({ "eth_feeHistory": c, "oldestBlock": oldest });
    }
    if ctx.print {
        report["proven"] = joined;
    }
    report["verified"] = json!(ok);
    Ok((report, ok))
}

fn count_of(joined: &Value) -> usize {
    joined["gasUsedRatio"].as_array().map_or(0, Vec::len)
}

/// Parts' `eth_feeHistory` results, oldest first, as one: each part's last base fee is the next
/// part's first.
fn join(parts: &[Value]) -> Value {
    if parts.len() == 1 {
        return parts[0].clone();
    }
    let mut out = serde_json::Map::new();
    out.insert("oldestBlock".into(), parts[0]["oldestBlock"].clone());
    let list = |k: &str, next_one: bool| -> Option<Value> {
        let mut v = Vec::new();
        for (i, p) in parts.iter().enumerate() {
            let a = p.get(k)?.as_array()?;
            let take = if next_one && i + 1 < parts.len() {
                a.len().saturating_sub(1)
            } else {
                a.len()
            };
            v.extend(a[..take].iter().cloned());
        }
        Some(Value::Array(v))
    };
    for (k, next_one) in [
        ("reward", false),
        ("baseFeePerGas", true),
        ("gasUsedRatio", false),
        ("baseFeePerBlobGas", true),
        ("blobGasUsedRatio", false),
    ] {
        if let Some(v) = list(k, next_one) {
            out.insert(k.into(), v);
        }
    }
    Value::Object(out)
}

/// Another node's gas-used ratios that are ours to 12 significant digits, written as ours: nodes
/// divide with their own float steps (items-wasm's rule).
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_history_arguments_parse_as_written() {
        assert_eq!(
            parse_fee_history("4,latest,25,75"),
            Ok(FeeHistoryArgs {
                count: 4,
                newest: "latest".into(),
                percentiles: vec![25.0, 75.0]
            })
        );
        assert_eq!(
            parse_fee_history("0x10, 0x192b3d7").unwrap(),
            FeeHistoryArgs {
                count: 16,
                newest: "0x192b3d7".into(),
                percentiles: vec![]
            }
        );
        assert!(parse_fee_history("0,latest").is_err());
        assert!(parse_fee_history("4").is_err());
        assert!(parse_fee_history("4,latest,x").is_err());
    }

    #[test]
    fn parts_join_oldest_first_with_one_next_base_fee() {
        let a = json!({"oldestBlock": "0x1", "baseFeePerGas": ["0x1", "0x2", "0x3"], "gasUsedRatio": [0.1, 0.2],
            "reward": [["0x1"], ["0x2"]]});
        let b = json!({"oldestBlock": "0x3", "baseFeePerGas": ["0x3", "0x4"], "gasUsedRatio": [0.3],
            "reward": [["0x3"]]});
        assert_eq!(
            join(&[a, b]),
            json!({"oldestBlock": "0x1", "baseFeePerGas": ["0x1", "0x2", "0x3", "0x4"],
                "gasUsedRatio": [0.1, 0.2, 0.3], "reward": [["0x1"], ["0x2"], ["0x3"]]})
        );
    }
}
