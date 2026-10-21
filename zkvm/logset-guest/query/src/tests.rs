use super::*;
use crate::fixtures::{addr, chain as make_chain, fake_proof, topic, Chain, FakeCheck, L1_VK};
use serde_json::{json, Value};
use std::collections::HashMap;

const CHAIN: u64 = 1;
const START: u64 = 26_065_000;

fn chain(n: usize, seed: u64) -> Chain {
    make_chain(CHAIN, START, n, seed)
}

enum Part {
    Seg(usize, usize),
    Blk(usize),
}

/// What the server assembles, plus the stand-in proof store.
fn serve(c: &Chain, layout: &[Part], f: &LogFilter) -> (LogsProof, HashMap<String, Vec<u8>>) {
    let leaf_vk = c.leaf_vkey();
    let mut store = HashMap::new();
    let mut parts = Vec::new();
    for p in layout {
        match *p {
            Part::Seg(i0, i1) => {
                let pv = c.l1(i0, i1);
                let ix = SegmentIndex::build(&pv, &c.records[i0..i1]).expect("build");
                let id = c.segment_id(i0, i1).to_string();
                store.insert(id.clone(), fake_proof(&L1_VK, &pv));
                parts.push(ProofPart::Segment(SegmentProof {
                    ranges: ix.prove(f).unwrap().into_iter().map(Into::into).collect(),
                    public_values: pv,
                    proof: id,
                }));
            }
            Part::Blk(i) => {
                let pv = c.leaf_pvs[i].clone();
                let id = c.block_id(i).to_string();
                store.insert(id.clone(), fake_proof(&leaf_vk, &pv));
                parts.push(ProofPart::Block(BlockProof {
                    public_values: pv,
                    proof: id,
                    records: c.records[i].clone(),
                }));
            }
        }
    }
    let proof = LogsProof {
        chain_id: CHAIN,
        from_block: f.from_block,
        to_block: f.to_block,
        parts,
    };
    (proof, store)
}

fn check(
    c: &Chain,
    p: &LogsProof,
    store: &HashMap<String, Vec<u8>>,
    f: &LogFilter,
) -> Result<Verified, VerifyError> {
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    verify(p, f, &c.trust(), &mut fetch, &mut FakeCheck)
}

fn filter(from: usize, to: usize, addrs: Option<&[u8]>, topics: &[Option<&[u8]>]) -> LogFilter {
    LogFilter {
        from_block: START + from as u64,
        to_block: START + to as u64,
        addresses: addrs.map(|a| a.iter().map(|&i| addr(i)).collect()),
        topics: topics
            .iter()
            .map(|t| t.map(|t| t.iter().map(|&i| topic(i)).collect()))
            .collect(),
    }
}

fn filters() -> Vec<LogFilter> {
    vec![
        filter(0, 39, Some(&[1]), &[Some(&[1])]),
        filter(5, 33, Some(&[1, 2]), &[Some(&[1, 3])]),
        filter(0, 39, Some(&[2]), &[]),
        filter(3, 36, None, &[Some(&[2])]),
        filter(10, 20, None, &[]),
        filter(0, 39, None, &[None, Some(&[10])]),
        filter(0, 39, Some(&[3]), &[Some(&[2]), Some(&[11])]),
        filter(0, 39, Some(&[9]), &[Some(&[1])]),
        filter(0, 39, Some(&[1, 1, 3]), &[Some(&[]), None]),
        filter(16, 16, Some(&[1, 2, 3]), &[Some(&[1, 2, 3])]),
    ]
}

const LAYOUT: &[Part] = &[
    Part::Seg(0, 16),
    Part::Seg(16, 32),
    Part::Blk(32),
    Part::Blk(33),
    Part::Blk(34),
    Part::Blk(35),
    Part::Blk(36),
    Part::Blk(37),
    Part::Blk(38),
    Part::Blk(39),
];

#[test]
fn every_filter_verifies_to_exactly_the_matching_logs() {
    let c = chain(40, 7);
    let all = c.all_logs();
    assert!(all.len() > 100, "fixture has {} logs", all.len());
    let mut sizes = Vec::new();
    for f in filters() {
        let (p, store) = serve(&c, LAYOUT, &f);
        let got = check(&c, &p, &store, &f).unwrap_or_else(|e| panic!("{f:?}: {e}"));
        let want: Vec<Log> = all.iter().filter(|l| f.matches(l)).cloned().collect();
        assert_eq!(got.logs, want, "{f:?}");
        assert_eq!(got.checkpoints.last().unwrap().0, START + 39);
        sizes.push(want.len());
    }
    // Two filters match nothing by construction; the rest must exercise real answers.
    assert_eq!(sizes.iter().filter(|&&n| n == 0).count(), 2, "{sizes:?}");
}

#[test]
fn a_gap_in_segments_can_be_filled_with_block_proofs() {
    let c = chain(40, 8);
    let layout = [
        Part::Seg(0, 16),
        Part::Blk(16),
        Part::Blk(17),
        Part::Seg(18, 34),
        Part::Blk(34),
    ];
    let f = filter(2, 34, Some(&[1, 2]), &[Some(&[1, 2])]);
    let (p, store) = serve(&c, &layout, &f);
    let want: Vec<Log> = c.all_logs().into_iter().filter(|l| f.matches(l)).collect();
    assert_eq!(check(&c, &p, &store, &f).unwrap().logs, want);
}

#[test]
fn wire_round_trip_keeps_the_proof_valid() {
    let c = chain(40, 9);
    let f = filters()[1].clone();
    let (p, store) = serve(&c, LAYOUT, &f);
    let text = serde_json::to_string(&p).unwrap();
    let back: LogsProof = serde_json::from_str(&text).unwrap();
    assert_eq!(back, p);
    assert!(check(&c, &back, &store, &f).is_ok());
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["parts"][0]["type"], "segment");
    assert_eq!(v["parts"][2]["type"], "block");
    assert_eq!(v["fromBlock"], format!("0x{:x}", START + 5));
}

/// The first segment's first range proof with at least `n` records.
fn first_range(p: &mut LogsProof, n: usize) -> &mut RangeProofWire {
    p.parts
        .iter_mut()
        .filter_map(|x| match x {
            ProofPart::Segment(s) => Some(s),
            _ => None,
        })
        .flat_map(|s| s.ranges.iter_mut())
        .find(|r| r.records.len() >= n)
        .expect("a range with enough records")
}

#[test]
fn dropping_a_record_from_a_range_is_caught() {
    let c = chain(40, 10);
    let f = filter(0, 39, Some(&[1, 2, 3]), &[]);
    let (mut p, store) = serve(&c, LAYOUT, &f);
    first_range(&mut p, 4).records.remove(1);
    assert!(matches!(
        check(&c, &p, &store, &f),
        Err(VerifyError::Range { .. })
    ));
}

#[test]
fn altering_a_record_is_caught() {
    let c = chain(40, 11);
    let f = filter(0, 39, None, &[Some(&[1, 2, 3])]);
    let (mut p, store) = serve(&c, LAYOUT, &f);
    let r = first_range(&mut p, 3);
    let last = r.records[1].record.len() - 1;
    r.records[1].record[last] ^= 1;
    assert!(matches!(
        check(&c, &p, &store, &f),
        Err(VerifyError::Range { .. })
    ));
}

#[test]
fn a_server_cannot_choose_narrower_ranges() {
    let c = chain(40, 12);
    let f = filter(0, 39, Some(&[1, 2]), &[Some(&[1, 2])]);
    let narrow = filter(0, 39, Some(&[1]), &[Some(&[1, 2])]);
    let (p, store) = serve(&c, LAYOUT, &narrow);
    assert!(matches!(
        check(&c, &p, &store, &f),
        Err(VerifyError::RangeCount { .. })
    ));
    // Same count, other ranges: the proofs do not open the filter's ranges.
    let shifted = filter(0, 39, Some(&[2, 3]), &[Some(&[1, 2])]);
    let (p, store) = serve(&c, LAYOUT, &shifted);
    assert!(matches!(
        check(&c, &p, &store, &f),
        Err(VerifyError::Range { .. })
    ));
}

#[test]
fn coverage_and_links_are_enforced() {
    let c = chain(40, 13);
    let f = filter(0, 39, Some(&[1]), &[]);
    let (mut p, store) = serve(&c, LAYOUT, &f);
    let mut gap = p.clone();
    gap.parts.remove(1);
    assert_eq!(
        check(&c, &gap, &store, &f).unwrap_err(),
        VerifyError::Gap { block: START + 16 }
    );
    let mut short = p.clone();
    short.parts.pop();
    assert_eq!(
        check(&c, &short, &store, &f).unwrap_err(),
        VerifyError::Gap { block: START + 39 }
    );
    let mut late = p.clone();
    late.parts.remove(0);
    assert_eq!(
        check(&c, &late, &store, &f).unwrap_err(),
        VerifyError::Gap { block: START }
    );
    // A block from another fork at the same height.
    let fork = chain(40, 99);
    let (fp, fstore) = serve(&fork, &[Part::Blk(39)], &f);
    let ProofPart::Block(b) = fp.parts[0].clone() else {
        unreachable!()
    };
    p.parts[9] = ProofPart::Block(b.clone());
    let mut store = store;
    store.insert(b.proof.clone(), fstore[&b.proof].clone());
    assert_eq!(
        check(&c, &p, &store, &f).unwrap_err(),
        VerifyError::Link { block: START + 39 }
    );
}

#[test]
fn proofs_programs_and_chain_are_checked() {
    let c = chain(40, 14);
    let f = filter(0, 39, Some(&[1]), &[Some(&[1])]);
    let (p, store) = serve(&c, LAYOUT, &f);
    let mut bad = store.clone();
    let ProofPart::Segment(s) = &p.parts[1] else {
        unreachable!()
    };
    bad.insert(s.proof.clone(), vec![0; 32]);
    assert!(matches!(
        check(&c, &p, &bad, &f),
        Err(VerifyError::Proof { .. })
    ));
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    let mut t = c.trust();
    t.leaf_vkeys = vec![[7; 32]];
    assert_eq!(
        verify(&p, &f, &t, &mut fetch, &mut FakeCheck).unwrap_err(),
        VerifyError::UnknownLeafProgram
    );
    let mut t = c.trust();
    t.chain_id = 42161;
    assert!(matches!(
        verify(&p, &f, &t, &mut fetch, &mut FakeCheck),
        Err(VerifyError::Chain { .. })
    ));
}

#[test]
fn tail_block_records_must_match_the_leaf() {
    let c = chain(40, 15);
    let f = filter(0, 39, None, &[]);
    let (mut p, store) = serve(&c, LAYOUT, &f);
    let ProofPart::Block(b) = &mut p.parts[3] else {
        unreachable!()
    };
    let (_, starts) = frame(0, &b.records).unwrap();
    assert!(starts.len() >= 2, "fixture block 33 has logs");
    b.records.truncate(starts[starts.len() - 1] as usize - 44);
    assert_eq!(
        check(&c, &p, &store, &f).unwrap_err(),
        VerifyError::BlockRecords { block: START + 33 }
    );
}

#[test]
fn segment_index_refuses_records_that_do_not_rebuild_the_commitment() {
    let c = chain(40, 16);
    let pv = c.l1(0, 16);
    let mut recs = c.records[0..16].to_vec();
    let i = recs.iter().position(|r| !r.is_empty()).unwrap();
    let last = recs[i].len() - 1;
    recs[i][last] ^= 1;
    assert_eq!(
        SegmentIndex::build(&pv, &recs).err(),
        Some(ServeError::Mismatch("index root"))
    );
    assert!(matches!(
        SegmentIndex::build(&pv, &c.records[0..15]),
        Err(ServeError::BlockCount { .. })
    ));
    let ix = SegmentIndex::build(&pv, &c.records[0..16]).unwrap();
    let f = filter(0, 15, None, &[]);
    let sent: usize = ix.prove(&f).unwrap().iter().map(|r| r.records.len()).sum();
    assert!(ix.records_for(&f).unwrap() >= sent);
}

#[test]
fn proof_ids_parse_strictly() {
    let leaf = ProofId::Block {
        number: 7,
        hash: [0xab; 32],
    };
    for id in [ProofId::Segment { start: 1, end: 64 }, leaf] {
        assert_eq!(id.to_string().parse::<ProofId>().unwrap(), id);
    }
    for bad in [
        "l1/1-2/../x",
        "../l1/1-2",
        "l1/+1-2",
        "l1/1-",
        "l2/1-2",
        "leaf/7-AB",
        &format!("leaf/7-{}", "AB".repeat(32)),
        &format!("leaf/7-{}/x", "ab".repeat(32)),
    ] {
        assert!(bad.parse::<ProofId>().is_err(), "{bad}");
    }
}

#[test]
fn compare_reports_missing_altered_and_extra_logs() {
    let c = chain(40, 17);
    let logs: Vec<Log> = c.all_logs().into_iter().take(5).collect();
    let as_json = |l: &Log| {
        json!({
            "address": format!("0x{}", hex::encode(l.address)),
            "topics": l.topics.iter().map(|t| format!("0x{}", hex::encode(t))).collect::<Vec<_>>(),
            "data": format!("0x{}", hex::encode(&l.data)),
            "blockNumber": format!("0x{:x}", l.block),
            "transactionIndex": format!("0x{:x}", l.tx_index),
            "logIndex": format!("0x{:x}", l.log_index),
            "blockHash": "0x00",
            "removed": false,
        })
    };
    let answer: Vec<Value> = logs.iter().map(as_json).collect();
    assert!(compare(&logs, &answer).is_exact());
    let mut altered = answer.clone();
    altered[2]["data"] = json!("0xdead");
    altered.remove(4);
    let mut extra = as_json(&logs[0]);
    extra["logIndex"] = json!("0x999");
    altered.push(extra);
    let cmp = compare(&logs, &altered);
    assert_eq!(cmp.matched, 3);
    assert_eq!(cmp.missing, vec![logs[2].clone(), logs[4].clone()]);
    assert_eq!(cmp.unproven.len(), 2);
}

#[test]
fn eth_filters_parse_with_the_log_rpc_rules() {
    let a = format!("0x{}", hex::encode(addr(1)));
    let t = format!("0x{}", hex::encode(topic(2)));
    let f = LogFilter::from_eth_filter(
        &json!({"address": a, "topics": [t, null, [], [t, t]]}),
        5,
        9,
    )
    .unwrap();
    assert_eq!(f.addresses, Some(vec![addr(1)]));
    assert_eq!(
        f.topics,
        vec![
            Some(vec![topic(2)]),
            None,
            None,
            Some(vec![topic(2), topic(2)])
        ]
    );
    assert_eq!((f.from_block, f.to_block), (5, 9));
    let any = LogFilter::from_eth_filter(&json!({}), 1, 1).unwrap();
    assert_eq!((any.addresses, any.topics.len()), (None, 0));
    let empty = LogFilter::from_eth_filter(&json!({"address": []}), 1, 1).unwrap();
    assert_eq!(empty.addresses, None);
    assert!(
        LogFilter::from_eth_filter(&json!({"topics": [null, null, null, null, null]}), 1, 1)
            .is_err()
    );
    assert!(LogFilter::from_eth_filter(&json!({"address": "0x12"}), 1, 1).is_err());
    assert!(LogFilter::from_eth_filter(&json!({"address": 7}), 1, 1).is_err());
}

// ---- Multi-block leaves (fast chains: one leaf per aligned S-block segment) ----

const ARB: u64 = 42161;
/// 2^20-aligned, so every power-of-two leaf and window up to 2^20 starts here.
const FAST_START: u64 = 510_656_512;

fn fast_chain(n: usize, seed: u64) -> Chain {
    make_chain(ARB, FAST_START, n, seed)
}

enum Fast {
    /// An L1 over blocks `i0..i1` from leaves of `span` blocks.
    Seg(usize, usize, usize),
    /// The leaf over blocks `i0..i1`.
    Leaf(usize, usize),
}

fn serve_fast(c: &Chain, layout: &[Fast], f: &LogFilter) -> (LogsProof, HashMap<String, Vec<u8>>) {
    let mut store = HashMap::new();
    let mut parts = Vec::new();
    for p in layout {
        match *p {
            Fast::Seg(span, i0, i1) => {
                let pv = c.l1_over(span, i0, i1);
                let ix = SegmentIndex::build(&pv, &c.records[i0..i1]).expect("build");
                let id = c.segment_id(i0, i1).to_string();
                store.insert(id.clone(), fake_proof(&L1_VK, &pv));
                parts.push(ProofPart::Segment(SegmentProof {
                    ranges: ix.prove(f).unwrap().into_iter().map(Into::into).collect(),
                    public_values: pv,
                    proof: id,
                }));
            }
            Fast::Leaf(i0, i1) => {
                let (pv, proof, _) = c.leaf_files(i0, i1);
                let id = c.leaf_id(i0, i1).to_string();
                store.insert(id.clone(), proof);
                parts.push(ProofPart::Leaf(LeafProof {
                    public_values: pv,
                    proof: id,
                    records: c.records[i0..i1].to_vec(),
                }));
            }
        }
    }
    let proof = LogsProof {
        chain_id: c.chain_id,
        from_block: f.from_block,
        to_block: f.to_block,
        parts,
    };
    (proof, store)
}

fn fast_filter(from: usize, to: usize, addrs: Option<&[u8]>, t0: Option<&[u8]>) -> LogFilter {
    LogFilter {
        from_block: FAST_START + from as u64,
        to_block: FAST_START + to as u64,
        addresses: addrs.map(|a| a.iter().map(|&i| addr(i)).collect()),
        topics: vec![t0.map(|t| t.iter().map(|&i| topic(i)).collect())],
    }
}

/// Two 16-block windows of 8-block leaves, then two lone leaves.
const FAST_LAYOUT: &[Fast] = &[
    Fast::Seg(8, 0, 16),
    Fast::Seg(8, 16, 32),
    Fast::Leaf(32, 40),
    Fast::Leaf(40, 48),
];

#[test]
fn multi_block_leaves_and_their_segments_verify_to_exactly_the_matching_logs() {
    let c = fast_chain(48, 41);
    let all = c.all_logs();
    assert!(all.len() >= 100, "fixture has {} logs", all.len());
    for f in [
        fast_filter(0, 47, Some(&[1]), Some(&[1])),
        fast_filter(5, 44, Some(&[1, 2]), Some(&[1, 3])),
        fast_filter(0, 47, None, Some(&[2])),
        fast_filter(3, 45, None, None),
        // Inside one leaf and one segment only.
        fast_filter(34, 36, Some(&[3]), None),
        fast_filter(17, 18, None, None),
    ] {
        let layout: Vec<&Fast> = FAST_LAYOUT
            .iter()
            .filter(|p| {
                let (a, b) = match **p {
                    Fast::Seg(_, a, b) | Fast::Leaf(a, b) => (a as u64, b as u64 - 1),
                };
                FAST_START + b >= f.from_block && FAST_START + a <= f.to_block
            })
            .collect();
        let layout: Vec<Fast> = layout
            .into_iter()
            .map(|p| match *p {
                Fast::Seg(s, a, b) => Fast::Seg(s, a, b),
                Fast::Leaf(a, b) => Fast::Leaf(a, b),
            })
            .collect();
        let (p, store) = serve_fast(&c, &layout, &f);
        let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
        let got = verify(&p, &f, &c.trust(), &mut fetch, &mut FakeCheck)
            .unwrap_or_else(|e| panic!("{f:?}: {e}"));
        let want: Vec<Log> = all.iter().filter(|l| f.matches(l)).cloned().collect();
        assert_eq!(got.logs, want, "{f:?}");
        let (last, hash) = *got.checkpoints.last().unwrap();
        let end = (last - FAST_START) as usize;
        assert_eq!(hash, c.hashes[end]);
    }
    // The L1 commits the same leaf key whatever the child span.
    assert_eq!(decode_l1_pv_key(&c.l1_over(8, 0, 16)), c.leaf_vkey());
    assert_eq!(decode_l1_pv_key(&c.l1(0, 1)), c.leaf_vkey());
}

fn decode_l1_pv_key(pv: &[u8]) -> [u8; 32] {
    logset_agg_core::index_agg::decode_l1_pv(pv).unwrap().2
}

/// A leaf's every block, a lone block and a segment: any mix that links verifies.
#[test]
fn leaves_blocks_and_segments_mix_when_they_link() {
    let c = fast_chain(40, 42);
    let f = fast_filter(0, 39, None, None);
    let mixed = [
        Fast::Leaf(0, 4),
        Fast::Seg(4, 4, 12),
        Fast::Leaf(12, 13),
        Fast::Leaf(13, 40),
    ];
    let (mut p, store) = serve_fast(&c, &mixed, &f);
    // A one-block leaf is served as a block part, as on Ethereum.
    let ProofPart::Leaf(one) = p.parts[2].clone() else {
        unreachable!()
    };
    p.parts[2] = ProofPart::Block(BlockProof {
        public_values: one.public_values,
        proof: one.proof,
        records: one.records[0].clone(),
    });
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    let got = verify(&p, &f, &c.trust(), &mut fetch, &mut FakeCheck).unwrap();
    assert_eq!(got.logs, c.all_logs());
    assert_eq!(got.first_parent, (FAST_START, [0x42; 32]));
    assert_eq!(
        got.checkpoints,
        [3, 11, 12, 39]
            .iter()
            .map(|&i| (FAST_START + i as u64, c.hashes[i]))
            .collect::<Vec<_>>()
    );
}

fn leaf_part(p: &mut LogsProof, i: usize) -> &mut LeafProof {
    match &mut p.parts[i] {
        ProofPart::Leaf(l) => l,
        _ => panic!("part {i} is not a leaf"),
    }
}

fn check_fast(
    c: &Chain,
    p: &LogsProof,
    store: &HashMap<String, Vec<u8>>,
    f: &LogFilter,
) -> Result<Verified, VerifyError> {
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    verify(p, f, &c.trust(), &mut fetch, &mut FakeCheck)
}

#[test]
fn a_leaf_must_carry_every_block_s_records_exactly() {
    let c = fast_chain(48, 43);
    let f = fast_filter(32, 47, None, None);
    let layout = [Fast::Leaf(32, 40), Fast::Leaf(40, 48)];
    let (p, store) = serve_fast(&c, &layout, &f);
    assert!(check_fast(&c, &p, &store, &f).is_ok());
    let at = |i: usize| FAST_START + i as u64;

    // One stream short, or one too many.
    let mut short = p.clone();
    leaf_part(&mut short, 0).records.pop();
    assert_eq!(
        check_fast(&c, &short, &store, &f).unwrap_err(),
        VerifyError::LeafBlocks {
            block: at(32),
            want: 8,
            got: 7
        }
    );
    let mut long = p.clone();
    leaf_part(&mut long, 1).records.push(vec![]);
    assert!(matches!(
        check_fast(&c, &long, &store, &f),
        Err(VerifyError::LeafBlocks { got: 9, .. })
    ));

    // A record moved to the next block: same bytes, same count, other digests.
    let mut moved = p.clone();
    let l = leaf_part(&mut moved, 0);
    let i = (0..7)
        .find(|&i| !l.records[i].is_empty())
        .expect("a block with logs");
    let (_, starts) = frame(0, &l.records[i]).unwrap();
    let cut = starts[starts.len() - 1] as usize - logset_core::erc::block_digest_prefix(0).len();
    let tail = l.records[i].split_off(cut);
    l.records[i + 1].splice(0..0, tail);
    assert_eq!(
        check_fast(&c, &moved, &store, &f).unwrap_err(),
        VerifyError::BlockRecords { block: at(32) }
    );

    // A dropped record, an altered byte, a stream that does not parse.
    let mut dropped = p.clone();
    let l = leaf_part(&mut dropped, 1);
    let i = l.records.iter().position(|r| !r.is_empty()).unwrap();
    let (_, starts) = frame(0, &l.records[i]).unwrap();
    let cut = starts[starts.len() - 1] as usize - logset_core::erc::block_digest_prefix(0).len();
    l.records[i].truncate(cut);
    assert_eq!(
        check_fast(&c, &dropped, &store, &f).unwrap_err(),
        VerifyError::BlockRecords { block: at(40) }
    );
    let mut altered = p.clone();
    let l = leaf_part(&mut altered, 1);
    let last = l.records[i].len() - 1;
    l.records[i][last] ^= 1;
    assert_eq!(
        check_fast(&c, &altered, &store, &f).unwrap_err(),
        VerifyError::BlockRecords { block: at(40) }
    );
    let mut garbage = p.clone();
    leaf_part(&mut garbage, 1).records[3] = vec![1, 2, 3];
    assert_eq!(
        check_fast(&c, &garbage, &store, &f).unwrap_err(),
        VerifyError::Record { block: at(43) }
    );
}

#[test]
fn a_leaf_is_linked_proven_and_on_the_trusted_chain() {
    let c = fast_chain(48, 44);
    let f = fast_filter(32, 47, Some(&[1]), None);
    let layout = [Fast::Leaf(32, 40), Fast::Leaf(40, 48)];
    let (p, store) = serve_fast(&c, &layout, &f);

    // The same blocks from another fork.
    let fork = fast_chain(48, 98);
    let (fp, fstore) = serve_fast(&fork, &[Fast::Leaf(40, 48)], &f);
    let mut forked = p.clone();
    forked.parts[1] = fp.parts[0].clone();
    let mut both = store.clone();
    both.extend(fstore);
    assert_eq!(
        check_fast(&c, &forked, &both, &f).unwrap_err(),
        VerifyError::Link {
            block: FAST_START + 40
        }
    );
    // A missing leaf leaves a gap.
    let mut gap = p.clone();
    gap.parts.remove(0);
    assert_eq!(
        check_fast(&c, &gap, &store, &f).unwrap_err(),
        VerifyError::Gap {
            block: FAST_START + 32
        }
    );
    // A bad proof, or one under an untrusted leaf key.
    let mut bad = store.clone();
    bad.insert(c.leaf_id(40, 48).to_string(), vec![0; 32]);
    assert!(matches!(
        check_fast(&c, &p, &bad, &f),
        Err(VerifyError::Proof { .. })
    ));
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    let mut t = c.trust();
    t.leaf_vkeys = vec![[7; 32]];
    assert!(matches!(
        verify(&p, &f, &t, &mut fetch, &mut FakeCheck),
        Err(VerifyError::Proof { .. })
    ));
    // The client pins the chain: an Arbitrum answer is not a Robinhood one.
    let mut t = c.trust();
    t.chain_id = 4663;
    assert_eq!(
        verify(&p, &f, &t, &mut fetch, &mut FakeCheck).unwrap_err(),
        VerifyError::Chain { got: ARB }
    );
    // A leaf is not a one-block part.
    let ProofPart::Leaf(l) = &p.parts[0] else {
        unreachable!()
    };
    assert_eq!(
        block_statement(&l.public_values, &l.records.concat()).unwrap_err(),
        VerifyError::Shape {
            block: FAST_START + 32
        }
    );
}

#[test]
fn a_leaf_part_round_trips_on_the_wire() {
    let c = fast_chain(48, 45);
    let f = fast_filter(0, 47, Some(&[2]), None);
    let (p, store) = serve_fast(&c, FAST_LAYOUT, &f);
    let text = serde_json::to_string(&p).unwrap();
    let back: LogsProof = serde_json::from_str(&text).unwrap();
    assert_eq!(back, p);
    assert!(check_fast(&c, &back, &store, &f).is_ok());
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["parts"][2]["type"], "leaf");
    assert_eq!(v["parts"][2]["records"].as_array().unwrap().len(), 8);
    assert_eq!(
        v["parts"][2]["proof"],
        format!(
            "leaf/{}-{}-{}",
            FAST_START + 32,
            FAST_START + 39,
            hex::encode(c.hashes[39])
        )
    );
    assert!(v["parts"][2]["records"][0]
        .as_str()
        .unwrap()
        .starts_with("0x"));
    let mut bad = v.clone();
    bad["parts"][2]["records"][1] = json!("12");
    assert!(serde_json::from_value::<LogsProof>(bad).is_err());
}

#[test]
fn multi_block_leaf_ids_parse_strictly() {
    let id = ProofId::Leaf {
        start: 1024,
        end: 2047,
        hash: [0xcd; 32],
    };
    assert_eq!(
        id.to_string(),
        format!("leaf/1024-2047-{}", "cd".repeat(32))
    );
    assert_eq!(id.to_string().parse::<ProofId>().unwrap(), id);
    let h = "cd".repeat(32);
    for bad in [
        format!("leaf/7-7-{h}"),
        format!("leaf/8-7-{h}"),
        format!("leaf/1-2-{}", "CD".repeat(32)),
        format!("leaf/1-2-{h}-3"),
        format!("leaf/1--2-{h}"),
        format!("leaf/1-2-{h}/x"),
        format!("l1/1-2-{h}"),
        "l1/1-2-3".to_string(),
    ] {
        assert!(bad.parse::<ProofId>().is_err(), "{bad}");
    }
}

#[test]
fn rec_files_frame_one_stream_per_block() {
    let streams = vec![vec![1, 2, 3], vec![], vec![9; 300]];
    let packed = pack_streams(&streams);
    assert_eq!(packed.len(), 3 * 4 + 303);
    assert_eq!(unpack_streams(&packed, 3), Some(streams.clone()));
    assert_eq!(unpack_streams(&packed, 2), None);
    assert_eq!(unpack_streams(&packed, 4), None);
    assert_eq!(unpack_streams(&packed[..packed.len() - 1], 3), None);
    assert_eq!(unpack_streams(&[], 0), Some(vec![]));
    assert_eq!(unpack_streams(&[0xff, 0xff, 0xff, 0xff], 1), None);
}

/// Real chain data: every `genblob` witness under `LOGSET_REAL_BLOBS`
/// (`<chain>/seg-<start>-<n>.{blob,expected.json,records.json}`).
/// The leaf's native run must give the recorded public values (host/guest
/// parity was checked when they were recorded), and the logs proven through two
/// half leaves, through an L1 over those two leaves, and through one leaf over
/// the whole segment must equal `records.json` (the receipt-derived set, equal
/// to `eth_getLogs` when it was generated). SP1 proofs are stood in for.
#[test]
#[ignore]
fn real_fast_chain_segments_verify() {
    let dir = std::path::PathBuf::from(
        std::env::var("LOGSET_REAL_BLOBS").expect("set LOGSET_REAL_BLOBS"),
    );
    let mut seen = Vec::new();
    for chain_dir in std::fs::read_dir(&dir).unwrap().flatten() {
        for e in std::fs::read_dir(chain_dir.path()).unwrap().flatten() {
            let path = e.path();
            if path.extension().is_none_or(|x| x != "blob") {
                continue;
            }
            let name = path.file_stem().unwrap().to_str().unwrap().to_string();
            let expected: Value = serde_json::from_slice(
                &std::fs::read(path.with_extension("expected.json")).unwrap(),
            )
            .unwrap();
            let t = std::time::Instant::now();
            let c = crate::fixtures::chain_from_blob(std::fs::read(&path).unwrap()).unwrap();
            let n = c.records.len();
            if n < 2 {
                continue;
            }
            let whole = c.leaf_pv(0, n);
            assert_eq!(
                format!("0x{}", hex::encode(&whole)),
                expected["public_values_hex"].as_str().unwrap(),
                "{name}: native leaf differs from the recorded public values"
            );
            let claimed: Vec<Vec<Value>> = serde_json::from_slice(
                &std::fs::read(path.with_extension("records.json")).unwrap(),
            )
            .unwrap();
            let claimed: Vec<Log> = claimed[0].iter().map(record_json_log).collect();
            assert_eq!(
                claimed.len() as u64,
                expected["per_filter"][0]["eth_getLogs_count"]
                    .as_u64()
                    .unwrap()
            );
            let h = n / 2;
            let end = n - 1;
            // The busiest (address, topic0) pair, and its topic0 alone.
            let mut pairs: HashMap<([u8; 20], [u8; 32]), usize> = HashMap::new();
            for l in &claimed {
                if let Some(t0) = l.topics.first() {
                    *pairs.entry((l.address, *t0)).or_default() += 1;
                }
            }
            let (&(a, t0), _) = pairs.iter().max_by_key(|(_, &k)| k).unwrap();
            let range = |f: LogFilter| LogFilter {
                from_block: c.start,
                to_block: c.start + end as u64,
                ..f
            };
            let filters = [
                range(LogFilter::default()),
                range(LogFilter {
                    addresses: Some(vec![a]),
                    topics: vec![Some(vec![t0])],
                    ..Default::default()
                }),
                range(LogFilter {
                    topics: vec![Some(vec![t0])],
                    ..Default::default()
                }),
            ];
            for layout in [
                vec![Fast::Leaf(0, h), Fast::Leaf(h, n)],
                vec![Fast::Seg(h, 0, n)],
                vec![Fast::Leaf(0, n)],
            ] {
                for f in &filters {
                    let (p, store) = serve_fast(&c, &layout, f);
                    let got =
                        check_fast(&c, &p, &store, f).unwrap_or_else(|e| panic!("{name}: {e}"));
                    let want: Vec<Log> = claimed.iter().filter(|l| f.matches(l)).cloned().collect();
                    assert_eq!(got.logs, want, "{name}");
                    assert_eq!(got.checkpoints.last().unwrap().1, c.hashes[end], "{name}");
                }
            }
            // What a server pays to rebuild this segment's index from its records.
            let pv = c.l1_over(h, 0, n);
            let tb = std::time::Instant::now();
            SegmentIndex::build(&pv, &c.records).unwrap();
            let build = tb.elapsed().as_secs_f64();
            let rec_bytes: usize = c.records.iter().map(Vec::len).sum();
            println!(
                "{name}: chain {} blocks {}-{} ({n}), {} logs, {:.1} MB of records, index \
                 rebuild {build:.2}s, {:.1}s",
                c.chain_id,
                c.start,
                c.start + end as u64,
                claimed.len(),
                rec_bytes as f64 / 1e6,
                t.elapsed().as_secs_f64()
            );
            seen.push(c.chain_id);
        }
    }
    assert!(!seen.is_empty(), "no blobs under {}", dir.display());
}

fn record_json_log(v: &Value) -> Log {
    let b = |k: &str| hex::decode(v[k].as_str().unwrap().strip_prefix("0x").unwrap()).unwrap();
    Log {
        block: v["block_number"].as_u64().unwrap(),
        tx_index: v["tx_index"].as_u64().unwrap() as u32,
        log_index: v["log_index"].as_u64().unwrap() as u32,
        address: b("address").try_into().unwrap(),
        topics: v["topics"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| {
                hex::decode(t.as_str().unwrap().strip_prefix("0x").unwrap())
                    .unwrap()
                    .try_into()
                    .unwrap()
            })
            .collect(),
        data: b("data"),
    }
}
