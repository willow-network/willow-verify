//! HOST: query proofs from what the segment tier stores.

use crate::filter::{plan, LogFilter, MAX_RANGES};
use crate::frame;
use logset_agg_core::index_agg::decode_l1_pv;
use logset_core::erc::LogSetStatement;
use logset_core::index::{
    index_root, prove_range, sorted_perm, tree_levels, IndexBuilder, IndexCommitment, Key, Order,
    RangeProof,
};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServeError {
    /// Not canonical L1 public values.
    PublicValues,
    /// One record stream per block of the segment is required.
    BlockCount {
        expected: u64,
        got: usize,
    },
    /// A stored record stream does not split into whole records.
    Records {
        block: u64,
    },
    Index(logset_core::Error),
    /// The stored records do not rebuild the committed index.
    Mismatch(&'static str),
    TooManyRanges(usize),
}

impl fmt::Display for ServeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServeError::PublicValues => write!(f, "not canonical L1 public values"),
            ServeError::BlockCount { expected, got } => {
                write!(f, "segment needs {expected} record streams, got {got}")
            }
            ServeError::Records { block } => write!(f, "block {block}: malformed record stream"),
            ServeError::Index(e) => write!(f, "index: {e}"),
            ServeError::Mismatch(what) => {
                write!(f, "stored records do not match the proof: {what}")
            }
            ServeError::TooManyRanges(n) => {
                write!(f, "filter needs {n} key ranges, at most {MAX_RANGES}")
            }
        }
    }
}

impl std::error::Error for ServeError {}

/// A final segment's sorted index, rebuilt from its stored record streams and
/// checked against the L1 commitment before anything is served from it.
pub struct SegmentIndex {
    pub statement: LogSetStatement,
    pub commitment: IndexCommitment,
    pub public_values: Vec<u8>,
    by_emitter: Sorted,
    by_topic0: Sorted,
}

struct Sorted {
    keys: Vec<Key>,
    records: Vec<(u64, Vec<u8>)>,
    levels: Vec<Vec<[u8; 32]>>,
    root: [u8; 32],
}

impl Sorted {
    fn new(ix: &IndexBuilder, o: Order) -> Self {
        let perm = sorted_perm(&ix.entries, o);
        let leaves: Vec<[u8; 32]> = perm.iter().map(|&p| ix.entries[p as usize].leaf).collect();
        Self {
            keys: perm
                .iter()
                .map(|&p| *ix.entries[p as usize].key(o))
                .collect(),
            records: perm
                .iter()
                .map(|&p| ix.records[p as usize].clone())
                .collect(),
            levels: tree_levels(&leaves),
            root: index_root(&leaves),
        }
    }
}

impl SegmentIndex {
    /// `blocks[i]` is block `start + i`'s match-all record stream.
    pub fn build(public_values: &[u8], blocks: &[Vec<u8>]) -> Result<Self, ServeError> {
        let (statement, commitment, _) =
            decode_l1_pv(public_values).ok_or(ServeError::PublicValues)?;
        let expected = statement
            .end_block
            .checked_sub(statement.start_block)
            .ok_or(ServeError::PublicValues)?
            + 1;
        if blocks.len() as u64 != expected {
            return Err(ServeError::BlockCount {
                expected,
                got: blocks.len(),
            });
        }
        let mut ix = IndexBuilder::new();
        ix.keep_records = true;
        for (i, rec) in blocks.iter().enumerate() {
            let block = statement.start_block + i as u64;
            let (buf, starts) = frame(block, rec).ok_or(ServeError::Records { block })?;
            if !starts.is_empty() {
                ix.push_keys(block, &buf, &starts)
                    .map_err(ServeError::Index)?;
                ix.fill_leaves(block, &buf, &starts);
            }
        }
        if ix.entries.len() as u64 != commitment.entry_count {
            return Err(ServeError::Mismatch("entry count"));
        }
        let by_emitter = Sorted::new(&ix, Order::ByEmitter);
        let by_topic0 = Sorted::new(&ix, Order::ByTopic0);
        if by_emitter.root != commitment.root_by_emitter
            || by_topic0.root != commitment.root_by_topic0
        {
            return Err(ServeError::Mismatch("index root"));
        }
        Ok(Self {
            statement,
            commitment,
            public_values: public_values.to_vec(),
            by_emitter,
            by_topic0,
        })
    }

    fn sorted(&self, o: Order) -> &Sorted {
        match o {
            Order::ByEmitter => &self.by_emitter,
            Order::ByTopic0 => &self.by_topic0,
        }
    }

    /// One range proof per [`plan`] range of `f`.
    pub fn prove(&self, f: &LogFilter) -> Result<Vec<RangeProof>, ServeError> {
        let ranges = self.ranges(f)?;
        Ok(ranges
            .iter()
            .map(|(o, kmin, kmax)| {
                let s = self.sorted(*o);
                prove_range(&s.keys, &s.records, &s.levels, kmin, kmax)
            })
            .collect())
    }

    /// How many records [`prove`](Self::prove) would send for `f`, boundary
    /// neighbours included, without building the proofs.
    pub fn records_for(&self, f: &LogFilter) -> Result<usize, ServeError> {
        let ranges = self.ranges(f)?;
        Ok(ranges
            .iter()
            .map(|(o, kmin, kmax)| {
                let keys = &self.sorted(*o).keys;
                let i = keys.partition_point(|k| k < kmin);
                let j = keys.partition_point(|k| k <= kmax);
                (j - i) + 2
            })
            .sum())
    }

    fn ranges(&self, f: &LogFilter) -> Result<Vec<(Order, Key, Key)>, ServeError> {
        let ranges = plan(f, self.statement.start_block, self.statement.end_block);
        if ranges.len() > MAX_RANGES {
            return Err(ServeError::TooManyRanges(ranges.len()));
        }
        Ok(ranges)
    }
}
