//! **The range log-set guest core** — one proof that a committed log set is
//! COMPLETE over a contiguous block range.
//!
//! For a range `start..=end` and a list of filters, every block is either
//! *provably free* of matching logs — its `logsBloom` rules the filter out, and
//! the bloom has no false negatives — or *opened*: its receipts trie is rebuilt
//! from the witnessed values and checked against the header's `receiptsRoot`,
//! then every log in the block is scanned. Nothing in between. The result is one
//! ERC `LogSetStatement` per filter, all over the same header walk.
//!
//! ```text
//!   header[i] bytes ──keccak──> h_i                (parentHash[i+1] == h_i)
//!        │                       └── endBlockHash = h_last  (EIP-2935 anchors it)
//!        ├── number       == start_block + i
//!        ├── logsBloom    ──> possible?  no ──> leaf = blockLogsHash(n, [])
//!        └── receiptsRoot ──> build_trie(values) must equal it
//!                                     └── scan every log ──> leaf = blockLogsHash(n, records)
//!   leaves ──padded Merkle──> logSetRoot        (one tree per filter)
//! ```
//!
//! # Where the completeness comes from
//!
//! 1. **The range cannot be shrunk.** `startParentHash` and `endBlockHash` are
//!    both committed. Dropping a block from either end changes one of them, and
//!    the consumer checks `endBlockHash` against the canonical chain.
//! 2. **No block can be skipped.** The walk is a hash chain and every header's
//!    `number` must be `start_block + i`, so the `N` leaves are exactly the `N`
//!    blocks of the range.
//! 3. **A block cannot be waved off.** The bloom predicate is computed IN-GUEST
//!    from the header's own `logsBloom`; the witness's opened/lean split must
//!    equal the guest's, both directions.
//! 4. **An opened block cannot hide a log.** The receipts root pins the exact set
//!    of receipts and their order; `txIndex` and `logIndex` are positions, not
//!    witnessed fields.
//! 5. **The filter cannot be narrowed after the fact.** `filterHash` is derived
//!    in-guest from the filter the scan actually used, and a non-canonical filter
//!    is rejected outright.
//!
//! # Assertion discipline
//!
//! One `Result` path. Every rejection is a real `Err` returned by [`run`] — the
//! exact function the SP1 program calls — so the native adversarial tests
//! exercise the shipping logic, not a host-side twin. Nothing here panics on
//! witness data:
//!
//! - `willow_receipts_trie::build_trie` PANICS above 65,536 values (and its
//!   `rlp_index` panics above 65,535). [`blob::decode_blob`] rejects any block
//!   declaring more than [`blob::MAX_RECEIPTS`] receipts before a plan is built.
//! - `build_trie(&[])` returns the empty root rather than panicking, so a
//!   bloom-positive block with zero receipts fails on the root comparison.
//! - Every slice index goes through a checked accessor or a length test first;
//!   every counter that feeds a committed field is `checked_add`ed.

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

pub mod blob;
pub mod bloom;
pub mod erc;
pub mod error;
pub mod header;
pub mod receipt;
pub mod rlp;

#[cfg(any(test, feature = "synth"))]
pub mod synth;

#[cfg(test)]
mod tests;

pub use blob::{decode_blob, encode_blob, LogFilter, OpenedBlock, Witness, BLOB_MAGIC};
pub use erc::{
    abi_encode_statements, block_logs_hash, filter_hash, log_set_root, node_hash, statement_hash,
    LogRecord, LogSetStatement, BLOCK_TAG, FILTER_TYPEHASH, NODE_TAG, STATEMENT_TYPEHASH,
};
pub use error::Error;

/// keccak256. Under the zkVM this rides the SP1 `keccak_permute` precompile: the
/// workspace patches `tiny-keccak` to `sp1-patches/tiny-keccak`, which also
/// covers every hash `willow-receipts-trie` computes while rebuilding a receipts
/// trie. On the host the identical crate falls back to software keccak, so a root
/// computed in a `#[test]` is byte-for-byte the root the guest commits.
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Keccak};
    let mut k = Keccak::v256();
    k.update(data);
    let mut out = [0u8; 32];
    k.finalize(&mut out);
    out
}

// SP1 `cycle-tracker-report` spans. Compiled out entirely off the zkVM so the
// native tests and `execrun`'s host-side `run()` stay silent — and so the spans
// cost nothing in the code path the adversarial tests exercise.
#[cfg(target_os = "zkvm")]
macro_rules! span_start {
    ($n:literal) => {
        println!(concat!("cycle-tracker-report-start: ", $n))
    };
}
#[cfg(target_os = "zkvm")]
macro_rules! span_end {
    ($n:literal) => {
        println!(concat!("cycle-tracker-report-end: ", $n))
    };
}
#[cfg(not(target_os = "zkvm"))]
macro_rules! span_start {
    ($n:literal) => {};
}
#[cfg(not(target_os = "zkvm"))]
macro_rules! span_end {
    ($n:literal) => {};
}

/// The guest's whole job: decode the witness, prove completeness, and return the
/// public values — `abi.encode(LogSetStatement[])`, one element per filter, in
/// filter-list order.
pub fn run(blob: &[u8]) -> Result<Vec<u8>, Error> {
    let statements = run_statements(blob)?;
    span_start!("commit");
    let pv = abi_encode_statements(&statements);
    span_end!("commit");
    Ok(pv)
}

/// [`run`], stopping one step short of ABI encoding. The tests assert on fields.
pub fn run_statements(blob: &[u8]) -> Result<Vec<LogSetStatement>, Error> {
    scan(blob, &mut NoSink)
}

/// A consumer of every opened block's match-all record stream, called once per
/// opened block right after its scan. This is the ONE hook the sorted index
/// (`index::IndexBuilder`) plugs into; the default guest uses [`NoSink`].
///
/// `ACTIVE` is an associated const, so `scan::<NoSink>` compiles every sink
/// branch away: the default ELF carries no index code and pays no cycles for it.
/// There is exactly one orchestration body ([`scan`]); the index guest is the
/// same walk with a sink attached, not a copy of it.
pub trait LogSink {
    /// `true` ⇒ the sink needs the record offsets and is called per opened block.
    const ACTIVE: bool;
    /// Which filter lists the sink accepts, checked right after decode. The index
    /// sink admits only the single match-all filter (`Error::IndexNeedsMatchAll`).
    #[inline(always)]
    fn admit(_filters: &[LogFilter]) -> Result<(), Error> {
        Ok(())
    }
    /// `buf` is block `number`'s match-all `blockLogsHash` preimage (the 44-byte
    /// prefix, then every record in scan order) and `starts[j]` the offset of
    /// record `j` in it. Record `j` ends where `j+1` starts, the last at `buf.len()`.
    fn block(&mut self, number: u64, buf: &[u8], starts: &[u32]) -> Result<(), Error>;
}

/// The default: no sink.
pub struct NoSink;

impl LogSink for NoSink {
    const ACTIVE: bool = false;
    #[inline(always)]
    fn block(&mut self, _: u64, _: &[u8], _: &[u32]) -> Result<(), Error> {
        Ok(())
    }
}

/// THE orchestration: decode, header walk, bloom pin, trie rebuild, scan,
/// statements. Every guest ELF runs this one body.
pub fn scan<S: LogSink>(blob: &[u8], sink: &mut S) -> Result<Vec<LogSetStatement>, Error> {
    // ---- filters: decode, canonical-form check, filterHash, bloom probes ----
    span_start!("filters");
    let w = decode_blob(blob)?;
    let n = w.n_blocks as usize;
    let n_f = w.filters.len();
    let probes: Vec<bloom::FilterProbes> = w.filters.iter().map(bloom::FilterProbes::of).collect();
    let filter_hashes: Vec<[u8; 32]> = w
        .filters
        .iter()
        .map(|f| filter_hash(&f.addresses, &f.topics))
        .collect();
    span_end!("filters");
    S::admit(&w.filters)?;

    // ---- header walk: hash, parse, chain ----
    span_start!("header_walk");
    let mut receipts_roots: Vec<&[u8; 32]> = Vec::with_capacity(n);
    let mut blooms: Vec<&[u8; 256]> = Vec::with_capacity(n);
    let mut prev_hash = [0u8; 32];
    let mut start_parent_hash = [0u8; 32];
    for i in 0..n {
        let bytes = w.headers[i];
        let h = keccak256(bytes);
        let f = header::parse_header(bytes, i as u32)?;
        let expected = w.start_block + i as u64;
        if f.number != expected {
            return Err(Error::BlockNumberMismatch {
                index: i as u32,
                expected,
                got: f.number,
            });
        }
        if i == 0 {
            start_parent_hash = *f.parent_hash;
        } else if f.parent_hash != &prev_hash {
            return Err(Error::ParentHashMismatch { index: i as u32 });
        }
        prev_hash = h;
        receipts_roots.push(f.receipts_root);
        blooms.push(f.logs_bloom);
    }
    let end_block_hash = prev_hash;
    let end_block = w.start_block + (n as u64 - 1);
    span_end!("header_walk");

    // ---- bloom: the guest's own opened/lean classification, pinned to the witness ----
    span_start!("bloom");
    let mut is_open = vec![false; n];
    for (i, b) in blooms.iter().enumerate() {
        is_open[i] = probes.iter().any(|p| p.possible(b));
    }
    let mut declared = vec![false; n];
    for o in &w.opened {
        declared[o.index as usize] = true;
    }
    for i in 0..n {
        if is_open[i] && !declared[i] {
            return Err(Error::BloomPositiveNotOpened { index: i as u32 });
        }
        if !is_open[i] && declared[i] {
            return Err(Error::LeanBlockOpened { index: i as u32 });
        }
    }
    span_end!("bloom");

    // ---- receipts: rebuild the trie, scan every log, per-block digests ----
    span_start!("receipts");
    let profile = receipt::profile_of(w.chain_id);
    let mut leaves: Vec<Vec<[u8; 32]>> = vec![vec![[0u8; 32]; n]; n_f];
    let mut log_counts = vec![0u64; n_f];
    for o in &w.opened {
        let i = o.index as usize;
        let number = w.start_block + i as u64;
        #[cfg(all(target_os = "zkvm", feature = "perblock"))]
        println!("cycle-tracker-report-start: blk{i}");

        // `build_trie` takes owned values; the receipt bytes are borrowed out of
        // the blob, so one copy per opened block is unavoidable at this API.
        span_start!("receipts_copy");
        let values: Vec<Vec<u8>> = o.receipts.iter().map(|v| v.to_vec()).collect();
        span_end!("receipts_copy");
        span_start!("receipts_trie");
        let root = willow_receipts_trie::build_trie(&values).root;
        span_end!("receipts_trie");
        if &root != receipts_roots[i] {
            return Err(Error::ReceiptsRootMismatch { index: i as u32 });
        }
        drop(values);

        span_start!("receipts_scan");
        let mut bufs: Vec<Vec<u8>> = (0..n_f).map(|_| erc::block_digest_prefix(number)).collect();
        let mut ks = vec![0u32; n_f];
        let mut log_index: u32 = 0;
        let mut starts: Vec<u32> = Vec::new();
        for (tx, value) in o.receipts.iter().enumerate() {
            let logs = receipt::parse_receipt(value, i as u32, tx as u32, profile)?;
            for lg in &logs {
                if S::ACTIVE {
                    // Match-all: this log's record is appended to bufs[0] next.
                    starts.push(bufs[0].len() as u32);
                }
                for (fi, f) in w.filters.iter().enumerate() {
                    if f.matches(&lg.emitter, lg.topics()) {
                        erc::push_log_record(
                            &mut bufs[fi],
                            tx as u32,
                            log_index,
                            &lg.emitter,
                            lg.topics(),
                            lg.data,
                        );
                        ks[fi] = ks[fi]
                            .checked_add(1)
                            .ok_or(Error::RecordCountOverflow { block: i as u32 })?;
                    }
                }
                log_index = log_index
                    .checked_add(1)
                    .ok_or(Error::LogIndexOverflow { block: i as u32 })?;
            }
        }

        span_end!("receipts_scan");
        if S::ACTIVE {
            sink.block(number, &bufs[0], &starts)?;
        }

        let mut empty: Option<[u8; 32]> = None;
        for fi in 0..n_f {
            leaves[fi][i] = if ks[fi] == 0 {
                *empty.get_or_insert_with(|| erc::empty_block_logs_hash(number))
            } else {
                erc::finish_block_digest(&mut bufs[fi], ks[fi])
            };
            log_counts[fi] = log_counts[fi]
                .checked_add(ks[fi] as u64)
                .ok_or(Error::LogCountOverflow)?;
        }
        #[cfg(all(target_os = "zkvm", feature = "perblock"))]
        println!("cycle-tracker-report-end: blk{i}");
    }
    span_end!("receipts");

    // ---- commit: lean digests, one tree per filter, the statements ----
    span_start!("commit");
    for i in 0..n {
        if !is_open[i] {
            // `blockLogsHash(n, [])` depends only on the block number, so one
            // keccak serves every filter.
            let d = erc::empty_block_logs_hash(w.start_block + i as u64);
            for f in leaves.iter_mut() {
                f[i] = d;
            }
        }
    }
    let mut statements = Vec::with_capacity(n_f);
    for fi in 0..n_f {
        statements.push(LogSetStatement {
            chain_id: w.chain_id,
            start_block: w.start_block,
            end_block,
            start_parent_hash,
            end_block_hash,
            filter_hash: filter_hashes[fi],
            log_set_root: log_set_root(&leaves[fi])?,
            log_count: log_counts[fi],
        });
    }
    span_end!("commit");
    Ok(statements)
}

#[cfg(feature = "index")]
pub mod index;
#[cfg(feature = "index")]
pub use index::{run_indexed, run_statements_indexed};
#[cfg(all(test, feature = "index"))]
mod index_tests;
