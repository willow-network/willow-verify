//! Synthetic, shape-exact witness fixtures.
//!
//! **Not part of the guest path.** Gated behind `cfg(any(test, feature = "synth"))`
//! so the ELF this crate builds into contains none of it.
//!
//! Every tree here is the real shape at the real depth — a full 8,192-entry
//! `block_roots` vector, a `historical_summaries` `List` merkleized to depth 24
//! with the length mixed in, a `HistoricalSummary` container of two fields, an
//! `ExecutionPayload` of 15 or 17 leaves, a `BeaconBlockBody` of 11/12/13 and a
//! `BeaconBlock` of 5 — only the *contents* are fake. So the branches these
//! fixtures produce have exactly the lengths, orders and padding a real beacon
//! node produces, and an adversarial edit is testing the same arithmetic a real
//! witness would exercise.
//!
//! The state container itself is built with `beacon_cl_core::fork::StateLeaves`,
//! which derives its field count from the real `crates/network` declarations —
//! this module never declares one.

use crate::blob::Witness;
use crate::fork_schedule::{
    fork_at, payload_field_count, payload_tree_depth, ForkVersion, BLOCK_ROOTS_TREE_DEPTH,
    CAPELLA_SLOT, HISTORICAL_ROOTS_LIMIT_DEPTH, SLOTS_PER_HISTORICAL_ROOT,
};
use crate::gindex::{PathKind, BLOCK_ROOTS_POSITION, HISTORICAL_SUMMARIES_POSITION};
use beacon_cl_core::fork::{state_tree_depth, StateLeaves};
use beacon_cl_core::ssz::{hash_pair, merkleize, mix_in_length, uint64_leaf, zero_hashes, Bytes32};

/// `BeaconBlockBody` fields: Capella 11, Deneb +`blob_kzg_commitments` = 12,
/// Electra +`execution_requests` = 13, Fulu unchanged. All in `(8, 16]`, so the
/// body tree is depth 4 at every one of them — which is why the composed
/// payload->block gindex 201 is fork-independent.
pub const fn body_field_count(fork: ForkVersion) -> usize {
    match fork {
        ForkVersion::Capella => 11,
        ForkVersion::Deneb => 12,
        ForkVersion::Electra => 13,
        ForkVersion::Fulu => 13,
    }
}

/// Depth of the `BeaconBlockBody` container tree.
pub const BODY_TREE_DEPTH: usize = 4;
/// Position of `execution_payload` in `BeaconBlockBody`.
pub const BODY_EXECUTION_PAYLOAD_POSITION: usize = 9;
/// Depth of the `BeaconBlock` container tree (5 fields -> 8 slots).
pub const BLOCK_TREE_DEPTH: usize = 3;
/// Position of `body` in `BeaconBlock`.
pub const BLOCK_BODY_POSITION: usize = 4;
/// Position of `state_root` in `BeaconBlock`.
pub const BLOCK_STATE_ROOT_POSITION: usize = 3;

const _: () = assert!(
    body_field_count(ForkVersion::Electra)
        .next_power_of_two()
        .trailing_zeros() as usize
        == BODY_TREE_DEPTH,
    "BeaconBlockBody no longer merkleizes to depth 4; composed gindex 201 is wrong"
);

/// A deterministic, distinct filler node — so a tampered sibling is never
/// accidentally equal to its neighbour and a passing test is a real pass.
pub fn filler(tag: u8, i: usize) -> Bytes32 {
    let mut n = [0u8; 32];
    n[0] = tag;
    n[1..9].copy_from_slice(&(i as u64).to_le_bytes());
    n[31] = 0xa5;
    n
}

/// The Merkle branch for `index` in `merkleize(chunks, depth)`, leaf->root, with
/// the same right-padding by zero subtrees that `merkleize` uses. Returned branch
/// length is exactly `depth`.
pub fn padded_branch(chunks: &[Bytes32], depth: usize, index: usize) -> Vec<Bytes32> {
    let z = zero_hashes(depth);
    let mut layer = chunks.to_vec();
    let mut idx = index;
    let mut branch = Vec::with_capacity(depth);
    for (level, zl) in z.iter().take(depth).enumerate() {
        let sib = idx ^ 1;
        branch.push(if sib < layer.len() { layer[sib] } else { *zl });
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            let right = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                *zl
            };
            next.push(hash_pair(&layer[i], &right));
            i += 2;
        }
        if next.is_empty() {
            next.push(z[level + 1]);
        }
        layer = next;
        idx >>= 1;
    }
    branch
}

/// Knobs for one synthetic witness. `build` produces a witness that `run` accepts
/// (with the default overrides), so every adversarial case in the tests is one
/// field's edit away from a passing control.
#[derive(Clone, Debug)]
pub struct Synth {
    /// Slot of the anchor beacon block. Fixes the `BeaconState` shape.
    pub anchor_slot: u64,
    /// Slot of the target block. Fixes the `ExecutionPayload` shape.
    pub target_slot: u64,
    /// Which state shape to route through.
    pub path: PathKind,
    /// Value placed in `payload_leaves[6]` as a canonical uint64 leaf.
    pub block_number: u64,
    /// Value placed in `payload_leaves[12]`.
    pub block_hash: Bytes32,
    /// Replace `payload_leaves[6]` wholesale (for the non-canonical-leaf case).
    /// The rest of the tree is rebuilt around it, so the branches still verify and
    /// the ONLY thing under test is the high-bytes check.
    pub block_number_leaf_override: Option<Bytes32>,
}

impl Synth {
    /// A Recent-path fixture: `target_slot` sits `behind` slots before the anchor.
    pub fn recent(anchor_slot: u64, behind: u64) -> Self {
        Self {
            anchor_slot,
            target_slot: anchor_slot - behind,
            path: PathKind::Recent,
            block_number: 25_778_841,
            block_hash: filler(0x77, 1),
            block_number_leaf_override: None,
        }
    }

    /// A Historical-path fixture for an explicit target slot.
    pub fn historical(anchor_slot: u64, target_slot: u64) -> Self {
        Self {
            anchor_slot,
            target_slot,
            path: PathKind::Historical,
            block_number: 25_015_683,
            block_hash: filler(0x77, 2),
            block_number_leaf_override: None,
        }
    }

    /// The `historical_summaries` chunk index this fixture's target lives in.
    pub fn chunk(&self) -> u64 {
        (self.target_slot - CAPELLA_SLOT) / SLOTS_PER_HISTORICAL_ROOT
    }

    /// Build the witness. Panics (test-only code) if the configuration is not
    /// constructible — e.g. a Historical target whose period had not completed by
    /// the anchor slot.
    pub fn build(&self) -> Witness {
        let anchor_fork = fork_at(self.anchor_slot).expect("anchor slot is post-Capella");
        let target_fork = fork_at(self.target_slot).expect("target slot is post-Capella");

        // ---- ExecutionPayload -------------------------------------------------
        let n_leaves = payload_field_count(target_fork);
        let mut payload_leaves: Vec<Bytes32> = (0..n_leaves).map(|i| filler(0x10, i)).collect();
        payload_leaves[crate::fork_schedule::PAYLOAD_BLOCK_NUMBER_POSITION] = self
            .block_number_leaf_override
            .unwrap_or_else(|| uint64_leaf(self.block_number));
        payload_leaves[crate::fork_schedule::PAYLOAD_BLOCK_HASH_POSITION] = self.block_hash;
        let payload_root = merkleize(&payload_leaves, payload_tree_depth(target_fork));

        // ---- BeaconBlockBody --------------------------------------------------
        let mut body_leaves: Vec<Bytes32> = (0..body_field_count(target_fork))
            .map(|i| filler(0x20, i))
            .collect();
        body_leaves[BODY_EXECUTION_PAYLOAD_POSITION] = payload_root;
        let body_root = merkleize(&body_leaves, BODY_TREE_DEPTH);
        let body_branch = padded_branch(
            &body_leaves,
            BODY_TREE_DEPTH,
            BODY_EXECUTION_PAYLOAD_POSITION,
        );

        // ---- BeaconBlock (the target) ----------------------------------------
        let mut block_leaves: Vec<Bytes32> = (0..5).map(|i| filler(0x30, i)).collect();
        block_leaves[0] = uint64_leaf(self.target_slot);
        block_leaves[BLOCK_BODY_POSITION] = body_root;
        let target_block_root = merkleize(&block_leaves, BLOCK_TREE_DEPTH);
        let block_branch = padded_branch(&block_leaves, BLOCK_TREE_DEPTH, BLOCK_BODY_POSITION);

        let mut branch_payload = [[0u8; 32]; 7];
        for (slot, node) in branch_payload
            .iter_mut()
            .zip(body_branch.iter().chain(block_branch.iter()))
        {
            *slot = *node;
        }

        // ---- block_roots: a real 8,192-entry Vector --------------------------
        let idx = (self.target_slot % SLOTS_PER_HISTORICAL_ROOT) as usize;
        let mut block_roots: Vec<Bytes32> = (0..SLOTS_PER_HISTORICAL_ROOT as usize)
            .map(|i| filler(0x40, i))
            .collect();
        block_roots[idx] = target_block_root;
        let block_roots_root = merkleize(&block_roots, BLOCK_ROOTS_TREE_DEPTH);
        let block_roots_branch = padded_branch(&block_roots, BLOCK_ROOTS_TREE_DEPTH, idx);

        // ---- historical_summaries: List[HistoricalSummary, 2^24] -------------
        // One entry per completed 8,192-slot period since Capella. Its
        // `block_summary_root` is this fixture's `block_roots` root for the target's
        // own period, filler elsewhere.
        let n_summaries =
            ((self.anchor_slot - CAPELLA_SLOT) / SLOTS_PER_HISTORICAL_ROOT).max(1) as usize;
        let chunk = self.chunk() as usize;
        let summary_state_root = filler(0x60, chunk);
        let summary_chunks: Vec<Bytes32> = (0..n_summaries)
            .map(|i| {
                let bsr = if i == chunk {
                    block_roots_root
                } else {
                    filler(0x50, i)
                };
                // HistoricalSummary is 2 fields, so its root is one hash.
                hash_pair(&bsr, &filler(0x60, i))
            })
            .collect();
        let summaries_data_root = merkleize(&summary_chunks, HISTORICAL_ROOTS_LIMIT_DEPTH);
        let summaries_root = mix_in_length(&summaries_data_root, n_summaries as u64);
        let summaries_data_branch =
            padded_branch(&summary_chunks, HISTORICAL_ROOTS_LIMIT_DEPTH, chunk);
        let length_leaf = uint64_leaf(n_summaries as u64);

        // ---- BeaconState -----------------------------------------------------
        let mut state = StateLeaves::new(anchor_fork);
        state.set_position(2, uint64_leaf(self.anchor_slot));
        state.set_position(BLOCK_ROOTS_POSITION, block_roots_root);
        state.set_position(HISTORICAL_SUMMARIES_POSITION, summaries_root);
        let anchor_state_root = state.root();
        let d = state_tree_depth(anchor_fork);
        let state_field_position = match self.path {
            PathKind::Recent => BLOCK_ROOTS_POSITION,
            PathKind::Historical => HISTORICAL_SUMMARIES_POSITION,
        };
        let state_branch = state.branch_at_gindex((1u64 << d) + state_field_position as u64);

        // ---- the composed sibling list, leaf->root ---------------------------
        let mut siblings: Vec<Bytes32> = Vec::new();
        siblings.extend_from_slice(&block_roots_branch);
        if self.path == PathKind::Historical {
            assert!(
                chunk < n_summaries,
                "target period {chunk} has not completed by anchor_slot {} \
                 ({n_summaries} summaries) — not a constructible Historical fixture",
                self.anchor_slot
            );
            // HistoricalSummary container: `block_summary_root` is field 0, so the
            // sibling is `state_summary_root`.
            siblings.push(summary_state_root);
            // the 2^24 List data tree
            siblings.extend_from_slice(&summaries_data_branch);
            // mix_in_length: the data root is the LEFT child, length leaf the right
            siblings.push(length_leaf);
        }
        siblings.extend_from_slice(&state_branch);

        // ---- the anchor BeaconBlock, and R -----------------------------------
        let mut anchor_block_leaves: Vec<Bytes32> = (0..5).map(|i| filler(0x70, i)).collect();
        anchor_block_leaves[0] = uint64_leaf(self.anchor_slot);
        anchor_block_leaves[BLOCK_STATE_ROOT_POSITION] = anchor_state_root;
        let anchor_state_branch = padded_branch(
            &anchor_block_leaves,
            BLOCK_TREE_DEPTH,
            BLOCK_STATE_ROOT_POSITION,
        );
        let mut branch_state_root = [[0u8; 32]; 3];
        branch_state_root.copy_from_slice(&anchor_state_branch);

        Witness {
            anchor_slot: self.anchor_slot,
            anchor_state_root,
            branch_state_root,
            target_slot: self.target_slot,
            path: self.path,
            siblings,
            payload_leaves,
            branch_payload,
        }
    }

    /// `R` for this fixture — the value the statement should commit and EIP-4788
    /// should return. Recomputed independently of `run`.
    pub fn expected_r(&self) -> Bytes32 {
        let w = self.build();
        beacon_cl_core::fork::branch_root(
            &w.anchor_state_root,
            &w.branch_state_root,
            crate::gindex::BLOCK_STATE_ROOT_GINDEX,
        )
        .expect("branch_state_root is [Bytes32; 3], the depth gindex 11 implies")
    }
}
