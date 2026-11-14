//! The block and fee fixtures' generator, and the runs against public-node
//! data that are too large to commit. Every test here is `#[ignore]`d and
//! reads a directory that `fixtures/fetch.py` filled:
//!
//! - `make_fixtures` (`WILLOW_RAW` = fetch.py's `fixtures/` directory) writes
//!   `fixtures/block/` and `fixtures/fees/`;
//! - the `live_*` runs (`WILLOW_LIVE` = its `live/` directory) print their
//!   results and fail on any difference that is not a recorded one.

use crate::fees::{
    build_fee_proof, next_base_fee, verify_fee_history, verify_fees, FeeHistoryRequest,
};
use crate::proof::{build_block, build_block_proof, decode_header, Block, BlockProof};
use alloy::primitives::U256;
use logset_blockcheck::receipts::values_and_logs_from_rpc;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub(crate) const CRATE: &str = env!("CARGO_MANIFEST_DIR");

pub(crate) fn read(path: impl AsRef<Path>) -> Value {
    let p = path.as_ref();
    serde_json::from_slice(&std::fs::read(p).unwrap_or_else(|e| panic!("{}: {e}", p.display())))
        .unwrap_or_else(|e| panic!("{}: {e}", p.display()))
}

fn write(path: impl AsRef<Path>, v: &Value) {
    let mut s = serde_json::to_string_pretty(v).unwrap();
    s.push('\n');
    std::fs::write(path, s).unwrap();
}

fn env_dir(var: &str) -> PathBuf {
    PathBuf::from(std::env::var(var).unwrap_or_else(|_| panic!("set {var}")))
}

/// A block's consensus parts, from a node's `eth_getBlockByNumber(n, true)`,
/// its receipts and its uncle headers, each verified against the block hash.
pub(crate) struct Parts {
    pub chain: u64,
    pub header: Vec<u8>,
    pub txs: Vec<Vec<u8>>,
    pub receipts: Option<Vec<Vec<u8>>>,
    pub withdrawals: Option<Vec<u8>>,
    pub uncles: Vec<Vec<u8>>,
}

impl Parts {
    pub fn new(chain: u64, full: &Value, receipts: Option<&Value>, uncles: &[Value]) -> Parts {
        let h = logset_blockcheck::verify_header(full).unwrap();
        let body = logset_blockcheck::verify_body_with_uncles(chain, &h, full, uncles).unwrap();
        let receipts = receipts.map(|r| {
            let (values, _) = values_and_logs_from_rpc(r, chain).unwrap();
            assert_eq!(
                logset_blockcheck::receipts::receipts_root(&values).unwrap(),
                h.info.receipts_root
            );
            values
        });
        Parts {
            chain,
            header: h.rlp,
            txs: body.tx_values,
            receipts,
            withdrawals: body.withdrawals.map(|w| alloy_rlp::encode(&w)),
            uncles: uncles
                .iter()
                .map(|u| logset_blockcheck::verify_header(u).unwrap().rlp)
                .collect(),
        }
    }

    fn refs(v: &[Vec<u8>]) -> Vec<&[u8]> {
        v.iter().map(Vec::as_slice).collect()
    }

    pub fn hash(&self) -> [u8; 32] {
        alloy_primitives::keccak256(&self.header).0
    }

    /// `willow_getBlockProof`; `with_uncles = false` as a server without the uncle headers.
    pub fn block_proof(&self, with_uncles: bool) -> BlockProof {
        let receipts = self.receipts.as_deref().map(Self::refs);
        let uncles = Self::refs(&self.uncles);
        build_block_proof(
            self.chain,
            &self.header,
            &Self::refs(&self.txs),
            receipts.as_deref(),
            self.withdrawals.as_deref(),
            with_uncles.then_some(uncles.as_slice()),
        )
        .unwrap()
    }

    /// `willow_getBlockReceiptsProof`.
    pub fn receipts_proof(&self) -> BlockProof {
        build_block(
            self.chain,
            &self.header,
            &Self::refs(&self.txs),
            &Self::refs(self.receipts.as_ref().unwrap()),
        )
        .unwrap()
    }
}

fn hash_param(h: [u8; 32]) -> Value {
    json!({"blockHash": format!("0x{}", hex::encode(h))})
}

fn chain_id(name: &str) -> u64 {
    match name {
        "ethereum" => 1,
        "op" => 10,
        "base" => 8453,
        "unichain" => 130,
        "ink" => 57073,
        "arbitrum-one" => 42161,
        "robinhood" => 4663,
        _ => panic!("{name}"),
    }
}

const CHAINS: [&str; 7] = [
    "ethereum",
    "op",
    "base",
    "unichain",
    "ink",
    "arbitrum-one",
    "robinhood",
];
const NITRO: [&str; 2] = ["arbitrum-one", "robinhood"];

/// Writes `fixtures/block/` and `fixtures/fees/` from fetch.py's raw data.
#[test]
#[ignore]
fn make_fixtures() {
    let raw = env_dir("WILLOW_RAW");
    let meta = read(raw.join("meta.json"));
    let blocks = PathBuf::from(CRATE).join("fixtures/block");
    let fee_dir = PathBuf::from(CRATE).join("fixtures/fees");
    std::fs::create_dir_all(&blocks).unwrap();
    std::fs::create_dir_all(&fee_dir).unwrap();
    let node = |name: &str| {
        let m = &meta[name];
        json!({"endpoint": m["endpoint"], "client": m["client"]})
    };
    let block_parts = |name: &str, n: u64, receipts: bool| -> (Value, Parts) {
        let full = read(raw.join(format!("{name}-{n}.block.json")));
        let r = receipts.then(|| read(raw.join(format!("{name}-{n}.receipts.json"))));
        let uncles_path = raw.join(format!("{name}-{n}.uncles.json"));
        let uncles: Vec<Value> = if uncles_path.exists() {
            serde_json::from_value(read(uncles_path)).unwrap()
        } else {
            vec![]
        };
        let p = Parts::new(chain_id(name), &full, r.as_ref(), &uncles);
        (full, p)
    };

    // I1: block proofs.
    let heads: Vec<(String, Value)> = CHAINS
        .iter()
        .map(|c| (c.to_string(), read(raw.join(format!("{c}-head.json")))))
        .collect();
    let mut wanted: Vec<(String, u64, bool, &str)> = vec![
        (
            "ethereum".into(),
            1_500_000,
            false,
            "Ethereum before the merge, with an uncle",
        ),
        (
            "ethereum".into(),
            17_034_874,
            false,
            "Ethereum after Shanghai: its first withdrawals",
        ),
    ];
    for (c, h) in &heads {
        let about = match c.as_str() {
            "ethereum" => "Ethereum after Fusaka: withdrawals, blob transactions, requestsHash",
            "op" => "OP Mainnet after Isthmus: withdrawalsRoot is the L2ToL1MessagePasser's storage root",
            "base" => "Base after Denim: timestampMs from its BaseTime deposit",
            "unichain" => "Unichain: a block of its L1 info deposit alone",
            "ink" => "Ink",
            _ => "Nitro: l1BlockNumber, sendRoot and sendCount from the header",
        };
        let n = h["fee"].as_u64().unwrap();
        wanted.push((c.clone(), n, c.as_str() != "ethereum", about));
    }
    for (c, n, receipts, about) in &wanted {
        let (full, p) = block_parts(c.as_str(), *n, *receipts && !NITRO.contains(&c.as_str()));
        for with_uncles in [true, false] {
            if !with_uncles && p.uncles.is_empty() {
                continue;
            }
            let proof = p.block_proof(with_uncles);
            let (name, about) = if with_uncles {
                (format!("{c}-{n}"), about.to_string())
            } else {
                (
                    format!("{c}-{n}-uncles-unproven"),
                    format!("{about}, from a server without its uncle headers"),
                )
            };
            let mut nd = node(c.as_str());
            nd["eth_getBlockByNumber"] = full.clone();
            write(
                blocks.join(format!("{name}.json")),
                &json!({
                    "chainId": p.chain,
                    "about": about,
                    "request": {"method": "willow_getBlockProof", "params": [hash_param(p.hash())]},
                    "result": proof,
                    "node": nd,
                }),
            );
        }
    }

    // I4: fee proofs at each chain's head (Ethereum: its four blocks).
    for (c, h) in &heads {
        let chain = chain_id(c);
        let n = h["fee"].as_u64().unwrap();
        let range: Vec<u64> = if c == "ethereum" {
            (n - 3..=n).collect()
        } else {
            vec![n]
        };
        let parts: Vec<Parts> = range
            .iter()
            .map(|&k| block_parts(c.as_str(), k, !NITRO.contains(&c.as_str())).1)
            .collect();
        let headers: Vec<&[u8]> = parts.iter().map(|p| p.header.as_slice()).collect();
        let bodies: Vec<BlockProof> = match c.as_str() {
            "ethereum" => parts.iter().map(|p| p.block_proof(true)).collect(),
            x if NITRO.contains(&x) => vec![],
            _ => vec![parts[0].receipts_proof()],
        };
        let proof = build_fee_proof(chain, &headers, bodies).unwrap();
        let last = parts.last().unwrap().hash();
        let fees = verify_fees(&proof, chain, Block::Hash(last)).unwrap();
        let mut nd = node(c.as_str());
        nd["head"] = h["head"].clone();
        nd["responses"] = h["responses"].clone();
        write(
            fee_dir.join(format!("{c}-{n}.fee.json")),
            &json!({
                "chainId": chain,
                "request": {"method": "willow_getFeeProof", "params": [hash_param(last)]},
                "result": proof,
                "verified": fees.json(),
                "node": nd,
            }),
        );
    }

    // I4: fee-history proofs, with rewards and without.
    for c in CHAINS {
        let chain = chain_id(c);
        let hist = read(raw.join(format!("{c}-history.json")));
        let r = &hist["request"];
        let (count, newest) = (r["count"].as_u64().unwrap(), r["newest"].as_u64().unwrap());
        let percentiles: Vec<f64> = serde_json::from_value(r["percentiles"].clone()).unwrap();
        let req = FeeHistoryRequest {
            block_count: count,
            newest: Block::Number(newest),
            reward_percentiles: percentiles.clone(),
        };
        let nitro = NITRO.contains(&c);
        let parts: Vec<Parts> = (newest + 1 - count..=newest)
            .map(|k| {
                if nitro {
                    let h = hist["headers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|x| crate::hex_u64(&x["number"]) == Some(k))
                        .unwrap()
                        .clone();
                    let (_, rlp) =
                        logset_blockcheck::header::header_rlp_from_block_json(&h).unwrap();
                    Parts {
                        chain,
                        header: rlp,
                        txs: vec![],
                        receipts: None,
                        withdrawals: None,
                        uncles: vec![],
                    }
                } else {
                    block_parts(c, k, true).1
                }
            })
            .collect();
        let headers: Vec<&[u8]> = parts.iter().map(|p| p.header.as_slice()).collect();
        let bodies = if nitro {
            vec![]
        } else {
            parts.iter().map(Parts::receipts_proof).collect()
        };
        let proof = build_fee_proof(chain, &headers, bodies).unwrap();
        let verified = verify_fee_history(&proof, chain, &req).unwrap();
        let mut nd = node(c);
        nd["eth_feeHistory"] = hist["eth_feeHistory"].clone();
        write(
            fee_dir.join(format!("{c}-{newest}-{count}.history.json")),
            &json!({
                "chainId": chain,
                "request": {"method": "willow_getFeeHistoryProof",
                            "params": crate::fees::fee_history_params(&req)},
                "result": proof,
                "verified": verified.json(),
                "node": nd,
            }),
        );
        // Without percentiles: 16 headers.
        let plain = &hist["plain"];
        let headers: Vec<Vec<u8>> = hist["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                logset_blockcheck::header::header_rlp_from_block_json(h)
                    .unwrap()
                    .1
            })
            .collect();
        let refs: Vec<&[u8]> = headers.iter().map(Vec::as_slice).collect();
        let proof = build_fee_proof(chain, &refs, vec![]).unwrap();
        let req = FeeHistoryRequest {
            block_count: plain["count"].as_u64().unwrap(),
            newest: Block::Number(plain["newest"].as_u64().unwrap()),
            reward_percentiles: vec![],
        };
        let verified = verify_fee_history(&proof, chain, &req).unwrap();
        let mut nd = node(c);
        nd["eth_feeHistory"] = plain["eth_feeHistory"].clone();
        write(
            fee_dir.join(format!(
                "{c}-{}-{}.history.json",
                req_newest(&req),
                req.block_count
            )),
            &json!({
                "chainId": chain,
                "request": {"method": "willow_getFeeHistoryProof",
                            "params": crate::fees::fee_history_params(&req)},
                "result": proof,
                "verified": verified.json(),
                "node": nd,
            }),
        );
    }
}

fn req_newest(r: &FeeHistoryRequest) -> u64 {
    match r.newest {
        Block::Number(n) => n,
        _ => panic!("by number"),
    }
}

fn lines(path: PathBuf) -> Vec<Value> {
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

/// `nextBaseFee(H)` is block H+1's `baseFeePerGas`, exactly, over every
/// consecutive pair fetched (fetch.py `headers`), on Ethereum and the OP stack.
#[test]
#[ignore]
fn live_next_base_fee() {
    let dir = env_dir("WILLOW_LIVE");
    let mut wrong = vec![];
    for c in ["ethereum", "op", "base", "unichain", "ink"] {
        let chain = chain_id(c);
        let hs: Vec<_> = lines(dir.join(format!("{c}-headers.jsonl")))
            .iter()
            .map(|h| {
                let (_, rlp) = logset_blockcheck::header::header_rlp_from_block_json(h).unwrap();
                decode_header(&rlp).unwrap()
            })
            .collect();
        let mut exact = 0;
        for w in hs.windows(2) {
            assert_eq!(w[1].number, w[0].number + 1, "{c}: not consecutive");
            let got = next_base_fee(chain, &w[0]).unwrap();
            let want = w[1].base_fee_per_gas.map(U256::from);
            if got == want {
                exact += 1;
            } else {
                wrong.push(format!(
                    "{c} {}: ours {got:?}, block {}'s {want:?}",
                    w[0].number, w[1].number
                ));
            }
        }
        println!(
            "LIVE nextBaseFee {c}: {exact} of {} exact, blocks {}..={}",
            hs.len() - 1,
            hs[0].number,
            hs[hs.len() - 1].number
        );
    }
    assert!(
        wrong.is_empty(),
        "{} differ: {:?}",
        wrong.len(),
        &wrong[..wrong.len().min(10)]
    );
}

/// The OP stack's `eth_maxPriorityFeePerGas` equals the public node's at the
/// same head (fetch.py `opheads`), and its `eth_gasPrice` differs from the
/// node's by exactly the next base fee less the head's.
#[test]
#[ignore]
fn live_op_heads() {
    let dir = env_dir("WILLOW_LIVE").join("opheads");
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let (mut same, mut wrong) = (
        std::collections::BTreeMap::<u64, (usize, usize)>::new(),
        vec![],
    );
    for f in files {
        let v = read(&f);
        let chain = v["chainId"].as_u64().unwrap();
        let p = Parts::new(chain, &v["block"], Some(&v["receipts"]), &[]);
        let proof =
            build_fee_proof(chain, &[p.header.as_slice()], vec![p.receipts_proof()]).unwrap();
        let fees = verify_fees(&proof, chain, Block::Hash(p.hash())).unwrap();
        let node_tip = U256::from_str_radix(
            v["responses"]["eth_maxPriorityFeePerGas"]
                .as_str()
                .unwrap()
                .trim_start_matches("0x"),
            16,
        )
        .unwrap();
        let node_price = U256::from_str_radix(
            v["responses"]["eth_gasPrice"]
                .as_str()
                .unwrap()
                .trim_start_matches("0x"),
            16,
        )
        .unwrap();
        let e = same.entry(chain).or_default();
        e.1 += 1;
        if fees.tip == node_tip && node_price == fees.base_fee.unwrap() + node_tip {
            e.0 += 1;
        } else {
            wrong.push(format!(
                "{}: tip ours {} node {node_tip}; node gasPrice {node_price}, base {:?}",
                f.display(),
                fees.tip,
                fees.base_fee
            ));
        }
    }
    for (c, (ok, n)) in &same {
        println!("LIVE OP tip chain {c}: {ok} of {n} heads equal the node's");
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Ethereum's `willow-fee-v1` tip at every head of fetch.py `ethfull`, written
/// for the Python port to compare (`out/ethtips-rust.json`).
#[test]
#[ignore]
fn live_eth_tips() {
    let dir = env_dir("WILLOW_LIVE");
    let mut files: Vec<_> = std::fs::read_dir(dir.join("ethfull"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let parts: Vec<Parts> = files
        .iter()
        .map(|f| Parts::new(1, &read(f), None, &[]))
        .collect();
    let mut out = vec![];
    for w in parts.windows(4) {
        let headers: Vec<&[u8]> = w.iter().map(|p| p.header.as_slice()).collect();
        let bodies = w.iter().map(|p| p.block_proof(true)).collect();
        let proof = build_fee_proof(1, &headers, bodies).unwrap();
        let fees = verify_fees(&proof, 1, Block::Hash(w[3].hash())).unwrap();
        out.push(fees.json());
    }
    std::fs::create_dir_all(dir.join("out")).unwrap();
    write(
        dir.join("out/ethtips-rust.json"),
        &Value::Array(out.clone()),
    );
    println!("LIVE Ethereum tips: {} heads written", out.len());
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        // serde_json's default float parse can miss the nearest double by an ulp or so.
        (Value::Number(x), Value::Number(y)) => {
            let (a, b) = (x.as_f64().unwrap(), y.as_f64().unwrap());
            a == b || (a - b).abs() <= 4.0 * f64::EPSILON * a.abs().max(b.abs())
        }
        (Value::String(x), Value::String(y)) => x.eq_ignore_ascii_case(y),
        (Value::Array(x), Value::Array(y)) => {
            x.len() == y.len() && x.iter().zip(y).all(|(p, q)| same_value(p, q))
        }
        _ => a == b,
    }
}

/// Where our `eth_feeHistory` differs from a node's: hex quantities exactly,
/// ratios as numbers, entry by entry.
pub(crate) fn history_differences(ours: &Value, node: &Value) -> Vec<String> {
    let mut out = vec![];
    for k in [
        "oldestBlock",
        "baseFeePerGas",
        "gasUsedRatio",
        "reward",
        "baseFeePerBlobGas",
        "blobGasUsedRatio",
    ] {
        match (ours.get(k), node.get(k)) {
            (None, None) => {}
            // reth's `reward` without percentiles: a row of nothing per block (geth leaves it out).
            (None, Some(Value::Array(rows)))
                if k == "reward"
                    && rows.iter().all(|r| r.as_array().is_some_and(Vec::is_empty)) => {}
            (Some(Value::Array(x)), Some(Value::Array(y))) if x.len() == y.len() => {
                for (i, (p, q)) in x.iter().zip(y).enumerate() {
                    if !same_value(p, q) {
                        out.push(format!("{k}[{i}]: ours {p}, node {q}"));
                    }
                }
            }
            (Some(a), Some(b)) if same_value(a, b) => {}
            (a, b) => out.push(format!("{k}: ours {a:?}, node {b:?}")),
        }
    }
    out
}

/// `eth_feeHistory` over 1,024 blocks (no percentiles) and over 16 or 4
/// blocks with rewards, against the public node (fetch.py `history`).
#[test]
#[ignore]
fn live_history() {
    let dir = env_dir("WILLOW_LIVE").join("history");
    for c in CHAINS {
        let chain = chain_id(c);
        let v = read(dir.join(format!("{c}.json")));
        let headers: Vec<Vec<u8>> = v["headers"]
            .as_array()
            .unwrap()
            .iter()
            .map(|h| {
                logset_blockcheck::header::header_rlp_from_block_json(h)
                    .unwrap()
                    .1
            })
            .collect();
        let refs: Vec<&[u8]> = headers.iter().map(Vec::as_slice).collect();
        let proof = build_fee_proof(chain, &refs, vec![]).unwrap();
        let req = FeeHistoryRequest {
            block_count: headers.len() as u64,
            newest: Block::Number(v["newest"].as_u64().unwrap()),
            reward_percentiles: vec![],
        };
        let ours = verify_fee_history(&proof, chain, &req).unwrap().json();
        let d = history_differences(&ours, &v["eth_feeHistory"]);
        println!(
            "LIVE feeHistory {c} ({} blocks): {} differences {:?}",
            headers.len(),
            d.len(),
            &d[..d.len().min(6)]
        );
        // Nitro's gas-used ratios are a fullness analogue from ArbOS state.
        let nitro = NITRO.contains(&c);
        assert!(
            d.iter().all(|x| nitro && x.starts_with("gasUsedRatio[")),
            "{c}: {d:?}"
        );
        let rewards = dir.join(format!("{c}-rewards.json"));
        if rewards.exists() {
            let v = read(rewards);
            let parts: Vec<Parts> = v["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .zip(v["receipts"].as_array().unwrap())
                .map(|(b, r)| Parts::new(chain, b, Some(r), &[]))
                .collect();
            let refs: Vec<&[u8]> = parts.iter().map(|p| p.header.as_slice()).collect();
            let proof = build_fee_proof(
                chain,
                &refs,
                parts.iter().map(Parts::receipts_proof).collect(),
            )
            .unwrap();
            let req = FeeHistoryRequest {
                block_count: v["count"].as_u64().unwrap(),
                newest: Block::Number(v["newest"].as_u64().unwrap()),
                reward_percentiles: serde_json::from_value(v["percentiles"].clone()).unwrap(),
            };
            let ours = verify_fee_history(&proof, chain, &req).unwrap().json();
            let d = history_differences(&ours, &v["eth_feeHistory"]);
            println!(
                "LIVE feeHistory {c} with rewards ({} blocks): {} differences {:?}",
                parts.len(),
                d.len(),
                &d[..d.len().min(8)]
            );
            assert!(d.is_empty(), "{c}: {d:?}");
        }
    }
}
