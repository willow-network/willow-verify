//! willow-logs-verify: check a Willow RPC response on your own machine, trusting no node. Logs
//! (`eth_getLogs`) by their SP1 and OpenVM proofs; transactions, receipts, blocks and fees by
//! Merkle-Patricia proofs; accounts, storage, `eth_call` and `eth_estimateGas` by state proofs
//! and the EVM run here; traces by the block (or the call) run again here. Every proven block is then anchored by the light client of its chain's
//! L1 (see [`anchor`]), so the block hash itself is checked too. `--help` lists every mode.
//!
//! Each proof is SP1 or OpenVM, told by its first bytes, and is checked only under its own
//! system's pinned keys. SP1 proofs are verified here; OpenVM proofs by
//! `willow-logs-verify-openvm` (OpenVM's verifier cannot be linked with SP1's), run as a helper:
//! put it next to this binary or pass `--openvm-verifier <path>`. Without it, a response with
//! OpenVM parts fails. A chain moves from SP1 to OpenVM at its cutover: where that is known
//! (pinned per chain once set, or `--openvm-from <block>`), a part ending before it must be SP1
//! and one ending at or after it OpenVM; otherwise the response may switch once, SP1 to OpenVM,
//! and the report's `openvmFrom` says where.
//!
//! Prints a JSON report; exits 1 if anything fails to verify, differs, or is not anchored.

use logset_query::{compare, verify, BySystem, LogFilter, LogsProof, ProofPart, ProofSystem};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use willow_logs_verify::{parse_vkey, trust_with, OvmProcess, Sp1Check};

mod anchor;
mod check;
mod items;
mod rpc;
mod state;

use anchor::{Anchoring, Level};
use check::{chain_name, AnchorBy, Ctx};

const USAGE: &str = "\
willow-logs-verify: check a Willow RPC response on your own machine.

  willow-logs-verify --rpc <Willow RPC URL> <mode> [options]

Modes (one):
  --filter '<eth_getLogs filter JSON>'     logs, by their SP1 and OpenVM proofs
  --proof <file> --proofs-dir <dir> --filter '<JSON>'
                                           a saved willow_getLogsProof result, offline
  --tx <hash | block:index>                a transaction
  --receipt <hash | block:index>           a receipt
  --block-receipts <block>                 a block's receipts
  --block <block> [--full]                 a block (eth_getBlockByNumber; --full: every transaction)
  --state <address> [--slots k1,k2] [--code] --at <block>
                                           balance, nonce, code, storage, eth_getProof
  --call '<call JSON>' --at <block>        eth_call, run here over proven state (Ethereum,
                                           Unichain, Robinhood Chain)
  --estimate '<call JSON>' --at <block>    eth_estimateGas, searched here over proven state
  --trace '[<method>, <params>]'           a trace method's response, the block (or the call) run
                                           again here: debug_traceTransaction,
                                           debug_traceBlockByNumber, debug_traceBlockByHash,
                                           trace_block, trace_transaction, trace_replay* (Ethereum,
                                           Unichain); debug_traceCall, trace_call (Ethereum)
  --fees --at <block>                      eth_gasPrice, eth_maxPriorityFeePerGas, eth_blobBaseFee
                                           by the formula willow-fee-v1
  --fee-history <count>,<newest>[,<p1>,<p2>...]
                                           eth_feeHistory
A block is a number (decimal or 0x), a hash, latest, finalized (Ethereum) or earliest; --at
defaults to latest. With the light client, latest is the newest block it anchors.

Anchoring (every mode): the block hash a proof ends at is checked against the validators of
the chain's L1 (Ethereum, Sepolia for the testnets, BNB Smart Chain's own), by their light
client run here from a pinned start. Nothing is trusted but that start.
  --checkpoint <root>:<slot>               Ethereum's start (default: built in, else Willow's
                                           published one, confirmed by two beacon APIs)
  --sepolia-checkpoint <root>:<slot>       Sepolia's start
  --bnb-start <number>:<hash>              BNB Smart Chain's start, an epoch block
  --checkpoint-url <url>                   where newer starts are published
  --beacon-api <url>                       a beacon API that must confirm a fetched checkpoint
                                           (repeat; default publicnode and ChainSafe's lodestar)
  --cache-dir <dir> | --no-cache           what the light clients verified, kept and verified
                                           again next time (default ~/.cache/willow-verify)
  --min-level signed|committed|final       the lowest anchor accepted (default signed)
  --max-head-age <seconds>                 how old the newest block may be (default 600)
  --light-client-rpc <url>                 Ethereum's light-client data from here (default: --rpc
                                           with its chain replaced by 1)
  --anchor-rpc <url>                       instead: compare the block hash with a node you trust
  --no-anchor                              instead: leave the block hash unchecked (the report says so)
  --now <unix seconds>                     verify as of this time (saved responses)

Other options:
  --chain-id N                             the chain the response must be about (default: the
                                           chain in --rpc's /rpc/<chain> path, else 1)
  --compare-rpc <url>                      compare another endpoint's standard response with
                                           the proven one, field by field
  --overrides '<state overrides JSON>'     with --call / --estimate (a call's trace: in its params)
  --block-overrides '<JSON>'               with --call / --estimate
  --print | --print-logs                   include the proven response
  --segment-vkey <hex> --leaf-vkey <hex>   replace the pinned SP1 keys (logs)
  --openvm-from <block> --openvm-verifier <path>
                                           OpenVM cutover and helper (logs)

Prints a JSON report; exits 1 if anything fails to verify, differs, or is not anchored.
";

fn main() {
    match run() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!(
                "willow-logs-verify: not verified: see the report (a proof failed, the block is \
                 not anchored, or another endpoint's response differs from the proven one)"
            );
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("willow-logs-verify: {e}");
            std::process::exit(1);
        }
    }
}

/// The mode asked for.
enum Mode {
    Logs,
    Items(items::Ask),
    State(state::Ask),
}

struct Args {
    rpc: String,
    proof_file: Option<String>,
    proofs_dir: Option<String>,
    filter: Value,
    chain_id: Option<u64>,
    compare_rpc: Option<String>,
    anchor_rpc: Option<String>,
    no_anchor: bool,
    print: bool,
    segment_vkeys: Vec<[u8; 32]>,
    leaf_vkeys: Vec<[u8; 32]>,
    openvm_from: Option<u64>,
    openvm_verifier: Option<PathBuf>,
    light_client_rpc: Option<String>,
    anchor: anchor::Options,
    beacon_apis: Vec<String>,
    modes: Vec<&'static str>,
    tx: Option<String>,
    receipt: Option<String>,
    block_receipts: Option<String>,
    block: Option<String>,
    full: bool,
    state: Option<String>,
    slots: Option<Vec<String>>,
    code: bool,
    call: Option<Value>,
    estimate: Option<Value>,
    overrides: Option<Value>,
    block_overrides: Option<Value>,
    trace: Option<(String, Value)>,
    fees: bool,
    fee_history: Option<items::FeeHistoryArgs>,
    at: Option<String>,
}

fn json_arg(flag: &str, s: &str) -> Result<Value, String> {
    serde_json::from_str(s).map_err(|e| format!("{flag}: {e}"))
}

fn args() -> Result<(Args, Mode), String> {
    let mut a = Args {
        rpc: String::new(),
        proof_file: None,
        proofs_dir: None,
        filter: Value::Null,
        chain_id: None,
        compare_rpc: None,
        anchor_rpc: None,
        no_anchor: false,
        print: false,
        segment_vkeys: vec![],
        leaf_vkeys: vec![],
        openvm_from: None,
        openvm_verifier: None,
        light_client_rpc: None,
        anchor: anchor::Options::default(),
        beacon_apis: vec![],
        modes: vec![],
        tx: None,
        receipt: None,
        block_receipts: None,
        block: None,
        full: false,
        state: None,
        slots: None,
        code: false,
        call: None,
        estimate: None,
        overrides: None,
        block_overrides: None,
        trace: None,
        fees: false,
        fee_history: None,
        at: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(flag) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{flag} needs a value"));
        match flag.as_str() {
            "--help" | "-h" => {
                print!("{USAGE}");
                std::process::exit(0);
            }
            "--rpc" => a.rpc = val()?,
            "--proof" => a.proof_file = Some(val()?),
            "--proofs-dir" => a.proofs_dir = Some(val()?),
            "--filter" => {
                a.filter = json_arg("--filter", &val()?)?;
                a.modes.push("--filter");
            }
            "--chain-id" => {
                a.chain_id = Some(val()?.parse().map_err(|e| format!("--chain-id: {e}"))?)
            }
            "--compare-rpc" => a.compare_rpc = Some(val()?),
            "--anchor-rpc" => a.anchor_rpc = Some(val()?),
            "--no-anchor" => a.no_anchor = true,
            "--segment-vkey" => a.segment_vkeys.push(parse_vkey(&val()?)?),
            "--leaf-vkey" => a.leaf_vkeys.push(parse_vkey(&val()?)?),
            "--openvm-from" => {
                let v = val()?;
                a.openvm_from =
                    Some(check::parse_number(&v).ok_or_else(|| format!("--openvm-from: {v:?}"))?)
            }
            "--openvm-verifier" => a.openvm_verifier = Some(val()?.into()),
            "--print-logs" | "--print" => a.print = true,
            "--tx" => {
                a.tx = Some(val()?);
                a.modes.push("--tx");
            }
            "--receipt" => {
                a.receipt = Some(val()?);
                a.modes.push("--receipt");
            }
            "--block-receipts" => {
                a.block_receipts = Some(val()?);
                a.modes.push("--block-receipts");
            }
            "--block" => {
                a.block = Some(val()?);
                a.modes.push("--block");
            }
            "--full" => a.full = true,
            "--state" => {
                a.state = Some(val()?);
                a.modes.push("--state");
            }
            "--slots" => a.slots = Some(state::parse_slots(&val()?)),
            "--code" => a.code = true,
            "--call" => {
                a.call = Some(json_arg("--call", &val()?)?);
                a.modes.push("--call");
            }
            "--estimate" => {
                a.estimate = Some(json_arg("--estimate", &val()?)?);
                a.modes.push("--estimate");
            }
            "--overrides" => a.overrides = Some(json_arg("--overrides", &val()?)?),
            "--block-overrides" => {
                a.block_overrides = Some(json_arg("--block-overrides", &val()?)?)
            }
            "--fees" => {
                a.fees = true;
                a.modes.push("--fees");
            }
            "--fee-history" => {
                a.fee_history = Some(items::parse_fee_history(&val()?)?);
                a.modes.push("--fee-history");
            }
            "--at" => a.at = Some(val()?),
            "--trace" => {
                a.trace = Some(state::parse_trace(json_arg("--trace", &val()?)?)?);
                a.modes.push("--trace");
            }
            "--checkpoint" => a.anchor.checkpoint = Some(anchor::parse_checkpoint(&val()?)?),
            "--sepolia-checkpoint" => a.anchor.sepolia = Some(anchor::parse_checkpoint(&val()?)?),
            "--bnb-start" => a.anchor.bnb = Some(anchor::parse_bnb_start(&val()?)?),
            "--checkpoint-url" => a.anchor.checkpoint_url = val()?,
            "--beacon-api" => a.beacon_apis.push(val()?),
            "--cache-dir" => a.anchor.cache_dir = Some(val()?.into()),
            "--no-cache" => a.anchor.cache_dir = None,
            "--min-level" => {
                let v = val()?;
                a.anchor.min_level = Level::parse(&v)
                    .ok_or_else(|| format!("--min-level {v:?}: signed, committed or final"))?
            }
            "--max-head-age" => {
                a.anchor.max_head_age =
                    val()?.parse().map_err(|e| format!("--max-head-age: {e}"))?
            }
            "--light-client-rpc" => a.light_client_rpc = Some(val()?),
            "--now" => a.anchor.now = Some(val()?.parse().map_err(|e| format!("--now: {e}"))?),
            other => return Err(format!("unknown argument {other} (see --help)")),
        }
    }
    if !a.beacon_apis.is_empty() {
        a.anchor.beacon_apis = std::mem::take(&mut a.beacon_apis);
    }
    if a.anchor_rpc.is_some() && a.no_anchor {
        return Err("--anchor-rpc and --no-anchor: choose one".into());
    }
    let mode = match a.modes.as_slice() {
        [] => return Err(format!("no mode given\n\n{USAGE}")),
        [_] => a.modes[0],
        many => return Err(format!("one mode at a time, not {}", many.join(" and "))),
    };
    if a.full && mode != "--block" {
        return Err("--full goes with --block".into());
    }
    if (a.slots.is_some() || a.code) && mode != "--state" {
        return Err("--slots and --code go with --state".into());
    }
    if (a.overrides.is_some() || a.block_overrides.is_some())
        && mode != "--call"
        && mode != "--estimate"
    {
        return Err("--overrides and --block-overrides go with --call or --estimate".into());
    }
    let at_modes = ["--state", "--call", "--estimate", "--fees"];
    if a.at.is_some() && !at_modes.contains(&mode) {
        return Err(format!("--at goes with {}", at_modes.join(", ")));
    }
    if mode == "--filter" {
        let offline = a.proof_file.is_some() && a.proofs_dir.is_some();
        if (a.rpc.is_empty() && !offline) || !a.filter.is_object() {
            return Err("usage: willow-logs-verify --rpc <url> --filter '<json>' [options]".into());
        }
        return Ok((a, Mode::Logs));
    }
    if a.rpc.is_empty() || a.proof_file.is_some() || a.proofs_dir.is_some() {
        return Err(format!(
            "usage: willow-logs-verify --rpc <url> {mode} <...> (see --help)"
        ));
    }
    let at = a.at.clone().unwrap_or_else(|| "latest".into());
    // A call's overrides: its third and fourth params.
    let overrides = match (&a.overrides, &a.block_overrides) {
        (None, None) => None,
        (s, b) => Some(json!([s, b])),
    };
    let m = match mode {
        "--tx" => Mode::Items(items::Ask::Tx(a.tx.clone().expect("given"))),
        "--receipt" => Mode::Items(items::Ask::Receipt(a.receipt.clone().expect("given"))),
        "--block-receipts" => Mode::Items(items::Ask::BlockReceipts(
            a.block_receipts.clone().expect("given"),
        )),
        "--block" => Mode::Items(items::Ask::Block {
            block: a.block.clone().expect("given"),
            full: a.full,
        }),
        "--fees" => Mode::Items(items::Ask::Fees { at }),
        "--fee-history" => Mode::Items(items::Ask::FeeHistory(
            a.fee_history.clone().expect("given"),
        )),
        "--state" => Mode::State(state::Ask::State {
            address: a.state.clone().expect("given"),
            slots: a.slots.clone().unwrap_or_default(),
            code: a.code,
            at,
        }),
        "--call" => Mode::State(state::Ask::Call {
            call: a.call.clone().expect("given"),
            overrides,
            at,
        }),
        "--trace" => {
            let (method, params) = a.trace.clone().expect("given");
            Mode::State(state::Ask::Trace { method, params })
        }
        _ => Mode::State(state::Ask::Estimate {
            call: a.estimate.clone().expect("given"),
            overrides,
            at,
        }),
    };
    Ok((a, m))
}

fn run() -> Result<bool, String> {
    let (a, mode) = args()?;
    let http = rpc::client()?;
    let chain_id = a.chain_id.or_else(|| rpc::url_chain(&a.rpc)).unwrap_or(1);
    let transport = rpc::Http {
        http: http.clone(),
        url: a.rpc.clone(),
        chain_id,
        ethereum: a.light_client_rpc.clone(),
    };
    let anchor = if let Some(u) = &a.anchor_rpc {
        AnchorBy::Rpc(u.clone())
    } else if a.no_anchor {
        AnchorBy::None("--no-anchor: the block hash was not checked")
    } else if a.rpc.is_empty() {
        AnchorBy::None("offline: pass --rpc to anchor the last proven block")
    } else {
        AnchorBy::LightClient(Box::new(Anchoring::new(
            &transport,
            chain_id,
            a.anchor.clone(),
            Some(&http),
        )?))
    };
    let mut ctx = Ctx {
        http: &http,
        t: &transport,
        chain_id,
        anchor,
        compare_rpc: a.compare_rpc.clone(),
        print: a.print,
        head: None,
        head_block: None,
    };
    let (report, ok) = match mode {
        Mode::Logs => logs(&a, &mut ctx)?,
        Mode::Items(ask) => items::run(&mut ctx, ask)?,
        Mode::State(ask) => state::run(&mut ctx, ask)?,
    };
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    Ok(ok)
}

/// `--filter`: a log response by its proofs, the last proven block anchored.
fn logs(a: &Args, ctx: &mut Ctx) -> Result<(Value, bool), String> {
    let chain_id = ctx.chain_id;
    let raw = match &a.proof_file {
        Some(path) => {
            serde_json::from_slice(&std::fs::read(path).map_err(|e| format!("{path}: {e}"))?)
                .map_err(|e| format!("{path}: {e}"))?
        }
        None => rpc::call(ctx.http, &a.rpc, "willow_getLogsProof", json!([a.filter]))?,
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
                    "the server's response has {key} 0x{got:x}, the filter asks for 0x{want}"
                ));
            }
        }
    }
    let f = LogFilter::from_eth_filter(&a.filter, proof.from_block, proof.to_block)?;
    let mut trust = trust_with(chain_id, a.segment_vkeys.clone(), a.leaf_vkeys.clone())?;
    if a.openvm_from.is_some() {
        trust.openvm_from = a.openvm_from;
    }
    let mut cache: HashMap<String, Vec<u8>> = HashMap::new();
    let http = ctx.http;
    let mut fetch = |id: &str| -> Result<Vec<u8>, String> {
        if let Some(b) = cache.get(id) {
            return Ok(b.clone());
        }
        if let Some(dir) = &a.proofs_dir {
            // Strict parse first: an id from the response never escapes `dir`.
            let id: logset_query::ProofId = id.parse()?;
            let path = std::path::Path::new(dir).join(format!("{id}.proof"));
            return std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display()));
        }
        let r = rpc::call(http, &a.rpc, "willow_getProof", json!([id]))?;
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
    let last_hash = format!("0x{}", hex::encode(last_hash));
    let mut report = json!({
        "verified": true,
        "method": "willow_getLogsProof",
        "chainId": chain_id,
        "chain": chain_name(chain_id),
        "fromBlock": format!("0x{:x}", f.from_block),
        "toBlock": format!("0x{:x}", f.to_block),
        "logs": verified.logs.len(),
        "parts": {"segments": segments, "blocks": blocks, "leaves": leaves},
        "proofs": {"sp1": count(ProofSystem::Sp1), "openvm": count(ProofSystem::OpenVm)},
        "openvmFrom": hexn(verified.openvm_from),
        "cutover": match trust.openvm_from {
            Some(n) => json!({"openvmFrom": format!("0x{n:x}"), "checked": true}),
            None => json!({"checked": false, "note": "this chain's first OpenVM block is not pinned: the response may switch from SP1 to OpenVM once, at openvmFrom"}),
        },
        "block": {"number": format!("0x{last:x}"), "hash": last_hash.clone()},
        "verifySeconds": t.elapsed().as_secs_f64(),
    });
    // The proofs chain every block's hash to the last one: anchoring it anchors them all.
    let mut ok = ctx.anchor_into(&mut report, last, &last_hash);
    report["lastProvenBlock"] = report["block"].clone();
    if let Some(url) = &a.compare_rpc {
        let mut q = a.filter.clone();
        q["fromBlock"] = json!(format!("0x{:x}", f.from_block));
        q["toBlock"] = json!(format!("0x{:x}", f.to_block));
        let theirs = rpc::call(ctx.http, url, "eth_getLogs", json!([q]));
        report["compare"] = match theirs.as_ref().map(Value::as_array) {
            Ok(Some(theirs)) => {
                let cmp = compare(&verified.logs, theirs);
                ok &= cmp.is_exact();
                json!({
                    "exact": cmp.is_exact(),
                    "matched": cmp.matched,
                    "missing": cmp.missing.iter().map(log_json).collect::<Vec<_>>(),
                    "unproven": cmp.unproven,
                })
            }
            Ok(None) => check::problem(json!("eth_getLogs: not a list")),
            Err(e) => check::problem(json!(e)),
        };
    }
    if a.print {
        report["provenLogs"] = verified.logs.iter().map(log_json).collect();
    }
    report["verified"] = json!(ok);
    Ok((report, ok))
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
