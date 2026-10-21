//! **The beacon-anchored window guest core — statement kind `0x4101`.**
//!
//! `WindowVerifier.verifyWindow` binds a window's end-block hash to Ethereum by
//! reading EIP-2935, whose ring holds 8,191 blocks (~27 h). Past that the window
//! proof is un-postable. This crate adds a SECOND, independent anchor: it proves
//! that the target execution block's `(block_number, block_hash)` sits under a
//! RECENT beacon block root `R`, reached through the anchor `BeaconState` —
//! `state.block_roots` for targets inside the 8,192-slot ring,
//! `state.historical_summaries` for anything older, back to Capella.
//!
//! The contract checks `R` against EIP-4788 (also an 8,191-slot ring). Only `R`
//! has to be fresh; the window proof itself becomes timeless.
//!
//! The anchor is its own SP1 proof, so the existing window guest / ELF / vkey are
//! untouched and only this cheap leg is refreshed per verify transaction.
//!
//! # The chain of custody, leaf to root
//!
//! ```text
//!   payload_leaves[6]  = block_number   \
//!   payload_leaves[12] = block_hash      | merkleize -> ExecutionPayload root
//!   ...the other 13 or 15 leaves        /
//!         | branch_payload, composed gindex 201 (depth 7)
//!         v
//!   target block root
//!         | siblings, gindex computed in-guest from (fork, path, slots)
//!         v
//!   anchor_state_root
//!         | branch_state_root, gindex 11 (depth 3)
//!         v
//!   R = anchor beacon block root      <- committed; EIP-4788 checks it on-chain
//! ```
//!
//! `block_number` and `block_hash` are read out of the SAME container root, so no
//! prover can mix a number from one block with a hash from another.
//!
//! # Assertion discipline
//!
//! One `Result` path. Every rejection is a real `Err` returned by [`run`] — the
//! exact function the SP1 program calls — so the native adversarial tests
//! exercise the shipping logic, not a host-side twin. Nothing panics on
//! witness-derived data:
//!
//! - `beacon_cl_core::fork::branch_root` returns `None` if the sibling count is
//!   not the depth its gindex implies; every call site here maps that to
//!   [`Error::SiblingCountMismatch`]. The fixed-size branch arrays are pinned
//!   to their depths by `const _: ()` asserts below, so those arms are
//!   unreachable — the asserts are the proof, the error arm is the belt.
//! - Slot arithmetic is ordered so no subtraction can underflow (see
//!   [`gindex::state_to_block_root_gindex`]).
//! - The `chunk` index is range-checked before it is shifted into a gindex.
//!
//! # Soundness notes
//!
//! 1. **Both shapes reach only canonical ancestors of `R`.** `block_roots`
//!    entries and every `historical_summaries` chunk are written by
//!    `process_slot` / `process_historical_summaries_update` from the canonical
//!    chain; there is no prover-chosen index anywhere on the path.
//! 2. **`target_slot` is committed for traceability only.** A skipped slot leaves
//!    the previous block's root duplicated in the ring, so a mislabeled
//!    `target_slot` can alias one canonical block onto a neighbouring slot index.
//!    It cannot reach a non-ancestor, and the committed number and hash come from
//!    the block itself. Consumers MUST NOT treat `target_slot` as proven.
//! 3. **Fork layouts are derived from slots in-guest.** A lie about either slot
//!    changes the container depth, the generalized index or the payload leaf
//!    count, and the branch stops reproducing its root — it fails closed.
//!    `BeaconState` field counts come from `beacon_cl_core::fork`, which carries
//!    an E0080 compile-time guard against the real `crates/network` declarations;
//!    this crate re-declares none of them.
//! 4. **`R` is derived, not checked.** The guest folds `anchor_state_root` up
//!    `branch_state_root` and commits whatever comes out. A prover who supplies a
//!    junk branch commits a junk `R`, and EIP-4788 rejects it on-chain.

pub mod blob;
pub mod error;
pub mod fork_schedule;
pub mod gindex;

/// Synthetic shape-exact fixtures. Test/host only — `cfg`-gated so none of it
/// reaches the ELF (and therefore none of it can move the vkey).
#[cfg(any(test, feature = "synth"))]
pub mod synth;

#[cfg(test)]
mod tests;

pub use blob::{decode_blob, encode_blob, Witness, BLOB_MAGIC};
pub use error::Error;
pub use fork_schedule::{
    fork_at, payload_field_count, payload_tree_depth, ForkVersion, CAPELLA_SLOT, DENEB_SLOT,
    ELECTRA_SLOT, FULU_SLOT, SLOTS_PER_HISTORICAL_ROOT,
};
pub use gindex::{
    depth_for_gindex, state_to_block_root_gindex, PathKind, BLOCK_STATE_ROOT_GINDEX,
    PAYLOAD_TO_BLOCK_ROOT_GINDEX,
};

use beacon_cl_core::fork::{branch_root, verify_branch};
use beacon_cl_core::ssz::{merkleize, Bytes32};
use fork_schedule::{PAYLOAD_BLOCK_HASH_POSITION, PAYLOAD_BLOCK_NUMBER_POSITION};

/// Statement family tag. 96 bytes is outside the set of lengths every other Willow
/// statement family uses ({136, 152, 160, 184, 200, 224, 232, 264}), so
/// cross-family confusion is impossible by length alone; the contract checks
/// `kind` as well.
pub const ANCHOR_KIND: u64 = 0x4101;

/// Committed statement length, in bytes.
pub const STATEMENT_LEN: usize = 96;

// `branch_root` rejects a sibling count that is not the depth its gindex
// implies. Both fixed-size branches in the witness are sized here, at compile
// time, so that arm is unreachable from witness data.
const _: () = assert!(
    depth_for_gindex(BLOCK_STATE_ROOT_GINDEX) == 3,
    "Witness::branch_state_root is [Bytes32; 3]; gindex 11 no longer implies depth 3"
);
const _: () = assert!(
    depth_for_gindex(PAYLOAD_TO_BLOCK_ROOT_GINDEX) == 7,
    "Witness::branch_payload is [Bytes32; 7]; gindex 201 no longer implies depth 7"
);

/// The 96-byte public statement. See `../SPEC.md` for the on-chain reader.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {
    /// Always [`ANCHOR_KIND`].
    pub kind: u64,
    /// `hash_tree_root(BeaconBlock)` at `anchor_slot` — the value EIP-4788 returns.
    pub anchor_beacon_root: Bytes32,
    /// The target block's `execution_payload.block_number`.
    pub el_block_number: u64,
    /// The target block's `execution_payload.block_hash` (== the EL header keccak).
    pub el_block_hash: Bytes32,
    /// Slot of the anchor beacon block.
    pub anchor_slot: u64,
    /// Slot the prover claimed for the target block. Traceability only — see
    /// soundness note 2 in the module docs.
    pub target_slot: u64,
}

impl Statement {
    /// The committed bytes. All integers little-endian, fixed offsets:
    /// `0 kind | 8 anchor_beacon_root | 40 el_block_number | 48 el_block_hash |
    /// 80 anchor_slot | 88 target_slot`.
    pub fn to_bytes(&self) -> [u8; STATEMENT_LEN] {
        let mut out = [0u8; STATEMENT_LEN];
        out[0..8].copy_from_slice(&self.kind.to_le_bytes());
        out[8..40].copy_from_slice(&self.anchor_beacon_root);
        out[40..48].copy_from_slice(&self.el_block_number.to_le_bytes());
        out[48..80].copy_from_slice(&self.el_block_hash);
        out[80..88].copy_from_slice(&self.anchor_slot.to_le_bytes());
        out[88..96].copy_from_slice(&self.target_slot.to_le_bytes());
        out
    }

    /// Parse committed bytes back. Used by `execrun` to diff the zkVM's public
    /// values against the native `run()`; not part of the guest path.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Error> {
        if bytes.len() != STATEMENT_LEN {
            return Err(Error::Truncated {
                field: "statement",
                need: STATEMENT_LEN,
                have: bytes.len(),
            });
        }
        let u64_at = |o: usize| {
            let mut a = [0u8; 8];
            a.copy_from_slice(&bytes[o..o + 8]);
            u64::from_le_bytes(a)
        };
        let node_at = |o: usize| {
            let mut a = [0u8; 32];
            a.copy_from_slice(&bytes[o..o + 32]);
            a
        };
        Ok(Statement {
            kind: u64_at(0),
            anchor_beacon_root: node_at(8),
            el_block_number: u64_at(40),
            el_block_hash: node_at(48),
            anchor_slot: u64_at(80),
            target_slot: u64_at(88),
        })
    }
}

/// **The guest entry point.** Decode the witness, walk leaf -> `R`, return the
/// statement. See the module docs for the chain of custody and the soundness
/// argument; `../SPEC.md` for the on-chain half.
pub fn run(blob: &[u8]) -> Result<Statement, Error> {
    let w = decode_blob(blob)?;

    // 1. Fork layouts, DERIVED from the two slots. Pre-Capella is a rejection.
    let fork_anchor = fork_at(w.anchor_slot)?;
    let fork_target = fork_at(w.target_slot)?;

    // 4 (leaf side first). The ExecutionPayload container: its leaf count must be
    // exactly the target fork's, so a Capella witness cannot be replayed as Deneb
    // (which would shift every field's position by a whole tree level).
    let expected_leaves = payload_field_count(fork_target);
    if w.payload_leaves.len() != expected_leaves {
        return Err(Error::PayloadLeafCountMismatch {
            expected: expected_leaves,
            got: w.payload_leaves.len(),
        });
    }
    let payload_root = merkleize(&w.payload_leaves, payload_tree_depth(fork_target));

    // ExecutionPayload root -> target block root, composed gindex 201.
    let target_block_root = branch_root(
        &payload_root,
        &w.branch_payload,
        PAYLOAD_TO_BLOCK_ROOT_GINDEX,
    )
    .ok_or(Error::SiblingCountMismatch {
        gindex: PAYLOAD_TO_BLOCK_ROOT_GINDEX,
        expected: depth_for_gindex(PAYLOAD_TO_BLOCK_ROOT_GINDEX),
        got: w.branch_payload.len(),
    })?;

    // 3. Target block root -> anchor_state_root. The index is computed here, never
    // witnessed, and the sibling count must be exactly the depth it implies.
    let gindex = state_to_block_root_gindex(fork_anchor, w.path, w.anchor_slot, w.target_slot)?;
    let expected_siblings = depth_for_gindex(gindex);
    if w.siblings.len() != expected_siblings {
        return Err(Error::SiblingCountMismatch {
            gindex,
            expected: expected_siblings,
            got: w.siblings.len(),
        });
    }
    if !verify_branch(
        &target_block_root,
        &w.siblings,
        gindex,
        &w.anchor_state_root,
    ) {
        return Err(Error::BlockRootBranchInvalid { gindex });
    }

    // 2. anchor_state_root -> R. Derived and committed, not checked: EIP-4788 is
    // what decides whether R is a real recent beacon root.
    let anchor_beacon_root = branch_root(
        &w.anchor_state_root,
        &w.branch_state_root,
        BLOCK_STATE_ROOT_GINDEX,
    )
    .ok_or(Error::SiblingCountMismatch {
        gindex: BLOCK_STATE_ROOT_GINDEX,
        expected: depth_for_gindex(BLOCK_STATE_ROOT_GINDEX),
        got: w.branch_state_root.len(),
    })?;

    // 5. Read the two EL values out of the one container we just authenticated.
    let number_leaf: Bytes32 = w.payload_leaves[PAYLOAD_BLOCK_NUMBER_POSITION];
    if number_leaf[8..] != [0u8; 24] {
        return Err(Error::BlockNumberLeafNotCanonical);
    }
    let mut n = [0u8; 8];
    n.copy_from_slice(&number_leaf[..8]);
    let el_block_number = u64::from_le_bytes(n);
    let el_block_hash = w.payload_leaves[PAYLOAD_BLOCK_HASH_POSITION];

    Ok(Statement {
        kind: ANCHOR_KIND,
        anchor_beacon_root,
        el_block_number,
        el_block_hash,
        anchor_slot: w.anchor_slot,
        target_slot: w.target_slot,
    })
}
