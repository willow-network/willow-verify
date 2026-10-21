//! The windowed credential-membership + balance scan, with every adversarial
//! soundness property the retired Binius64 `membership.rs`/`membership_split.rs`
//! carried re-derived in Rust.
//!
//! # The cut, and why it is windowed
//!
//! A mainnet scan is 2.3M validators; a guest windows it for RAM (the A2 witness
//! is ~410 MB held all at once). Fix a `slice_depth` `d ≥ 2`. A window owns
//! `V = 2^d` consecutive validators, an aligned subtree of *both* lists:
//! `V` leaves of the depth-40 `validators` tree and `V/4` chunks of the depth-38
//! `balances` tree, both sitting `40 − d` / `38 − (d−2)` levels below their roots.
//! One `slice_depth` parameterises the whole cut.
//!
//! # ★ The ragged tail is a soundness surface (the derived liveness selector)
//!
//! `balances` pads with zero *bytes*, and a zero balance packs to zero, so a dead
//! slot merkleizes to the zero subtree for free. **`validators` does not.** SSZ
//! pads the registry with zero *nodes*, but `hash_tree_root` of an all-zero
//! `Validator` is not the zero node — its `pubkey` field alone is `zero_hashes[1]`
//! (proved in [`crate::tests`]). So every slot carries a liveness selector that
//! emits the zero node when `i ≥ length`, and the central rule is:
//!
//! > The selector is **derived** from the public `slice_index` and the public
//! > `length` — the same `length` mixed into both anchored roots. It is never
//! > witnessed, and it masks the accounting with the *same* value that masks the
//! > Merkle leaf.
//!
//! A prover-chosen selector is a false-exclusion lever: clearing the accounting
//! bit on a live matching validator drops it from `matched_total`/`matched_count`
//! while both subtree roots stay bit-identical — a complete, anchored, verifying
//! report with a validator omitted, in the product whose entire claim is that
//! omission is impossible. [`Liveness::WitnessedAccounting`] is that broken
//! variant, kept so [`crate::tests`] can execute the attack against it and show
//! the shipped [`Liveness::Derived`] rule refusing it.

use crate::ssz::{
    self, apply_credential_branch, merkleize, pack_balances, pubkey_hash_tree_root,
    validator_hash_tree_root, Bytes32, Validator, BALANCES_TREE_DEPTH, VALIDATORS_TREE_DEPTH,
};

/// Balances packed per 32-byte SSZ chunk.
pub const BALANCES_PER_CHUNK: usize = 4;

/// The shallowest slice that owns whole chunks of *both* lists.
pub const MIN_SLICE_DEPTH: usize = 2;

/// Validators in one slice.
pub const fn validators_per_slice(slice_depth: usize) -> usize {
    1 << slice_depth
}

/// Balance chunks in one slice.
pub const fn balance_chunks_per_slice(slice_depth: usize) -> usize {
    1 << (slice_depth - MIN_SLICE_DEPTH)
}

/// Height of a slice's subtree in the `balances` chunk tree.
pub const fn balances_slice_depth(slice_depth: usize) -> usize {
    slice_depth - MIN_SLICE_DEPTH
}

/// Levels the top hashes above the `validators` slice roots.
pub const fn top_levels(slice_depth: usize) -> usize {
    VALIDATORS_TREE_DEPTH - slice_depth
}

/// The same count reached from the `balances` list — asserted equal, not assumed.
pub const fn balances_top_levels(slice_depth: usize) -> usize {
    BALANCES_TREE_DEPTH - balances_slice_depth(slice_depth)
}

/// How many uniform slices cover a registry of `n` validators (≥ 1).
pub const fn slices_for(n: usize, slice_depth: usize) -> usize {
    let per = validators_per_slice(slice_depth);
    if n == 0 {
        1
    } else {
        n.div_ceil(per)
    }
}

/// Whether global index `i = (slice_index << d) | slot` is inside a registry of
/// `length` validators — the one-line predicate the whole ragged-tail argument
/// reduces to, a function of public values only.
pub const fn is_live(slice_index: u64, slot: usize, slice_depth: usize, length: u64) -> bool {
    ((slice_index << slice_depth) | slot as u64) < length
}

/// How the scan reaches each validator's `withdrawal_credentials`, and how it
/// forms the Merkle leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanMode {
    /// Recompute each container root from all eight field leaves (8 hashes); every
    /// field is bound.
    FullContainer,
    /// Witness the credential and the three siblings on the path to the container
    /// root, and verify only that branch (3 hashes, 2.2× cheaper). `siblings[0]`
    /// is `htr(pubkey)`, the D2 join key, exposed for free.
    CredentialBranch,
}

/// How liveness is decided for the accounting mask. The whole soundness
/// argument is that the shipped rule is [`Liveness::Derived`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Liveness {
    /// **Shipped.** `live = (index < length)`, derived from two public values, used
    /// to mask the Merkle leaf, the balance and the accounting with one value.
    Derived,
    /// **Broken on purpose — tests only.** Both *tree* inputs are still masked by
    /// the derived rule (so both anchored roots come out right), but the accounting
    /// mask is a caller-supplied bit. Clearing it on a live matching validator is a
    /// silent, anchored false exclusion.
    WitnessedAccounting,
}

/// One validator's witness, in whichever [`ScanMode`] shape.
#[derive(Clone, Debug)]
pub enum ValidatorLeaf {
    /// All eight fields.
    Full(Validator),
    /// The credential and its three branch siblings
    /// (`[htr(pubkey), hash(eff_bal,slashed), hash(epochs)]`).
    Branch {
        withdrawal_credentials: Bytes32,
        siblings: [Bytes32; 3],
    },
}

impl ValidatorLeaf {
    /// The credential the scan matches against the target.
    pub fn withdrawal_credentials(&self) -> Bytes32 {
        match self {
            ValidatorLeaf::Full(v) => v.withdrawal_credentials,
            ValidatorLeaf::Branch {
                withdrawal_credentials,
                ..
            } => *withdrawal_credentials,
        }
    }

    /// `hash_tree_root(Validator)` — recomputed from eight fields, or rebuilt from
    /// the credential and its three siblings. In branch mode the anchored
    /// `validators_root` pins the siblings, so a forged one cannot reach the anchor
    /// (proved in [`crate::tests::branch_mode_rejects_forged_siblings`]).
    pub fn container_root(&self) -> Bytes32 {
        match self {
            ValidatorLeaf::Full(v) => validator_hash_tree_root(v),
            ValidatorLeaf::Branch {
                withdrawal_credentials,
                siblings,
            } => apply_credential_branch(withdrawal_credentials, siblings),
        }
    }

    /// The D2 join key: `htr(pubkey)`. Free in branch mode (`siblings[0]`).
    pub fn pubkey_root(&self) -> Bytes32 {
        match self {
            ValidatorLeaf::Full(v) => pubkey_hash_tree_root(&v.pubkey),
            ValidatorLeaf::Branch { siblings, .. } => siblings[0],
        }
    }

    /// A padded slot: contents are arbitrary and unconstrained — a padded slot's
    /// Merkle leaf is the zero node whatever this holds, and its balance and
    /// credential are masked out of every accumulator.
    pub fn padding() -> Self {
        ValidatorLeaf::Branch {
            withdrawal_credentials: [0u8; 32],
            siblings: [[0u8; 32]; 3],
        }
    }
}

/// Everything one window publishes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WindowResult {
    /// Subtree root at height `slice_depth` in the `validators` chunk tree.
    pub validators_subtree_root: Bytes32,
    /// Subtree root at height `slice_depth − 2` in the `balances` chunk tree.
    pub balances_subtree_root: Bytes32,
    /// `Σ balances[i]` over this window's live matching validators.
    pub matched_total: u64,
    /// How many of this window's live validators carry the target credential.
    pub matched_count: u64,
    /// `Σ balances[i]` over this window's live slots — A1's half, for free.
    pub balances_total: u64,
    /// `(pubkey_root, balance)` for each matched live validator, in slot order —
    /// the stage-2 join's inputs. Derived from the anchored data, never accepted
    /// as input (the BA5-class tautology guard).
    pub matched: Vec<(Bytes32, u64)>,
}

/// Scan one window of exactly `V = 2^slice_depth` slots.
///
/// `validators` and `balances` are the window's slots (live data on the low side,
/// padding above `length`). `witnessed_live` is consulted only under
/// [`Liveness::WitnessedAccounting`]; pass `&[]` for [`Liveness::Derived`].
///
/// The Merkle leaf and the balance tree input are *always* masked by the derived
/// rule — that is the only way a padded slot reaches the anchored root at all. The
/// only thing the two [`Liveness`] policies disagree about is the accounting mask.
pub fn scan_window(
    validators: &[ValidatorLeaf],
    balances: &[u64],
    slice_index: u64,
    length: u64,
    target: &Bytes32,
    slice_depth: usize,
    liveness: Liveness,
    witnessed_live: &[bool],
) -> WindowResult {
    let per = validators_per_slice(slice_depth);
    assert_eq!(
        validators.len(),
        per,
        "a window is exactly {per} validators"
    );
    assert_eq!(balances.len(), per, "a window is exactly {per} balances");

    let mut leaves = Vec::with_capacity(per);
    let mut live_balances = Vec::with_capacity(per);
    let mut matched_total = 0u64;
    let mut matched_count = 0u64;
    let mut balances_total = 0u64;
    let mut matched = Vec::new();

    for (j, v) in validators.iter().enumerate() {
        let live = is_live(slice_index, j, slice_depth, length);

        // Tree inputs: masked by the DERIVED rule in both policies.
        leaves.push(if live { v.container_root() } else { [0u8; 32] });
        let bal = if live { balances[j] } else { 0 };
        live_balances.push(bal);
        balances_total = balances_total
            .checked_add(bal)
            .expect("window balance total overflows u64");

        // Accounting mask: the one place the two policies differ.
        let accounting_live = match liveness {
            Liveness::Derived => live,
            Liveness::WitnessedAccounting => witnessed_live[j],
        };
        if accounting_live && &v.withdrawal_credentials() == target {
            matched_total = matched_total
                .checked_add(bal)
                .expect("window matched total overflows u64");
            matched_count += 1;
            matched.push((v.pubkey_root(), bal));
        }
    }

    WindowResult {
        validators_subtree_root: merkleize(&leaves, slice_depth),
        balances_subtree_root: merkleize(
            &pack_balances(&live_balances),
            balances_slice_depth(slice_depth),
        ),
        matched_total,
        matched_count,
        balances_total,
        matched,
    }
}

/// Lift `L` window results to the two anchored list roots (`validators`, `balances`),
/// each with its length mixin. This is the top of the split.
pub fn top_list_roots(
    windows: &[WindowResult],
    length: u64,
    slice_depth: usize,
) -> (Bytes32, Bytes32) {
    let v_roots: Vec<Bytes32> = windows.iter().map(|w| w.validators_subtree_root).collect();
    let b_roots: Vec<Bytes32> = windows.iter().map(|w| w.balances_subtree_root).collect();
    (
        ssz::mix_in_length(
            &ssz::merkleize_above(&v_roots, top_levels(slice_depth), slice_depth),
            length,
        ),
        ssz::mix_in_length(
            &ssz::merkleize_above(
                &b_roots,
                balances_top_levels(slice_depth),
                balances_slice_depth(slice_depth),
            ),
            length,
        ),
    )
}

/// Cut `validators` into `n_slices` windows in `mode`'s shape, padding the tail.
pub fn split_validators(
    validators: &[Validator],
    n_slices: usize,
    slice_depth: usize,
    mode: ScanMode,
) -> Vec<Vec<ValidatorLeaf>> {
    let per = validators_per_slice(slice_depth);
    (0..n_slices)
        .map(|i| {
            (0..per)
                .map(|j| match validators.get(i * per + j) {
                    Some(v) => match mode {
                        ScanMode::FullContainer => ValidatorLeaf::Full(v.clone()),
                        ScanMode::CredentialBranch => ValidatorLeaf::Branch {
                            withdrawal_credentials: v.withdrawal_credentials,
                            siblings: ssz::credential_branch_siblings(v),
                        },
                    },
                    None => ValidatorLeaf::padding(),
                })
                .collect()
        })
        .collect()
}

/// Cut `balances` into `n_slices` windows, zero-padding the tail.
pub fn split_balances(balances: &[u64], n_slices: usize, slice_depth: usize) -> Vec<Vec<u64>> {
    let per = validators_per_slice(slice_depth);
    (0..n_slices)
        .map(|i| {
            let mut slice = vec![0u64; per];
            let start = i * per;
            if start < balances.len() {
                let end = (start + per).min(balances.len());
                slice[..end - start].copy_from_slice(&balances[start..end]);
            }
            slice
        })
        .collect()
}

/// The whole scan, natively: `(validators, balances)` → windows, the two list
/// roots, and the three totals — the reference for the guest and every test.
///
/// The returned roots must equal [`ssz::validators_hash_tree_root`] /
/// [`ssz::balances_hash_tree_root`], and the totals must equal
/// [`ssz::credential_scan`] plus [`ssz::total_balance`]. That identity is the
/// split's correctness statement.
pub fn scan_all(
    validators: &[Validator],
    balances: &[u64],
    target: &Bytes32,
    slice_depth: usize,
    mode: ScanMode,
) -> (Vec<WindowResult>, Bytes32, Bytes32) {
    assert_eq!(
        validators.len(),
        balances.len(),
        "parallel-list invariant: validators and balances must be the same length"
    );
    let length = validators.len() as u64;
    let n_slices = slices_for(validators.len(), slice_depth);
    let v_slices = split_validators(validators, n_slices, slice_depth, mode);
    let b_slices = split_balances(balances, n_slices, slice_depth);
    let windows: Vec<WindowResult> = (0..n_slices)
        .map(|i| {
            scan_window(
                &v_slices[i],
                &b_slices[i],
                i as u64,
                length,
                target,
                slice_depth,
                Liveness::Derived,
                &[],
            )
        })
        .collect();
    let (v_root, b_root) = top_list_roots(&windows, length, slice_depth);
    (windows, v_root, b_root)
}
