//! willow-logs-verify — check that an `eth_getLogs` answer is complete and
//! correct, or that a transaction, a receipt or a block's receipts are the
//! block's.
//!
//! ```sh
//! willow-logs-verify --rpc <willow rpc url> --filter '<eth_getLogs filter json>'
//!     [--chain-id N] [--compare-rpc <url>] [--anchor-rpc <url>] [--print-logs]
//!     [--segment-vkey <hex>]... [--leaf-vkey <hex>]...
//!     [--openvm-from <block>] [--openvm-verifier <path>]
//! willow-logs-verify --proof <saved willow_getLogsProof result> --proofs-dir <dir>
//!     --filter '<json>' [same options]
//! willow-logs-verify --rpc <willow rpc url>
//!     (--tx <hash | block:index> | --receipt <hash | block:index> | --block-receipts <block>)
//!     [--chain-id N] [--compare-rpc <url>] [--anchor-rpc <url>] [--print]
//! ```
//! A block is a number (decimal or 0x), a block hash, or `latest`. The third
//! form checks a Merkle-Patricia proof against the block hash ([`items`]).
//! The second form checks an archived answer offline, reading each proof
//! from `<dir>/<id>.proof` (the segment tier's store layout).
//!
//! Each proof is SP1 or OpenVM, told by its first bytes, and is checked only
//! under its own system's pinned keys. SP1 proofs are verified here; OpenVM
//! proofs by `willow-logs-verify-openvm` (OpenVM's verifier cannot be linked
//! with SP1's), run as a helper: put it next to this binary or pass
//! `--openvm-verifier <path>`. Without it, an answer with OpenVM parts fails.
//! `--segment-vkey`/`--leaf-vkey` replace the SP1 keys only.
//!
//! A chain moves from SP1 to OpenVM at its cutover. Where that is known (pinned
//! per chain once set, or `--openvm-from <block>`: the chain's first OpenVM
//! block), a part ending before it must be SP1 and one ending at or after it
//! OpenVM. Otherwise the answer may switch once, SP1 to OpenVM, and the report's
//! `openvmFrom` says where.
//!
//! `--chain-id` is the chain the answer must be about: Ethereum 1, Base 8453,
//! OP 10, Unichain 130, Ink 57073, Arbitrum One 42161, Robinhood 4663 have
//! pinned program keys (the same pair per system for all). Without it, the chain in the
//! `--rpc` URL's `/rpc/<chain>` path is used, else 1. Never the server's answer.
//! 1. Asks the Willow RPC for `willow_getLogsProof(filter)` and each proof it
//!    names (`willow_getProof`).
//! 2. Verifies every proof against its system's pinned program keys, every range
//!    proof, and the hash links between parts. The result is every log the
//!    filter matches, proven complete.
//! 3. `--compare-rpc`: fetches that provider's `eth_getLogs` answer for the same
//!    blocks and reports logs it is missing or reports differently.
//! 4. `--anchor-rpc`: confirms the last proven block hash is canonical there.
//!
//! Prints a JSON report; exits 1 if anything fails to verify or differs.

use logset_query::{compare, verify, BySystem, LogFilter, LogsProof, ProofPart, ProofSystem};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use willow_logs_verify::{parse_vkey, pinned_chain, trust_with, OvmProcess, Sp1Check};

mod items;

fn main() {
    if let Err(e) = run() {
        eprintln!("willow-logs-verify: {e}");
        std::process::exit(1);
    }
}

struct Args {
    rpc: String,
    proof_file: Option<String>,
    proofs_dir: Option<String>,
    filter: Value,
    chain_id: Option<u64>,
    compare_rpc: Option<String>,
    anchor_rpc: Option<String>,
    print_logs: bool,
    segment_vkeys: Vec<[u8; 32]>,
    leaf_vkeys: Vec<[u8; 32]>,
    openvm_from: Option<u64>,
    openvm_verifier: Option<PathBuf>,
    items: Option<items::Ask>,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        rpc: String::new(),
        proof_file: None,
        proofs_dir: None,
        filter: Value::Null,
        chain_id: None,
        compare_rpc: None,
        anchor_rpc: None,
        print_logs: false,
        segment_vkeys: vec![],
        leaf_vkeys: vec![],
        openvm_from: None,
        openvm_verifier: None,
        items: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--rpc" => a.rpc = val()?,
            "--proof" => a.proof_file = Some(val()?),
            "--proofs-dir" => a.proofs_dir = Some(val()?),
            "--filter" => {
                a.filter = serde_json::from_str(&val()?).map_err(|e| format!("--filter: {e}"))?
            }
            "--chain-id" => {
                a.chain_id = Some(val()?.parse().map_err(|e| format!("--chain-id: {e}"))?)
            }
            "--compare-rpc" => a.compare_rpc = Some(val()?),
            "--anchor-rpc" => a.anchor_rpc = Some(val()?),
            "--segment-vkey" => a.segment_vkeys.push(parse_vkey(&val()?)?),
            "--leaf-vkey" => a.leaf_vkeys.push(parse_vkey(&val()?)?),
            "--openvm-from" => {
                let v = val()?;
                let n = match v.strip_prefix("0x") {
                    Some(h) => u64::from_str_radix(h, 16),
                    None => v.parse(),
                };
                a.openvm_from = Some(n.map_err(|e| format!("--openvm-from: {e}"))?)
            }
            "--openvm-verifier" => a.openvm_verifier = Some(val()?.into()),
            "--print-logs" | "--print" => a.print_logs = true,
            "--tx" => a.items = Some(items::Ask::Tx(items::parse_item(&val()?)?)),
            "--receipt" => a.items = Some(items::Ask::Receipt(items::parse_item(&val()?)?)),
            "--block-receipts" => {
                a.items = Some(items::Ask::BlockReceipts(items::parse_block(&val()?)?))
            }
            other => return Err(format!("unknown argument {other} (see the module docs)")),
        }
    }
    if a.items.is_some() {
        if a.rpc.is_empty() || !a.filter.is_null() || a.proof_file.is_some() {
            return Err(
                "usage: willow-logs-verify --rpc <url> (--tx | --receipt | --block-receipts) <...>"
                    .into(),
            );
        }
        return Ok(a);
    }
    let offline = a.proof_file.is_some() && a.proofs_dir.is_some();
    if (a.rpc.is_empty() && !offline) || !a.filter.is_object() {
        return Err("usage: willow-logs-verify --rpc <url> --filter '<json>' [options]".into());
    }
    Ok(a)
}

/// The chain in a log RPC URL's `/rpc/<chain>[/<key>]` path.
fn url_chain(url: &str) -> Option<u64> {
    let path = url.split_once("://").map_or(url, |(_, r)| r);
    let mut it = path.split(['/', '?', '#']);
    while let Some(seg) = it.next() {
        if seg == "rpc" {
            return it.next()?.parse().ok();
        }
    }
    None
}

fn call(
    http: &reqwest::blocking::Client,
    url: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let body = json!({"jsonrpc": "2.0", "id": 1, "method": method, "params": params});
    let out: Value = http
        .post(url)
        .json(&body)
        .send()
        .and_then(|r| r.json())
        .map_err(|e| format!("{method}: {}", e.without_url()))?;
    if let Some(err) = out.get("error") {
        return Err(format!("{method}: {err}"));
    }
    out.get("result")
        .cloned()
        .ok_or_else(|| format!("{method}: no result"))
}

fn run() -> Result<(), String> {
    let a = args()?;
    let http = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(ask) = a.items {
        let chain_id = a.chain_id.or_else(|| url_chain(&a.rpc)).unwrap_or(1);
        let (report, ok) = items::run(
            &http,
            &a.rpc,
            chain_id,
            ask,
            a.compare_rpc.as_deref(),
            a.anchor_rpc.as_deref(),
            a.print_logs,
        )?;
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
        return if ok {
            Ok(())
        } else {
            Err("the answer differs from the proven one, or the block is not canonical".into())
        };
    }
    let raw = match &a.proof_file {
        Some(path) => {
            serde_json::from_slice(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?)
                .map_err(|e| format!("{path}: {e}"))?
        }
        None => call(&http, &a.rpc, "willow_getLogsProof", json!([a.filter]))?,
    };
    let proof: LogsProof =
        serde_json::from_value(raw).map_err(|e| format!("willow_getLogsProof: {e}"))?;
    // Tags were resolved by the server; explicit numbers must match.
    for (key, got) in [("fromBlock", proof.from_block), ("toBlock", proof.to_block)] {
        if let Some(want) = a
            .filter
            .get(key)
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("0x"))
        {
            if u64::from_str_radix(want, 16).ok() != Some(got) {
                return Err(format!(
                    "server answered {key} 0x{got:x}, filter asks 0x{want}"
                ));
            }
        }
    }
    let f = LogFilter::from_eth_filter(&a.filter, proof.from_block, proof.to_block)?;
    let chain_id = a.chain_id.or_else(|| url_chain(&a.rpc)).unwrap_or(1);
    let mut trust = trust_with(chain_id, a.segment_vkeys.clone(), a.leaf_vkeys.clone())?;
    if a.openvm_from.is_some() {
        trust.openvm_from = a.openvm_from;
    }
    let mut cache: HashMap<String, Vec<u8>> = HashMap::new();
    let mut fetch = |id: &str| -> Result<Vec<u8>, String> {
        if let Some(b) = cache.get(id) {
            return Ok(b.clone());
        }
        if let Some(dir) = &a.proofs_dir {
            // Strict parse first: an id from the answer never escapes `dir`.
            let id: logset_query::ProofId = id.parse()?;
            let path = std::path::Path::new(dir).join(format!("{id}.proof"));
            return std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()));
        }
        let r = call(&http, &a.rpc, "willow_getProof", json!([id]))?;
        let hexed = r
            .get("proof")
            .and_then(Value::as_str)
            .ok_or("willow_getProof: no proof")?;
        let bytes = hex::decode(hexed.trim_start_matches("0x")).map_err(|e| e.to_string())?;
        cache.insert(id.to_string(), bytes.clone());
        Ok(bytes)
    };
    let t = std::time::Instant::now();
    let mut check = BySystem {
        sp1: Sp1Check::default(),
        openvm: OvmProcess::new(a.openvm_verifier.clone()),
    };
    let verified = verify(&proof, &f, &trust, &mut fetch, &mut check).map_err(|e| e.to_string())?;
    let count = |s| verified.systems.iter().filter(|&&x| x == s).count();
    let hexn = |n: Option<u64>| n.map(|n| format!("0x{n:x}"));
    let (segments, blocks, leaves) = proof.parts.iter().fold((0, 0, 0), |(s, b, l), p| match p {
        ProofPart::Segment(_) => (s + 1, b, l),
        ProofPart::Block(_) => (s, b + 1, l),
        ProofPart::Leaf(_) => (s, b, l + 1),
    });
    let (last, last_hash) = *verified.checkpoints.last().expect("non-empty");
    let mut report = json!({
        "verified": true,
        "chainId": chain_id,
        "chain": pinned_chain(chain_id),
        "fromBlock": format!("0x{:x}", f.from_block),
        "toBlock": format!("0x{:x}", f.to_block),
        "logs": verified.logs.len(),
        "parts": {"segments": segments, "blocks": blocks, "leaves": leaves},
        "proofs": {"sp1": count(ProofSystem::Sp1), "openvm": count(ProofSystem::OpenVm)},
        "openvmFrom": hexn(verified.openvm_from),
        "cutover": match trust.openvm_from {
            Some(n) => json!({"openvmFrom": format!("0x{n:x}"), "checked": true}),
            None => json!({"checked": false, "note": "this chain's first OpenVM block is not pinned: the answer may switch from SP1 to OpenVM once, at openvmFrom"}),
        },
        "lastProvenBlock": {"number": format!("0x{last:x}"), "hash": format!("0x{}", hex::encode(last_hash))},
        "verifySeconds": t.elapsed().as_secs_f64(),
    });
    let mut ok = true;
    report["lastProvenBlock"]["canonical"] =
        json!("not checked: pass --anchor-rpc <a node you trust>");
    if let Some(url) = &a.anchor_rpc {
        let header = call(
            &http,
            url,
            "eth_getBlockByNumber",
            json!([format!("0x{last:x}"), false]),
        )?;
        let canonical = header.get("hash").and_then(Value::as_str)
            == Some(&format!("0x{}", hex::encode(last_hash)));
        ok &= canonical;
        report["lastProvenBlock"]["canonical"] = json!(canonical);
    }
    if let Some(url) = &a.compare_rpc {
        let mut q = a.filter.clone();
        q["fromBlock"] = json!(format!("0x{:x}", f.from_block));
        q["toBlock"] = json!(format!("0x{:x}", f.to_block));
        let answer = call(&http, url, "eth_getLogs", json!([q]))?;
        let answer = answer.as_array().ok_or("eth_getLogs: not an array")?;
        let cmp = compare(&verified.logs, answer);
        ok &= cmp.is_exact();
        report["compare"] = json!({
            "matched": cmp.matched,
            "missing": cmp.missing.iter().map(log_json).collect::<Vec<_>>(),
            "unproven": cmp.unproven,
        });
    }
    if a.print_logs {
        report["provenLogs"] = verified.logs.iter().map(log_json).collect();
    }
    report["verified"] = json!(ok);
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if ok {
        Ok(())
    } else {
        Err("the answer differs from the proven logs, or the last block is not canonical".into())
    }
}

fn log_json(l: &logset_query::Log) -> Value {
    json!({
        "address": format!("0x{}", hex::encode(l.address)),
        "topics": l.topics.iter().map(|t| format!("0x{}", hex::encode(t))).collect::<Vec<_>>(),
        "data": format!("0x{}", hex::encode(&l.data)),
        "blockNumber": format!("0x{:x}", l.block),
        "transactionIndex": format!("0x{:x}", l.tx_index),
        "logIndex": format!("0x{:x}", l.log_index),
    })
}

#[cfg(test)]
mod tests {
    use super::url_chain;

    #[test]
    fn the_chain_comes_from_the_rpc_path_when_not_given() {
        assert_eq!(
            url_chain("https://api.willow.tech/v1/rpc/8453/k3y"),
            Some(8453)
        );
        assert_eq!(url_chain("http://127.0.0.1:8545/rpc/4663"), Some(4663));
        assert_eq!(url_chain("https://api.willow.tech/v1/rpc/health"), None);
        assert_eq!(url_chain("https://eth.example/"), None);
        assert_eq!(url_chain(""), None);
    }
}
