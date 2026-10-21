//! Adversarial + KAT suite for the beacon-anchored window guest.
//!
//! Discipline: every adversarial case carries its own passing CONTROL in the same
//! test. A rejection test that would also pass if `run` rejected everything is not
//! a test, so the control is asserted first, then exactly one field is edited.
//!
//! Everything runs against `crate::run` — the same function `program/src/main.rs`
//! calls — so a green suite is a statement about the shipping guest, not about a
//! host-side twin.

use crate::blob::{
    decode_blob, encode_blob, Witness, BLOB_FIXED_LEN, BLOB_MAGIC, OFF_MAGIC, OFF_N_SIBLINGS,
    OFF_PATH_KIND, OFF_TARGET_SLOT,
};
use crate::error::Error;
use crate::fork_schedule::{
    fork_at, payload_field_count, payload_tree_depth, ForkVersion, CAPELLA_SLOT, DENEB_SLOT,
    ELECTRA_SLOT, FULU_SLOT, PAYLOAD_BLOCK_HASH_POSITION, PAYLOAD_BLOCK_NUMBER_POSITION,
    SLOTS_PER_HISTORICAL_ROOT,
};
use crate::gindex::{
    depth_for_gindex, expected_sibling_count, state_to_block_root_gindex, PathKind,
    BLOCK_ROOTS_POSITION, BLOCK_STATE_ROOT_GINDEX, HISTORICAL_SUMMARIES_POSITION,
    PAYLOAD_TO_BLOCK_ROOT_GINDEX,
};
use crate::synth::Synth;
use crate::{run, Statement, ANCHOR_KIND, STATEMENT_LEN};
use beacon_cl_core::fork::state_tree_depth;
use beacon_cl_core::ssz::{uint64_leaf, Bytes32};

// ---------------------------------------------------------------------------
// Fixture slots. Chosen so each fork's shape is actually exercised.
// ---------------------------------------------------------------------------

/// A Deneb anchor: 28 state fields -> depth 5.
const DENEB_ANCHOR: u64 = 9_000_000;
/// An Electra anchor: 37 state fields -> depth 6.
const ELECTRA_ANCHOR: u64 = 12_000_000;
/// A Fulu anchor: 38 state fields -> still depth 6.
const FULU_ANCHOR: u64 = 13_300_000;

fn enc(w: &Witness) -> Vec<u8> {
    encode_blob(w).expect("encode")
}

fn flip(node: &mut Bytes32) {
    node[0] ^= 0xff;
}

/// Assert the fixture verifies, and hand back its statement — the CONTROL every
/// adversarial case in this file is one edit away from.
fn control(w: &Witness) -> Statement {
    run(&enc(w)).expect("control fixture must verify")
}

// ---------------------------------------------------------------------------
// Positives
// ---------------------------------------------------------------------------

#[test]
fn recent_path_verifies_at_deneb() {
    let s = Synth::recent(DENEB_ANCHOR, 1_000);
    assert_eq!(fork_at(s.anchor_slot).unwrap(), ForkVersion::Deneb);
    assert_eq!(state_tree_depth(ForkVersion::Deneb), 5);
    let st = control(&s.build());
    assert_eq!(st.kind, ANCHOR_KIND);
    assert_eq!(st.el_block_number, s.block_number);
    assert_eq!(st.el_block_hash, s.block_hash);
    assert_eq!(st.anchor_slot, s.anchor_slot);
    assert_eq!(st.target_slot, s.target_slot);
    assert_eq!(st.anchor_beacon_root, s.expected_r());
}

#[test]
fn recent_path_verifies_at_electra_and_fulu() {
    for (anchor, fork) in [
        (ELECTRA_ANCHOR, ForkVersion::Electra),
        (FULU_ANCHOR, ForkVersion::Fulu),
    ] {
        let s = Synth::recent(anchor, 7);
        assert_eq!(fork_at(anchor).unwrap(), fork);
        assert_eq!(state_tree_depth(fork), 6);
        let st = control(&s.build());
        assert_eq!(st.el_block_hash, s.block_hash);
        assert_eq!(st.anchor_beacon_root, s.expected_r());
    }
}

#[test]
fn recent_path_verifies_at_both_range_boundaries() {
    // 1 slot behind and exactly SLOTS_PER_HISTORICAL_ROOT behind both verify;
    // 8,193 is the first rejection (see `recent_range_is_closed_on_both_ends`).
    for behind in [1u64, SLOTS_PER_HISTORICAL_ROOT] {
        let s = Synth::recent(FULU_ANCHOR, behind);
        let st = control(&s.build());
        assert_eq!(st.target_slot, FULU_ANCHOR - behind);
    }
}

#[test]
fn recent_path_verifies_across_a_fork_boundary() {
    // Anchor in Fulu, target the last Electra slot: the ANCHOR fork drives the
    // state container depth (6) while the TARGET fork drives the payload shape.
    // Both are 17-leaf here, but the two lookups are genuinely independent.
    let s = Synth::recent(FULU_SLOT + 10, 11);
    assert_eq!(fork_at(s.anchor_slot).unwrap(), ForkVersion::Fulu);
    assert_eq!(fork_at(s.target_slot).unwrap(), ForkVersion::Electra);
    control(&s.build());
}

#[test]
fn historical_path_verifies_at_deneb_with_a_nonzero_chunk() {
    let target = CAPELLA_SLOT + 5 * SLOTS_PER_HISTORICAL_ROOT + 77;
    let s = Synth::historical(DENEB_ANCHOR, target);
    assert_eq!(s.chunk(), 5, "the test wants a chunk > 0");
    assert_eq!(fork_at(s.anchor_slot).unwrap(), ForkVersion::Deneb);
    let st = control(&s.build());
    assert_eq!(st.el_block_number, s.block_number);
    assert_eq!(st.target_slot, target);
}

#[test]
fn historical_path_verifies_at_electra_and_fulu_with_a_nonzero_chunk() {
    for anchor in [ELECTRA_ANCHOR, FULU_ANCHOR] {
        let target = CAPELLA_SLOT + 700 * SLOTS_PER_HISTORICAL_ROOT + 4_321;
        let s = Synth::historical(anchor, target);
        assert_eq!(s.chunk(), 700);
        let st = control(&s.build());
        assert_eq!(st.anchor_slot, anchor);
        assert_eq!(st.anchor_beacon_root, s.expected_r());
    }
}

#[test]
fn capella_target_with_fifteen_payload_leaves_under_a_fulu_anchor() {
    let target = 6_300_000;
    assert_eq!(fork_at(target).unwrap(), ForkVersion::Capella);
    let s = Synth::historical(FULU_ANCHOR, target);
    let w = s.build();
    assert_eq!(
        w.payload_leaves.len(),
        15,
        "Capella ExecutionPayload is 15 fields"
    );
    assert_eq!(payload_tree_depth(ForkVersion::Capella), 4);
    assert_eq!(payload_tree_depth(ForkVersion::Fulu), 5);
    let st = control(&w);
    assert_eq!(st.el_block_number, s.block_number);
    assert_eq!(st.el_block_hash, s.block_hash);
}

// ---------------------------------------------------------------------------
// Adversarial: branch tampering
// ---------------------------------------------------------------------------

#[test]
fn a_tampered_sibling_at_any_layer_is_rejected() {
    for s in [
        Synth::recent(FULU_ANCHOR, 3_000),
        Synth::recent(DENEB_ANCHOR, 3_000),
        Synth::historical(
            FULU_ANCHOR,
            CAPELLA_SLOT + 700 * SLOTS_PER_HISTORICAL_ROOT + 9,
        ),
        Synth::historical(
            DENEB_ANCHOR,
            CAPELLA_SLOT + 5 * SLOTS_PER_HISTORICAL_ROOT + 9,
        ),
    ] {
        let base = s.build();
        control(&base); // the control, per fixture
        for layer in 0..base.siblings.len() {
            let mut w = base.clone();
            flip(&mut w.siblings[layer]);
            match run(&enc(&w)) {
                Err(Error::BlockRootBranchInvalid { .. }) => {}
                other => panic!(
                    "layer {layer} of {:?} path (anchor {}) was not rejected: {other:?}",
                    s.path, s.anchor_slot
                ),
            }
        }
    }
}

#[test]
fn a_tampered_payload_branch_node_at_any_layer_is_rejected() {
    let base = Synth::recent(FULU_ANCHOR, 500).build();
    control(&base);
    for layer in 0..base.branch_payload.len() {
        let mut w = base.clone();
        flip(&mut w.branch_payload[layer]);
        assert!(
            matches!(run(&enc(&w)), Err(Error::BlockRootBranchInvalid { .. })),
            "payload branch layer {layer} was not rejected"
        );
    }
}

#[test]
fn a_tampered_payload_leaf_is_rejected() {
    let base = Synth::recent(FULU_ANCHOR, 500).build();
    control(&base);
    for leaf in 0..base.payload_leaves.len() {
        let mut w = base.clone();
        flip(&mut w.payload_leaves[leaf]);
        assert!(
            matches!(run(&enc(&w)), Err(Error::BlockRootBranchInvalid { .. })),
            "payload leaf {leaf} was not rejected"
        );
    }
}

#[test]
fn a_tampered_anchor_state_root_is_rejected() {
    let base = Synth::historical(
        FULU_ANCHOR,
        CAPELLA_SLOT + 300 * SLOTS_PER_HISTORICAL_ROOT + 11,
    )
    .build();
    control(&base);
    let mut w = base.clone();
    flip(&mut w.anchor_state_root);
    assert!(matches!(
        run(&enc(&w)),
        Err(Error::BlockRootBranchInvalid { .. })
    ));
}

#[test]
fn tampering_the_state_root_branch_moves_r_rather_than_erroring() {
    // Soundness note 4, made executable. `R` is DERIVED from
    // `anchor_state_root` + `branch_state_root` and committed as-is; the guest has
    // nothing to check it against. A junk branch therefore yields a junk `R` —
    // which EIP-4788 rejects on-chain. This is the design, not a gap, and it is
    // pinned here so nobody "fixes" it into a witnessed-and-compared `R`.
    let base = Synth::recent(FULU_ANCHOR, 42).build();
    let good = control(&base);
    for layer in 0..base.branch_state_root.len() {
        let mut w = base.clone();
        flip(&mut w.branch_state_root[layer]);
        let bad = run(&enc(&w)).expect("still a valid anchor of a DIFFERENT beacon root");
        assert_ne!(
            bad.anchor_beacon_root, good.anchor_beacon_root,
            "layer {layer}"
        );
        // Everything the contract cross-checks against the window statement is
        // unchanged; only the EIP-4788 lookup can fail.
        assert_eq!(bad.el_block_number, good.el_block_number);
        assert_eq!(bad.el_block_hash, good.el_block_hash);
    }
}

// ---------------------------------------------------------------------------
// Adversarial: shape / count
// ---------------------------------------------------------------------------

#[test]
fn sibling_count_off_by_one_in_either_direction_is_rejected() {
    for s in [
        Synth::recent(FULU_ANCHOR, 77),
        Synth::historical(
            FULU_ANCHOR,
            CAPELLA_SLOT + 400 * SLOTS_PER_HISTORICAL_ROOT + 3,
        ),
    ] {
        let base = s.build();
        control(&base);
        let n = base.siblings.len();

        let mut too_many = base.clone();
        too_many.siblings.push([0x5a; 32]);
        match run(&enc(&too_many)) {
            Err(Error::SiblingCountMismatch { expected, got, .. }) => {
                assert_eq!(expected, n);
                assert_eq!(got, n + 1);
            }
            other => panic!("+1 sibling not rejected: {other:?}"),
        }

        let mut too_few = base.clone();
        too_few.siblings.pop();
        match run(&enc(&too_few)) {
            Err(Error::SiblingCountMismatch { expected, got, .. }) => {
                assert_eq!(expected, n);
                assert_eq!(got, n - 1);
            }
            other => panic!("-1 sibling not rejected: {other:?}"),
        }
    }
}

#[test]
fn a_payload_leaf_count_that_is_not_the_targets_fork_is_rejected() {
    // Deneb target (17) truncated to Capella's 15: a valid SHAPE, wrong FORK.
    let base = Synth::recent(DENEB_ANCHOR, 100).build();
    control(&base);
    let mut short = base.clone();
    short.payload_leaves.truncate(15);
    assert_eq!(
        run(&enc(&short)),
        Err(Error::PayloadLeafCountMismatch {
            expected: 17,
            got: 15
        })
    );

    // ...and the other direction: a Capella target padded to 17.
    let cap = Synth::historical(FULU_ANCHOR, 6_300_000).build();
    control(&cap);
    let mut long = cap.clone();
    long.payload_leaves.push([0x01; 32]);
    long.payload_leaves.push([0x02; 32]);
    assert_eq!(
        run(&enc(&long)),
        Err(Error::PayloadLeafCountMismatch {
            expected: 15,
            got: 17
        })
    );
}

#[test]
fn a_payload_leaf_count_that_is_no_fork_at_all_is_rejected_at_decode() {
    let mut w = Synth::recent(FULU_ANCHOR, 100).build();
    control(&w);
    w.payload_leaves.pop(); // 16 — not an ExecutionPayload shape
    assert_eq!(
        decode_blob(&enc(&w)),
        Err(Error::PayloadLeafCountInvalid { got: 16 })
    );
    assert_eq!(
        run(&enc(&w)),
        Err(Error::PayloadLeafCountInvalid { got: 16 })
    );
}

#[test]
fn payload_leaves_six_and_twelve_swapped_are_rejected() {
    // The container tree binds each value to its POSITION, so `block_number` and
    // `block_hash` cannot be exchanged: the payload root moves and the branch to
    // the target block root stops closing.
    let base = Synth::recent(ELECTRA_ANCHOR, 250).build();
    let good = control(&base);
    let mut w = base.clone();
    w.payload_leaves
        .swap(PAYLOAD_BLOCK_NUMBER_POSITION, PAYLOAD_BLOCK_HASH_POSITION);
    assert!(matches!(
        run(&enc(&w)),
        Err(Error::BlockRootBranchInvalid { .. })
    ));
    // ...and the control really did read them from those two positions.
    assert_eq!(
        base.payload_leaves[PAYLOAD_BLOCK_HASH_POSITION],
        good.el_block_hash
    );
}

#[test]
fn a_non_canonical_block_number_leaf_is_rejected() {
    // The whole tree is rebuilt AROUND the bad leaf, so every branch still
    // verifies and the only thing under test is the high-bytes check. Without it a
    // prover could commit `el_block_number = N` from a leaf that is not the SSZ
    // encoding of `N`, and the contract's `el_block_number == endBlock` compare
    // would pass on a forged leaf.
    let mut leaf = uint64_leaf(25_778_841);
    leaf[8] = 0x01; // first byte ABOVE the uint64
    let mut s = Synth::recent(FULU_ANCHOR, 900);
    s.block_number_leaf_override = Some(leaf);
    assert_eq!(
        run(&enc(&s.build())),
        Err(Error::BlockNumberLeafNotCanonical)
    );

    // Control: the same fixture with byte 8 cleared verifies and reads back 25,778,841.
    let mut ok = s.clone();
    ok.block_number_leaf_override = Some(uint64_leaf(25_778_841));
    assert_eq!(control(&ok.build()).el_block_number, 25_778_841);

    // The LAST byte of the leaf is checked too, not just the byte after the u64.
    let mut top = s.clone();
    let mut leaf31 = uint64_leaf(25_778_841);
    leaf31[31] = 0x80;
    top.block_number_leaf_override = Some(leaf31);
    assert_eq!(
        run(&enc(&top.build())),
        Err(Error::BlockNumberLeafNotCanonical)
    );
}

// ---------------------------------------------------------------------------
// Adversarial: slots and path kind
// ---------------------------------------------------------------------------

#[test]
fn recent_range_is_closed_on_both_ends() {
    let base = Synth::recent(FULU_ANCHOR, 100).build();
    control(&base);

    // target_slot == anchor_slot: the anchor block's own post-state has not
    // written its slot into block_roots yet.
    let mut same = base.clone();
    same.target_slot = same.anchor_slot;
    assert_eq!(
        run(&enc(&same)),
        Err(Error::TargetNotBeforeAnchor {
            anchor_slot: FULU_ANCHOR,
            target_slot: FULU_ANCHOR
        })
    );

    // target_slot > anchor_slot: a future block cannot be under the anchor state.
    let mut ahead = base.clone();
    ahead.target_slot = ahead.anchor_slot + 1;
    assert_eq!(
        run(&enc(&ahead)),
        Err(Error::TargetNotBeforeAnchor {
            anchor_slot: FULU_ANCHOR,
            target_slot: FULU_ANCHOR + 1
        })
    );

    // anchor_slot - 8193: the ring entry has been overwritten. 8192 verifies
    // (see `recent_path_verifies_at_both_range_boundaries`), so this is the exact
    // edge of the window, not a blanket rejection.
    let mut stale = base.clone();
    stale.target_slot = FULU_ANCHOR - (SLOTS_PER_HISTORICAL_ROOT + 1);
    assert_eq!(
        run(&enc(&stale)),
        Err(Error::TargetOutsideRecentRange {
            anchor_slot: FULU_ANCHOR,
            target_slot: FULU_ANCHOR - 8193
        })
    );
}

#[test]
fn pre_capella_slots_are_rejected_on_both_sides() {
    let base = Synth::historical(
        FULU_ANCHOR,
        CAPELLA_SLOT + 200 * SLOTS_PER_HISTORICAL_ROOT + 5,
    )
    .build();
    control(&base);

    let mut target = base.clone();
    target.target_slot = CAPELLA_SLOT - 1;
    assert_eq!(
        run(&enc(&target)),
        Err(Error::PreCapellaSlot {
            slot: CAPELLA_SLOT - 1
        })
    );

    let mut anchor = base.clone();
    anchor.anchor_slot = CAPELLA_SLOT - 1;
    assert_eq!(
        run(&enc(&anchor)),
        Err(Error::PreCapellaSlot {
            slot: CAPELLA_SLOT - 1
        })
    );

    // ...and the first Capella slot itself is accepted by `fork_at`.
    assert_eq!(fork_at(CAPELLA_SLOT).unwrap(), ForkVersion::Capella);
}

#[test]
fn the_wrong_path_kind_over_otherwise_valid_data_is_rejected() {
    // Recent witness relabelled Historical: the gindex becomes 39 levels deeper,
    // so the (correct, 19-sibling) list is now the wrong length.
    let recent = Synth::recent(FULU_ANCHOR, 100).build();
    control(&recent);
    let mut as_hist = recent.clone();
    as_hist.path = PathKind::Historical;
    match run(&enc(&as_hist)) {
        Err(Error::SiblingCountMismatch { expected, got, .. }) => {
            assert_eq!(got, 6 + 13);
            assert_eq!(expected, 6 + 39);
        }
        other => panic!("Recent-as-Historical not rejected: {other:?}"),
    }

    // Historical witness relabelled Recent: its target is far outside the ring.
    let hist = Synth::historical(
        FULU_ANCHOR,
        CAPELLA_SLOT + 400 * SLOTS_PER_HISTORICAL_ROOT + 3,
    )
    .build();
    control(&hist);
    let mut as_recent = hist.clone();
    as_recent.path = PathKind::Recent;
    assert!(matches!(
        run(&enc(&as_recent)),
        Err(Error::TargetOutsideRecentRange { .. })
    ));

    // ...and a Historical target that IS inside the ring still fails, on length,
    // rather than sneaking through the range check.
    let mut near = hist.clone();
    near.path = PathKind::Recent;
    near.target_slot = FULU_ANCHOR - 5;
    assert!(matches!(
        run(&enc(&near)),
        Err(Error::SiblingCountMismatch { .. })
    ));
}

// ---------------------------------------------------------------------------
// Adversarial: the wire format
// ---------------------------------------------------------------------------

#[test]
fn bad_magic_is_rejected() {
    let mut blob = enc(&Synth::recent(FULU_ANCHOR, 60).build());
    assert!(run(&blob).is_ok());
    blob[OFF_MAGIC] ^= 0x01;
    match run(&blob) {
        Err(Error::BadMagic { got }) => assert_eq!(got, BLOB_MAGIC ^ 0x01),
        other => panic!("bad magic not rejected: {other:?}"),
    }
}

#[test]
fn a_single_trailing_byte_is_rejected() {
    let mut blob = enc(&Synth::recent(FULU_ANCHOR, 60).build());
    assert!(run(&blob).is_ok());
    blob.push(0x00);
    assert_eq!(run(&blob), Err(Error::TrailingBytes { extra: 1 }));
}

#[test]
fn a_short_read_is_rejected_at_every_truncation_point() {
    let blob = enc(&Synth::recent(FULU_ANCHOR, 60).build());
    assert!(run(&blob).is_ok());
    for cut in 0..blob.len() {
        assert!(
            matches!(
                run(&blob[..cut]),
                Err(Error::Truncated { .. }) | Err(Error::BadMagic { .. })
            ),
            "truncation to {cut} bytes was not rejected"
        );
    }
}

#[test]
fn an_unknown_path_kind_byte_is_rejected() {
    let mut blob = enc(&Synth::recent(FULU_ANCHOR, 60).build());
    assert!(run(&blob).is_ok());
    for byte in [2u8, 3, 0xff] {
        blob[OFF_PATH_KIND] = byte;
        assert_eq!(run(&blob), Err(Error::BadPathKind { got: byte }));
    }
}

#[test]
fn a_lying_sibling_count_cannot_over_read_the_blob() {
    let mut blob = enc(&Synth::recent(FULU_ANCHOR, 60).build());
    assert!(run(&blob).is_ok());
    blob[OFF_N_SIBLINGS..OFF_N_SIBLINGS + 2].copy_from_slice(&u16::MAX.to_le_bytes());
    assert!(matches!(run(&blob), Err(Error::Truncated { .. })));
}

#[test]
fn the_blob_offsets_are_where_the_layout_says() {
    let w = Synth::recent(FULU_ANCHOR, 60).build();
    let blob = enc(&w);
    assert_eq!(
        blob.len(),
        BLOB_FIXED_LEN + 32 * (w.siblings.len() + w.payload_leaves.len())
    );
    assert_eq!(&blob[OFF_MAGIC..4], &BLOB_MAGIC.to_le_bytes());
    assert_eq!(&blob[OFF_MAGIC..4], b"1NAW", "u32 LE of \"WAN1\"");
    assert_eq!(
        &blob[OFF_TARGET_SLOT..OFF_TARGET_SLOT + 8],
        &w.target_slot.to_le_bytes()
    );
    assert_eq!(blob[OFF_PATH_KIND], 0);
    assert_eq!(
        &blob[OFF_N_SIBLINGS..OFF_N_SIBLINGS + 2],
        &(w.siblings.len() as u16).to_le_bytes()
    );
    assert_eq!(decode_blob(&blob).unwrap(), w, "encode/decode round-trips");
}

// ---------------------------------------------------------------------------
// KATs
// ---------------------------------------------------------------------------

#[test]
fn statement_byte_layout_kat() {
    let st = Statement {
        kind: ANCHOR_KIND,
        anchor_beacon_root: [0xaa; 32],
        el_block_number: 25_778_841,
        el_block_hash: [0xbb; 32],
        anchor_slot: 13_300_000,
        target_slot: 12_345_678,
    };
    let b = st.to_bytes();
    assert_eq!(b.len(), STATEMENT_LEN);
    assert_eq!(STATEMENT_LEN, 96);

    // Fixed offsets, all integers little-endian.
    assert_eq!(&b[0..8], &0x4101u64.to_le_bytes());
    assert_eq!(&b[0..8], &[0x01, 0x41, 0, 0, 0, 0, 0, 0]);
    assert_eq!(&b[8..40], &[0xaa; 32]);
    assert_eq!(&b[40..48], &25_778_841u64.to_le_bytes());
    assert_eq!(&b[40..48], &[0x99, 0x5a, 0x89, 0x01, 0, 0, 0, 0]);
    assert_eq!(&b[48..80], &[0xbb; 32]);
    assert_eq!(&b[80..88], &13_300_000u64.to_le_bytes());
    assert_eq!(&b[88..96], &12_345_678u64.to_le_bytes());

    assert_eq!(
        hex::encode(b),
        concat!(
            "0141000000000000",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "995a890100000000",
            "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            "20f1ca0000000000",
            "4e61bc0000000000",
        )
    );
    assert_eq!(Statement::from_bytes(&b).unwrap(), st);

    // 96 is outside every other Willow statement length, so a cross-family
    // mix-up is impossible on length alone.
    for other in [136usize, 152, 160, 184, 200, 224, 232, 264] {
        assert_ne!(STATEMENT_LEN, other);
    }
}

#[test]
fn gindex_kat() {
    // --- Fulu, Recent, target_slot % 8192 == 0 --------------------------------
    // Fulu BeaconState has 38 fields -> next_power_of_two = 64 -> depth d = 6.
    // `block_roots` is field 5, so its state gindex is 64 + 5 = 69.
    // Descending 13 levels into the 8,192-entry Vector at index 0:
    //     g = 69 << 13 | 0 = 69 * 8192 = 565,248
    // depth(g) = 6 + 13 = 19, and 2^19 = 524,288 <= 565,248 < 2^20 = 1,048,576.
    let target = FULU_SLOT + SLOTS_PER_HISTORICAL_ROOT; // 13,172,736; % 8192 == 0
    assert_eq!(target % SLOTS_PER_HISTORICAL_ROOT, 0);
    let g = state_to_block_root_gindex(ForkVersion::Fulu, PathKind::Recent, target + 1, target)
        .unwrap();
    assert_eq!(g, 565_248);
    assert_eq!(g, 69 << 13);
    assert_eq!(depth_for_gindex(g), 19);

    // --- Fulu, Historical, chunk 0, idx 0 ------------------------------------
    // `historical_summaries` is field 27:            g = 64 + 27 = 91
    // List data root is the LEFT child of mix_in_length:
    //                                                g = 91 * 2 = 182
    // element `chunk` of the 2^24-leaf data tree:    g = (182 << 24) | 0
    //                                                  = 182 * 16,777,216
    //                                                  = 3,053,453,312
    // `block_summary_root` is field 0 of 2:          g = 3,053,453,312 * 2
    //                                                  = 6,106,906,624
    // `block_roots[idx]`, 13 levels:                 g = 6,106,906,624 << 13
    //                                                  = 6,106,906,624 * 8,192
    //                                                  = 50,027,779,063,808
    // depth(g) = 6 + 1 + 24 + 1 + 13 = 45, and
    //     2^45 = 35,184,372,088,832 <= 50,027,779,063,808 < 2^46 = 70,368,744,177,664.
    // chunk 0 / idx 0 is exactly CAPELLA_SLOT, because Capella began on a period
    // boundary (6,209,536 = 758 * 8,192).
    let g = state_to_block_root_gindex(
        ForkVersion::Fulu,
        PathKind::Historical,
        FULU_ANCHOR,
        CAPELLA_SLOT,
    )
    .unwrap();
    assert_eq!(g, 50_027_779_063_808);
    assert_eq!(g, ((((91u64 * 2) << 24) * 2) << 13));
    assert_eq!(depth_for_gindex(g), 45);

    // --- Deneb, Recent, idx 0: depth 5 halves the state gindex ---------------
    // 28 fields -> 32 slots -> d = 5; block_roots gindex 32 + 5 = 37;
    //     g = 37 << 13 = 303,104, depth 5 + 13 = 18.
    let dtarget = DENEB_SLOT + SLOTS_PER_HISTORICAL_ROOT;
    let g = state_to_block_root_gindex(ForkVersion::Deneb, PathKind::Recent, dtarget + 1, dtarget)
        .unwrap();
    assert_eq!(g, 303_104);
    assert_eq!(g, 37 << 13);
    assert_eq!(depth_for_gindex(g), 18);
}

#[test]
fn capella_began_on_a_period_boundary() {
    // The historical path computes `chunk = (slot - CAPELLA_SLOT) / 8192` but
    // `idx = slot % 8192`. Those describe the same period only because
    // CAPELLA_SLOT is a multiple of SLOTS_PER_HISTORICAL_ROOT.
    assert_eq!(CAPELLA_SLOT % SLOTS_PER_HISTORICAL_ROOT, 0);
    assert_eq!(CAPELLA_SLOT / SLOTS_PER_HISTORICAL_ROOT, 758);
    // The other three pinned forks happen to land on boundaries too, but nothing
    // depends on that — only Capella's does.
    for slot in [DENEB_SLOT, ELECTRA_SLOT, FULU_SLOT] {
        assert!(slot > CAPELLA_SLOT);
    }
}

#[test]
fn the_fixed_gindices_have_the_depths_their_arrays_assume() {
    // `Witness::branch_state_root` is [Bytes32; 3] and `branch_payload` is
    // [Bytes32; 7]; `branch_root` rejects a length that disagrees with its
    // gindex, so these are the runtime twins of the `const _: ()` asserts in lib.rs.
    assert_eq!(BLOCK_STATE_ROOT_GINDEX, 8 + 3);
    assert_eq!(depth_for_gindex(BLOCK_STATE_ROOT_GINDEX), 3);
    assert_eq!(PAYLOAD_TO_BLOCK_ROOT_GINDEX, ((8 + 4) << 4) | 9);
    assert_eq!(PAYLOAD_TO_BLOCK_ROOT_GINDEX, 201);
    assert_eq!(depth_for_gindex(PAYLOAD_TO_BLOCK_ROOT_GINDEX), 7);
}

#[test]
fn the_closed_form_sibling_count_matches_the_gindex_depth() {
    for fork in beacon_cl_core::fork::ALL_FORKS {
        let d = state_tree_depth(fork);
        let anchor = CAPELLA_SLOT + 900 * SLOTS_PER_HISTORICAL_ROOT;
        let g = state_to_block_root_gindex(fork, PathKind::Recent, anchor, anchor - 5).unwrap();
        assert_eq!(
            depth_for_gindex(g),
            expected_sibling_count(fork, PathKind::Recent)
        );
        assert_eq!(depth_for_gindex(g), d + 13);

        let g = state_to_block_root_gindex(fork, PathKind::Historical, anchor, CAPELLA_SLOT + 7)
            .unwrap();
        assert_eq!(
            depth_for_gindex(g),
            expected_sibling_count(fork, PathKind::Historical)
        );
        assert_eq!(depth_for_gindex(g), d + 39);
    }
}

#[test]
fn payload_shapes_are_derived_not_hardcoded() {
    assert_eq!(payload_field_count(ForkVersion::Capella), 15);
    assert_eq!(payload_tree_depth(ForkVersion::Capella), 4);
    for fork in [ForkVersion::Deneb, ForkVersion::Electra, ForkVersion::Fulu] {
        assert_eq!(payload_field_count(fork), 17);
        assert_eq!(payload_tree_depth(fork), 5);
    }
}

#[test]
fn fork_at_pins_the_mainnet_schedule() {
    let cases: [(u64, ForkVersion); 8] = [
        (CAPELLA_SLOT, ForkVersion::Capella),
        (DENEB_SLOT - 1, ForkVersion::Capella),
        (DENEB_SLOT, ForkVersion::Deneb),
        (ELECTRA_SLOT - 1, ForkVersion::Deneb),
        (ELECTRA_SLOT, ForkVersion::Electra),
        (FULU_SLOT - 1, ForkVersion::Electra),
        (FULU_SLOT, ForkVersion::Fulu),
        (u64::MAX, ForkVersion::Fulu),
    ];
    for (slot, fork) in cases {
        assert_eq!(fork_at(slot).unwrap(), fork, "slot {slot}");
    }
    assert!(fork_at(0).is_err());
    assert!(fork_at(CAPELLA_SLOT - 1).is_err());
}

#[test]
fn the_state_field_positions_match_the_real_declarations() {
    // `beacon_cl_core::fork` guards the field COUNT at compile time but has no
    // `StateField` variant for `block_roots` or `historical_summaries`, so this
    // crate pins those two positions itself — against the same `crates/network`
    // declarations `state_fetcher.rs` merkleizes real mainnet states with.
    const ELECTRA_SRC: &str =
        include_str!("../../../../crates/network/src/beacon/electra_final.rs");
    const FULU_SRC: &str = include_str!("../../../../crates/network/src/beacon/fulu_final.rs");

    fn nth_field(src: &str, n: usize) -> String {
        let body = src
            .split("pub struct BeaconState<")
            .nth(1)
            .expect("BeaconState declaration");
        let body = body.split("\n> {\n").nth(1).expect("field list");
        let body = body.split("\n}").next().expect("closing brace");
        body.split("\n    pub ")
            .skip(1)
            .nth(n)
            .and_then(|f| f.split(':').next())
            .expect("field")
            .trim()
            .to_string()
    }

    for src in [ELECTRA_SRC, FULU_SRC] {
        assert_eq!(nth_field(src, BLOCK_ROOTS_POSITION), "block_roots");
        assert_eq!(
            nth_field(src, HISTORICAL_SUMMARIES_POSITION),
            "historical_summaries"
        );
        // The scanner really is reading the field list, not matching anywhere.
        assert_eq!(nth_field(src, 0), "genesis_time");
        assert_eq!(nth_field(src, 2), "slot");
    }
}

#[test]
fn path_kind_round_trips_and_rejects_everything_else() {
    for p in [PathKind::Recent, PathKind::Historical] {
        assert_eq!(PathKind::from_u8(p.as_u8()).unwrap(), p);
    }
    for b in 2u8..=255 {
        assert_eq!(PathKind::from_u8(b), Err(Error::BadPathKind { got: b }));
    }
}

#[test]
fn every_error_variant_has_a_distinct_message() {
    let variants = [
        Error::BadMagic { got: 1 },
        Error::Truncated {
            field: "x",
            need: 1,
            have: 0,
        },
        Error::TrailingBytes { extra: 1 },
        Error::BadPathKind { got: 2 },
        Error::PayloadLeafCountInvalid { got: 16 },
        Error::PreCapellaSlot { slot: 1 },
        Error::TargetNotBeforeAnchor {
            anchor_slot: 1,
            target_slot: 1,
        },
        Error::TargetOutsideRecentRange {
            anchor_slot: 2,
            target_slot: 1,
        },
        Error::HistoricalChunkOutOfRange { chunk: 1 },
        Error::SiblingCountMismatch {
            gindex: 1,
            expected: 1,
            got: 2,
        },
        Error::PayloadLeafCountMismatch {
            expected: 15,
            got: 17,
        },
        Error::BlockRootBranchInvalid { gindex: 1 },
        Error::BlockNumberLeafNotCanonical,
    ];
    let mut seen: Vec<String> = variants.iter().map(|e| e.to_string()).collect();
    seen.sort();
    let n = seen.len();
    seen.dedup();
    assert_eq!(seen.len(), n, "two Error variants Display identically");
}
