//! Anchor-child tests: real-mainnet KATs (the committed period-1762 fixtures)
//! through `verify::run`/`verify_witness` — the EXACT guest path — plus a fully
//! synthetic committee (own keys) for the adversarial cases real chain data
//! cannot express (period handoff, sub-threshold participation, extra signer).
//! Every negative is a control/fix pair: the mutation trips the exact `Err`, the
//! honest witness still verifies.

use beacon_cl_core::fork::{ForkVersion, StateField, StateLeaves};
use beacon_cl_core::ssz::{hash_pair, uint64_leaf, Bytes32};
use bls12_381::{G1Projective, G2Affine, G2Projective, Scalar};

use crate::bls::hash_to_g2;
use crate::gindex::{finalized_root_gindex, next_sync_committee_gindex};
use crate::header::{
    compute_domain, compute_signing_root, sync_committee_hash_tree_root, DOMAIN_SYNC_COMMITTEE,
    SYNC_COMMITTEE_SIZE,
};
use crate::schedule::{fork_version_bytes, FULU_SLOT, SLOTS_PER_SYNC_COMMITTEE_PERIOD};
use crate::verify::{
    decode_blob, encode_blob, run, verify_witness, AnchorError, AnchorWitness, CommitteeWitness,
    HeaderFields, MIN_PARTICIPANTS,
};
use crate::wire::{AnchorStatement, ANCHOR_KIND, ANCHOR_LEN};

// ---------------------------------------------------------------------------
// Fixture loading — the SAME committed mainnet data the native light client
// KATs against (period-1762 committee; Fulu finality update).
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct BootstrapFx {
    pubkeys: Vec<String>,
    aggregate_pubkey: String,
}

#[derive(serde::Deserialize)]
struct UpdateFx {
    attested_slot: String,
    attested_proposer_index: String,
    attested_parent_root: String,
    attested_state_root: String,
    attested_body_root: String,
    signature_slot: String,
    sync_committee_bits: String,
    sync_committee_signature: String,
    finalized_slot: String,
    finalized_proposer_index: String,
    finalized_parent_root: String,
    finalized_state_root: String,
    finalized_body_root: String,
    finality_branch: Vec<String>,
    genesis_validators_root: String,
    fork_version: String,
}

fn unhex_vec(s: &str) -> Vec<u8> {
    hex::decode(s.trim_start_matches("0x")).expect("hex")
}
fn unhex<const N: usize>(s: &str) -> [u8; N] {
    unhex_vec(s).try_into().expect("length")
}

/// The real-mainnet witness: period-1762 committee signing a Fulu finality
/// update, exactly as fetched from a beacon node.
fn mainnet_witness() -> AnchorWitness {
    let bs: BootstrapFx =
        serde_json::from_str(include_str!("../fixtures/mainnet_lc_bootstrap.json")).unwrap();
    let up: UpdateFx =
        serde_json::from_str(include_str!("../fixtures/mainnet_lc_finality_update.json")).unwrap();
    assert_eq!(bs.pubkeys.len(), SYNC_COMMITTEE_SIZE);

    let mut pubkeys = Box::new([[0u8; 48]; SYNC_COMMITTEE_SIZE]);
    for (dst, pk) in pubkeys.iter_mut().zip(bs.pubkeys.iter()) {
        *dst = unhex::<48>(pk);
    }
    // Cross-check: the derived signing-domain fork version must be the one the
    // beacon node reported for this update.
    let sig_slot: u64 = up.signature_slot.parse().unwrap();
    let fork_sig = crate::schedule::fork_at_slot(sig_slot - 1).unwrap();
    assert_eq!(fork_version_bytes(fork_sig), unhex::<4>(&up.fork_version));

    AnchorWitness {
        attested: HeaderFields {
            slot: up.attested_slot.parse().unwrap(),
            proposer_index: up.attested_proposer_index.parse().unwrap(),
            parent_root: unhex::<32>(&up.attested_parent_root),
            state_root: unhex::<32>(&up.attested_state_root),
            body_root: unhex::<32>(&up.attested_body_root),
        },
        finalized: HeaderFields {
            slot: up.finalized_slot.parse().unwrap(),
            proposer_index: up.finalized_proposer_index.parse().unwrap(),
            parent_root: unhex::<32>(&up.finalized_parent_root),
            state_root: unhex::<32>(&up.finalized_state_root),
            body_root: unhex::<32>(&up.finalized_body_root),
        },
        signature_slot: sig_slot,
        genesis_validators_root: unhex::<32>(&up.genesis_validators_root),
        participation_bits: unhex_vec(&up.sync_committee_bits).try_into().unwrap(),
        signature: unhex::<96>(&up.sync_committee_signature),
        committee: CommitteeWitness {
            pubkeys,
            aggregate_pubkey: unhex::<48>(&bs.aggregate_pubkey),
        },
        finality_branch: up.finality_branch.iter().map(|h| unhex::<32>(h)).collect(),
        next_committee: None,
    }
}

fn run_witness(w: &AnchorWitness) -> Result<AnchorStatement, AnchorError> {
    // Through the FULL guest path: serialize → parse → verify.
    run(&encode_blob(w))
}

// ---------------------------------------------------------------------------
// Real-mainnet KATs
// ---------------------------------------------------------------------------

#[test]
fn real_mainnet_anchor_verifies_and_binds_the_finalized_pair() {
    let w = mainnet_witness();
    let stmt = run_witness(&w).expect("the real mainnet finality update must verify");
    assert_eq!(stmt.kind, ANCHOR_KIND);
    assert_eq!(stmt.fork, 3, "slot 14,435,200 is Fulu");
    assert_eq!(stmt.beacon_state_root, w.finalized.state_root);
    assert_eq!(stmt.slot, 14_435_200);
    assert_eq!(stmt.attested_slot, 14_435_284);
    assert_eq!(stmt.signature_slot, 14_435_285);
    assert!(
        stmt.participation as usize >= MIN_PARTICIPANTS,
        "near-full mainnet participation expected, got {}",
        stmt.participation
    );
    assert_eq!(stmt.sync_committee_root, w.committee.root());
    assert_eq!(
        stmt.next_sync_committee_root, [0u8; 32],
        "no handoff section in this witness"
    );
    assert_eq!(stmt.encode().len(), ANCHOR_LEN);
}

#[test]
fn statement_roundtrips_and_rejects_wrong_length() {
    let stmt = run_witness(&mainnet_witness()).unwrap();
    let bytes = stmt.encode();
    assert_eq!(AnchorStatement::decode_bytes(&bytes), Some(stmt));
    assert_eq!(
        AnchorStatement::decode_bytes(&bytes[..ANCHOR_LEN - 1]),
        None
    );
    let mut long = bytes.clone();
    long.push(0);
    assert_eq!(AnchorStatement::decode_bytes(&long), None);
}

/// THE gap this child closes: a fabricated-but-internally-consistent
/// `beacon_state_root` — not the one the committee signed over — must be
/// rejected by the PROOF, not by consumer diligence.
#[test]
fn fabricated_beacon_state_root_is_rejected() {
    let mut w = mainnet_witness();
    let honest = w.finalized.state_root;
    w.finalized.state_root = [0x42; 32]; // internally consistent: every derived value recomputed
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::FinalityBranchInvalid
    );
    // Control/fix: restoring the honest root restores the anchor.
    w.finalized.state_root = honest;
    assert!(run_witness(&w).is_ok());
}

/// A mislabeled slot — right state root, wrong claimed slot — dies in the same
/// recomputed-header-root binding.
#[test]
fn mislabeled_finalized_slot_is_rejected() {
    let mut w = mainnet_witness();
    let honest = w.finalized.slot;
    for bad in [honest - 32, honest + 32, honest + 1] {
        w.finalized.slot = bad;
        assert_eq!(
            run_witness(&w).unwrap_err(),
            AnchorError::FinalityBranchInvalid,
            "slot {bad} must not anchor"
        );
    }
    w.finalized.slot = honest;
    assert!(run_witness(&w).is_ok());
}

/// A mislabeled ATTESTED slot changes the signed header root, so it dies at the
/// signature, not the branch.
#[test]
fn mislabeled_attested_slot_is_rejected_by_the_signature() {
    let mut w = mainnet_witness();
    let honest = w.attested.slot;
    // −1, not +1: the fixture's signature_slot is attested+1, so +1 would trip
    // the ordering check before the signature.
    w.attested.slot = honest - 1;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
    w.attested.slot = honest;
    assert!(run_witness(&w).is_ok());
}

/// The attested state root is bound twice over: it is a field of the SIGNED
/// header AND the root the finality branch folds to. Tampering it breaks the
/// signature first.
#[test]
fn tampered_attested_state_root_is_rejected() {
    let mut w = mainnet_witness();
    w.attested.state_root[0] ^= 1;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
}

#[test]
fn forged_finality_branch_is_rejected_at_every_level() {
    let w0 = mainnet_witness();
    assert_eq!(w0.finality_branch.len(), 7, "Electra/Fulu gindex 169 depth");
    for level in 0..w0.finality_branch.len() {
        let mut w = mainnet_witness();
        w.finality_branch[level][0] ^= 1;
        assert_eq!(
            run_witness(&w).unwrap_err(),
            AnchorError::FinalityBranchInvalid,
            "level {level}"
        );
    }
}

#[test]
fn claiming_an_extra_signer_is_rejected() {
    let mut w = mainnet_witness();
    // The real update has a few unset bits; set the first unset one.
    let unset = (0..SYNC_COMMITTEE_SIZE)
        .find(|&i| !crate::header::bit_set(&w.participation_bits, i))
        .expect("mainnet update is never literally 512/512");
    w.participation_bits[unset / 8] |= 1 << (unset % 8);
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
}

#[test]
fn dropping_a_real_signer_is_rejected() {
    let mut w = mainnet_witness();
    let set = (0..SYNC_COMMITTEE_SIZE)
        .find(|&i| crate::header::bit_set(&w.participation_bits, i))
        .unwrap();
    w.participation_bits[set / 8] &= !(1 << (set % 8));
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
}

#[test]
fn substituted_committee_member_is_rejected() {
    let mut w = mainnet_witness();
    let signer = (0..SYNC_COMMITTEE_SIZE)
        .find(|&i| crate::header::bit_set(&w.participation_bits, i))
        .unwrap();
    // A VALID foreign point (another member's key) in a signer's slot: the
    // aggregate pubkey shifts, the pairing fails.
    let foreign = w.committee.pubkeys[signer + 1];
    w.committee.pubkeys[signer] = foreign;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
}

#[test]
fn garbage_pubkey_and_signature_bytes_are_rejected_as_invalid_points() {
    let mut w = mainnet_witness();
    let signer = (0..SYNC_COMMITTEE_SIZE)
        .find(|&i| crate::header::bit_set(&w.participation_bits, i))
        .unwrap();
    w.committee.pubkeys[signer] = [0xFF; 48];
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::InvalidPublicKey(signer)
    );

    let mut w = mainnet_witness();
    w.signature = [0xFF; 96];
    assert_eq!(run_witness(&w).unwrap_err(), AnchorError::InvalidSignature);
}

#[test]
fn sub_threshold_participation_is_rejected_with_the_exact_count() {
    let mut w = mainnet_witness();
    // Keep exactly MIN_PARTICIPANTS − 1 of the real signers.
    let mut kept = 0usize;
    let mut bits = [0u8; SYNC_COMMITTEE_SIZE / 8];
    for i in 0..SYNC_COMMITTEE_SIZE {
        if kept < MIN_PARTICIPANTS - 1 && crate::header::bit_set(&w.participation_bits, i) {
            bits[i / 8] |= 1 << (i % 8);
            kept += 1;
        }
    }
    w.participation_bits = bits;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::LowParticipation {
            got: MIN_PARTICIPANTS - 1,
            need: MIN_PARTICIPANTS,
        }
    );
}

#[test]
fn wrong_genesis_validators_root_is_rejected() {
    let mut w = mainnet_witness();
    w.genesis_validators_root = [0xAA; 32]; // a different chain's domain
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureVerificationFailed
    );
}

#[test]
fn slot_ordering_violations_are_rejected() {
    let mut w = mainnet_witness();
    w.signature_slot = w.attested.slot; // must be strictly after
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureSlotNotAfterAttested {
            signature_slot: w.attested.slot,
            attested_slot: w.attested.slot,
        }
    );

    let mut w = mainnet_witness();
    w.finalized.slot = w.attested.slot + 1;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::FinalizedAfterAttested {
            finalized_slot: w.attested.slot + 1,
            attested_slot: w.attested.slot,
        }
    );
}

/// The contract attributes the handoff to `period(signature_slot)+1`, but it is
/// proven under the ATTESTED state — a boundary-straddling signature would shift
/// the attribution off by one. Without this check the mutation verifies
/// end-to-end (the signing domain and every branch are unchanged within Fulu).
#[test]
fn period_straddling_signature_is_rejected() {
    let mut w = mainnet_witness();
    let honest = w.signature_slot;
    let attested_period = w.attested.slot / SLOTS_PER_SYNC_COMMITTEE_PERIOD;
    assert_eq!(attested_period, 1762, "the committed fixture's period");
    assert_eq!(
        honest / SLOTS_PER_SYNC_COMMITTEE_PERIOD,
        attested_period,
        "the real fixture satisfies the same-period invariant"
    );
    // Spec-legal ordering (still strictly after attested), but the FIRST slot of
    // period 1763: the NEXT committee would be the signer.
    w.signature_slot = (attested_period + 1) * SLOTS_PER_SYNC_COMMITTEE_PERIOD;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::SignatureNotInAttestedPeriod {
            signature_slot: (attested_period + 1) * SLOTS_PER_SYNC_COMMITTEE_PERIOD,
            attested_slot: w.attested.slot,
        }
    );
    // Control/fix: the honest same-period signature slot verifies.
    w.signature_slot = honest;
    assert!(run_witness(&w).is_ok());
}

#[test]
fn truncated_blob_is_rejected() {
    let blob = encode_blob(&mainnet_witness());
    for cut in [0, 10, 80, 240, blob.len() - 1] {
        assert_eq!(run(&blob[..cut]).unwrap_err(), AnchorError::Truncated);
    }
}

#[test]
fn blob_roundtrips_through_the_wire() {
    let w = mainnet_witness();
    let blob = encode_blob(&w);
    let (w2, fork_att) = decode_blob(&blob).unwrap();
    assert_eq!(fork_att, ForkVersion::Fulu);
    assert_eq!(w2.attested, w.attested);
    assert_eq!(w2.finalized, w.finalized);
    assert_eq!(w2.finality_branch, w.finality_branch);
    assert_eq!(verify_witness(&w2, fork_att), run(&blob));
}

// ---------------------------------------------------------------------------
// Synthetic committee (own keys): the cases real chain data cannot express —
// the period handoff and controlled-participation variants.
// ---------------------------------------------------------------------------

fn synth_sk(i: usize) -> Scalar {
    Scalar::from(i as u64 + 1)
}

fn synth_committee(seed: u64) -> CommitteeWitness {
    let mut pubkeys = Box::new([[0u8; 48]; SYNC_COMMITTEE_SIZE]);
    for (i, pk) in pubkeys.iter_mut().enumerate() {
        let p = G1Projective::generator() * synth_sk(i + seed as usize * SYNC_COMMITTEE_SIZE);
        *pk = bls12_381::G1Affine::from(p).to_compressed();
    }
    let agg = G1Projective::generator() * Scalar::from(0xA6u64 + seed);
    CommitteeWitness {
        pubkeys,
        aggregate_pubkey: bls12_381::G1Affine::from(agg).to_compressed(),
    }
}

/// A fully synthetic Fulu-era witness WITH a period-handoff section: our own
/// 512-key committee signs an attested header whose state commits both a
/// finalized checkpoint and a next committee, all branches derived from the real
/// `StateLeaves` container layout.
fn synth_witness(n_signers: usize) -> AnchorWitness {
    let committee = synth_committee(0);
    let next = synth_committee(1);
    let genesis_validators_root = [0xAB; 32];

    let finalized = HeaderFields {
        slot: FULU_SLOT + 64,
        proposer_index: 7,
        parent_root: [0x01; 32],
        state_root: [0x02; 32],
        body_root: [0x03; 32],
    };
    let finalized_root = finalized.root();

    // The attested BeaconState: finalized_checkpoint at field 20 (a Checkpoint
    // container — epoch leaf ‖ root), next_sync_committee at field 23.
    let fork = ForkVersion::Fulu;
    let mut state = StateLeaves::new(fork);
    let checkpoint_epoch = uint64_leaf((finalized.slot) / 32);
    state.set_position(20, hash_pair(&checkpoint_epoch, &finalized_root));
    let next_root = next.root();
    state.set_position(23, next_root);
    // Unrelated fields set so the tree is not degenerate.
    state.set(StateField::Slot, uint64_leaf(FULU_SLOT + 160));
    state.set(StateField::Validators, [0x11; 32]);

    // finality_branch (gindex 169 = 2·84 + 1): the checkpoint's epoch leaf, then
    // the field-20 branch. next branch: the field-23 branch (gindex 87).
    let mut finality_branch = vec![checkpoint_epoch];
    finality_branch.extend(state.branch_at_gindex(64 + 20));
    assert_eq!(finality_branch.len(), 7);
    assert_eq!(finalized_root_gindex(fork), 169);
    let next_branch = state.branch_at_gindex(64 + 23);
    assert_eq!(next_sync_committee_gindex(fork), 87);

    let attested = HeaderFields {
        slot: FULU_SLOT + 160,
        proposer_index: 9,
        parent_root: [0x04; 32],
        state_root: state.root(),
        body_root: [0x05; 32],
    };
    let signature_slot = attested.slot + 1;

    // Sign with the first n_signers keys.
    let fork_sig = crate::schedule::fork_at_slot(signature_slot - 1).unwrap();
    let domain = compute_domain(
        DOMAIN_SYNC_COMMITTEE,
        fork_version_bytes(fork_sig),
        &genesis_validators_root,
    );
    let signing_root = compute_signing_root(&attested.root(), &domain);
    let msg_hash = hash_to_g2(&signing_root);
    let mut bits = [0u8; SYNC_COMMITTEE_SIZE / 8];
    let mut agg_sig = G2Projective::identity();
    for i in 0..n_signers {
        bits[i / 8] |= 1 << (i % 8);
        agg_sig += msg_hash * synth_sk(i);
    }
    let signature = G2Affine::from(agg_sig).to_compressed();

    AnchorWitness {
        attested,
        finalized,
        signature_slot,
        genesis_validators_root,
        participation_bits: bits,
        signature,
        committee,
        finality_branch,
        next_committee: Some((next, next_branch)),
    }
}

#[test]
fn synthetic_anchor_with_handoff_verifies() {
    let w = synth_witness(400);
    let stmt = run_witness(&w).expect("honest synthetic witness must verify");
    assert_eq!(stmt.participation, 400);
    assert_eq!(stmt.beacon_state_root, [0x02; 32]);
    assert_eq!(stmt.slot, FULU_SLOT + 64);
    assert_eq!(
        stmt.next_sync_committee_root,
        w.next_committee.as_ref().unwrap().0.root(),
        "the handoff commits the NEXT period's committee root"
    );
    assert_ne!(stmt.next_sync_committee_root, stmt.sync_committee_root);
}

#[test]
fn forged_next_committee_branch_is_rejected_at_every_level() {
    let w0 = synth_witness(400);
    let depth = w0.next_committee.as_ref().unwrap().1.len();
    assert_eq!(depth, 6);
    for level in 0..depth {
        let mut w = synth_witness(400);
        w.next_committee.as_mut().unwrap().1[level][0] ^= 1;
        assert_eq!(
            run_witness(&w).unwrap_err(),
            AnchorError::NextCommitteeBranchInvalid,
            "level {level}"
        );
    }
}

#[test]
fn tampered_next_committee_key_is_rejected() {
    let mut w = synth_witness(400);
    w.next_committee.as_mut().unwrap().0.pubkeys[0][0] ^= 1;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::NextCommitteeBranchInvalid
    );
}

/// A handoff cannot be smuggled from a DIFFERENT state: swapping in the honest
/// branch of another period's state dies because the attested state root — the
/// value the committee signed — is where the branch must fold to.
#[test]
fn handoff_from_a_foreign_state_is_rejected() {
    let mut w = synth_witness(400);
    let foreign = synth_committee(2);
    w.next_committee.as_mut().unwrap().0 = foreign;
    assert_eq!(
        run_witness(&w).unwrap_err(),
        AnchorError::NextCommitteeBranchInvalid
    );
}

#[test]
fn omitting_the_handoff_commits_zeros() {
    let mut w = synth_witness(400);
    w.next_committee = None;
    let stmt = run_witness(&w).unwrap();
    assert_eq!(stmt.next_sync_committee_root, [0u8; 32]);
}

#[test]
fn synthetic_threshold_boundary_is_exact() {
    // MIN_PARTICIPANTS signers pass; one fewer fails with the exact counts.
    let stmt = run_witness(&synth_witness(MIN_PARTICIPANTS)).expect("342 must pass");
    assert_eq!(stmt.participation as usize, MIN_PARTICIPANTS);
    assert_eq!(
        run_witness(&synth_witness(MIN_PARTICIPANTS - 1)).unwrap_err(),
        AnchorError::LowParticipation {
            got: MIN_PARTICIPANTS - 1,
            need: MIN_PARTICIPANTS,
        }
    );
}

#[test]
fn pre_capella_slots_are_refused() {
    let mut w = synth_witness(400);
    w.attested.slot = 1_000_000;
    // Attested pre-Capella dies at DECODE (the branch depth derives from it).
    assert_eq!(
        run(&encode_blob(&w)).unwrap_err(),
        AnchorError::PreCapellaSlot(1_000_000)
    );
}

#[test]
fn min_participants_is_the_spec_supermajority() {
    assert_eq!(MIN_PARTICIPANTS, 342);
    assert!(3 * MIN_PARTICIPANTS >= 2 * SYNC_COMMITTEE_SIZE);
    assert!(3 * (MIN_PARTICIPANTS - 1) < 2 * SYNC_COMMITTEE_SIZE);
}

// ---------------------------------------------------------------------------
// Statement-layout pins (the aggregate and the contract parse these offsets)
// ---------------------------------------------------------------------------

#[test]
fn statement_layout_is_pinned() {
    let stmt = AnchorStatement {
        kind: ANCHOR_KIND,
        fork: 3,
        beacon_state_root: [0xBB; 32],
        slot: 0x1111_2222_3333_4444,
        attested_slot: 2,
        signature_slot: 3,
        participation: 500,
        genesis_validators_root: [0xCC; 32],
        sync_committee_root: [0xDD; 32],
        next_sync_committee_root: [0xEE; 32],
    };
    let b = stmt.encode();
    assert_eq!(b.len(), ANCHOR_LEN);
    assert_eq!(u64::from_le_bytes(b[0..8].try_into().unwrap()), ANCHOR_KIND);
    assert_eq!(&b[16..48], &[0xBB; 32]);
    assert_eq!(
        u64::from_le_bytes(b[48..56].try_into().unwrap()),
        0x1111_2222_3333_4444
    );
    assert_eq!(&b[80..112], &[0xCC; 32]);
    assert_eq!(&b[112..144], &[0xDD; 32]);
    assert_eq!(&b[144..176], &[0xEE; 32]);
}
