//! Fee and fee-history proofs (interface I4, `willow-fee-v1`) on the committed
//! fixtures of all seven chains, against the public node's responses, and the
//! formula's parts on blocks built to reach each branch.

use crate::fault::{fault, Fault};
use crate::fees::{
    excess_blob_gas, next_base_fee, verify_fee_history, verify_fees, FeeHistoryRequest, FeeProof,
    MAX_PRICE, MIN_SUGGESTED_PRIORITY_FEE,
};
use crate::proof::{build_block, decode_header, verify, Block, Subject};
use crate::tests_block::{block_param, fixtures, flip, hexs, Fixture};
use crate::tests_live::history_differences;
use alloy::consensus::Header;
use alloy::primitives::U256;
use serde_json::Value;

fn qty(v: &Value) -> U256 {
    U256::from_str_radix(v.as_str().unwrap().trim_start_matches("0x"), 16).unwrap()
}

fn fee_proof(f: &Fixture) -> FeeProof {
    serde_json::from_value(f.result.clone()).unwrap()
}

fn request(f: &Fixture) -> FeeHistoryRequest {
    FeeHistoryRequest {
        block_count: crate::hex_u64(&f.params[0]).unwrap(),
        newest: block_param(&f.params[1]),
        reward_percentiles: serde_json::from_value(f.params[2].clone()).unwrap(),
    }
}

fn fee_fails(p: &FeeProof, chain: u64, b: Block, want: Fault, what: &str) {
    match verify_fees(p, chain, b) {
        Ok(_) => panic!("{what}: a tampered fee proof verified"),
        Err(e) => assert_eq!(fault(&e), Some(want), "{what}: {e:#}"),
    }
}

fn history_fails(p: &FeeProof, chain: u64, r: &FeeHistoryRequest, want: Fault, what: &str) {
    match verify_fee_history(p, chain, r) {
        Ok(_) => panic!("{what}: a tampered fee-history proof verified"),
        Err(e) => assert_eq!(fault(&e), Some(want), "{what}: {e:#}"),
    }
}

const NITRO: [u64; 2] = [42161, 4663];
const OP: [u64; 4] = [10, 8453, 130, 57073];

/// Each fee proof verifies to the fees recorded with it. On the OP stack the
/// tip is the node's at the same head, exactly, and its gas price the node's
/// plus the next base fee less the head's (we add the next block's, by
/// design); on Nitro all three responses are the node's.
#[test]
fn every_fee_proof_verifies_and_the_node_agrees() {
    let all = fixtures("fees", ".fee.json");
    assert_eq!(all.len(), 7);
    for f in &all {
        let fees = verify_fees(&fee_proof(f), f.chain, block_param(&f.params[0]))
            .unwrap_or_else(|e| panic!("{}: {e:#}", f.name));
        assert_eq!(fees.json(), f.file["verified"], "{}", f.name);
        assert!(fees.tip <= U256::from(MAX_PRICE));
        assert_eq!(fees.gas_price, fees.next_base_fee.unwrap() + fees.tip);
        let node = &f.file["node"];
        let at_head = node["head"].as_u64() == Some(fees.block_number);
        let r = &node["responses"];
        if OP.contains(&f.chain) {
            assert!(at_head, "{}", f.name);
            assert_eq!(fees.tip, qty(&r["eth_maxPriorityFeePerGas"]), "{}", f.name);
            assert_eq!(
                qty(&r["eth_gasPrice"]),
                fees.base_fee.unwrap() + fees.tip,
                "{}",
                f.name
            );
            assert_eq!(
                fees.blob_base_fee,
                Some(qty(&r["eth_blobBaseFee"])),
                "{}",
                f.name
            );
        }
        if NITRO.contains(&f.chain) {
            assert!(at_head, "{}", f.name);
            assert_eq!(fees.tip, qty(&r["eth_maxPriorityFeePerGas"]));
            assert_eq!(fees.gas_price, qty(&r["eth_gasPrice"]));
            assert!(fees.blob_base_fee.is_none() && r["eth_blobBaseFee"].is_null());
        }
    }
}

/// Each fee-history proof verifies to the history recorded with it, which is
/// the node's on Ethereum and the OP stack, field for field. On Nitro the
/// gas-used ratios differ (Nitro reports a fullness analogue from ArbOS state
/// no header commits to) and so may the last base fee (Nitro reports the next
/// block's when it exists).
#[test]
fn every_fee_history_proof_verifies_and_matches_the_node() {
    let all = fixtures("fees", ".history.json");
    assert_eq!(all.len(), 14);
    for f in &all {
        let req = request(f);
        let ours = verify_fee_history(&fee_proof(f), f.chain, &req)
            .unwrap_or_else(|e| panic!("{}: {e:#}", f.name))
            .json();
        // As read back from JSON text, the way the fixture was (serde_json's float parse is not exact).
        let ours: Value = serde_json::from_str(&ours.to_string()).unwrap();
        assert_eq!(ours, f.file["verified"], "{}", f.name);
        let d = history_differences(&ours, &f.file["node"]["eth_feeHistory"]);
        if NITRO.contains(&f.chain) {
            let last = format!("baseFeePerGas[{}]", req.count().min(16));
            assert!(
                d.iter()
                    .all(|x| x.starts_with("gasUsedRatio[") || x.starts_with(&last)),
                "{}: {d:?}",
                f.name
            );
        } else {
            assert!(d.is_empty(), "{}: {d:?}", f.name);
        }
    }
}

/// `nextBaseFee(H)` is block H+1's base fee on each fixture's 16 headers
/// (the live run verifies 1,000 per chain).
#[test]
fn next_base_fee_is_the_next_headers() {
    for f in fixtures("fees", "-16.history.json") {
        if NITRO.contains(&f.chain) {
            continue;
        }
        let p = fee_proof(&f);
        let hs: Vec<Header> = p
            .headers
            .iter()
            .map(|h| decode_header(&hex::decode(&h[2..]).unwrap()).unwrap())
            .collect();
        for w in hs.windows(2) {
            assert_eq!(
                next_base_fee(f.chain, &w[0]).unwrap(),
                w[1].base_fee_per_gas.map(U256::from),
                "{} block {}",
                f.name,
                w[0].number
            );
        }
    }
}

fn header(number: u64, timestamp: u64, gas_limit: u64, gas_used: u64, base: u64) -> Header {
    Header {
        number,
        timestamp,
        gas_limit,
        gas_used,
        base_fee_per_gas: Some(base),
        ..Default::default()
    }
}

/// EIP-1559 on Ethereum: unchanged at the target, at most 1/8 either way,
/// at least 1 wei up.
#[test]
fn ethereum_base_fee_follows_eip_1559() {
    let next = |used| {
        next_base_fee(1, &header(20_000_000, 0, 30_000_000, used, 1_000_000_000))
            .unwrap()
            .unwrap()
    };
    assert_eq!(next(15_000_000), U256::from(1_000_000_000u64));
    assert_eq!(next(30_000_000), U256::from(1_125_000_000u64));
    assert_eq!(next(0), U256::from(875_000_000u64));
    assert_eq!(next(15_000_001), U256::from(1_000_000_008u64));
    let tiny = next_base_fee(1, &header(20_000_000, 0, 30_000_000, 15_000_001, 7)).unwrap();
    assert_eq!(tiny, Some(U256::from(8u8)), "at least 1 wei up");
    // London's first block, and before it none.
    assert_eq!(
        next_base_fee(1, &header(12_964_999, 0, 30_000_000, 0, 0)).unwrap(),
        Some(U256::from(1_000_000_000u64))
    );
    assert_eq!(
        next_base_fee(1, &header(12_000_000, 0, 30_000_000, 0, 0)).unwrap(),
        None
    );
}

/// The OP stack from Holocene reads its parameters from the parent's
/// extraData; from Jovian it meters the DA footprint and keeps a minimum.
#[test]
fn op_base_fee_reads_holocene_and_jovian_extra_data() {
    let jovian = 1_764_691_201;
    let mut h = header(150_000_000, jovian, 40_000_000, 10_000_000, 1_000);
    // denominator 250, elasticity 2, minimum 2,000 wei.
    let mut extra = vec![1u8];
    extra.extend_from_slice(&250u32.to_be_bytes());
    extra.extend_from_slice(&2u32.to_be_bytes());
    extra.extend_from_slice(&2_000u64.to_be_bytes());
    h.extra_data = extra.into();
    h.blob_gas_used = Some(5_000_000);
    // Under the target (20M): down, but not under the minimum.
    assert_eq!(next_base_fee(10, &h).unwrap(), Some(U256::from(2_000u64)));
    h.base_fee_per_gas = Some(1_000_000);
    // 1,000,000 - 1,000,000 * 10M / 20M / 250 = 998,000.
    assert_eq!(next_base_fee(10, &h).unwrap(), Some(U256::from(998_000u64)));
    // A DA footprint over the gas used is what is metered: 30M, so up by
    // 1,000,000 * 10M / 20M / 250 = 2,000.
    h.blob_gas_used = Some(30_000_000);
    assert_eq!(
        next_base_fee(10, &h).unwrap(),
        Some(U256::from(1_002_000u64))
    );
    // Before Jovian the footprint is not metered and there is no minimum
    // (Holocene's 9-byte extraData).
    let mut pre = h.clone();
    pre.timestamp = jovian - 10;
    pre.extra_data = h.extra_data[..9].to_vec().into();
    assert_eq!(
        next_base_fee(10, &pre).unwrap(),
        Some(U256::from(998_000u64))
    );
    // Nitro: the head's own.
    assert_eq!(
        next_base_fee(42161, &h).unwrap(),
        Some(U256::from(1_000_000u64))
    );
}

/// EIP-4844's excess, and EIP-7918's from Osaka when blob gas is priced
/// under the reserve.
#[test]
fn excess_blob_gas_follows_eip_7918_from_osaka() {
    let blobs = crate::derive::BlobSchedule::builtin(1);
    let gas = crate::derive::GAS_PER_BLOB;
    let mut parent = header(23_000_000, 0, 60_000_000, 0, 1_000_000_000);
    parent.excess_blob_gas = Some(10 * gas);
    parent.blob_gas_used = Some(9 * gas);
    // Prague (target 6): 10 + 9 - 6 blobs.
    let prague = 1_750_000_000;
    assert_eq!(
        excess_blob_gas(1, &parent, prague, &blobs).unwrap(),
        13 * gas
    );
    // BPO2 after Osaka (target 14, max 21), a blob price under the reserve
    // (8,192 * 1 gwei): the parent's excess plus its blobs * (21 - 14) / 21.
    let bpo2 = 1_770_000_000;
    assert_eq!(
        excess_blob_gas(1, &parent, bpo2, &blobs).unwrap(),
        10 * gas + 9 * gas * 7 / 21
    );
    let mut more = parent.clone();
    more.blob_gas_used = Some(18 * gas);
    assert_eq!(
        excess_blob_gas(1, &more, bpo2, &blobs).unwrap(),
        (10 + 6) * gas
    );
    // A base fee too low for the reserve: EIP-4844's formula.
    more.base_fee_per_gas = Some(1);
    assert_eq!(
        excess_blob_gas(1, &more, bpo2, &blobs).unwrap(),
        (10 + 18 - 14) * gas
    );
    // Under the target: 0.
    let mut few = parent.clone();
    few.excess_blob_gas = Some(2 * gas);
    few.blob_gas_used = Some(3 * gas);
    assert_eq!(excess_blob_gas(1, &few, bpo2, &blobs).unwrap(), 0);
    // A forged excess no chain reaches is refused, not summed for ever.
    let mut forged = parent.clone();
    forged.excess_blob_gas = Some(u64::MAX);
    assert!(crate::fees::blob_base_fee(1, &forged, &blobs).is_err());
    assert!(excess_blob_gas(1, &forged, bpo2, &blobs).is_err());
    assert_eq!(blobs.price(bpo2, u64::MAX), None);
    // The OP stack: always 0.
    assert_eq!(excess_blob_gas(10, &more, bpo2, &blobs).unwrap(), 0);
}

/// op-geth's tip at capacity: 10% over the median tip of the block,
/// deposits counting as 0.
#[test]
fn an_op_block_at_capacity_suggests_its_median_tip_and_a_tenth() {
    let f = fixtures("fees", ".fee.json")
        .into_iter()
        .find(|f| f.chain == 10)
        .unwrap();
    let p = fee_proof(&f);
    let body = &p.bodies[0];
    let mut h = decode_header(&hex::decode(&body.header[2..]).unwrap()).unwrap();
    let base = U256::from(h.base_fee_per_gas.unwrap());
    // The same block, as if it had filled its gas limit.
    h.gas_limit = h.gas_used;
    let forged = alloy_rlp::encode(&h);
    let txs: Vec<Vec<u8>> = body
        .transactions
        .as_ref()
        .unwrap()
        .iter()
        .map(|t| hex::decode(&t[2..]).unwrap())
        .collect();
    let rs: Vec<Vec<u8>> = body
        .receipts
        .as_ref()
        .unwrap()
        .iter()
        .map(|t| hex::decode(&t[2..]).unwrap())
        .collect();
    let t: Vec<&[u8]> = txs.iter().map(Vec::as_slice).collect();
    let r: Vec<&[u8]> = rs.iter().map(Vec::as_slice).collect();
    let b = build_block(10, &forged, &t, &r).unwrap();
    let v = verify(&b, 10, &Subject::BlockReceipts(Block::Tag)).unwrap();
    let tip = crate::fees::op_tip(&v).unwrap();
    // Independently: each transaction's tip from the node's own JSON.
    let node: Value = crate::tests_live::read(format!(
        "{}/fixtures/block/op-{}.json",
        crate::tests_live::CRATE,
        h.number
    ));
    let mut tips: Vec<U256> = node["node"]["eth_getBlockByNumber"]["transactions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            if t["type"] == "0x7e" {
                return U256::ZERO;
            }
            let cap = qty(t.get("maxFeePerGas").unwrap_or(&t["gasPrice"]));
            let tip = qty(t.get("maxPriorityFeePerGas").unwrap_or(&t["gasPrice"]));
            tip.min(cap - base)
        })
        .collect();
    tips.sort();
    let median = tips[tips.len() / 2];
    let want = (median + median / U256::from(10u8)).max(U256::from(MIN_SUGGESTED_PRIORITY_FEE));
    assert_eq!(tip, want);
    // Not at capacity: the minimum.
    assert_eq!(
        crate::fees::op_tip(&verify(body, 10, &Subject::BlockReceipts(Block::Tag)).unwrap())
            .unwrap(),
        U256::from(MIN_SUGGESTED_PRIORITY_FEE)
    );
}

/// `value` with its receipt's cumulative gas raised by one.
fn bump_cumulative_gas(value: &[u8]) -> Vec<u8> {
    use logset_blockcheck::receipts::rlp_item;
    let typed = value[0] < 0xc0;
    let body = if typed { &value[1..] } else { value };
    let (payload, _, _) = rlp_item(body).unwrap();
    let mut items: Vec<Vec<u8>> = vec![];
    let mut rest = payload;
    while !rest.is_empty() {
        let (_, n, _) = rlp_item(rest).unwrap();
        items.push(rest[..n].to_vec());
        rest = &rest[n..];
    }
    let (g, _, _) = rlp_item(&items[1]).unwrap();
    let gas = g.iter().fold(0u64, |a, b| a << 8 | u64::from(*b));
    items[1] = alloy_rlp::encode(gas + 1);
    let mut out = if typed { vec![value[0]] } else { vec![] };
    out.extend(crate::block::rlp_list(&items));
    out
}

#[test]
fn a_tampered_fee_proof_fails_with_its_kind() {
    let all = fixtures("fees", ".fee.json");
    // Ethereum: four headers and their blocks.
    let f = all.iter().find(|f| f.chain == 1).unwrap();
    let good = fee_proof(f);
    let at = block_param(&f.params[0]);
    assert!(verify_fees(&good, 1, at).is_ok());
    let n = crate::proof::parse_qty(&good.block_number, "n").unwrap();

    let mut p = good.clone();
    p.headers.swap(1, 2);
    fee_fails(&p, 1, at, Fault::Link, "two headers swapped");
    let other = &all.iter().find(|f| f.chain == 10).unwrap().result["headers"][0];
    let mut p = good.clone();
    p.headers[1] = other.as_str().unwrap().into();
    fee_fails(&p, 1, at, Fault::Link, "a header out of its chain");
    let mut p = good.clone();
    p.headers.remove(0);
    p.bodies.remove(0);
    fee_fails(&p, 1, at, Fault::Range, "a header dropped");
    let mut p = good.clone();
    p.headers[3] = flip(&p.headers[3], 60);
    fee_fails(&p, 1, at, Fault::Header, "a byte of the newest header");
    let mut p = good.clone();
    p.bodies.swap(0, 1);
    fee_fails(&p, 1, at, Fault::Body, "a body for another block");
    let mut p = good.clone();
    let t = p.bodies[2].transactions.as_mut().unwrap();
    t[0] = flip(&t[0], 30);
    fee_fails(
        &p,
        1,
        at,
        Fault::Transactions,
        "a body's transaction changed",
    );
    let mut p = good.clone();
    p.bodies[1].transactions.as_mut().unwrap().pop();
    fee_fails(
        &p,
        1,
        at,
        Fault::Transactions,
        "a body's transaction dropped",
    );
    let mut p = good.clone();
    p.bodies.pop();
    fee_fails(&p, 1, at, Fault::Missing, "a body dropped");
    let mut p = good.clone();
    p.bodies.push(good.bodies[0].clone());
    fee_fails(&p, 1, at, Fault::Malformed, "a body too many");
    let mut p = good.clone();
    p.formula = "willow-fee-v2".into();
    fee_fails(&p, 1, at, Fault::Formula, "another formula");
    fee_fails(
        &good,
        1,
        Block::Number(n - 1),
        Fault::Block,
        "another block asked for",
    );
    fee_fails(&good, 10, at, Fault::Chain, "another chain");
    let mut p = good.clone();
    p.block_number = format!("0x{:x}", n + 1);
    fee_fails(&p, 1, Block::Tag, Fault::Header, "the block number");

    // The OP stack: a receipt's cumulative gas.
    let f = all.iter().find(|f| f.chain == 10).unwrap();
    let good = fee_proof(f);
    let at = block_param(&f.params[0]);
    assert!(verify_fees(&good, 10, at).is_ok());
    let mut p = good.clone();
    let r = p.bodies[0].receipts.as_mut().unwrap();
    let k = r.len() / 2;
    r[k] = hexs(&bump_cumulative_gas(&hex::decode(&r[k][2..]).unwrap()));
    fee_fails(&p, 10, at, Fault::Receipts, "a receipt's cumulative gas");
    let mut p = good.clone();
    p.bodies[0].receipts = None;
    fee_fails(&p, 10, at, Fault::Missing, "no receipts");

    // Nitro: a header alone.
    let f = all.iter().find(|f| f.chain == 42161).unwrap();
    let good = fee_proof(f);
    let at = block_param(&f.params[0]);
    let mut p = good.clone();
    p.bodies = fee_proof(all.iter().find(|f| f.chain == 10).unwrap()).bodies;
    fee_fails(&p, 42161, at, Fault::Malformed, "a body where none is read");
}

#[test]
fn a_tampered_fee_history_proof_fails_with_its_kind() {
    let all = fixtures("fees", ".history.json");
    let f = all
        .iter()
        .find(|f| f.chain == 8453 && !f.name.ends_with("-16.history.json"))
        .unwrap();
    let good = fee_proof(f);
    let req = request(f);
    assert!(!req.reward_percentiles.is_empty());
    assert!(verify_fee_history(&good, 8453, &req).is_ok());
    let mut p = good.clone();
    p.headers.swap(0, 1);
    history_fails(&p, 8453, &req, Fault::Link, "two headers swapped");
    let mut p = good.clone();
    p.bodies.swap(0, 1);
    history_fails(&p, 8453, &req, Fault::Body, "a body for another block");
    let mut p = good.clone();
    let r = p.bodies[1].receipts.as_mut().unwrap();
    r[0] = hexs(&bump_cumulative_gas(&hex::decode(&r[0][2..]).unwrap()));
    history_fails(
        &p,
        8453,
        &req,
        Fault::Receipts,
        "a receipt's cumulative gas",
    );
    let fewer = FeeHistoryRequest {
        block_count: req.block_count - 1,
        ..req.clone()
    };
    history_fails(
        &good,
        8453,
        &fewer,
        Fault::Range,
        "more blocks than asked for",
    );
    let bad = FeeHistoryRequest {
        reward_percentiles: vec![50.0, 10.0],
        ..req.clone()
    };
    history_fails(
        &good,
        8453,
        &bad,
        Fault::Malformed,
        "percentiles out of order",
    );
    let n = crate::proof::parse_qty(&good.block_number, "n").unwrap();
    let other = FeeHistoryRequest {
        newest: Block::Number(n + 1),
        ..req.clone()
    };
    history_fails(&good, 8453, &other, Fault::Block, "another newest block");

    // Headers alone serve only a request without percentiles.
    let f = all
        .iter()
        .find(|f| f.chain == 8453 && f.name.ends_with("-16.history.json"))
        .unwrap();
    let plain = fee_proof(f);
    let with = FeeHistoryRequest {
        reward_percentiles: vec![50.0],
        ..request(f)
    };
    history_fails(
        &plain,
        8453,
        &with,
        Fault::Missing,
        "rewards asked of headers alone",
    );
    let mut p = plain.clone();
    p.headers.remove(7);
    history_fails(
        &p,
        8453,
        &request(f),
        Fault::Link,
        "a header dropped from the middle",
    );
}
