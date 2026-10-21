//! Every way the range log-set guest can reject, one variant each.
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
    /// The blob does not start with the little-endian `WLS1` magic.
    BadMagic { got: u32 },
    /// The first four bytes are the magic BYTE-REVERSED. Diagnostic only — still a
    /// rejection, but it names the one mistake a host encoder is likely to make.
    MagicByteOrder,
    /// A field ran off the end of the blob.
    Truncated {
        field: &'static str,
        need: usize,
        have: usize,
    },
    /// Bytes remain after the last declared field. A witness is a fixed shape;
    /// extra bytes mean the encoder and this decoder disagree.
    TrailingBytes { extra: usize },
    /// `n_blocks` was zero. A statement covers `start..=end` inclusive.
    ZeroBlocks,
    /// `n_filters` was zero. There would be nothing to commit.
    ZeroFilters,
    /// A filter list is not strictly ascending as an unsigned integer, so it is not
    /// in the ERC's canonical form and its `filterHash` would be ambiguous.
    /// `list`: 0 = addresses, 1..=4 = topics0..topics3.
    FilterNotCanonical {
        filter: usize,
        list: u8,
        index: usize,
    },
    /// An `opened` entry names a block outside `0..n_blocks`.
    OpenedIndexOutOfRange { index: u32, n_blocks: u32 },
    /// `opened` indices must be strictly ascending; a duplicate or a step backwards
    /// would let a prover open one block twice and skip another.
    OpenedIndexNotAscending { prev: u32, got: u32 },
    /// More receipts than the receipts trie can key (`rlp(0..=65535)`).
    /// `willow_receipts_trie::build_trie` PANICS above this, so it is checked here.
    TooManyReceipts { index: u32, n: usize },

    /// Structurally malformed RLP. `what` names the rule that was broken.
    Rlp { what: &'static str },
    /// A header RLP list has fewer items than the fields this guest reads.
    HeaderFieldCount { block: u32, got: usize },
    /// A fixed-width header field (`parentHash`, `receiptsRoot`, `logsBloom`) has
    /// the wrong length.
    HeaderFieldLen {
        block: u32,
        field: &'static str,
        got: usize,
    },
    /// The header's `number` is not a minimal-length big-endian integer that fits
    /// `u64`, so the committed range endpoints would be a truncation.
    BlockNumberNotCanonical { block: u32 },
    /// `header[i].number != start_block + i`.
    BlockNumberMismatch { index: u32, expected: u64, got: u64 },
    /// `header[i].parentHash != keccak256(header[i-1])`. The walk is not a chain.
    ParentHashMismatch { index: u32 },
    /// `start_block + n_blocks - 1` does not fit `u64`.
    RangeOverflow,

    /// The guest's own bloom predicate says a filter is possible in this block, but
    /// the witness declared it lean. This is the check that pins the host's
    /// classification to the guest's.
    BloomPositiveNotOpened { index: u32 },
    /// Receipts were supplied for a block no filter can match. Opening a lean block
    /// is not a soundness break, but it is a disagreement about the predicate, so
    /// it is fail-closed in both directions.
    LeanBlockOpened { index: u32 },
    /// The rebuilt receipts trie root does not equal the header's `receiptsRoot`.
    ReceiptsRootMismatch { index: u32 },

    /// A receipt trie value was empty, so it has neither a type byte nor an RLP body.
    ReceiptEmpty { block: u32, tx: u32 },
    /// A receipt's leading byte is neither a known EIP-2718 type (`0x01..=0x04`,
    /// or `0x7E` on an OP-stack chain) nor the start of an RLP list (legacy).
    ReceiptTypeUnknown { block: u32, tx: u32, ty: u8 },
    /// A receipt RLP list is not `[status_or_state, cumulativeGas, bloom, logs]`
    /// (a `0x7E` deposit may add `depositNonce` and `depositReceiptVersion`).
    ReceiptFieldCount { block: u32, tx: u32, got: usize },
    /// A deposit receipt's `depositNonce` / `depositReceiptVersion` is a list or
    /// not a canonical u64.
    DepositTailMalformed { block: u32, tx: u32 },
    /// A receipt's `logsBloom` field is not 256 bytes.
    ReceiptBloomLen { block: u32, tx: u32, got: usize },
    /// A receipt's `logs` field is not an RLP list.
    LogsNotAList { block: u32, tx: u32 },
    /// A log RLP list is not `[emitter, topics, data]`.
    LogFieldCount {
        block: u32,
        tx: u32,
        log: u32,
        got: usize,
    },
    /// A log's `emitter` is not 20 bytes.
    LogEmitterLen {
        block: u32,
        tx: u32,
        log: u32,
        got: usize,
    },
    /// A log's `topics` field is not an RLP list.
    LogTopicsNotAList { block: u32, tx: u32, log: u32 },
    /// A log topic is not 32 bytes.
    LogTopicLen {
        block: u32,
        tx: u32,
        log: u32,
        topic: usize,
        got: usize,
    },
    /// A log carries more than four topics; the ERC's record form encodes the count
    /// in one nibble's worth of range and the reference reverts.
    TooManyTopics {
        block: u32,
        tx: u32,
        log: u32,
        got: usize,
    },
    /// A log's `data` is longer than the record form's `u32` length field.
    LogDataTooLong { block: u32, tx: u32, log: u32 },
    /// A block emitted more than `u32::MAX` logs, so the block-wide `logIndex`
    /// would wrap.
    LogIndexOverflow { block: u32 },
    /// A block matched more than `u32::MAX` logs, so the per-block record count `k`
    /// would wrap.
    RecordCountOverflow { block: u32 },
    /// The statement's `logCount` would exceed `u64`.
    LogCountOverflow,

    /// A log-set tree over zero leaves has no root.
    EmptyRange,

    // ---- sorted log index (feature `index`) ----
    /// The index is only defined over the single match-all filter: anything else
    /// leaves blocks unopened and logs unscanned.
    #[cfg(feature = "index")]
    IndexNeedsMatchAll,
    /// The hint is not `empty` or `1 | n | permE | permT` of exact length.
    #[cfg(feature = "index")]
    IndexHintMalformed { what: &'static str },
    /// The hint declares a different entry count than the scan produced (a
    /// dropped or an added entry).
    #[cfg(feature = "index")]
    IndexHintCount { declared: u64, entries: u64 },
    /// A permutation index is `>= n` (the only way a host could name an entry the
    /// guest did not derive).
    #[cfg(feature = "index")]
    IndexPermOutOfRange { order: u8, at: u32 },
    /// A permutation index appears twice (a duplicated entry, hence a dropped one).
    #[cfg(feature = "index")]
    IndexPermDuplicate { order: u8, at: u32 },
    /// Adjacent entries are not in strictly ascending key order.
    #[cfg(feature = "index")]
    IndexNotAscending { order: u8, at: u32 },
    /// `entryCount != logCount` of the match-all statement.
    #[cfg(feature = "index")]
    IndexCountMismatch { entries: u64, log_count: u64 },
    /// A log record's bytes do not parse.
    #[cfg(feature = "index")]
    IndexRecordMalformed,
    /// A query range proof is invalid.
    #[cfg(feature = "index")]
    IndexProof { what: &'static str },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic { got } => write!(f, "bad blob magic 0x{got:08x}, want 0x574c5331"),
            Error::MagicByteOrder => write!(
                f,
                "blob magic is byte-reversed: the host wrote b\"WLS1\", the layout is the \
                 little-endian u32 0x574c5331 (bytes 31 53 4c 57)"
            ),
            Error::Truncated { field, need, have } => {
                write!(f, "truncated blob at `{field}`: need {need} bytes, have {have}")
            }
            Error::TrailingBytes { extra } => write!(f, "{extra} trailing byte(s) after the witness"),
            Error::ZeroBlocks => write!(f, "n_blocks is zero"),
            Error::ZeroFilters => write!(f, "n_filters is zero"),
            Error::FilterNotCanonical {
                filter,
                list,
                index,
            } => write!(
                f,
                "filter {filter} list {list} is not strictly ascending at element {index}"
            ),
            Error::OpenedIndexOutOfRange { index, n_blocks } => {
                write!(f, "opened index {index} is outside 0..{n_blocks}")
            }
            Error::OpenedIndexNotAscending { prev, got } => {
                write!(f, "opened indices must ascend: {prev} then {got}")
            }
            Error::TooManyReceipts { index, n } => {
                write!(f, "block {index} declares {n} receipts, the trie keys only 65536")
            }
            Error::Rlp { what } => write!(f, "malformed rlp: {what}"),
            Error::HeaderFieldCount { block, got } => {
                write!(f, "header {block} has {got} rlp items, need at least 9")
            }
            Error::HeaderFieldLen { block, field, got } => {
                write!(f, "header {block} field `{field}` is {got} bytes")
            }
            Error::BlockNumberNotCanonical { block } => {
                write!(f, "header {block} `number` is not a canonical u64")
            }
            Error::BlockNumberMismatch {
                index,
                expected,
                got,
            } => write!(f, "header {index} is block {got}, expected {expected}"),
            Error::ParentHashMismatch { index } => {
                write!(
                    f,
                    "header {index} parentHash != keccak256(header {})",
                    index.saturating_sub(1)
                )
            }
            Error::RangeOverflow => write!(f, "start_block + n_blocks - 1 overflows u64"),
            Error::BloomPositiveNotOpened { index } => write!(
                f,
                "block {index} is bloom-positive for some filter but was declared lean"
            ),
            Error::LeanBlockOpened { index } => {
                write!(f, "block {index} cannot match any filter but receipts were supplied")
            }
            Error::ReceiptsRootMismatch { index } => {
                write!(f, "block {index} rebuilt receipts root != header receiptsRoot")
            }
            Error::ReceiptEmpty { block, tx } => write!(f, "block {block} receipt {tx} is empty"),
            Error::ReceiptTypeUnknown { block, tx, ty } => write!(
                f,
                "block {block} receipt {tx} leads with 0x{ty:02x}: not a known type byte nor an rlp list"
            ),
            Error::ReceiptFieldCount { block, tx, got } => {
                write!(f, "block {block} receipt {tx} has {got} rlp items, need 4 (deposit: 4..=6)")
            }
            Error::DepositTailMalformed { block, tx } => write!(
                f,
                "block {block} deposit receipt {tx}: nonce/version is not a canonical u64"
            ),
            Error::ReceiptBloomLen { block, tx, got } => {
                write!(f, "block {block} receipt {tx} logsBloom is {got} bytes, need 256")
            }
            Error::LogsNotAList { block, tx } => {
                write!(f, "block {block} receipt {tx} `logs` is not an rlp list")
            }
            Error::LogFieldCount {
                block,
                tx,
                log,
                got,
            } => write!(f, "block {block} receipt {tx} log {log} has {got} rlp items, need 3"),
            Error::LogEmitterLen {
                block,
                tx,
                log,
                got,
            } => write!(f, "block {block} receipt {tx} log {log} emitter is {got} bytes"),
            Error::LogTopicsNotAList { block, tx, log } => {
                write!(f, "block {block} receipt {tx} log {log} `topics` is not an rlp list")
            }
            Error::LogTopicLen {
                block,
                tx,
                log,
                topic,
                got,
            } => write!(
                f,
                "block {block} receipt {tx} log {log} topic {topic} is {got} bytes, need 32"
            ),
            Error::TooManyTopics {
                block,
                tx,
                log,
                got,
            } => write!(f, "block {block} receipt {tx} log {log} has {got} topics, max 4"),
            Error::LogDataTooLong { block, tx, log } => {
                write!(f, "block {block} receipt {tx} log {log} data exceeds u32 bytes")
            }
            Error::LogIndexOverflow { block } => write!(f, "block {block} logIndex overflows u32"),
            Error::RecordCountOverflow { block } => {
                write!(f, "block {block} matching-record count overflows u32")
            }
            Error::LogCountOverflow => write!(f, "statement logCount overflows u64"),
            Error::EmptyRange => write!(f, "a log-set tree needs at least one leaf"),
            #[cfg(feature = "index")]
            Error::IndexNeedsMatchAll => {
                write!(f, "the sorted index needs exactly one filter, the match-all filter")
            }
            #[cfg(feature = "index")]
            Error::IndexHintMalformed { what } => write!(f, "index hint malformed: {what}"),
            #[cfg(feature = "index")]
            Error::IndexHintCount { declared, entries } => {
                write!(f, "index hint declares {declared} entries, the scan produced {entries}")
            }
            #[cfg(feature = "index")]
            Error::IndexPermOutOfRange { order, at } => {
                write!(f, "index order {order} position {at}: permutation index out of range")
            }
            #[cfg(feature = "index")]
            Error::IndexPermDuplicate { order, at } => {
                write!(f, "index order {order} position {at}: permutation index repeated")
            }
            #[cfg(feature = "index")]
            Error::IndexNotAscending { order, at } => {
                write!(f, "index order {order} position {at}: keys not strictly ascending")
            }
            #[cfg(feature = "index")]
            Error::IndexCountMismatch { entries, log_count } => {
                write!(f, "index has {entries} entries, the match-all logCount is {log_count}")
            }
            #[cfg(feature = "index")]
            Error::IndexRecordMalformed => write!(f, "malformed log record"),
            #[cfg(feature = "index")]
            Error::IndexProof { what } => write!(f, "index range proof rejected: {what}"),
        }
    }
}

impl std::error::Error for Error {}
