//! Every way the anchor can fail, one variant each.
//!
//! There is exactly one assertion path in this crate: return `Err`. The guest's
//! `main` unwraps it, so a rejection here is a rejection of the proof — the same
//! code the native adversarial tests exercise. Nothing in this crate panics on
//! witness-derived data.

use core::fmt;

/// A rejection reason. One variant per distinct failure so a test can assert on
/// *which* check fired, not merely that something did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The blob does not start with `WAN1`.
    BadMagic { got: u32 },
    /// A field ran off the end of the blob.
    Truncated {
        field: &'static str,
        need: usize,
        have: usize,
    },
    /// Bytes remain after the last declared field. A witness is a fixed shape;
    /// extra bytes mean the encoder and this decoder disagree.
    TrailingBytes { extra: usize },
    /// `path_kind` was neither 0 (Recent) nor 1 (Historical).
    BadPathKind { got: u8 },
    /// `n_payload_leaves` was not a shape this crate knows (15 or 17).
    PayloadLeafCountInvalid { got: u8 },
    /// A slot predates the Capella fork, so `historical_summaries` does not exist
    /// and no shape here applies.
    PreCapellaSlot { slot: u64 },
    /// Recent path: the target must be strictly before the anchor. The anchor
    /// block's post-state has not yet written its own slot into `block_roots`.
    TargetNotBeforeAnchor { anchor_slot: u64, target_slot: u64 },
    /// Recent path: `anchor_slot - target_slot > SLOTS_PER_HISTORICAL_ROOT`, so the
    /// ring entry has been overwritten.
    TargetOutsideRecentRange { anchor_slot: u64, target_slot: u64 },
    /// Historical path: the computed chunk index does not fit `HISTORICAL_ROOTS_LIMIT`.
    HistoricalChunkOutOfRange { chunk: u64 },
    /// The witness supplied a sibling list whose length is not the depth implied by
    /// the *in-guest computed* generalized index. The depth is a property of the
    /// index, never of the prover's data.
    SiblingCountMismatch {
        gindex: u64,
        expected: usize,
        got: usize,
    },
    /// The `ExecutionPayload` leaf count does not match the target slot's fork.
    PayloadLeafCountMismatch { expected: usize, got: usize },
    /// The target block root did not fold up to `anchor_state_root` along the
    /// computed `block_roots` / `historical_summaries` path.
    BlockRootBranchInvalid { gindex: u64 },
    /// `leaves[6]` (`block_number`) had non-zero bytes above the low 8, so it is not
    /// a canonical SSZ `uint64` leaf and the committed number would be a truncation.
    BlockNumberLeafNotCanonical,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic { got } => {
                write!(
                    f,
                    "bad witness magic 0x{got:08x}, expected 0x57414e31 (WAN1)"
                )
            }
            Error::Truncated { field, need, have } => write!(
                f,
                "witness truncated reading `{field}`: need {need} bytes, {have} remain"
            ),
            Error::TrailingBytes { extra } => {
                write!(f, "{extra} trailing byte(s) after the last witness field")
            }
            Error::BadPathKind { got } => {
                write!(
                    f,
                    "path_kind {got} is neither 0 (Recent) nor 1 (Historical)"
                )
            }
            Error::PayloadLeafCountInvalid { got } => write!(
                f,
                "n_payload_leaves {got} is not an ExecutionPayload shape (15 Capella, 17 Deneb+)"
            ),
            Error::PreCapellaSlot { slot } => {
                write!(f, "slot {slot} predates the Capella fork")
            }
            Error::TargetNotBeforeAnchor {
                anchor_slot,
                target_slot,
            } => write!(
                f,
                "Recent path needs target_slot < anchor_slot, got {target_slot} >= {anchor_slot}"
            ),
            Error::TargetOutsideRecentRange {
                anchor_slot,
                target_slot,
            } => write!(
                f,
                "target_slot {target_slot} is more than 8192 slots behind anchor_slot {anchor_slot}"
            ),
            Error::HistoricalChunkOutOfRange { chunk } => write!(
                f,
                "historical_summaries chunk {chunk} exceeds HISTORICAL_ROOTS_LIMIT (2^24)"
            ),
            Error::SiblingCountMismatch {
                gindex,
                expected,
                got,
            } => write!(
                f,
                "gindex {gindex} implies a depth-{expected} branch, witness supplied {got} siblings"
            ),
            Error::PayloadLeafCountMismatch { expected, got } => write!(
                f,
                "target fork's ExecutionPayload has {expected} fields, witness supplied {got}"
            ),
            Error::BlockRootBranchInvalid { gindex } => write!(
                f,
                "target block root does not reach anchor_state_root at gindex {gindex}"
            ),
            Error::BlockNumberLeafNotCanonical => write!(
                f,
                "block_number leaf has non-zero bytes above the low 8 (not a canonical uint64 leaf)"
            ),
        }
    }
}

impl std::error::Error for Error {}
