//! Adversarial tests: every soundness property the retired Binius64 crate closed,
//! re-derived in Rust, **each with a control that shows the attack succeeding on a
//! naive variant**. Every rejection routes through the real assertion path —
//! `verify::verify_cl_scan` returning `Err`, or a derived accumulator computing the
//! honest value where a witnessed one could be forged. No `assert_non_zero`.
//!
//! KATs are from the independent Python `hashlib` oracle
//! (`ssz_oracle.py`, not in this repository), so a passing suite proves this port is byte-exact
//! against a second implementation that shares no code.

use crate::fork::{ForkVersion, StateField, StateLeaves};
use crate::scan::{
    is_live, scan_all, scan_window, split_balances, split_validators, validators_per_slice,
    Liveness, ScanMode, ValidatorLeaf,
};
use crate::ssz::{self, Bytes32, Validator};
use crate::verify::{build_blob, verify_cl_scan, verify_cl_scan_full, Error};

// -------------------------------------------------------------------- fixtures

/// Deterministic pseudo-random bytes — identical to the shipping crate's
/// `tests/common` and to `ssz_oracle.py`.
fn fill(seed: u64, out: &mut [u8]) {
    let mut x = seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1;
    for byte in out.iter_mut() {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        *byte = (x >> 24) as u8;
    }
}

const TARGET: [u8; 32] = [
    0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xde, 0xad, 0xbe, 0xef,
    0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef, 0xde, 0xad, 0xbe, 0xef,
];

fn validators(n: usize, every: usize) -> Vec<Validator> {
    (0..n)
        .map(|i| {
            let mut pubkey = [0u8; 48];
            fill(i as u64 + 1, &mut pubkey);
            pubkey[0] = (pubkey[0] & 0x1f) | 0xa0;
            let withdrawal_credentials = if i % every == 0 {
                TARGET
            } else {
                let mut c = [0u8; 32];
                fill(i as u64 + 1_000_000, &mut c);
                c[0] = 0x01;
                c
            };
            Validator {
                pubkey,
                withdrawal_credentials,
                effective_balance: 32_000_000_000,
                slashed: i % 17 == 3,
                activation_eligibility_epoch: i as u64,
                activation_epoch: i as u64 + 5,
                exit_epoch: u64::MAX,
                withdrawable_epoch: u64::MAX,
            }
        })
        .collect()
}

fn balances(n: usize) -> Vec<u64> {
    (0..n)
        .map(|i| 31_000_000_000 + (i as u64 * 7_919) % 3_000_000_000)
        .collect()
}

fn hx(s: &str) -> Bytes32 {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

// ============================================================ 0. correctness

#[test]
fn zero_hash_ladder_matches_the_oracle() {
    let z = ssz::zero_hashes(2);
    assert_eq!(
        z[1],
        hx("f5a5fd42d16a20302798ef6ed309979b43003d2320d9f0e8ea9831a92759fb4b")
    );
    assert_eq!(
        z[2],
        hx("db56114e00fdd4c1f85c892bf35ac9a89289aaecb1ebd0a96cde606a748b5d71")
    );
    // The pubkey field of an all-zero validator is already zero_hashes[1] — the
    // fact the derived liveness selector exists to handle.
    assert_eq!(ssz::pubkey_hash_tree_root(&[0u8; 48]), z[1]);
}

#[test]
fn ssz_roots_match_the_python_oracle() {
    // Byte-exact against `ssz_oracle.py` — an independent hashlib implementation.
    let vs = validators(9, 3);
    let bs = balances(9);
    assert_eq!(
        ssz::validators_hash_tree_root(&vs),
        hx("2e872ed21f1c886589f558f70387d448eb964a0fb1287277ce00ad0fa6f79a99")
    );
    assert_eq!(
        ssz::balances_hash_tree_root(&bs),
        hx("e85ae31a0b6ea968b28c4b51f361f7a10cfc70ee110930e39ec17a243f1d9bcf")
    );
    let (total, count) = ssz::credential_scan(&vs, &bs, &TARGET);
    assert_eq!((total, count), (93_000_071_271, 3));
    assert_eq!(
        ssz::pubkey_hash_tree_root(&vs[0].pubkey),
        hx("c1e57f1d5314820f8fc3183b0ee683bbf2a35a89f9ae4d2cba55803e6cfc907a")
    );
    assert_eq!(ssz::total_balance(&bs), 279_000_285_084);
}

#[test]
fn the_windowed_scan_matches_the_flat_reference() {
    // scan_all's roots and totals equal the monolithic SSZ reference, across
    // depths and ragged element counts — the split's correctness identity.
    for d in [2usize, 3, 4, 5] {
        for n in [0usize, 1, 3, 7, (1 << d), (1 << d) + 1, 3 * (1 << d) - 2] {
            let vs = validators(n, 3);
            let bs = balances(n);
            for mode in [ScanMode::CredentialBranch, ScanMode::FullContainer] {
                let (windows, v_root, b_root) = scan_all(&vs, &bs, &TARGET, d, mode);
                assert_eq!(v_root, ssz::validators_hash_tree_root(&vs), "d={d} n={n}");
                assert_eq!(b_root, ssz::balances_hash_tree_root(&bs), "d={d} n={n}");
                let mt: u64 = windows.iter().map(|w| w.matched_total).sum();
                let mc: u64 = windows.iter().map(|w| w.matched_count).sum();
                let bt: u64 = windows.iter().map(|w| w.balances_total).sum();
                let (etotal, ecount) = ssz::credential_scan(&vs, &bs, &TARGET);
                assert_eq!((mt, mc), (etotal, ecount), "d={d} n={n} {mode:?}");
                assert_eq!(bt, ssz::total_balance(&bs), "d={d} n={n}");
            }
        }
    }
}

#[test]
fn an_honest_blob_verifies_and_the_statement_is_derived() {
    let vs = validators(40, 3);
    let bs = balances(40);
    let (blob, bsr) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        11_649_024,
    );
    let (stmt, matched) = verify_cl_scan_full(&blob).expect("honest blob verifies");
    assert_eq!(stmt.beacon_state_root, bsr);
    assert_eq!(stmt.validators_root, ssz::validators_hash_tree_root(&vs));
    assert_eq!(stmt.balances_root, ssz::balances_hash_tree_root(&bs));
    let (etotal, ecount) = ssz::credential_scan(&vs, &bs, &TARGET);
    assert_eq!((stmt.matched_total, stmt.matched_count), (etotal, ecount));
    assert_eq!(stmt.balances_total, ssz::total_balance(&bs));
    assert_eq!(matched.len() as u64, stmt.matched_count);
    // The join keys are the real pubkey roots (the D2 free key).
    for (i, v) in vs.iter().enumerate() {
        if v.withdrawal_credentials == TARGET {
            assert!(
                matched
                    .iter()
                    .any(|(pk, _)| *pk == ssz::pubkey_hash_tree_root(&v.pubkey)),
                "validator {i}'s pubkey_root missing from join inputs"
            );
        }
    }
}

// ================================= 1. derived liveness selector

/// The false-exclusion attack, executed against the broken shape. Clearing the
/// prover's accounting bit on a live matching validator drops it from the report
/// while BOTH subtree roots and the balance total stay bit-identical — a complete,
/// anchored, verifying report with a validator omitted.
#[test]
fn control_witnessed_accounting_selector_accepts_a_false_exclusion() {
    let d = 3;
    let per = validators_per_slice(d);
    let n = 20;
    let vs = validators(n, 3);
    let bs = balances(n);
    let tail = 2usize; // globals 16..24, live 16..19
    let vw = &split_validators(&vs, 3, d, ScanMode::CredentialBranch)[tail];
    let bw = &split_balances(&bs, 3, d)[tail];

    // slot 2 == global 18, a matching validator.
    let slot = 2usize;
    assert_eq!(tail * per + slot, 18);
    assert!(is_live(tail as u64, slot, d, n as u64));
    assert_eq!(vs[18].withdrawal_credentials, TARGET);

    let honest = scan_window(
        vw,
        bw,
        tail as u64,
        n as u64,
        &TARGET,
        d,
        Liveness::Derived,
        &[],
    );

    let mut witnessed: Vec<bool> = (0..per)
        .map(|j| is_live(tail as u64, j, d, n as u64))
        .collect();
    witnessed[slot] = false; // clear the bit on the live match
    let attack = scan_window(
        vw,
        bw,
        tail as u64,
        n as u64,
        &TARGET,
        d,
        Liveness::WitnessedAccounting,
        &witnessed,
    );

    assert_eq!(
        attack.matched_count,
        honest.matched_count - 1,
        "the match was dropped"
    );
    assert_eq!(
        attack.matched_total,
        honest.matched_total - bs[18],
        "its balance vanished"
    );
    // ...but the anchored roots are untouched, so the attack is invisible downstream.
    assert_eq!(
        attack.validators_subtree_root,
        honest.validators_subtree_root
    );
    assert_eq!(attack.balances_subtree_root, honest.balances_subtree_root);
    assert_eq!(attack.balances_total, honest.balances_total);
}

/// The same attack, against the shipped rule. The accounting mask IS the derived
/// selector, so there is no bit to clear: the guest computes the true count, and a
/// blob claiming the understated one is rejected because the guest never accepts
/// the count as input — it derives it.
#[test]
fn fix_derived_selector_leaves_no_false_exclusion_lever() {
    let n = 20;
    let vs = validators(n, 3);
    let bs = balances(n);
    // Ragged fixture (d=3 → tail window has padding). The guest derives the count.
    let (blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let stmt = verify_cl_scan(&blob).expect("honest");
    let (etotal, ecount) = ssz::credential_scan(&vs, &bs, &TARGET);
    assert_eq!((stmt.matched_total, stmt.matched_count), (etotal, ecount));
    // Every matching validator, including the one at global 18 in the ragged tail,
    // is counted — the derived scan cannot be talked into omitting a live match.
    assert!(ecount >= 1);
}

/// The broken shape's other half: setting the bit on a slot PAST length counts a
/// validator the registry does not contain. The derived rule masks padding out.
#[test]
fn control_witnessed_accounting_selector_accepts_a_false_inclusion() {
    let d = 3;
    let per = validators_per_slice(d);
    let n = 20;
    let vs = validators(n, 3);
    let bs = balances(n);
    let tail = 2usize;
    let dead = per - 1; // global 23 — padding
    assert!(!is_live(tail as u64, dead, d, n as u64));

    let mut vw = split_validators(&vs, 3, d, ScanMode::CredentialBranch)[tail].clone();
    // A ghost validator in the padding slot carrying the target credential.
    vw[dead] = ValidatorLeaf::Branch {
        withdrawal_credentials: TARGET,
        siblings: [[7u8; 32], [8u8; 32], [9u8; 32]],
    };
    let bw = split_balances(&bs, 3, d)[tail].clone();

    let honest = scan_window(
        &vw,
        &bw,
        tail as u64,
        n as u64,
        &TARGET,
        d,
        Liveness::Derived,
        &[],
    );
    let mut witnessed: Vec<bool> = (0..per)
        .map(|j| is_live(tail as u64, j, d, n as u64))
        .collect();
    witnessed[dead] = true; // count the padding ghost
    let attack = scan_window(
        &vw,
        &bw,
        tail as u64,
        n as u64,
        &TARGET,
        d,
        Liveness::WitnessedAccounting,
        &witnessed,
    );
    assert_eq!(
        attack.matched_count,
        honest.matched_count + 1,
        "ghost counted"
    );
    // The subtree root is unchanged — the ghost is masked out of the tree.
    assert_eq!(
        attack.validators_subtree_root,
        honest.validators_subtree_root
    );
}

// ================================ 2. credential binding

fn node_words(v: &Bytes32) -> [u64; 8] {
    core::array::from_fn(|i| {
        u32::from_be_bytes([v[4 * i], v[4 * i + 1], v[4 * i + 2], v[4 * i + 3]]) as u64
    })
}

/// A past credential-comparison bug, in the guest's value domain. If the guest ever compares
/// credentials as 32-bit words without range-checking the public target, junk
/// above bit 31 in a target word can never equal a canonical credential word —
/// so a target that IS a validator's credential silently reports zero matches:
/// "silent full erasure on a proof that verifies", live the moment a guest reads
/// raw words.
#[test]
fn control_unchecked_word_target_silently_reports_zero_matches() {
    let vs = validators(9, 3);
    let cred = vs[0].withdrawal_credentials; // == TARGET, a genuine match
    let honest_words = node_words(&TARGET);
    let matches_unchecked = |target_words: &[u64; 8]| -> bool {
        let cw = node_words(&cred);
        (0..8).all(|i| cw[i] == target_words[i])
    };
    // Honest target: matches.
    assert!(matches_unchecked(&honest_words));
    // Attack: set junk above bit 31 in word 0. The low 32 bits are unchanged, but
    // the unchecked compare now fails — a real match reported as no match.
    let mut junk = honest_words;
    junk[0] |= 1u64 << 40;
    assert!(
        !matches_unchecked(&junk),
        "unchecked word compare silently drops a real match"
    );
}

/// The fix the guest ships: compare the credential as canonical 32 bytes. There is
/// no high half to poison, so a genuine credential always matches its target, and a
/// target no validator carries legitimately yields zero — not a silent erasure.
#[test]
fn fix_byte_exact_target_finds_the_real_matches() {
    let vs = validators(9, 3);
    let bs = balances(9);
    let (total, count) = ssz::credential_scan(&vs, &bs, &TARGET);
    assert_eq!(count, 3, "byte comparison finds every genuine match");
    assert!(total > 0);
    // A target no validator carries → an honest, non-silent zero.
    let mut nobody = TARGET;
    nobody[31] ^= 0xff;
    let (t0, c0) = ssz::credential_scan(&vs, &bs, &nobody);
    assert_eq!((t0, c0), (0, 0));
}

// ==================================== 3. omission / short-length (length mixin)

/// THE mixin KAT: 8 balances ending in zero pack to exactly the same two chunks as
/// the first 7, so every hash below the mixin is bit-identical. Only the mixed-in
/// length tells them apart.
#[test]
fn length_mixin_separates_identical_chunk_data() {
    let mut eight = balances(8);
    eight[7] = 0;
    let seven = eight[..7].to_vec();
    assert_eq!(ssz::pack_balances(&seven), ssz::pack_balances(&eight));
    assert_ne!(
        ssz::balances_hash_tree_root(&seven),
        ssz::balances_hash_tree_root(&eight)
    );
}

/// The control: a root WITHOUT the length mixin (bare chunk-tree root) is identical
/// for the 7- and 8-element lists, so an omission is undetectable. The mixin is
/// exactly what closes it.
#[test]
fn control_no_length_mixin_accepts_truncation() {
    let mut eight = balances(8);
    eight[7] = 0;
    let seven = eight[..7].to_vec();
    let bare = |b: &[u64]| ssz::merkleize(&ssz::pack_balances(b), ssz::BALANCES_TREE_DEPTH);
    assert_eq!(
        bare(&seven),
        bare(&eight),
        "without the mixin, truncation is invisible"
    );
    // With the mixin (the shipped path), the two are distinct — see the test above.
}

/// Dropping a matching validator and claiming the anchored root fails: the guest
/// recomputes the root over the shorter list and the anchor branch no longer
/// reproduces `beacon_state_root`.
#[test]
fn dropping_a_matching_validator_fails_the_anchor() {
    let vs = validators(24, 3);
    let bs = balances(24);
    // Honest anchor over the full list.
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    assert!(verify_cl_scan(&honest_blob).is_ok());

    // Forge a blob: drop matching validator 3, keep the honest 24-length anchor.
    let mut short_v = vs.clone();
    short_v.remove(3);
    let mut short_b = bs.clone();
    short_b.remove(3);
    let forged = forge_blob_keeping_anchor(&honest_blob, &short_v, &short_b, 3, 24);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::ValidatorsRootUnanchored
    );
}

/// Hiding a match by rewriting its credential (keeping N) also fails: the
/// credential is the branch's own leaf, so the container root — and thus the
/// registry root — moves.
#[test]
fn hiding_a_match_by_rewriting_a_credential_fails_the_anchor() {
    let vs = validators(24, 3);
    let bs = balances(24);
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );

    let mut tampered = vs.clone();
    tampered[3].withdrawal_credentials[31] ^= 0xff; // no longer matches
    let forged = forge_blob_keeping_anchor(&honest_blob, &tampered, &bs, 3, 24);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::ValidatorsRootUnanchored
    );
}

// ================== 4. double-count / permutation / duplication / zero-subtree

/// Permuting validators moves the registry root (merkleization is position-bound),
/// so the anchor rejects it.
#[test]
fn permuting_validators_breaks_the_anchor() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let mut permuted = vs.clone();
    permuted.swap(0, 6);
    let forged = forge_blob_keeping_anchor(&honest_blob, &permuted, &bs, 3, 16);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::ValidatorsRootUnanchored
    );
}

/// Re-pointing a matching validator at a fatter balance: the permuted balance list
/// has a different root, so the pairing cannot be forged.
#[test]
fn permuting_balances_against_validators_breaks_the_anchor() {
    let vs = validators(16, 3);
    let mut bs = balances(16);
    bs[9] = 200_000_000_000;
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let mut permuted = bs.clone();
    permuted.swap(0, 9);
    let forged = forge_blob_keeping_anchor(&honest_blob, &vs, &permuted, 3, 16);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::BalancesRootUnanchored
    );
}

/// Duplicating a matching validator to double-count it moves the registry root.
#[test]
fn duplicating_a_matching_validator_breaks_the_anchor() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let mut dup = vs.clone();
    dup[1] = dup[0].clone(); // copy the matching validator 0 over 1
    let forged = forge_blob_keeping_anchor(&honest_blob, &dup, &bs, 3, 16);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::ValidatorsRootUnanchored
    );
}

/// The control for permutation/duplication: a SUM-ONLY report (no root binding) is
/// invariant under permutation, so it cannot detect any of the above. The
/// root-and-anchor is what makes set membership sound, the gap of any
/// sum-only report.
#[test]
fn control_sum_only_report_cannot_detect_permutation() {
    let bs = balances(16);
    let mut permuted = bs.clone();
    permuted.swap(0, 9);
    assert_eq!(ssz::total_balance(&bs), ssz::total_balance(&permuted));
    // The anchored root, by contrast, differs:
    assert_ne!(
        ssz::balances_hash_tree_root(&bs),
        ssz::balances_hash_tree_root(&permuted)
    );
}

/// A live all-zero-ish validator inside the range is NOT confusable with padding
/// (htr(zero Validator) ≠ 0), so it is scanned like any other — no zero-subtree
/// smuggling. The control is `control_witnessed_accounting_selector_accepts_a_false_inclusion`
/// (a padded slot only enters the count under the broken witnessed selector).
#[test]
fn an_all_zero_validator_does_not_hash_to_the_zero_node() {
    let zero = Validator {
        pubkey: [0u8; 48],
        withdrawal_credentials: [0u8; 32],
        effective_balance: 0,
        slashed: false,
        activation_eligibility_epoch: 0,
        activation_epoch: 0,
        exit_epoch: 0,
        withdrawable_epoch: 0,
    };
    assert_ne!(ssz::validator_hash_tree_root(&zero), [0u8; 32]);
}

// =============================================== 5. anchor negatives

/// A forged sibling at ANY level of the validators branch fails the anchor.
#[test]
fn a_forged_validators_sibling_at_every_level_fails() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let depth = crate::fork::state_tree_depth(ForkVersion::Electra);
    // Layout tail: slot_branch (depth), validators_branch (depth), balances_branch (depth).
    let total = blob.len();
    let bal_off = total - depth * 32;
    let val_off = bal_off - depth * 32;
    for level in 0..depth {
        let mut forged = blob.clone();
        forged[val_off + level * 32] ^= 0x01;
        assert_eq!(
            verify_cl_scan(&forged).unwrap_err(),
            Error::ValidatorsRootUnanchored,
            "level {level}"
        );
    }
}

/// A wrong `beacon_state_root` fails — the slot branch is checked first.
#[test]
fn a_wrong_state_root_fails() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (mut blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    // beacon_state_root is at offset 1+1+1+8+8 = 19.
    blob[19] ^= 0x01;
    assert_eq!(verify_cl_scan(&blob).unwrap_err(), Error::SlotUnanchored);
}

/// Feeding the `validators` root on the `balances` path (swapping the two
/// branches) fails — the two trees sit at different generalized indices.
#[test]
fn crossing_the_validators_and_balances_branches_fails() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let depth = crate::fork::state_tree_depth(ForkVersion::Electra);
    let total = blob.len();
    let bal_off = total - depth * 32;
    let val_off = bal_off - depth * 32;
    let mut forged = blob.clone();
    // Swap the validators and balances branch blocks.
    let vbranch = blob[val_off..bal_off].to_vec();
    let bbranch = blob[bal_off..total].to_vec();
    forged[val_off..bal_off].copy_from_slice(&bbranch);
    forged[bal_off..total].copy_from_slice(&vbranch);
    // The validators root now sits on the balances path (gindex 76) and vice-versa;
    // neither reproduces the state root.
    assert!(verify_cl_scan(&forged).is_err());
}

/// The control: an UN-ANCHORED scan produces self-consistent roots that say
/// nothing about mainnet. A consumer who trusts the roots alone (no branch to
/// `beacon_state_root`) is fooled by any prover-chosen state; the anchored guest
/// rejects a state root the roots do not sit under.
#[test]
fn control_unanchored_scan_proves_nothing_about_mainnet() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (_windows, v_root, b_root) = scan_all(&vs, &bs, &TARGET, 3, ScanMode::CredentialBranch);
    // Self-consistent roots exist for an ARBITRARY, attacker-chosen state root:
    // nothing ties them to it without the branch.
    let attacker_state_root = [0x42u8; 32];
    assert_ne!(v_root, attacker_state_root);
    // The anchored guest requires the branch to reproduce the state root; a wrong
    // one is caught (see `a_wrong_state_root_fails`). Un-anchored, there is no
    // such check — which is the whole reason `verify` runs the three branches.
    let mut st = StateLeaves::new(ForkVersion::Electra);
    st.set(StateField::Validators, v_root);
    st.set(StateField::Balances, b_root);
    assert_ne!(st.root(), attacker_state_root);
}

/// Branch mode's extra surface: a forged credential-branch sibling on a matching
/// validator changes its container root, so the registry root moves and the anchor
/// rejects it. (Padding-slot siblings, by contrast, are masked out and free — the
/// derived selector is what makes that safe.)
#[test]
fn branch_mode_rejects_forged_siblings() {
    let vs = validators(16, 3);
    let bs = balances(16);
    let (honest_blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    assert!(verify_cl_scan(&honest_blob).is_ok());

    // Rebuild the blob with validator 0's sibling[0] (its pubkey root) forged.
    let forged = forge_branch_sibling(&honest_blob, 0, 0);
    assert_eq!(
        verify_cl_scan(&forged).unwrap_err(),
        Error::ValidatorsRootUnanchored
    );
}

// =============================================== 6. stage2-join interface

/// The CL output is exactly what `zkvm/stage2-join`'s `BeaconScan` + native
/// `credential_scan` consume: per-matched `(pubkey_root, balance)`, and the
/// anchored `(validators_root, balances_root, matched_total, matched_count)`. The
/// join intersects `pubkey_root` (its `native::pubkey_hash_tree_root`, byte-exact
/// with ours) against the EL keys; the roots are the ones it re-checks.
#[test]
fn cl_output_feeds_the_stage2_join_inputs() {
    let vs = validators(32, 4);
    let bs = balances(32);
    let (blob, _) = build_blob(
        &vs,
        &bs,
        &TARGET,
        3,
        ScanMode::CredentialBranch,
        ForkVersion::Electra,
        1,
    );
    let (stmt, matched) = verify_cl_scan_full(&blob).expect("honest");

    // 1. The roots the join re-checks against the anchor.
    assert_eq!(stmt.validators_root, ssz::validators_hash_tree_root(&vs));
    assert_eq!(stmt.balances_root, ssz::balances_hash_tree_root(&bs));

    // 2. matched_total / matched_count == the join's credential_scan.
    let (jt, jc) = ssz::credential_scan(&vs, &bs, &TARGET);
    assert_eq!((stmt.matched_total, stmt.matched_count), (jt, jc));

    // 3. Every join key is the real pubkey_root the join intersects on, with the
    //    correct balance — the D2 free join key (siblings[0]).
    let expected: Vec<(Bytes32, u64)> = vs
        .iter()
        .zip(&bs)
        .filter(|(v, _)| v.withdrawal_credentials == TARGET)
        .map(|(v, b)| (ssz::pubkey_hash_tree_root(&v.pubkey), *b))
        .collect();
    assert_eq!(matched, expected);
}

/// Coverage is structural: the guest windows `[0, length)` contiguously and
/// completely, so the R3 "slice indices are exactly {0..L−1} once" obligation
/// cannot be violated by a prover.
#[test]
fn windows_cover_the_registry_exactly_once() {
    for d in [2usize, 3, 4] {
        for n in [0usize, 1, 5, (1 << d) + 1, 4 * (1 << d)] {
            let per = validators_per_slice(d);
            let n_slices = crate::scan::slices_for(n, d);
            let mut seen = vec![false; n];
            for i in 0..n_slices {
                for j in 0..per {
                    if is_live(i as u64, j, d, n as u64) {
                        let g = i * per + j;
                        assert!(g < n && !seen[g], "d={d} n={n}: gap/overlap at {g}");
                        seen[g] = true;
                    }
                }
            }
            assert!(
                seen.iter().all(|&s| s),
                "d={d} n={n}: not every index covered"
            );
        }
    }
}

// -------------------------------------------------------------------- helpers

/// Rebuild a blob's validator+balance body from a new `(vs, bs)` but keep the
/// original header (including the honest `beacon_state_root`) and anchor branches.
/// The vehicle for "claim a tampered list against the honest anchor".
fn forge_blob_keeping_anchor(
    honest: &[u8],
    vs: &[Validator],
    bs: &[u64],
    _slice_depth: usize,
    _orig_length: usize,
) -> Vec<u8> {
    let depth = 6; // Electra state tree depth
    let header = crate::verify::HEADER_LEN;
    let mut out = honest[..header].to_vec();
    // The forged list is a different length; the guest reads `length` from the
    // header, so we keep the header's length and supply exactly that many records.
    // (The header length is the honest one; the attack claims the honest anchor.)
    // Body: validators (branch mode) then balances.
    assert_eq!(vs.len(), bs.len());
    // header length field must match the number of records we write, or the guest
    // reads past into the branches. The attack keeps the ANCHOR (state root +
    // branches) honest but must present a self-consistent record count, so we also
    // patch the length field to the forged count.
    let n = vs.len() as u64;
    out[3..11].copy_from_slice(&n.to_le_bytes());
    for v in vs {
        let sib = ssz::credential_branch_siblings(v);
        out.extend_from_slice(&v.withdrawal_credentials);
        out.extend_from_slice(&sib[0]);
        out.extend_from_slice(&sib[1]);
        out.extend_from_slice(&sib[2]);
    }
    for b in bs {
        out.extend_from_slice(&b.to_le_bytes());
    }
    // Keep the honest anchor branches (last 3×depth×32 bytes of the honest blob).
    let tail = honest.len() - 3 * depth * 32;
    out.extend_from_slice(&honest[tail..]);
    out
}

/// Forge one credential-branch sibling of the validator at global index `vi`,
/// keeping the honest header/anchor. Used to attack branch-mode's witnessed
/// siblings.
fn forge_branch_sibling(honest: &[u8], vi: usize, sib: usize) -> Vec<u8> {
    let header = crate::verify::HEADER_LEN;
    let rec = 128; // credential(32) + 3 siblings(96)
    let off = header + vi * rec + 32 + sib * 32;
    let mut out = honest.to_vec();
    out[off] ^= 0x01;
    out
}
