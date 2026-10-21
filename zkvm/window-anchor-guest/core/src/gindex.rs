//! The generalized indices this guest walks, all COMPUTED in-guest.
//!
//! Nothing here is ever read from the witness. The prover supplies sibling nodes;
//! the guest decides where they go. That is the whole soundness story of the
//! anchor: a lie about `target_slot` moves the index, the branch then fails to
//! reproduce `anchor_state_root`, and `run()` returns `Err`.
//!
//! # The two shapes
//!
//! ```text
//!  Recent  (target within 8,192 slots of the anchor)
//!    BeaconState                       depth d  (5 Capella/Deneb, 6 Electra/Fulu)
//!      .block_roots        field 5     Vector[Root, 8192]      depth 13
//!        [target_slot % 8192]  <- the target block root
//!
//!  Historical (anything older, back to Capella)
//!    BeaconState                       depth d
//!      .historical_summaries field 27  List[HistoricalSummary, 2^24]
//!        mix_in_length                 depth  1   (data root is the LEFT child)
//!        data tree                     depth 24
//!          [chunk]                     HistoricalSummary, 2 fields, depth 1
//!            .block_summary_root field 0  Vector[Root, 8192]   depth 13
//!              [target_slot % 8192]  <- the target block root
//! ```
//!
//! # Why `state.block_roots[target_slot % 8192]` is the target block
//!
//! `process_slot` writes `state.block_roots[state.slot % 8192] =
//! hash_tree_root(state.latest_block_header)` *before* incrementing the slot, so
//! the post-state of the block at `anchor_slot` has entries filled for every slot
//! up to and including `anchor_slot - 1`; its own index still holds the entry from
//! `anchor_slot - 8192`. Hence the Recent window is exactly
//! `target_slot in [anchor_slot - 8192, anchor_slot - 1]`.
//!
//! A skipped slot leaves the previous slot's block root duplicated in the ring, so
//! a mislabeled `target_slot` can alias one *canonical* block onto another slot
//! index. It cannot reach a non-ancestor. `target_slot` is therefore committed for
//! traceability only — the numbers that matter (`el_block_number`, `el_block_hash`)
//! come out of the block's own `ExecutionPayload`.

use crate::error::Error;
use crate::fork_schedule::{
    ForkVersion, BLOCK_ROOTS_TREE_DEPTH, CAPELLA_SLOT, HISTORICAL_ROOTS_LIMIT_DEPTH,
    SLOTS_PER_HISTORICAL_ROOT,
};
use beacon_cl_core::fork::state_tree_depth;

/// 0-based `BeaconState` position of `block_roots`. Unchanged at every fork from
/// phase0 onward (fields 0..5 are `genesis_time`, `genesis_validators_root`,
/// `slot`, `fork`, `latest_block_header`, `block_roots`); the tests re-read the
/// real declarations in `crates/network/src/beacon/` to prove it.
pub const BLOCK_ROOTS_POSITION: usize = 5;

/// 0-based `BeaconState` position of `historical_summaries` — the last Capella
/// field. Electra's nine additions and Fulu's one all append after it, so the
/// position is the same at every fork this crate knows.
pub const HISTORICAL_SUMMARIES_POSITION: usize = 27;

/// 0-based `HistoricalSummary` position of `block_summary_root` (the container's
/// other field is `state_summary_root`). Two fields, so a depth-1 tree.
pub const BLOCK_SUMMARY_ROOT_POSITION: u64 = 0;

/// Depth of the `HistoricalSummary` container tree.
pub const HISTORICAL_SUMMARY_TREE_DEPTH: usize = 1;

/// Generalized index of `BeaconBlock.state_root`: the block container has 5 fields
/// (`slot`, `proposer_index`, `parent_root`, `state_root`, `body`), so 8 leaf slots
/// and `8 + 3 = 11`. Fork-independent Capella..Fulu.
pub const BLOCK_STATE_ROOT_GINDEX: u64 = 11;

/// Generalized index of `BeaconBlockBody.execution_payload` inside `BeaconBlock`.
///
/// `BeaconBlockBody` has 11 (Capella) / 12 (Deneb) / 13 (Electra, Fulu) fields —
/// all <= 16, so a depth-4 tree — with `execution_payload` at position 9:
/// gindex `16 + 9 = 25`. `BeaconBlock.body` is position 4 of 5: gindex `8 + 4 = 12`.
/// Composing the payload subtree under the block tree gives
/// `(12 << 4) | 9 = 192 + 9 = 201`, a depth-7 branch.
///
/// Fork-independent Capella..Fulu because the body field count never leaves
/// `(8, 16]` across those forks.
pub const PAYLOAD_TO_BLOCK_ROOT_GINDEX: u64 = 201;

/// Which shape of the state the target block root is reached through.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {
    /// `state.block_roots[target_slot % 8192]`.
    Recent,
    /// `state.historical_summaries[chunk].block_summary_root[target_slot % 8192]`.
    Historical,
}

impl PathKind {
    /// Wire encoding: `0` Recent, `1` Historical.
    pub const fn as_u8(self) -> u8 {
        match self {
            PathKind::Recent => 0,
            PathKind::Historical => 1,
        }
    }

    /// Decode the wire byte. Anything else is a rejection, not a default.
    pub const fn from_u8(b: u8) -> Result<Self, Error> {
        match b {
            0 => Ok(PathKind::Recent),
            1 => Ok(PathKind::Historical),
            got => Err(Error::BadPathKind { got }),
        }
    }
}

/// Depth implied by a generalized index, `floor(log2(g))`. Re-exported from
/// `beacon_cl_core` so the witness's sibling count is checked against the same
/// definition `branch_root` folds with.
pub use beacon_cl_core::fork::depth_for_gindex;

/// The generalized index of the TARGET BLOCK ROOT within the anchor
/// `BeaconState`, per SPEC step 3.
///
/// `fork` is the anchor's fork (it fixes the state container depth `d`); the two
/// slots decide range validity and the ring/chunk indices. See the module docs for
/// the tree shapes and the `[anchor_slot - 8192, anchor_slot - 1]` derivation.
pub fn state_to_block_root_gindex(
    fork: ForkVersion,
    path: PathKind,
    anchor_slot: u64,
    target_slot: u64,
) -> Result<u64, Error> {
    let d = state_tree_depth(fork);
    // `next_power_of_two(field_count)`, i.e. the state container's leaf-slot count.
    let slots = 1u64 << d;

    match path {
        PathKind::Recent => {
            if target_slot >= anchor_slot {
                return Err(Error::TargetNotBeforeAnchor {
                    anchor_slot,
                    target_slot,
                });
            }
            // `target_slot < anchor_slot` above, so this cannot underflow.
            if anchor_slot - target_slot > SLOTS_PER_HISTORICAL_ROOT {
                return Err(Error::TargetOutsideRecentRange {
                    anchor_slot,
                    target_slot,
                });
            }
            let g = slots + BLOCK_ROOTS_POSITION as u64;
            Ok((g << BLOCK_ROOTS_TREE_DEPTH) | (target_slot % SLOTS_PER_HISTORICAL_ROOT))
        }
        PathKind::Historical => {
            // `fork_at` has already rejected pre-Capella slots before we get here;
            // re-check so this function is sound called on its own.
            if target_slot < CAPELLA_SLOT {
                return Err(Error::PreCapellaSlot { slot: target_slot });
            }
            let chunk = (target_slot - CAPELLA_SLOT) / SLOTS_PER_HISTORICAL_ROOT;
            let idx = target_slot % SLOTS_PER_HISTORICAL_ROOT;
            if chunk >= 1u64 << HISTORICAL_ROOTS_LIMIT_DEPTH {
                return Err(Error::HistoricalChunkOutOfRange { chunk });
            }

            // state.historical_summaries
            let mut g = slots + HISTORICAL_SUMMARIES_POSITION as u64;
            // ...its List data root: the LEFT child under mix_in_length.
            g *= 2;
            // ...element `chunk` of the 2^24-leaf data tree.
            g = (g << HISTORICAL_ROOTS_LIMIT_DEPTH) | chunk;
            // ...HistoricalSummary.block_summary_root, field 0 of 2.
            g = g * 2 + BLOCK_SUMMARY_ROOT_POSITION;
            // ...block_roots[idx] of that period.
            Ok((g << BLOCK_ROOTS_TREE_DEPTH) | idx)
        }
    }
}

/// Number of siblings a witness must carry for `path` at `fork`. Equal to
/// `depth_for_gindex(state_to_block_root_gindex(..))` by construction; kept as a
/// separate closed form so the README's sibling table has something to check
/// against, and asserted equal in the tests.
pub const fn expected_sibling_count(fork: ForkVersion, path: PathKind) -> usize {
    let d = state_tree_depth(fork);
    match path {
        PathKind::Recent => BLOCK_ROOTS_TREE_DEPTH + d,
        PathKind::Historical => {
            BLOCK_ROOTS_TREE_DEPTH
                + HISTORICAL_SUMMARY_TREE_DEPTH
                + HISTORICAL_ROOTS_LIMIT_DEPTH
                + 1 // mix_in_length
                + d
        }
    }
}
