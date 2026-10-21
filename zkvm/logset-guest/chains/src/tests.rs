use super::plan::*;
use super::*;

#[test]
fn fast_chain_geometry_follows_the_rules() {
    for id in FAST_CHAINS.into_iter().chain([BASE]) {
        let p = profile(id).unwrap();
        let c = p
            .measured
            .expect("every fast chain carries its measured composition");
        let s = seg_log2_for(&c);
        assert_eq!(p.seg_log2, s, "{}: seg_log2", p.name);
        assert_eq!(
            p.index_log2,
            index_log2_for(&c, s),
            "{}: index_log2",
            p.name
        );
        // The geometry the planner accepts.
        Planner::new(p.seg_log2, p.index_log2, 0).unwrap();
    }
}

/// The derived numbers the design doc quotes: model cycles per block and per
/// segment, segment fill time, and logs per index tree.
#[test]
fn derived_numbers_match_the_design_doc() {
    let row = |id: u64| {
        let p = profile(id).unwrap();
        let c = p.measured.unwrap();
        let cyc = c.model_cycles_per_block();
        (
            p.seg_blocks(),
            p.index_blocks(),
            (cyc / 1e3).round() as u64,
            (cyc * p.seg_blocks() as f64 / 1e6).round() as u64,
            (c.logs_per_block * p.index_blocks() as f64 / 1e3).round() as u64,
        )
    };
    assert_eq!(row(ROBINHOOD), (1024, 2048, 385, 394, 76));
    assert_eq!(row(ARBITRUM_ONE), (2048, 4096, 188, 385, 70));
    assert_eq!(row(OP_MAINNET), (128, 256, 2705, 346, 99));
    assert_eq!(row(UNICHAIN), (1024, 2048, 165, 169, 20));
    assert_eq!(row(INK), (1024, 2048, 296, 303, 55));
    assert_eq!(row(BASE), (64, 128, 6769, 433, 106));
}

#[test]
fn receipt_gates() {
    assert_eq!(
        receipt_gate(1),
        ReceiptGate {
            deposits: false,
            nitro: false
        }
    );
    for id in [BASE, OP_MAINNET, UNICHAIN, INK] {
        assert!(receipt_gate(id).deposits && !receipt_gate(id).nitro);
    }
    for id in [ARBITRUM_ONE, ROBINHOOD] {
        assert!(receipt_gate(id).nitro && !receipt_gate(id).deposits);
    }
    // Families agree with the gates.
    for p in CHAINS.iter() {
        let g = p.receipt_gate();
        assert_eq!(g.deposits, p.family == Family::OpStack, "{}", p.name);
        assert_eq!(g.nitro, p.family == Family::Orbit, "{}", p.name);
    }
}

#[test]
fn anchors_match_families() {
    for p in CHAINS.iter() {
        let ok = matches!(
            (p.family, p.anchor),
            (Family::Ethereum, Anchor::Ethereum)
                | (Family::OpStack, Anchor::OpSuperRoot { .. })
                | (Family::OpStack, Anchor::OpOutputRoot { .. })
                | (Family::Orbit, Anchor::Bold { .. })
                | (Family::Stellar, Anchor::StellarScp { .. })
        );
        assert!(ok, "{}", p.name);
        assert!(!p.public_rpc.is_empty(), "{}", p.name);
        assert!(p.rpc_env.starts_with("WILLOW_"), "{}", p.name);
    }
    let ids: Vec<u64> = CHAINS.iter().map(|p| p.chain_id).collect();
    let mut dedup = ids.clone();
    dedup.sort();
    dedup.dedup();
    assert_eq!(ids.len(), dedup.len(), "chain ids are unique");
}

#[test]
fn pieces_are_aligned_and_exact() {
    let p = pieces(1024, 5000, 10);
    assert_eq!(p.len(), 4);
    assert_eq!(
        p[0],
        Range {
            start: 1024,
            end: 2047
        }
    );
    assert_eq!(
        p[3],
        Range {
            start: 4096,
            end: 5000
        }
    );
    assert!(p[..3].iter().all(|r| r.len() == 1024));
    assert_eq!(pieces(0, 0, 4), vec![Range { start: 0, end: 0 }]);
}

#[test]
#[should_panic]
fn pieces_reject_an_unaligned_start() {
    pieces(5, 100, 3);
}

/// Statements end on anchors, start on Q boundaries, overlap by < Q, never gap.
#[test]
fn statements_cover_every_block_and_end_on_anchors() {
    let (s, q) = (4u8, 6u8); // S = 16, Q = 64
    let mut pl = Planner::new(s, q, 640).unwrap();
    let anchors = [700u64, 703, 767, 768, 900, 1_000, 1_087];
    let mut covered_to = 639u64;
    for a in anchors {
        let st = pl.statement_to(a).unwrap();
        assert_eq!(st.range.end, a);
        assert_eq!(align_down(st.range.start, q), st.range.start);
        assert!(st.range.start <= covered_to + 1, "no gap");
        assert!(covered_to + 1 - st.range.start < 64, "overlap < Q");
        // Fold children: full Q scopes except the last.
        let n = st.l1.len();
        for (i, r) in st.l1.iter().enumerate() {
            if i + 1 < n {
                assert_eq!(r.len(), 64);
            }
        }
        assert_eq!(st.l1[0].start, st.range.start);
        assert_eq!(st.l1[n - 1].end, a);
        // Tails exactly when A does not close a segment / a scope.
        assert_eq!(st.tail_segment.is_some(), (a + 1) % 16 != 0);
        assert_eq!(st.tail_l1.is_some(), (a + 1) % 64 != 0);
        if let Some(t) = st.tail_segment {
            assert_eq!((t.start % 16, t.end), (0, a));
        }
        covered_to = a;
    }
}

#[test]
fn anchor_closing_a_scope_needs_no_tail_and_no_overlap() {
    let mut pl = Planner::new(4, 6, 0).unwrap();
    let st = pl.statement_to(127).unwrap();
    assert_eq!(st.range, Range { start: 0, end: 127 });
    assert_eq!(st.l1.len(), 2);
    assert_eq!((st.tail_segment, st.tail_l1), (None, None));
    assert_eq!(pl.next_start(), 128);
}

#[test]
fn stale_anchor_and_bad_geometry_are_refused() {
    let mut pl = Planner::new(4, 6, 0).unwrap();
    pl.statement_to(200).unwrap(); // next start = 192
    assert_eq!(
        pl.statement_to(150),
        Err(PlanError::StaleAnchor {
            anchor: 150,
            next_start: 192
        })
    );
    // An anchor inside the overlap is still a valid statement (it re-covers 192..).
    assert!(pl.statement_to(195).is_ok());
    assert!(matches!(
        Planner::new(7, 6, 0),
        Err(PlanError::Geometry { .. })
    ));
    assert!(matches!(
        Planner::new(0, 7, 0),
        Err(PlanError::Geometry { .. })
    ));
    assert!(matches!(
        Planner::new(4, 6, 65),
        Err(PlanError::UnalignedStart { .. })
    ));
}

#[test]
fn daily_anchors_close_on_the_days_last_anchor() {
    let d = DAY_SECS;
    let pt = |block, timestamp| AnchorPoint { block, timestamp };
    let mut da = DailyAnchors::default();
    let out = da.push(&[
        pt(10, d + 5),
        pt(20, d + 100),
        pt(20, d + 100),
        pt(15, d + 90),
    ]);
    assert!(out.is_empty());
    let out = da.push(&[pt(30, 2 * d + 1), pt(40, 2 * d + 7)]);
    assert_eq!(out, vec![pt(20, d + 100)]);
    // A day with no anchor rolls into the next one: no empty statement.
    let out = da.push(&[pt(50, 4 * d + 3)]);
    assert_eq!(out, vec![pt(40, 2 * d + 7)]);
    assert_eq!(da.pending(), Some(pt(50, 4 * d + 3)));
}
