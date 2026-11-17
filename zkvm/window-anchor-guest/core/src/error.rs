






use core::fmt;



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {

    BadMagic { got: u32 },

    Truncated {
        field: &'static str,
        need: usize,
        have: usize,
    },


    TrailingBytes { extra: usize },

    BadPathKind { got: u8 },

    PayloadLeafCountInvalid { got: u8 },


    PreCapellaSlot { slot: u64 },


    TargetNotBeforeAnchor { anchor_slot: u64, target_slot: u64 },


    TargetOutsideRecentRange { anchor_slot: u64, target_slot: u64 },

    HistoricalChunkOutOfRange { chunk: u64 },



    SiblingCountMismatch {
        gindex: u64,
        expected: usize,
        got: usize,
    },

    PayloadLeafCountMismatch { expected: usize, got: usize },


    BlockRootBranchInvalid { gindex: u64 },


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
