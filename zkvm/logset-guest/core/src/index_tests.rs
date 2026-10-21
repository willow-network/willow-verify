//! Sorted-index tests: the binding (index multiset == log set), the adversarial
//! hints, and the query range proofs. Every rejection is a real `Err` from the
//! code the index guest runs.

use crate::blob::LogFilter;
use crate::erc::{block_logs_hash, log_set_root, push_log_record, LogRecord};
use crate::error::Error;
use crate::index::*;
use crate::synth::{build_witness, OwnedWitness, Rng, SynthBlock, SynthLog, SynthReceipt};

const START: u64 = 26_065_070;
const PARENT: [u8; 32] = [0x5a; 32];

fn usdc() -> [u8; 20] {
    let mut a = [0u8; 20];
    a.copy_from_slice(&hex::decode("a0b86991c6218b36c1d19d4a2e9eb0ce3606eb48").unwrap());
    a
}
fn transfer() -> [u8; 32] {
    let mut t = [0u8; 32];
    t.copy_from_slice(
        &hex::decode("ddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef").unwrap(),
    );
    t
}
fn approval() -> [u8; 32] {
    let mut t = [0u8; 32];
    t.copy_from_slice(
        &hex::decode("8c5be1e5ebec7d5bd14f71427d1e84f3dd0314c0f7b2291e5b200ac8c7c3b925").unwrap(),
    );
    t
}

/// A match-all range with the shapes that stress the keys: many logs of one
/// (emitter, topic0), zero-topic logs, a zero-valued topic0, an empty block, a
/// block with receipts but no logs, and random noise.
fn blocks(n_blocks: usize, seed: u64) -> Vec<SynthBlock> {
    let mut rng = Rng(seed);
    let mut out = Vec::new();
    for b in 0..n_blocks {
        let mut receipts = Vec::new();
        if b % 5 == 3 {
            out.push(SynthBlock::default());
            continue;
        }
        let n_rx = 1 + (rng.next_u64() % 6) as usize;
        for t in 0..n_rx {
            let n_logs = (rng.next_u64() % 4) as usize;
            let mut logs = Vec::new();
            for _ in 0..n_logs {
                let pick = rng.next_u64() % 7;
                let lg = match pick {
                    0 | 1 => SynthLog {
                        emitter: usdc(),
                        topics: vec![transfer(), rng.bytes(), rng.bytes()],
                        data: rng.bytes::<32>().to_vec(),
                    },
                    2 => SynthLog {
                        emitter: usdc(),
                        topics: vec![approval(), rng.bytes(), rng.bytes()],
                        data: rng.bytes::<32>().to_vec(),
                    },
                    3 => SynthLog {
                        emitter: usdc(),
                        topics: vec![],
                        data: vec![7; (rng.next_u64() % 40) as usize],
                    },
                    4 => SynthLog {
                        emitter: usdc(),
                        topics: vec![[0u8; 32]],
                        data: vec![],
                    },
                    5 => SynthLog {
                        emitter: rng.bytes(),
                        topics: vec![transfer(), rng.bytes()],
                        data: rng.bytes::<64>().to_vec(),
                    },
                    _ => SynthLog {
                        emitter: rng.bytes(),
                        topics: (0..(rng.next_u64() % 5)).map(|_| rng.bytes()).collect(),
                        data: rng.bytes::<16>().to_vec(),
                    },
                };
                logs.push(lg);
            }
            receipts.push(SynthReceipt {
                tx_type: (t % 3) as u8,
                status: true,
                cum_gas: 21_000 * (t as u64 + 1),
                logs,
                deposit_tail: vec![],
            });
        }
        out.push(SynthBlock {
            receipts,
            ..Default::default()
        });
    }
    out
}

fn witness(n_blocks: usize, seed: u64) -> (OwnedWitness, Vec<SynthBlock>) {
    let bs = blocks(n_blocks, seed);
    (
        build_witness(1, START, PARENT, vec![LogFilter::default()], &bs),
        bs,
    )
}

/// The ORACLE: every log as `(block, record bytes)`, built from the synthetic
/// blocks directly (never from the guest's scan).
fn oracle_records(bs: &[SynthBlock]) -> Vec<(u64, Vec<u8>)> {
    let mut out = Vec::new();
    for (i, b) in bs.iter().enumerate() {
        let mut li = 0u32;
        for (tx, r) in b.receipts.iter().enumerate() {
            for l in &r.logs {
                let mut rec = Vec::new();
                push_log_record(&mut rec, tx as u32, li, &l.emitter, &l.topics, &l.data);
                out.push((START + i as u64, rec));
                li += 1;
            }
        }
    }
    out
}

/// Oracle key: the 69 key bytes spelled out, compared as a byte string.
fn oracle_key(order: Order, block: u64, rec: &[u8]) -> Vec<u8> {
    let emitter = &rec[8..28];
    let nt = rec[28];
    let (flag, t0): (u8, &[u8]) = if nt > 0 {
        (1, &rec[29..61])
    } else {
        (0, &[0u8; 32][..])
    };
    let mut k = Vec::new();
    match order {
        Order::ByEmitter => {
            k.extend_from_slice(emitter);
            k.push(flag);
            k.extend_from_slice(t0);
        }
        Order::ByTopic0 => {
            k.extend_from_slice(t0);
            k.push(flag);
            k.extend_from_slice(&emitter[16..20]);
            k.extend_from_slice(&emitter[0..16]);
        }
    }
    k.extend_from_slice(&block.to_be_bytes());
    k.extend_from_slice(&rec[0..8]);
    k
}

/// Naive padded tree, no zero cache.
fn naive_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    if leaves.is_empty() {
        return [0u8; 32];
    }
    let mut cap = 1;
    while cap < leaves.len() {
        cap *= 2;
    }
    let mut lv = leaves.to_vec();
    lv.resize(cap, [0u8; 32]);
    while lv.len() > 1 {
        lv = lv.chunks(2).map(|c| index_node(&c[0], &c[1])).collect();
    }
    lv[0]
}

fn oracle_root(order: Order, recs: &[(u64, Vec<u8>)]) -> [u8; 32] {
    let mut v: Vec<(Vec<u8>, [u8; 32])> = recs
        .iter()
        .map(|(b, r)| (oracle_key(order, *b, r), leaf_hash(*b, r)))
        .collect();
    v.sort();
    naive_root(&v.iter().map(|x| x.1).collect::<Vec<_>>())
}

fn entries_of(blob: &[u8]) -> (IndexBuilder, u64) {
    let mut ix = IndexBuilder::new();
    ix.keep_records = true;
    let st = run_statements_core(blob, Some(&mut ix)).expect("honest");
    (ix, st[0].log_count)
}

fn run_with(blob: &[u8], hint: &[u8]) -> Result<IndexCommitment, Error> {
    run_statements_indexed(blob, hint, &mut IndexBuilder::new()).map(|(_, c)| c)
}

// ---------------------------------------------------------------------------
// Binding: the index holds exactly the logs of logSetRoot
// ---------------------------------------------------------------------------

#[test]
fn honest_index_matches_oracle_and_log_set() {
    for (n_blocks, seed) in [(1, 1), (2, 2), (7, 3), (16, 4), (33, 5)] {
        let (w, bs) = witness(n_blocks, seed);
        let blob = w.encode();
        let recs = oracle_records(&bs);
        let (ix, log_count) = entries_of(&blob);
        let hint = build_hint(&ix.entries);
        let (st, com) =
            run_statements_indexed(&blob, &hint, &mut IndexBuilder::new()).expect("honest");
        // Same statement as the default guest, byte for byte.
        assert_eq!(st, crate::run_statements(&blob).unwrap());
        // Count binding.
        assert_eq!(com.entry_count, recs.len() as u64);
        assert_eq!(log_count, recs.len() as u64);
        // Multiset equality, independently: the index's leaves == the oracle's.
        let mut got: Vec<[u8; 32]> = ix.entries.iter().map(|e| e.leaf).collect();
        let mut want: Vec<[u8; 32]> = recs.iter().map(|(b, r)| leaf_hash(*b, r)).collect();
        got.sort();
        want.sort();
        assert_eq!(got, want, "index multiset != log set ({n_blocks} blocks)");
        // And the oracle's records ARE the logSetRoot's records.
        let mut per_block: Vec<Vec<LogRecord>> = vec![Vec::new(); n_blocks];
        for (b, r) in &recs {
            let nt = r[28] as usize;
            let dl = 33 + 32 * nt;
            per_block[(*b - START) as usize].push(LogRecord {
                tx_index: u32::from_be_bytes(r[0..4].try_into().unwrap()),
                log_index: u32::from_be_bytes(r[4..8].try_into().unwrap()),
                emitter: r[8..28].try_into().unwrap(),
                topics: (0..nt)
                    .map(|i| r[29 + 32 * i..61 + 32 * i].try_into().unwrap())
                    .collect(),
                data: r[dl..].to_vec(),
            });
        }
        let leaves: Vec<[u8; 32]> = per_block
            .iter()
            .enumerate()
            .map(|(i, rs)| block_logs_hash(START + i as u64, rs))
            .collect();
        assert_eq!(st[0].log_set_root, log_set_root(&leaves).unwrap());
        // Roots: hint path == guest-sort path == naive oracle.
        assert_eq!(com.root_by_emitter, oracle_root(Order::ByEmitter, &recs));
        assert_eq!(com.root_by_topic0, oracle_root(Order::ByTopic0, &recs));
        assert_eq!(run_with(&blob, &[]).unwrap(), com);
        // The public values: the default pv, then the commitment.
        let pv = run_indexed(&blob, &hint).unwrap();
        let base = crate::run(&blob).unwrap();
        assert_eq!(&pv[..base.len()], &base[..]);
        assert_eq!(IndexCommitment::decode(&pv[base.len()..]), Some(com));
    }
}

#[test]
fn key_order_is_byte_order() {
    let (w, bs) = witness(20, 9);
    let recs = oracle_records(&bs);
    let _ = w;
    for o in [Order::ByEmitter, Order::ByTopic0] {
        for (b1, r1) in recs.iter().take(60) {
            for (b2, r2) in recs.iter().take(60) {
                let h1 = parse_record(r1).unwrap();
                let h2 = parse_record(r2).unwrap();
                let k1 = make_key(o, &h1.emitter, h1.topic0.as_ref(), *b1, h1.tx, h1.log);
                let k2 = make_key(o, &h2.emitter, h2.topic0.as_ref(), *b2, h2.tx, h2.log);
                if o == Order::ByEmitter {
                    assert_eq!(
                        topic0_key_from(&k1),
                        make_key(
                            Order::ByTopic0,
                            &h1.emitter,
                            h1.topic0.as_ref(),
                            *b1,
                            h1.tx,
                            h1.log
                        )
                    );
                }
                assert_eq!(
                    k1.cmp(&k2),
                    oracle_key(o, *b1, r1).cmp(&oracle_key(o, *b2, r2))
                );
            }
        }
    }
}

#[test]
fn lane_keccak_matches_tiny_keccak() {
    let mut rng = Rng(3);
    let data: Vec<u8> = (0..700).map(|_| rng.next_u64() as u8).collect();
    for len in 0..700 {
        for split in [0usize, 1, 7, 8, 13, 136, 137] {
            let split = split.min(len);
            let mut k = Keccak256::new();
            k.update(&data[..split]);
            k.update(&data[split..len]);
            assert_eq!(
                k.finalize(),
                crate::keccak256(&data[..len]),
                "len {len} split {split}"
            );
        }
    }
    let (l, r) = (rng.bytes::<32>(), rng.bytes::<32>());
    let mut pre = INDEX_NODE_TAG.to_vec();
    pre.extend_from_slice(&l);
    pre.extend_from_slice(&r);
    assert_eq!(index_node(&l, &r), crate::keccak256(&pre));
}

#[test]
fn empty_range_has_zero_root() {
    let bs = vec![SynthBlock::default(); 3];
    let w = build_witness(1, START, PARENT, vec![LogFilter::default()], &bs);
    let c = run_with(&w.encode(), &build_hint(&[])).unwrap();
    assert_eq!(c.entry_count, 0);
    assert_eq!(c.root_by_emitter, [0u8; 32]);
    assert_eq!(run_with(&w.encode(), &[]).unwrap(), c);
}

#[test]
fn tree_padding_matches_naive() {
    let mut rng = Rng(77);
    for n in 0..=17 {
        let leaves: Vec<[u8; 32]> = (0..n).map(|_| rng.bytes()).collect();
        assert_eq!(index_root(&leaves), naive_root(&leaves), "n = {n}");
    }
}

#[test]
fn needs_the_match_all_filter() {
    let bs = blocks(4, 11);
    let narrow = LogFilter {
        addresses: vec![usdc()],
        ..Default::default()
    };
    for filters in [
        vec![narrow.clone()],
        vec![LogFilter::default(), narrow.clone()],
        vec![narrow, LogFilter::default()],
    ] {
        let w = build_witness(1, START, PARENT, filters, &bs);
        assert_eq!(run_with(&w.encode(), &[]), Err(Error::IndexNeedsMatchAll));
    }
}

// ---------------------------------------------------------------------------
// Adversarial hints — each next to its honest control
// ---------------------------------------------------------------------------

fn hint_parts(h: &[u8]) -> (u32, Vec<u32>, Vec<u32>) {
    let n = u32::from_le_bytes(h[1..5].try_into().unwrap());
    let body = &h[5..];
    let rd = |s: &[u8]| -> Vec<u32> {
        s.chunks(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect()
    };
    (n, rd(&body[..4 * n as usize]), rd(&body[4 * n as usize..]))
}

fn make_hint(n: u32, pe: &[u32], pt: &[u32]) -> Vec<u8> {
    let mut h = vec![HINT_MODE_PERM];
    h.extend_from_slice(&n.to_le_bytes());
    for p in pe.iter().chain(pt.iter()) {
        h.extend_from_slice(&p.to_le_bytes());
    }
    h
}

#[test]
fn attack_hints() {
    let (w, _) = witness(16, 21);
    let blob = w.encode();
    let (ix, _) = entries_of(&blob);
    let honest = build_hint(&ix.entries);
    let (n, pe, pt) = hint_parts(&honest);
    assert!(n > 20);
    assert!(run_with(&blob, &honest).is_ok(), "control");

    // Dropped entry, count adjusted: the scan's count wins.
    let mut pe_d = pe.clone();
    pe_d.remove(5);
    let mut pt_d = pt.clone();
    let gone = pe[5];
    pt_d.retain(|p| *p != gone);
    assert_eq!(
        run_with(&blob, &make_hint(n - 1, &pe_d, &pt_d)),
        Err(Error::IndexHintCount {
            declared: n as u64 - 1,
            entries: n as u64
        })
    );
    // Dropped entry, count kept: its slot re-used for a duplicate.
    let mut pe_dup = pe.clone();
    pe_dup[6] = pe_dup[5];
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe_dup, &pt)),
        Err(Error::IndexPermDuplicate { order: 0, at: 6 })
    );
    // Duplicated entry in the second ordering.
    let mut pt_dup = pt.clone();
    pt_dup[n as usize - 1] = pt_dup[0];
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe, &pt_dup)),
        Err(Error::IndexPermDuplicate {
            order: 1,
            at: n - 1
        })
    );
    // Mis-sorted adjacent pair, in each ordering.
    let mut pe_sw = pe.clone();
    pe_sw.swap(10, 11);
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe_sw, &pt)),
        Err(Error::IndexNotAscending { order: 0, at: 11 })
    );
    let mut pt_sw = pt.clone();
    pt_sw.swap(0, 1);
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe, &pt_sw)),
        Err(Error::IndexNotAscending { order: 1, at: 1 })
    );
    // A whole-list reversal (a valid permutation, wrong order).
    let rev: Vec<u32> = pe.iter().rev().copied().collect();
    assert_eq!(
        run_with(&blob, &make_hint(n, &rev, &pt)),
        Err(Error::IndexNotAscending { order: 0, at: 1 })
    );
    // Forged entry: the only way to name an entry the scan did not produce is an
    // index past the end.
    let mut pe_f = pe.clone();
    pe_f[3] = n;
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe_f, &pt)),
        Err(Error::IndexPermOutOfRange { order: 0, at: 3 })
    );
    let mut pt_f = pt.clone();
    pt_f[0] = u32::MAX;
    assert_eq!(
        run_with(&blob, &make_hint(n, &pe, &pt_f)),
        Err(Error::IndexPermOutOfRange { order: 1, at: 0 })
    );
    // Forged entry, count inflated: the scan's count wins.
    let mut pe_x = pe.clone();
    pe_x.push(n);
    let mut pt_x = pt.clone();
    pt_x.push(n);
    assert_eq!(
        run_with(&blob, &make_hint(n + 1, &pe_x, &pt_x)),
        Err(Error::IndexHintCount {
            declared: n as u64 + 1,
            entries: n as u64
        })
    );
    // Framing.
    let mut trailing = honest.clone();
    trailing.push(0);
    assert_eq!(
        run_with(&blob, &trailing),
        Err(Error::IndexHintMalformed { what: "length" })
    );
    let mut short = honest.clone();
    short.pop();
    assert_eq!(
        run_with(&blob, &short),
        Err(Error::IndexHintMalformed { what: "length" })
    );
    let mut mode = honest.clone();
    mode[0] = 2;
    assert_eq!(
        run_with(&blob, &mode),
        Err(Error::IndexHintMalformed { what: "mode" })
    );
    assert_eq!(
        run_with(&blob, &[HINT_MODE_PERM, 0, 0]),
        Err(Error::IndexHintMalformed { what: "short" })
    );
    // A hint for a DIFFERENT range's entries (right length, valid for that range)
    // cannot be satisfied here either.
    let (w2, _) = witness(16, 22);
    let (ix2, _) = entries_of(&w2.encode());
    if ix2.entries.len() == ix.entries.len() {
        assert!(run_with(&blob, &build_hint(&ix2.entries)).is_err());
    }
}

/// The hint can only choose an order, and there is one valid order: whatever
/// the hint, an accepted run commits the same roots.
#[test]
fn accepted_roots_do_not_depend_on_the_hint() {
    let (w, _) = witness(12, 31);
    let blob = w.encode();
    let (ix, _) = entries_of(&blob);
    let good = run_with(&blob, &build_hint(&ix.entries)).unwrap();
    assert_eq!(run_with(&blob, &[]).unwrap(), good);
    let (n, pe, pt) = hint_parts(&build_hint(&ix.entries));
    let mut rng = Rng(5);
    for _ in 0..200 {
        let mut a = pe.clone();
        let i = (rng.next_u64() % n as u64) as usize;
        let j = (rng.next_u64() % n as u64) as usize;
        a.swap(i, j);
        match run_with(&blob, &make_hint(n, &a, &pt)) {
            Ok(c) => {
                assert_eq!(i, j);
                assert_eq!(c, good);
            }
            Err(Error::IndexNotAscending { .. }) => assert_ne!(i, j),
            Err(e) => panic!("unexpected {e:?}"),
        }
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

struct Built {
    com: IndexCommitment,
    keys: [Vec<Key>; 2],
    recs: [Vec<(u64, Vec<u8>)>; 2],
    levels: [Vec<Vec<[u8; 32]>>; 2],
}

fn built(blob: &[u8]) -> Built {
    let (ix, _) = entries_of(blob);
    let com = run_with(blob, &build_hint(&ix.entries)).unwrap();
    let mut keys: [Vec<Key>; 2] = [Vec::new(), Vec::new()];
    let mut recs: [Vec<(u64, Vec<u8>)>; 2] = [Vec::new(), Vec::new()];
    let mut levels: [Vec<Vec<[u8; 32]>>; 2] = [Vec::new(), Vec::new()];
    for (oi, o) in [Order::ByEmitter, Order::ByTopic0].into_iter().enumerate() {
        let perm = sorted_perm(&ix.entries, o);
        keys[oi] = perm
            .iter()
            .map(|&p| *ix.entries[p as usize].key(o))
            .collect();
        recs[oi] = perm
            .iter()
            .map(|&p| ix.records[p as usize].clone())
            .collect();
        let leaves: Vec<[u8; 32]> = perm.iter().map(|&p| ix.entries[p as usize].leaf).collect();
        levels[oi] = tree_levels(&leaves);
        assert_eq!(levels[oi].last().unwrap()[0], *com.root(o));
    }
    Built {
        com,
        keys,
        recs,
        levels,
    }
}

fn naive_match(
    recs: &[(u64, Vec<u8>)],
    order: Order,
    kmin: &Key,
    kmax: &Key,
) -> Vec<(u64, Vec<u8>)> {
    let mut v: Vec<(Key, (u64, Vec<u8>))> = recs
        .iter()
        .filter_map(|(b, r)| {
            let h = parse_record(r).unwrap();
            let k = make_key(order, &h.emitter, h.topic0.as_ref(), *b, h.tx, h.log);
            (k >= *kmin && k <= *kmax).then(|| (k, (*b, r.clone())))
        })
        .collect();
    v.sort();
    v.into_iter().map(|x| x.1).collect()
}

fn ranges() -> Vec<(Order, Key, Key)> {
    let mut v = vec![
        {
            let (a, b) = key_range(Order::ByEmitter, &usdc(), Some(&transfer()));
            (Order::ByEmitter, a, b)
        },
        {
            let (a, b) = key_range(Order::ByEmitter, &usdc(), Some(&approval()));
            (Order::ByEmitter, a, b)
        },
        {
            let (a, b) = key_range(Order::ByEmitter, &usdc(), None);
            (Order::ByEmitter, a, b)
        },
        {
            // every USDC log: from no-topic0 through the largest topic0
            let (a, _) = key_range(Order::ByEmitter, &usdc(), None);
            let (_, b) = key_range(Order::ByEmitter, &usdc(), Some(&[0xff; 32]));
            (Order::ByEmitter, a, b)
        },
        {
            let (a, b) = key_range(Order::ByEmitter, &[0x11; 20], Some(&transfer()));
            (Order::ByEmitter, a, b) // absent: empty result
        },
        {
            let (a, b) = key_range(Order::ByEmitter, &[0x00; 20], None);
            (Order::ByEmitter, a, b) // below everything
        },
        {
            let (a, b) = key_range(Order::ByEmitter, &[0xff; 20], Some(&[0xff; 32]));
            (Order::ByEmitter, a, b) // above everything
        },
    ];
    // topic0-only filters, served by the second ordering
    for t in [transfer(), approval(), [0u8; 32], [0x01; 32]] {
        let (_, _) = (0, 0);
        let lo = make_key(Order::ByTopic0, &[0u8; 20], Some(&t), 0, 0, 0);
        let hi = make_key(
            Order::ByTopic0,
            &[0xff; 20],
            Some(&t),
            u64::MAX,
            u32::MAX,
            u32::MAX,
        );
        v.push((Order::ByTopic0, lo, hi));
    }
    v
}

#[test]
fn query_results_are_complete_and_verify() {
    for (n_blocks, seed) in [(1, 41), (5, 42), (40, 43)] {
        let (w, bs) = witness(n_blocks, seed);
        let b = built(&w.encode());
        let all = oracle_records(&bs);
        for (o, kmin, kmax) in ranges() {
            let oi = o as usize;
            let p = prove_range(&b.keys[oi], &b.recs[oi], &b.levels[oi], &kmin, &kmax);
            let got = verify_range(b.com.root(o), b.com.entry_count, o, &kmin, &kmax, &p)
                .unwrap_or_else(|e| panic!("{n_blocks} blocks, {o:?}: {e}"));
            assert_eq!(got, naive_match(&all, o, &kmin, &kmax));
        }
    }
}

#[test]
fn attack_query_proofs() {
    let (w, bs) = witness(40, 51);
    let b = built(&w.encode());
    let all = oracle_records(&bs);
    let o = Order::ByEmitter;
    let (kmin, kmax) = key_range(o, &usdc(), Some(&transfer()));
    let p = prove_range(&b.keys[0], &b.recs[0], &b.levels[0], &kmin, &kmax);
    let root = *b.com.root(o);
    let n = b.com.entry_count;
    let want = naive_match(&all, o, &kmin, &kmax);
    assert!(want.len() >= 4, "fixture needs several matches");
    assert_eq!(
        verify_range(&root, n, o, &kmin, &kmax, &p).unwrap(),
        want,
        "control"
    );
    let v = |p: &RangeProof| verify_range(&root, n, o, &kmin, &kmax, p);

    // Drop a result from the middle: the run is no longer the committed run.
    let mut q = p.clone();
    q.records.remove(2);
    assert!(v(&q).is_err());
    // Drop the right neighbour: the run now ends inside the range.
    let mut q = p.clone();
    q.records.pop();
    assert!(v(&q).is_err());
    // Drop the left neighbour and shift lo: the run now starts inside the range.
    let mut q = p.clone();
    q.records.remove(0);
    q.lo += 1;
    assert!(v(&q).is_err());
    // Forge a result's data.
    let mut q = p.clone();
    let last = q.records[2].1.len() - 1;
    q.records[2].1[last] ^= 1;
    assert!(v(&q).is_err());
    // Forge a result's block number.
    let mut q = p.clone();
    q.records[2].0 += 1;
    assert!(v(&q).is_err());
    // Claim another position.
    let mut q = p.clone();
    q.lo += 1;
    assert!(v(&q).is_err());
    // Tamper / drop / add a sibling.
    let mut q = p.clone();
    q.siblings[0][0] ^= 1;
    assert!(v(&q).is_err());
    let mut q = p.clone();
    q.siblings.pop();
    assert!(v(&q).is_err());
    let mut q = p.clone();
    q.siblings.push([0u8; 32]);
    assert!(v(&q).is_err());
    // Wrong root.
    assert!(verify_range(b.com.root(Order::ByTopic0), n, o, &kmin, &kmax, &p).is_err());
    // `entryCount` is what proves the END of the list, so a run that ends there
    // fails under any other count (mid-list runs never consult it).
    let (tmin, tmax) = key_range(o, &[0xff; 20], Some(&[0xff; 32]));
    let pe = prove_range(&b.keys[0], &b.recs[0], &b.levels[0], &tmin, &tmax);
    assert_eq!(pe.lo, n - 1);
    assert!(verify_range(&root, n, o, &tmin, &tmax, &pe)
        .unwrap()
        .is_empty());
    assert!(verify_range(&root, n + 1, o, &tmin, &tmax, &pe).is_err());
    assert!(verify_range(&root, n - 1, o, &tmin, &tmax, &pe).is_err());
    // A correct proof of a NARROWER range cannot pass for the wider one: its
    // neighbours sit inside the wider range.
    let (wmin, wmax) = key_range(o, &usdc(), None);
    let (_, wmax2) = key_range(o, &usdc(), Some(&[0xff; 32]));
    let _ = wmax;
    assert!(verify_range(&root, n, o, &wmin, &wmax2, &p).is_err());
    // Malformed record bytes.
    let mut q = p.clone();
    q.records[1].1.push(0);
    assert_eq!(v(&q), Err(Error::IndexRecordMalformed));
}
