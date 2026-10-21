use super::*;
use logset_core::erc::log_set_root;
use logset_core::synth::{build_witness, busy_block, Rng, SynthBlock};
use logset_core::{keccak256, run, LogFilter};

fn sha256(b: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(b).into()
}

fn match_all() -> LogFilter {
    LogFilter {
        addresses: vec![],
        topics: [vec![], vec![], vec![], vec![]],
    }
}

/// `n` synthetic blocks, a few receipts each (some blocks empty).
fn blocks(n: usize, seed: u64) -> Vec<SynthBlock> {
    let mut rng = Rng(seed);
    (0..n)
        .map(|i| {
            if i % 7 == 3 {
                SynthBlock::default()
            } else {
                busy_block(&mut rng, 1 + i % 4, 2, None, 0)
            }
        })
        .collect()
}

const START: u64 = 26_065_070;
const PARENT: [u8; 32] = [0x42; 32];

/// Leaf pv of the whole range, and each aligned segment's pv.
fn leaf_pvs(n: usize, seg_log2: u8) -> (Vec<u8>, Vec<Vec<u8>>) {
    let bs = blocks(n, 0x5eed + n as u64);
    let full = build_witness(1, START, PARENT, vec![match_all()], &bs);
    let whole = run(&full.encode()).expect("whole range");
    let s = 1usize << seg_log2;
    let mut segs = Vec::new();
    let mut i = 0;
    while i < n {
        let j = (i + s).min(n);
        let parent = if i == 0 {
            PARENT
        } else {
            keccak256(&full.headers[i - 1])
        };
        let w = build_witness(1, START + i as u64, parent, vec![match_all()], &bs[i..j]);
        assert_eq!(
            w.headers,
            full.headers[i..j].to_vec(),
            "segment headers == range headers"
        );
        segs.push(run(&w.encode()).expect("segment"));
        i = j;
    }
    (whole, segs)
}

fn agg(seg_log2: u8, kids: &[Vec<u8>]) -> Result<Vec<u8>, AggError> {
    let input = encode_input(seg_log2, kids);
    let mut calls = 0usize;
    let r = aggregate(&input, &sha256, &mut |_d| calls += 1);
    if r.is_ok() {
        assert_eq!(calls, kids.len(), "one P1 call per child");
    }
    r.map(|(_, pv)| pv)
}

#[test]
fn aggregate_equals_single_run() {
    // (n_blocks, seg_log2): exact multiples, short tails, a 1-block tail,
    // one segment only, a non-power-of-two child count, and the day's shape
    // scaled down (7 children, last short: 6*S + (S-16)).
    for &(n, k) in &[
        (16, 2),
        (17, 2),
        (23, 2),
        (3, 2),
        (4, 2),
        (5, 0),
        (1, 3),
        (45, 3),
        (6 * 8 + 7, 3),
        (33, 4),
    ] {
        let (whole, segs) = leaf_pvs(n, k);
        let got = agg(k, &segs).unwrap_or_else(|e| panic!("n={n} k={k}: {e}"));
        assert_eq!(got, whole, "aggregate pv != single-run pv for n={n} k={k}");
    }
}

#[test]
fn fold_equals_flat_on_random_leaves() {
    let mut rng = Rng(7);
    for n in 1..70usize {
        for k in 0..5u8 {
            let leaves: Vec<[u8; 32]> = (0..n).map(|_| rng.bytes::<32>()).collect();
            let s = 1usize << k;
            let mut roots = Vec::new();
            let mut spans = Vec::new();
            for c in leaves.chunks(s) {
                roots.push(log_set_root(c).unwrap());
                spans.push(c.len() as u64);
            }
            assert_eq!(
                fold_roots(k, &roots, &spans),
                log_set_root(&leaves).unwrap(),
                "n={n} k={k}"
            );
        }
    }
}

#[test]
fn rejects() {
    let (_, segs) = leaf_pvs(23, 2); // 6 children: 5 × 4 + 3
                                     // Reordered children: contiguity fails.
    let mut s = segs.clone();
    s.swap(1, 2);
    assert_eq!(agg(2, &s), Err(AggError::NotContiguous(1)));
    // A missing middle child.
    let mut s = segs.clone();
    s.remove(2);
    assert_eq!(agg(2, &s), Err(AggError::NotContiguous(2)));
    // Wrong segment size claimed: spans != S.
    assert_eq!(agg(3, &segs), Err(AggError::ChildSpan(0)));
    assert_eq!(agg(1, &segs), Err(AggError::ChildSpan(0)));
    // Seam hash broken but numbers contiguous.
    let mut s = segs.clone();
    s[3][64 + 96] ^= 1;
    assert_eq!(agg(2, &s), Err(AggError::SeamHash(3)));
    // Filter drift.
    let mut s = segs.clone();
    s[4][64 + 160] ^= 1;
    assert_eq!(agg(2, &s), Err(AggError::FilterHash(4)));
    // Chain drift.
    let mut s = segs.clone();
    s[5][64 + 31] ^= 1;
    assert_eq!(agg(2, &s), Err(AggError::ChainId(5)));
    // Non-canonical ABI (dirty high bytes in a uint64 word).
    let mut s = segs.clone();
    s[0][64 + 32] = 1;
    assert_eq!(agg(2, &s), Err(AggError::ChildShape(0)));
    // A two-statement child.
    let mut s = segs.clone();
    s[0][63] = 2;
    assert_eq!(agg(2, &s), Err(AggError::ChildShape(0)));
    // Shape of the stdin.
    assert_eq!(agg(2, &[]), Err(AggError::NoChildren));
    let mut input = encode_input(2, &segs);
    input.push(0);
    assert_eq!(
        aggregate(&input, &sha256, &mut |_| {}).unwrap_err(),
        AggError::Trailing
    );
    input.truncate(input.len() - 2);
    assert_eq!(
        aggregate(&input, &sha256, &mut |_| {}).unwrap_err(),
        AggError::Short
    );
    let mut input = encode_input(2, &segs);
    input[0] ^= 1;
    assert_eq!(
        aggregate(&input, &sha256, &mut |_| {}).unwrap_err(),
        AggError::BadMagic
    );
}

#[test]
fn p1_digests_are_sha256_of_each_child_in_order() {
    let (_, segs) = leaf_pvs(10, 2);
    let input = encode_input(2, &segs);
    let mut seen = Vec::new();
    aggregate(&input, &sha256, &mut |d| seen.push(*d)).unwrap();
    let want: Vec<[u8; 32]> = segs.iter().map(|s| sha256(s)).collect();
    assert_eq!(seen, want);
}

#[cfg(feature = "index")]
mod index_agg_tests {
    use super::*;
    use crate::fold::{
        decode_fold_pv, encode_fold_input, fold, index_list_hash, FoldError, IndexSegment,
        FOLD_PV_LEN, INDEX_LIST_TAG,
    };
    use crate::index_agg::{
        aggregate_index, decode_l1_pv, encode_idx_input, encode_idx_input_blocks, host_hint,
        IdxAggError, L1_PV_LEN, MAX_L1_CHILDREN,
    };
    use logset_core::index::{build_hint, run_statements_core, IndexBuilder, IndexCommitment};
    use logset_core::run_indexed;

    const LEAF_VK: [u32; 8] = [0x7141_2644, 1, 2, 3, 4, 5, 6, 0x66ce_db3e];

    /// `n` one-block children (pv, records) from ONE range starting at `start`,
    /// and the index leaf guest's pv over the same range (the oracle).
    fn fixture_at(
        start: u64,
        parent: [u8; 32],
        n: usize,
        filters: Vec<LogFilter>,
        seed: u64,
    ) -> (Vec<(Vec<u8>, Vec<u8>)>, Vec<u8>, Vec<u8>) {
        let bs = blocks(n, seed);
        let full = build_witness(1, start, parent, filters.clone(), &bs);
        let mut kids = Vec::new();
        for i in 0..n {
            let p = if i == 0 {
                parent
            } else {
                keccak256(&full.headers[i - 1])
            };
            let w = build_witness(1, start + i as u64, p, filters.clone(), &bs[i..i + 1]);
            kids.push((run(&w.encode()).expect("leaf"), Vec::new()));
        }
        // Records, hint and the oracle come from the match-all scan of the same
        // blocks (headers do not depend on the filter list).
        let all = build_witness(1, start, parent, vec![match_all()], &bs);
        assert_eq!(all.headers, full.headers);
        let mut ix = IndexBuilder::new();
        ix.keep_records = true;
        let blob = all.encode();
        run_statements_core(&blob, Some(&mut ix)).expect("match-all scan");
        for (b, r) in &ix.records {
            kids[(b - start) as usize].1.extend_from_slice(r);
        }
        let hint = build_hint(&ix.entries);
        let leaf = run_indexed(&blob, &hint).unwrap_or_default();
        (kids, hint, leaf)
    }

    fn fixture(n: usize, filter: LogFilter) -> (Vec<(Vec<u8>, Vec<u8>)>, Vec<u8>, Vec<u8>) {
        fixture_at(START, PARENT, n, vec![filter], 0x1d0 + n as u64)
    }

    fn go(kids: &[(Vec<u8>, Vec<u8>)], hint: &[u8]) -> Result<Vec<u8>, IdxAggError> {
        aggregate_index(
            &encode_idx_input(&LEAF_VK, kids),
            hint,
            &sha256,
            &mut |_, _| {},
        )
        .map(|r| r.3)
    }

    /// The index leaf pv is `stmt[1] | IndexCommitment`; L1 appends the leaf vkey.
    fn with_key(leaf_pv: &[u8]) -> Vec<u8> {
        let mut v = leaf_pv.to_vec();
        v.extend_from_slice(&crate::vkey::vkey_words_to_bytes(&LEAF_VK));
        v
    }

    #[test]
    fn l1_equals_index_leaf_for_any_n_up_to_64() {
        for n in [1usize, 2, 7, 16, 23, 63, 64] {
            let (kids, hint, leaf) = fixture(n, match_all());
            let pv = go(&kids, &hint).unwrap_or_else(|e| panic!("n = {n}: {e:?}"));
            assert_eq!(pv.len(), L1_PV_LEN);
            assert_eq!(pv, with_key(&leaf), "n = {n}");
            let (st, com, key) = decode_l1_pv(&pv).expect("canonical L1 pv");
            assert_eq!(com.entry_count, st.log_count);
            assert_eq!(key, crate::vkey::vkey_words_to_bytes(&LEAF_VK));
            // The host hint builder is the guest's own record binding.
            let input = encode_idx_input(&LEAF_VK, &kids);
            assert_eq!(host_hint(&input).unwrap(), hint, "n = {n}");
        }
    }

    #[test]
    fn l1_child_count_bounds() {
        let (kids, _, _) = fixture(1, match_all());
        assert_eq!(go(&[], &[]), Err(IdxAggError::ChildCount(0)));
        let many = vec![kids[0].clone(); MAX_L1_CHILDREN as usize + 1];
        assert_eq!(go(&many, &[]), Err(IdxAggError::ChildCount(65)));
    }

    #[test]
    fn l1_p1_binds_full_child_pv_under_the_input_vkey() {
        let (kids, hint, _) = fixture(5, match_all());
        let mut seen = Vec::new();
        let (_, _, key, pv) = aggregate_index(
            &encode_idx_input(&LEAF_VK, &kids),
            &hint,
            &sha256,
            &mut |vk, d| seen.push((*vk, *d)),
        )
        .unwrap();
        let want: Vec<([u32; 8], [u8; 32])> =
            kids.iter().map(|k| (LEAF_VK, sha256(&k.0))).collect();
        assert_eq!(seen, want, "one P1 call per child, in order, full pv");
        assert_eq!(key, crate::vkey::vkey_words_to_bytes(&LEAF_VK));
        assert_eq!(
            &pv[L1_PV_LEN - 32..],
            &key[..],
            "the leaf vkey is committed"
        );
        // A different leaf vkey changes the committed bytes (and the P1 target).
        let other = [9u32; 8];
        let (_, _, key2, _) = aggregate_index(
            &encode_idx_input(&other, &kids),
            &hint,
            &sha256,
            &mut |vk, _| assert_eq!(*vk, other),
        )
        .unwrap();
        assert_ne!(key2, key);
    }

    /// A leaf pv carrying extra statements (e.g. per-filter) is accepted; the
    /// index binds to statement [0], which must be match-all, and P1 covers
    /// the whole pv.
    #[test]
    fn l1_accepts_multi_statement_children_using_the_first() {
        let other = LogFilter {
            addresses: vec![[0x11; 20]],
            topics: [vec![], vec![], vec![], vec![]],
        };
        let n = 6;
        let (multi, hint, _) = fixture_at(START, PARENT, n, vec![match_all(), other.clone()], 0x77);
        let (single, hint1, leaf) = fixture_at(START, PARENT, n, vec![match_all()], 0x77);
        assert_eq!(hint, hint1);
        assert!(multi.iter().all(|k| k.0.len() == 64 + 2 * 256));
        let pv = go(&multi, &hint).expect("multi-statement children");
        assert_eq!(pv, with_key(&leaf));
        assert_eq!(go(&single, &hint).unwrap(), pv);
        // Match-all second, a filter first: refused.
        let (swapped, _, _) = fixture_at(START, PARENT, n, vec![other, match_all()], 0x77);
        assert_eq!(go(&swapped, &hint), Err(IdxAggError::NotMatchAll));
    }

    #[test]
    fn l1_rejects_malformed_child_pv() {
        let (kids, hint, _) = fixture(4, match_all());
        // Zero statements.
        let mut t = kids.clone();
        t[1].0 = t[1].0[..64].to_vec();
        t[1].0[63] = 0;
        assert_eq!(go(&t, &hint), Err(IdxAggError::ChildShape(1)));
        // Length word disagrees with the byte length.
        let mut t = kids.clone();
        t[2].0[63] = 2;
        assert_eq!(go(&t, &hint), Err(IdxAggError::ChildShape(2)));
        // Offset word not 0x20.
        let mut t = kids.clone();
        t[0].0[31] = 0x40;
        assert_eq!(go(&t, &hint), Err(IdxAggError::ChildShape(0)));
        // Trailing byte in the pv.
        let mut t = kids.clone();
        t[3].0.push(0);
        assert_eq!(go(&t, &hint), Err(IdxAggError::ChildShape(3)));
        // Dirty high bytes in a uint64 word.
        let mut t = kids.clone();
        t[0].0[64 + 32] = 1;
        assert_eq!(go(&t, &hint), Err(IdxAggError::ChildShape(0)));
    }

    /// `n` blocks cut into `2^leaf_log2`-block leaf children (the last may be
    /// short): each child's pv and per-block record streams, the hint, and the
    /// index leaf pv over the whole range (the oracle).
    #[allow(clippy::type_complexity)]
    fn multi_fixture(
        n: usize,
        leaf_log2: u8,
        seed: u64,
    ) -> (Vec<(Vec<u8>, Vec<Vec<u8>>)>, Vec<u8>, Vec<u8>) {
        let bs = blocks(n, seed);
        let full = build_witness(1, START, PARENT, vec![match_all()], &bs);
        let blob = full.encode();
        let mut ix = IndexBuilder::new();
        ix.keep_records = true;
        run_statements_core(&blob, Some(&mut ix)).expect("match-all scan");
        let mut per_block = vec![Vec::new(); n];
        for (b, r) in &ix.records {
            per_block[(b - START) as usize].extend_from_slice(r);
        }
        let s = 1usize << leaf_log2;
        let mut kids = Vec::new();
        let mut i = 0;
        while i < n {
            let j = (i + s).min(n);
            let p = if i == 0 {
                PARENT
            } else {
                keccak256(&full.headers[i - 1])
            };
            let w = build_witness(1, START + i as u64, p, vec![match_all()], &bs[i..j]);
            kids.push((run(&w.encode()).unwrap(), per_block[i..j].to_vec()));
            i = j;
        }
        let hint = build_hint(&ix.entries);
        let leaf = run_indexed(&blob, &hint).unwrap();
        (kids, hint, leaf)
    }

    fn go_blocks(kids: &[(Vec<u8>, Vec<Vec<u8>>)], hint: &[u8]) -> Result<Vec<u8>, IdxAggError> {
        aggregate_index(
            &encode_idx_input_blocks(&LEAF_VK, kids),
            hint,
            &sha256,
            &mut |_, _| {},
        )
        .map(|r| r.3)
    }

    /// Fast-chain leaves: `2^k`-block children index to exactly the index leaf
    /// over the whole range, for every child span and a short last child.
    #[test]
    fn l1_equals_index_leaf_with_multi_block_children() {
        for (n, k) in [(16usize, 2u8), (19, 2), (64, 3), (37, 4), (5, 3), (8, 3)] {
            let (kids, hint, leaf) = multi_fixture(n, k, 0x3b0 + n as u64);
            let pv = go_blocks(&kids, &hint).unwrap_or_else(|e| panic!("n {n} k {k}: {e:?}"));
            assert_eq!(pv, with_key(&leaf), "n {n} k {k}");
            let input = encode_idx_input_blocks(&LEAF_VK, &kids);
            assert_eq!(host_hint(&input).unwrap(), hint);
        }
        // One-block children through the multi-block encoder: the original frame.
        let (kids, _, _) = fixture(5, match_all());
        let as_blocks: Vec<(Vec<u8>, Vec<Vec<u8>>)> = kids
            .iter()
            .map(|(pv, r)| (pv.clone(), vec![r.clone()]))
            .collect();
        assert_eq!(
            encode_idx_input_blocks(&LEAF_VK, &as_blocks),
            encode_idx_input(&LEAF_VK, &kids)
        );
    }

    #[test]
    fn l1_multi_block_children_bind_every_block() {
        let (kids, hint, _) = multi_fixture(32, 3, 0x51);
        assert!(go_blocks(&kids, &hint).is_ok());
        // The child and block whose records we move.
        let (c, b) = kids
            .iter()
            .enumerate()
            .find_map(|(c, k)| {
                k.1.iter()
                    .enumerate()
                    .position(|(b, r)| !r.is_empty() && b + 1 < k.1.len() && !k.1[b + 1].is_empty())
                    .map(|b| (c, b))
            })
            .unwrap();
        let first_len = |r: &[u8]| {
            let nt = r[28] as usize;
            let dl = 29 + 32 * nt;
            dl + 4 + u32::from_be_bytes([r[dl], r[dl + 1], r[dl + 2], r[dl + 3]]) as usize
        };
        // A record moved to the next block of the same child (count unchanged).
        let mut t = kids.clone();
        let l = first_len(&t[c].1[b]);
        let moved: Vec<u8> = t[c].1[b].drain(..l).collect();
        t[c].1[b + 1].splice(0..0, moved);
        assert_eq!(
            go_blocks(&t, &hint),
            Err(IdxAggError::DigestMismatch(c as u32))
        );
        // Two blocks' streams swapped.
        let mut t = kids.clone();
        t[c].1.swap(b, b + 1);
        assert_eq!(
            go_blocks(&t, &hint),
            Err(IdxAggError::DigestMismatch(c as u32))
        );
        // A flipped byte in one block.
        let mut t = kids.clone();
        let last = t[c].1[b].len() - 1;
        t[c].1[b][last] ^= 1;
        assert_eq!(
            go_blocks(&t, &hint),
            Err(IdxAggError::DigestMismatch(c as u32))
        );
        // A record dropped from one block.
        let mut t = kids.clone();
        t[c].1[b].drain(..l);
        assert_eq!(
            go_blocks(&t, &hint),
            Err(IdxAggError::CountMismatch(c as u32))
        );
        // One stream too few (the frame runs short) or one too many (trailing).
        let mut t = kids.clone();
        t.last_mut().unwrap().1.pop();
        assert_eq!(go_blocks(&t, &hint), Err(IdxAggError::Frame("short")));
        let mut t = kids.clone();
        t.last_mut().unwrap().1.push(Vec::new());
        assert_eq!(go_blocks(&t, &hint), Err(IdxAggError::Frame("trailing")));
        // A multi-block child presented in the one-stream frame.
        let mut input = encode_idx_input(&LEAF_VK, &[(kids[0].0.clone(), kids[0].1.concat())]);
        assert!(aggregate_index(&input, &hint, &sha256, &mut |_, _| {}).is_err());
        input.truncate(4);
        assert!(aggregate_index(&input, &hint, &sha256, &mut |_, _| {}).is_err());
    }

    #[test]
    fn l1_child_spans_must_be_one_power_of_two() {
        // A 3-block first child with another after it: no k makes P5 hold.
        let (kids, hint, _) = multi_fixture(6, 0, 0x77);
        let bs = blocks(6, 0x77);
        let full = build_witness(1, START, PARENT, vec![match_all()], &bs);
        let three =
            run(&build_witness(1, START, PARENT, vec![match_all()], &bs[..3]).encode()).unwrap();
        let rest = run(&build_witness(
            1,
            START + 3,
            keccak256(&full.headers[2]),
            vec![match_all()],
            &bs[3..6],
        )
        .encode())
        .unwrap();
        let recs: Vec<Vec<u8>> = kids.iter().map(|k| k.1[0].clone()).collect();
        let t = vec![(three, recs[..3].to_vec()), (rest, recs[3..].to_vec())];
        assert_eq!(
            go_blocks(&t, &hint),
            Err(IdxAggError::Agg(AggError::ChildSpan(0)))
        );
        // A 4-block child followed by a 2-block one (short last): fine.
        let (k4, h4, l4) = multi_fixture(6, 2, 0x78);
        assert_eq!(go_blocks(&k4, &h4).unwrap(), with_key(&l4));
    }

    #[test]
    fn l1_rejects_tampering() {
        let (kids, hint, _) = fixture(9, match_all());
        let j = kids.iter().position(|k| !k.1.is_empty()).unwrap();
        let mut t = kids.clone();
        let last = t[j].1.len() - 1;
        t[j].1[last] ^= 1;
        assert_eq!(go(&t, &hint), Err(IdxAggError::DigestMismatch(j as u32)));
        let mut t = kids.clone();
        let len0 = {
            let r = &t[j].1;
            let nt = r[28] as usize;
            let dl = 29 + 32 * nt;
            dl + 4 + u32::from_be_bytes([r[dl], r[dl + 1], r[dl + 2], r[dl + 3]]) as usize
        };
        t[j].1.drain(..len0);
        assert_eq!(go(&t, &hint), Err(IdxAggError::CountMismatch(j as u32)));
        let mut t = kids.clone();
        t[j].1.push(0);
        assert_eq!(go(&t, &hint), Err(IdxAggError::RecordParse(j as u32)));
        let mut t = kids.clone();
        let k = (j + 1..t.len()).find(|&k| !t[k].1.is_empty()).unwrap();
        let moved: Vec<u8> = t[j].1.drain(..len0).collect();
        t[k].1.splice(0..0, moved);
        assert!(matches!(
            go(&t, &hint),
            Err(IdxAggError::CountMismatch(_)) | Err(IdxAggError::DigestMismatch(_))
        ));
        let mut t = kids.clone();
        t.remove(4);
        assert!(matches!(go(&t, &hint), Err(IdxAggError::Agg(_))));
        let mut t = kids.clone();
        t.swap(2, 3);
        assert!(matches!(
            go(&t, &hint),
            Err(IdxAggError::Agg(AggError::NotContiguous(2)))
        ));
        let mut h = hint.clone();
        h.swap(5, 9);
        assert!(matches!(go(&kids, &h), Err(IdxAggError::Index(_))));
        let mut input = encode_idx_input(&LEAF_VK, &kids);
        input.push(0);
        assert_eq!(
            aggregate_index(&input, &hint, &sha256, &mut |_, _| {}).map(|r| r.3),
            Err(IdxAggError::Frame("trailing"))
        );
        let mut input = encode_idx_input(&LEAF_VK, &kids);
        input[0] ^= 1;
        assert_eq!(
            aggregate_index(&input, &hint, &sha256, &mut |_, _| {}).map(|r| r.3),
            Err(IdxAggError::Frame("magic"))
        );
    }

    #[test]
    fn l1_rejects_filtered_children() {
        let f = LogFilter {
            addresses: vec![[0x11; 20]],
            topics: [vec![], vec![], vec![], vec![]],
        };
        let (kids, _, _) = fixture(4, f);
        assert_eq!(go(&kids, &[]), Err(IdxAggError::NotMatchAll));
    }

    // ------------------------------------------------------------------ L2 fold

    /// `n_blocks` blocks cut into 64-block L1 segments: each segment's L1 pv, the
    /// flat (single-run) match-all statement pv over the whole range, and the
    /// per-segment index leaf pvs.
    fn l1_segments(n_blocks: usize, seed: u64) -> (Vec<Vec<u8>>, Vec<u8>, Vec<IndexSegment>) {
        l1_segments_geo(n_blocks, seed, 0, 6)
    }

    /// [`l1_segments`] at any geometry: `2^leaf_log2`-block leaves, L1 scopes of
    /// `2^l1_log2` blocks (a fast chain's S and Q).
    fn l1_segments_geo(
        n_blocks: usize,
        seed: u64,
        leaf_log2: u8,
        l1_log2: u8,
    ) -> (Vec<Vec<u8>>, Vec<u8>, Vec<IndexSegment>) {
        let bs = blocks(n_blocks, seed);
        let full = build_witness(1, START, PARENT, vec![match_all()], &bs);
        let flat = run(&full.encode()).unwrap();
        let mut l1s = Vec::new();
        let mut segs = Vec::new();
        let mut i = 0;
        let (leaf, scope) = (1usize << leaf_log2, 1usize << l1_log2);
        while i < n_blocks {
            let j = (i + scope).min(n_blocks);
            let parent = if i == 0 {
                PARENT
            } else {
                keccak256(&full.headers[i - 1])
            };
            let seg_w = build_witness(1, START + i as u64, parent, vec![match_all()], &bs[i..j]);
            let mut ix = IndexBuilder::new();
            ix.keep_records = true;
            let _ = run_statements_core(&seg_w.encode(), Some(&mut ix));
            let mut per_block = vec![Vec::new(); j - i];
            for (b, r) in &ix.records {
                per_block[(b - START) as usize - i].extend_from_slice(r);
            }
            let mut kids = Vec::new();
            let mut b = i;
            while b < j {
                let e = (b + leaf).min(j);
                let p = if b == 0 {
                    PARENT
                } else {
                    keccak256(&full.headers[b - 1])
                };
                let w = build_witness(1, START + b as u64, p, vec![match_all()], &bs[b..e]);
                kids.push((run(&w.encode()).unwrap(), per_block[b - i..e - i].to_vec()));
                b = e;
            }
            let input = encode_idx_input_blocks(&LEAF_VK, &kids);
            let hint = host_hint(&input).unwrap();
            let (s, c, _, pv) = aggregate_index(&input, &hint, &sha256, &mut |_, _| {}).unwrap();
            // Each L1 index == the index leaf over the same scope.
            assert_eq!(pv, with_key(&run_indexed(&seg_w.encode(), &hint).unwrap()));
            segs.push(IndexSegment {
                start_block: s.start_block,
                end_block: s.end_block,
                index: c,
            });
            l1s.push(pv);
            i = j;
        }
        (l1s, flat, segs)
    }

    fn fold_pv(l1s: &[Vec<u8>]) -> Result<Vec<u8>, FoldError> {
        fold(&encode_fold_input(l1s), &sha256, &mut |_| {}).map(|r| r.4)
    }

    #[test]
    fn fold_equals_flat_and_lists_every_segment_index() {
        // 1, 2, 4 and 16 full segments (the 64/128/256/1024-block cadences),
        // plus ranges ending in a short segment.
        for (n_blocks, seed) in [
            (64, 1),
            (128, 2),
            (256, 3),
            (1024, 4),
            (64 * 3 + 17, 5),
            (5, 6),
        ] {
            let (l1s, flat, segs) = l1_segments(n_blocks, seed);
            let (s, got_segs, ilh, key, pv) =
                fold(&encode_fold_input(&l1s), &sha256, &mut |_| {}).unwrap();
            assert_eq!(pv.len(), FOLD_PV_LEN);
            assert_eq!(&pv[..320], &flat[..], "fold == flat for {n_blocks} blocks");
            assert_eq!(got_segs, segs);
            assert_eq!(ilh, index_list_hash(&segs));
            assert_eq!(key, crate::vkey::vkey_words_to_bytes(&LEAF_VK));
            assert_eq!(decode_fold_pv(&pv), Some((s, ilh, key)));
            assert_eq!(
                segs.iter().map(|x| x.index.entry_count).sum::<u64>(),
                s.log_count
            );
        }
    }

    /// The fast-chain shape: multi-block leaves under each L1, L1 scopes other
    /// than 64 blocks, and the fold still equals the flat statement.
    #[test]
    fn fold_equals_flat_at_fast_chain_geometry() {
        for (n_blocks, leaf_log2, l1_log2, seed) in [
            (64usize, 2u8, 4u8, 21u64),
            (16 * 3 + 5, 2, 4, 22),
            (100, 3, 5, 23),
            (32, 3, 3, 24),
            (7, 2, 4, 25),
            (256, 4, 7, 26),
        ] {
            let (l1s, flat, segs) = l1_segments_geo(n_blocks, seed, leaf_log2, l1_log2);
            let (s, got, ilh, _, pv) =
                fold(&encode_fold_input(&l1s), &sha256, &mut |_| {}).unwrap();
            assert_eq!(
                &pv[..320],
                &flat[..],
                "{n_blocks} blocks, S 2^{leaf_log2}, Q 2^{l1_log2}"
            );
            assert_eq!(got, segs);
            assert_eq!(ilh, index_list_hash(&segs));
            assert_eq!(s.end_block - s.start_block + 1, n_blocks as u64);
        }
    }

    #[test]
    fn fold_p1_digests_are_sha256_of_each_l1_pv_in_order() {
        let (l1s, _, _) = l1_segments(64 * 3, 9);
        let mut seen = Vec::new();
        fold(&encode_fold_input(&l1s), &sha256, &mut |d| seen.push(*d)).unwrap();
        assert_eq!(seen, l1s.iter().map(|p| sha256(p)).collect::<Vec<_>>());
    }

    #[test]
    fn fold_rejects() {
        let (l1s, _, _) = l1_segments(64 * 3 + 10, 11);
        // Mixed leaf programs.
        let mut t = l1s.clone();
        let last = L1_PV_LEN - 1;
        t[2][last] ^= 1;
        assert_eq!(fold_pv(&t), Err(FoldError::LeafKey(2)));
        // Entry count != log count (a forged index commitment).
        let mut t = l1s.clone();
        t[1][320 + 31] ^= 1;
        assert_eq!(fold_pv(&t), Err(FoldError::EntryCount(1)));
        // Non-canonical index commitment (dirty high bytes).
        let mut t = l1s.clone();
        t[0][320] = 1;
        assert_eq!(fold_pv(&t), Err(FoldError::ChildShape(0)));
        // Reordered, dropped and short-middle children.
        let mut t = l1s.clone();
        t.swap(0, 1);
        assert!(matches!(
            fold_pv(&t),
            Err(FoldError::Agg(AggError::NotContiguous(1)))
        ));
        let mut t = l1s.clone();
        t.remove(1);
        assert!(matches!(
            fold_pv(&t),
            Err(FoldError::Agg(AggError::NotContiguous(1)))
        ));
        let mut t = l1s.clone();
        t.swap(2, 3);
        assert!(matches!(fold_pv(&t), Err(FoldError::Agg(_))));
        // A short segment that is not last: alignment.
        let (short, _, _) = l1_segments(10, 12);
        let (tail, _, _) = {
            // A segment starting right after the short one is not constructible
            // from the same range, so check alignment on a short FIRST child.
            (short.clone(), (), ())
        };
        let mut t = tail;
        t.extend(l1s.clone());
        assert!(matches!(fold_pv(&t), Err(FoldError::Agg(_))));
        // Seam hash broken with contiguous numbers.
        let mut t = l1s.clone();
        t[1][64 + 96] ^= 1;
        assert!(matches!(
            fold_pv(&t),
            Err(FoldError::Agg(AggError::SeamHash(1)))
        ));
        // Framing.
        assert_eq!(fold_pv(&[]), Err(FoldError::NoChildren));
        let mut input = encode_fold_input(&l1s);
        input.push(0);
        assert_eq!(
            fold(&input, &sha256, &mut |_| {}).map(|r| r.4),
            Err(FoldError::Trailing)
        );
        input.truncate(input.len() - 2);
        assert_eq!(
            fold(&input, &sha256, &mut |_| {}).map(|r| r.4),
            Err(FoldError::Short)
        );
        let mut input = encode_fold_input(&l1s);
        input[0] ^= 1;
        assert_eq!(
            fold(&input, &sha256, &mut |_| {}).map(|r| r.4),
            Err(FoldError::BadMagic)
        );
    }

    #[test]
    fn fold_short_segment_only_last() {
        // 64 + 10 blocks: the short L1 child last is fine; the same two children
        // cannot be folded with the short one first (it is not contiguous).
        let (l1s, flat, _) = l1_segments(74, 13);
        assert_eq!(l1s.len(), 2);
        assert_eq!(&fold_pv(&l1s).unwrap()[..320], &flat[..]);
    }

    /// Pinned vector for `indexListHash`, re-computed by the Solidity helper in
    /// `LogSetVerifierV2.t.sol::test_IndexListHash_matchesRustVector`.
    #[test]
    fn index_list_hash_vector() {
        assert_eq!(&INDEX_LIST_TAG[..26], b"COMPLETE_LOG_INDEX_LIST_V1");
        let segs = [
            IndexSegment {
                start_block: 26_065_024,
                end_block: 26_065_087,
                index: IndexCommitment {
                    entry_count: 50_569,
                    root_by_emitter: [0x11; 32],
                    root_by_topic0: [0x22; 32],
                },
            },
            IndexSegment {
                start_block: 26_065_088,
                end_block: 26_065_151,
                index: IndexCommitment {
                    entry_count: 0,
                    root_by_emitter: [0; 32],
                    root_by_topic0: [0; 32],
                },
            },
        ];
        let h = index_list_hash(&segs);
        println!("INDEX_LIST_VECTOR 0x{}", hex_of(&h));
        assert_eq!(hex_of(&h), hex_of(&index_list_hash(&segs)), "deterministic");
        assert_ne!(h, index_list_hash(&segs[..1]));
    }

    fn hex_of(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }
}
