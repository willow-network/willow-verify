//! The guest entry: `verify_cl_scan(blob) -> Result<Statement, Error>`.
//!
//! Reads one witness blob, runs the windowed scan ([`crate::scan`]) over `[0,
//! length)`, lifts the window roots to the two SSZ list roots, and **anchors**
//! them: an SSZ Merkle branch from each list root (and the slot leaf) up to a
//! public `beacon_state_root` at the fork's generalized index. Everything the
//! statement publishes is DERIVED from the witnessed data — `matched_total`,
//! `matched_count`, `validators_root`, the join keys — never read from the blob
//! and asserted, so there is no BA5-class tautology and no `assert_non_zero`. A
//! forgery is a real `Err` from this exact code, which is what the zkVM runs.
//!
//! Coverage (the R3 "slice indices are exactly `{0..L−1}` once" obligation) is
//! structural here: the guest itself windows `[0, length)` contiguously and
//! completely, so a prover cannot present a gapped or overlapping cover.

use sha2::{Digest, Sha256};

use crate::fork::{require_state_gindex, state_tree_depth, verify_branch, ForkVersion, StateField};
use crate::scan::{
    scan_window, slices_for, split_balances, split_validators, top_list_roots,
    validators_per_slice, Liveness, ScanMode, ValidatorLeaf, MIN_SLICE_DEPTH,
};
use crate::ssz::{uint64_leaf, Bytes32, Validator};

/// The CL half's public statement — the inputs the stage-2 join consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    /// Public. The light-client-verified anchor.
    pub beacon_state_root: Bytes32,
    /// Public. Proven at `gi(slot)` under `beacon_state_root`, so a consumer can
    /// match `(beacon_state_root, slot)` against `BeaconAnchor` on both halves.
    pub slot: u64,
    /// Public. The `withdrawal_credentials` scanned for.
    pub target_credential: Bytes32,
    /// The registry length, mixed into both roots and bound by the anchor.
    pub length: u64,
    /// `hash_tree_root(state.validators)`, proven at gindex 75 under the anchor.
    pub validators_root: Bytes32,
    /// `hash_tree_root(state.balances)`, proven at gindex 76 under the anchor.
    pub balances_root: Bytes32,
    /// `Σ balances[i]` over matching validators — the A2 output, DERIVED.
    pub matched_total: u64,
    /// `#{i : credential == target}` — DERIVED.
    pub matched_count: u64,
    /// `Σ balances[i]` over the whole live set — A1's output, for free.
    pub balances_total: u64,
    /// Rolling `sha256` over each matched `(pubkey_root ‖ balance)` in scan order.
    /// The stage-2 join consumes the raw pairs and checks them against this.
    pub keys_commit: Bytes32,
}

/// Why the guest rejected a blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BlobTruncated,
    BadFork(u8),
    BadMode(u8),
    SliceDepthTooSmall(usize),
    /// The `validators` branch does not reproduce `beacon_state_root` at gindex 75.
    ValidatorsRootUnanchored,
    /// The `balances` branch does not reproduce `beacon_state_root` at gindex 76.
    BalancesRootUnanchored,
    /// The `slot` leaf does not sit at `gi(slot)` under `beacon_state_root`.
    SlotUnanchored,
}

fn fork_from_u8(b: u8) -> Result<ForkVersion, Error> {
    Ok(match b {
        0 => ForkVersion::Capella,
        1 => ForkVersion::Deneb,
        2 => ForkVersion::Electra,
        3 => ForkVersion::Fulu,
        other => return Err(Error::BadFork(other)),
    })
}

/// The blob header, fixed-size.
///
/// ```text
///  1  fork          (0=Capella 1=Deneb 2=Electra 3=Fulu)
///  1  mode          (0=CredentialBranch 1=FullContainer)
///  1  slice_depth   d ≥ 2
///  8  length        u64 LE  (number of live validators, == balances)
///  8  slot          u64 LE
/// 32  beacon_state_root
/// 32  target_credential
///  -- length × validator records (128 B branch / 121 B full)
///  -- length × balance           (8 B LE)
///  -- slot branch, validators branch, balances branch (each state_tree_depth × 32)
/// ```
pub const HEADER_LEN: usize = 1 + 1 + 1 + 8 + 8 + 32 + 32;

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}
impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, i: 0 }
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        if self.i + n > self.b.len() {
            return Err(Error::BlobTruncated);
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u64_le(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn b32(&mut self) -> Result<Bytes32, Error> {
        Ok(self.take(32)?.try_into().unwrap())
    }
    fn b48(&mut self) -> Result<[u8; 48], Error> {
        Ok(self.take(48)?.try_into().unwrap())
    }
}

/// [`verify_cl_scan`], also returning the matched `(pubkey_root, balance)` pairs
/// the stage-2 join intersects with the EL registered set. Mirrors the EL half's
/// `verify_enumeration_full`: the committed `keys_commit` binds them.
pub fn verify_cl_scan_full(blob: &[u8]) -> Result<(Statement, Vec<(Bytes32, u64)>), Error> {
    let mut r = Reader::new(blob);
    let fork = fork_from_u8(r.u8()?)?;
    let mode = match r.u8()? {
        0 => ScanMode::CredentialBranch,
        1 => ScanMode::FullContainer,
        other => return Err(Error::BadMode(other)),
    };
    let slice_depth = r.u8()? as usize;
    if slice_depth < MIN_SLICE_DEPTH {
        return Err(Error::SliceDepthTooSmall(slice_depth));
    }
    let length = r.u64_le()?;
    let slot = r.u64_le()?;
    let beacon_state_root = r.b32()?;
    let target_credential = r.b32()?;

    // The live validators, in global order. Padding is synthesized in-guest, so
    // the blob is O(length), not O(n_slices · 2^d).
    let n = length as usize;
    let mut validators: Vec<Validator> = Vec::with_capacity(n);
    let mut leaves_direct: Vec<ValidatorLeaf> = Vec::with_capacity(n);
    for _ in 0..n {
        match mode {
            ScanMode::FullContainer => {
                let pubkey = r.b48()?;
                let withdrawal_credentials = r.b32()?;
                let effective_balance = r.u64_le()?;
                let slashed = r.u8()? != 0;
                let activation_eligibility_epoch = r.u64_le()?;
                let activation_epoch = r.u64_le()?;
                let exit_epoch = r.u64_le()?;
                let withdrawable_epoch = r.u64_le()?;
                validators.push(Validator {
                    pubkey,
                    withdrawal_credentials,
                    effective_balance,
                    slashed,
                    activation_eligibility_epoch,
                    activation_epoch,
                    exit_epoch,
                    withdrawable_epoch,
                });
            }
            ScanMode::CredentialBranch => {
                let withdrawal_credentials = r.b32()?;
                let siblings = [r.b32()?, r.b32()?, r.b32()?];
                leaves_direct.push(ValidatorLeaf::Branch {
                    withdrawal_credentials,
                    siblings,
                });
            }
        }
    }
    let mut balances: Vec<u64> = Vec::with_capacity(n);
    for _ in 0..n {
        balances.push(r.u64_le()?);
    }

    let slice_depth_u = slice_depth;
    let n_slices = slices_for(n, slice_depth_u);
    let per = validators_per_slice(slice_depth_u);

    // Window the live set contiguously (coverage is structural).
    let v_windows: Vec<Vec<ValidatorLeaf>> = match mode {
        ScanMode::FullContainer => split_validators(&validators, n_slices, slice_depth_u, mode),
        ScanMode::CredentialBranch => (0..n_slices)
            .map(|i| {
                (0..per)
                    .map(|j| {
                        leaves_direct
                            .get(i * per + j)
                            .cloned()
                            .unwrap_or_else(ValidatorLeaf::padding)
                    })
                    .collect()
            })
            .collect(),
    };
    let b_windows = split_balances(&balances, n_slices, slice_depth_u);

    let mut windows = Vec::with_capacity(n_slices);
    let mut matched_total = 0u64;
    let mut matched_count = 0u64;
    let mut balances_total = 0u64;
    let mut matched: Vec<(Bytes32, u64)> = Vec::new();
    for i in 0..n_slices {
        let w = scan_window(
            &v_windows[i],
            &b_windows[i],
            i as u64,
            length,
            &target_credential,
            slice_depth_u,
            Liveness::Derived,
            &[],
        );
        matched_total = matched_total
            .checked_add(w.matched_total)
            .expect("matched total overflow");
        matched_count += w.matched_count;
        balances_total = balances_total
            .checked_add(w.balances_total)
            .expect("balances total overflow");
        matched.extend_from_slice(&w.matched);
        windows.push(w);
    }

    let (validators_root, balances_root) = top_list_roots(&windows, length, slice_depth_u);

    // --- Anchor: the three branches must reproduce beacon_state_root. ---
    let branch_depth = state_tree_depth(fork);
    let read_branch = |r: &mut Reader| -> Result<Vec<Bytes32>, Error> {
        let mut v = Vec::with_capacity(branch_depth);
        for _ in 0..branch_depth {
            v.push(r.b32()?);
        }
        Ok(v)
    };
    let slot_branch = read_branch(&mut r)?;
    let validators_branch = read_branch(&mut r)?;
    let balances_branch = read_branch(&mut r)?;

    let gi_slot = require_state_gindex(fork, StateField::Slot);
    let gi_val = require_state_gindex(fork, StateField::Validators);
    let gi_bal = require_state_gindex(fork, StateField::Balances);

    if !verify_branch(
        &uint64_leaf(slot),
        &slot_branch,
        gi_slot,
        &beacon_state_root,
    ) {
        return Err(Error::SlotUnanchored);
    }
    if !verify_branch(
        &validators_root,
        &validators_branch,
        gi_val,
        &beacon_state_root,
    ) {
        return Err(Error::ValidatorsRootUnanchored);
    }
    if !verify_branch(&balances_root, &balances_branch, gi_bal, &beacon_state_root) {
        return Err(Error::BalancesRootUnanchored);
    }

    // Rolling commitment over the join keys (derived, never an input).
    let mut keys_commit = [0u8; 32];
    for (pk_root, bal) in &matched {
        let mut buf = [0u8; 64];
        buf[..32].copy_from_slice(&keys_commit);
        buf[32..64].copy_from_slice(pk_root);
        // fold the balance into the tail 8 bytes of the second node
        buf[56..64].copy_from_slice(&bal.to_le_bytes());
        let mut h = Sha256::new();
        h.update(buf);
        keys_commit = h.finalize().into();
    }

    Ok((
        Statement {
            beacon_state_root,
            slot,
            target_credential,
            length,
            validators_root,
            balances_root,
            matched_total,
            matched_count,
            balances_total,
            keys_commit,
        },
        matched,
    ))
}

/// The guest's public statement, dropping the raw join keys.
pub fn verify_cl_scan(blob: &[u8]) -> Result<Statement, Error> {
    verify_cl_scan_full(blob).map(|(s, _keys)| s)
}

// ---------------------------------------------------------------------------
// Blob construction — used by tests and the fixture generator (host side).
// ---------------------------------------------------------------------------

/// Build a witness blob and the self-consistent synthetic anchor around a
/// `(validators, balances)` slice.
///
/// The scan and its roots are REAL over the supplied data; the anchor
/// `beacon_state_root` is *computed* from those roots via [`crate::fork::StateLeaves`]
/// (a real ~500 MB co-slot state is not available offline and no beacon API serves
/// one field's root plus branch, a known limitation of testing offline
/// with no full state). The anchor negatives in [`crate::tests`] prove the
/// branch verification is sound.
pub fn build_blob(
    validators: &[Validator],
    balances: &[u64],
    target: &Bytes32,
    slice_depth: usize,
    mode: ScanMode,
    fork: ForkVersion,
    slot: u64,
) -> (Vec<u8>, Bytes32) {
    use crate::fork::StateLeaves;
    use crate::scan::scan_all;

    let (_windows, v_root, b_root) = scan_all(validators, balances, target, slice_depth, mode);

    let mut state = StateLeaves::new(fork);
    state.set(StateField::Slot, uint64_leaf(slot));
    state.set(StateField::Validators, v_root);
    state.set(StateField::Balances, b_root);
    let beacon_state_root = state.root();

    let mode_byte = match mode {
        ScanMode::CredentialBranch => 0u8,
        ScanMode::FullContainer => 1u8,
    };
    let fork_byte = match fork {
        ForkVersion::Capella => 0u8,
        ForkVersion::Deneb => 1u8,
        ForkVersion::Electra => 2u8,
        ForkVersion::Fulu => 3u8,
    };

    let mut blob = Vec::new();
    blob.push(fork_byte);
    blob.push(mode_byte);
    blob.push(slice_depth as u8);
    blob.extend_from_slice(&(validators.len() as u64).to_le_bytes());
    blob.extend_from_slice(&slot.to_le_bytes());
    blob.extend_from_slice(&beacon_state_root);
    blob.extend_from_slice(target);

    for v in validators {
        match mode {
            ScanMode::FullContainer => {
                blob.extend_from_slice(&v.pubkey);
                blob.extend_from_slice(&v.withdrawal_credentials);
                blob.extend_from_slice(&v.effective_balance.to_le_bytes());
                blob.push(v.slashed as u8);
                blob.extend_from_slice(&v.activation_eligibility_epoch.to_le_bytes());
                blob.extend_from_slice(&v.activation_epoch.to_le_bytes());
                blob.extend_from_slice(&v.exit_epoch.to_le_bytes());
                blob.extend_from_slice(&v.withdrawable_epoch.to_le_bytes());
            }
            ScanMode::CredentialBranch => {
                let sib = crate::ssz::credential_branch_siblings(v);
                blob.extend_from_slice(&v.withdrawal_credentials);
                blob.extend_from_slice(&sib[0]);
                blob.extend_from_slice(&sib[1]);
                blob.extend_from_slice(&sib[2]);
            }
        }
    }
    for b in balances {
        blob.extend_from_slice(&b.to_le_bytes());
    }
    for field in [
        StateField::Slot,
        StateField::Validators,
        StateField::Balances,
    ] {
        for sib in state.branch(field) {
            blob.extend_from_slice(&sib);
        }
    }

    (blob, beacon_state_root)
}
