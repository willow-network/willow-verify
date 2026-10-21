//! CLIENT: check a [`LogsProof`] against the filter it answers.

use crate::filter::{plan, Log, LogFilter};
use crate::frame;
use crate::wire::{LogsProof, ProofPart};
use logset_agg_core::decode_statements;
use logset_agg_core::index_agg::decode_l1_pv;
use logset_core::erc::{filter_hash, finish_block_digest, log_set_root, LogSetStatement};
use logset_core::index::{verify_range, RangeProof};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;

/// Accepts an SP1 proof of exactly `public_values` under the program whose
/// verifying key hash is `vkey` (the `.koalabear` bytes in `vkeys/`).
pub trait ProofCheck {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String>;
}

/// What a client trusts: the chain and the program keys.
#[derive(Clone, Debug)]
pub struct Trust {
    pub chain_id: u64,
    /// L1 index aggregator keys.
    pub segment_vkeys: Vec<[u8; 32]>,
    /// Leaf program keys: checked against each L1's committed leaf key, and
    /// used for the leaves outside every segment.
    pub leaf_vkeys: Vec<[u8; 32]>,
}

/// A checked answer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Verified {
    /// Every log in the filter's blocks that matches it, in block and log order.
    pub logs: Vec<Log>,
    /// The parent hash of the first proven block.
    pub first_parent: (u64, [u8; 32]),
    /// `(number, hash)` of each part's last block. The proofs link every block
    /// in between, so the answer is about the chain these hashes are on: a
    /// client confirms the last one is canonical with a node it trusts.
    pub checkpoints: Vec<(u64, [u8; 32])>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VerifyError {
    EmptyRange,
    Chain {
        got: u64,
    },
    PublicValues,
    NotMatchAll {
        block: u64,
    },
    /// A tail part spans more than one block.
    Shape {
        block: u64,
    },
    /// No proof covers this block.
    Gap {
        block: u64,
    },
    /// A part's parent hash is not the previous part's last hash.
    Link {
        block: u64,
    },
    UnknownLeafProgram,
    Proof {
        id: String,
        reason: String,
    },
    /// The server's range proofs do not match the filter's ranges.
    RangeCount {
        block: u64,
        want: usize,
        got: usize,
    },
    Range {
        block: u64,
        reason: String,
    },
    /// A tail block's records do not hash to its proven root.
    BlockRecords {
        block: u64,
    },
    /// A leaf part does not carry one record stream per block.
    LeafBlocks {
        block: u64,
        want: u64,
        got: usize,
    },
    Record {
        block: u64,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use VerifyError::*;
        match self {
            EmptyRange => write!(f, "fromBlock is after toBlock"),
            Chain { got } => write!(f, "proof is for chain {got}"),
            PublicValues => write!(f, "malformed public values"),
            NotMatchAll { block } => {
                write!(f, "part at block {block} is not a match-all statement")
            }
            Shape { block } => write!(f, "tail part at block {block} spans more than one block"),
            Gap { block } => write!(f, "no proof covers block {block}"),
            Link { block } => write!(f, "block {block} does not link to the previous part"),
            UnknownLeafProgram => write!(f, "segment was built from an untrusted leaf program"),
            Proof { id, reason } => write!(f, "proof {id} rejected: {reason}"),
            RangeCount { block, want, got } => {
                write!(
                    f,
                    "segment at block {block}: {got} range proofs for {want} ranges"
                )
            }
            Range { block, reason } => write!(f, "segment at block {block}: {reason}"),
            BlockRecords { block } => write!(f, "block {block}: records do not match its proof"),
            LeafBlocks { block, want, got } => write!(
                f,
                "leaf at block {block}: {got} record streams for {want} blocks"
            ),
            Record { block } => write!(f, "block {block}: malformed record"),
        }
    }
}

impl std::error::Error for VerifyError {}

/// The `[0]` statement of a one-block leaf, after checking the block's record
/// stream hashes to its proven `logSetRoot` with `k = logCount`.
pub fn block_statement(
    public_values: &[u8],
    records: &[u8],
) -> Result<LogSetStatement, VerifyError> {
    let s = first_statement(public_values)?;
    if s.end_block != s.start_block {
        return Err(VerifyError::Shape {
            block: s.start_block,
        });
    }
    bind_streams(&s, [records])?;
    Ok(s)
}

/// The `[0]` statement of a leaf of any span, after checking `records` holds
/// one stream per block, each block's stream hashes to its `blockLogsHash`, the
/// ERC root over those equals `logSetRoot`, and the counts sum to `logCount`.
pub fn leaf_statement(
    public_values: &[u8],
    records: &[Vec<u8>],
) -> Result<LogSetStatement, VerifyError> {
    let s = first_statement(public_values)?;
    let span = s
        .end_block
        .checked_sub(s.start_block)
        .and_then(|d| d.checked_add(1))
        .ok_or(VerifyError::PublicValues)?;
    if records.len() as u64 != span {
        return Err(VerifyError::LeafBlocks {
            block: s.start_block,
            want: span,
            got: records.len(),
        });
    }
    bind_streams(&s, records.iter().map(Vec::as_slice))?;
    Ok(s)
}

fn first_statement(public_values: &[u8]) -> Result<LogSetStatement, VerifyError> {
    decode_statements(public_values)
        .and_then(|v| v.into_iter().next())
        .ok_or(VerifyError::PublicValues)
}

/// Each block's digest commits its number, so no record can move between blocks.
fn bind_streams<'a>(
    s: &LogSetStatement,
    streams: impl IntoIterator<Item = &'a [u8]>,
) -> Result<(), VerifyError> {
    let mut digests = Vec::new();
    let mut count = 0u64;
    for (i, rec) in streams.into_iter().enumerate() {
        let block = s.start_block + i as u64;
        let (mut buf, starts) = frame(block, rec).ok_or(VerifyError::Record { block })?;
        count += starts.len() as u64;
        digests.push(finish_block_digest(&mut buf, starts.len() as u32));
    }
    if count != s.log_count || log_set_root(&digests).ok() != Some(s.log_set_root) {
        return Err(VerifyError::BlockRecords {
            block: s.start_block,
        });
    }
    Ok(())
}

/// Every log of one block's record stream.
fn block_logs(block: u64, records: &[u8]) -> Result<Vec<Log>, VerifyError> {
    let (buf, starts) = frame(block, records).ok_or(VerifyError::Record { block })?;
    starts
        .iter()
        .enumerate()
        .map(|(j, &at)| {
            let end = starts.get(j + 1).map_or(buf.len(), |&n| n as usize);
            Log::decode(block, &buf[at as usize..end]).ok_or(VerifyError::Record { block })
        })
        .collect()
}

/// Check `p` against `f`. `fetch` returns a stored proof by id; `check` runs
/// the SP1 verifier. Returns every log `f` matches, proven complete.
pub fn verify(
    p: &LogsProof,
    f: &LogFilter,
    trust: &Trust,
    fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    check: &mut dyn ProofCheck,
) -> Result<Verified, VerifyError> {
    if f.from_block > f.to_block {
        return Err(VerifyError::EmptyRange);
    }
    if p.chain_id != trust.chain_id {
        return Err(VerifyError::Chain { got: p.chain_id });
    }
    let mut chain = Chain::new(f.from_block, trust.chain_id);
    let mut logs = BTreeMap::new();
    let mut keep = |log: Log| {
        if f.matches(&log) {
            logs.insert((log.block, log.log_index), log);
        }
    };
    for part in &p.parts {
        match part {
            ProofPart::Segment(seg) => {
                let (s, c, leaf_vkey) =
                    decode_l1_pv(&seg.public_values).ok_or(VerifyError::PublicValues)?;
                chain.link(&s)?;
                if !trust.leaf_vkeys.contains(&leaf_vkey) {
                    return Err(VerifyError::UnknownLeafProgram);
                }
                let block = s.start_block;
                let ranges = plan(f, s.start_block, s.end_block);
                if ranges.len() != seg.ranges.len() {
                    return Err(VerifyError::RangeCount {
                        block,
                        want: ranges.len(),
                        got: seg.ranges.len(),
                    });
                }
                prove_part(
                    fetch,
                    check,
                    &seg.proof,
                    &trust.segment_vkeys,
                    &seg.public_values,
                )?;
                for ((o, kmin, kmax), r) in ranges.iter().zip(&seg.ranges) {
                    let found = verify_range(
                        c.root(*o),
                        c.entry_count,
                        *o,
                        kmin,
                        kmax,
                        &RangeProof::from(r),
                    )
                    .map_err(|e| VerifyError::Range {
                        block,
                        reason: e.to_string(),
                    })?;
                    for (b, rec) in found {
                        keep(Log::decode(b, &rec).ok_or(VerifyError::Record { block: b })?);
                    }
                }
            }
            ProofPart::Block(blk) => {
                let s = block_statement(&blk.public_values, &blk.records)?;
                chain.link(&s)?;
                prove_part(
                    fetch,
                    check,
                    &blk.proof,
                    &trust.leaf_vkeys,
                    &blk.public_values,
                )?;
                block_logs(s.start_block, &blk.records)?
                    .into_iter()
                    .for_each(&mut keep);
            }
            ProofPart::Leaf(leaf) => {
                let s = leaf_statement(&leaf.public_values, &leaf.records)?;
                chain.link(&s)?;
                prove_part(
                    fetch,
                    check,
                    &leaf.proof,
                    &trust.leaf_vkeys,
                    &leaf.public_values,
                )?;
                for (i, rec) in leaf.records.iter().enumerate() {
                    block_logs(s.start_block + i as u64, rec)?
                        .into_iter()
                        .for_each(&mut keep);
                }
            }
        }
    }
    let (end, _) = chain.prev.ok_or(VerifyError::Gap {
        block: f.from_block,
    })?;
    if end < f.to_block {
        return Err(VerifyError::Gap { block: end + 1 });
    }
    Ok(Verified {
        logs: logs.into_values().collect(),
        first_parent: chain.first_parent.expect("set with prev"),
        checkpoints: chain.checkpoints,
    })
}

fn prove_part(
    fetch: &mut dyn FnMut(&str) -> Result<Vec<u8>, String>,
    check: &mut dyn ProofCheck,
    id: &str,
    vkeys: &[[u8; 32]],
    public_values: &[u8],
) -> Result<(), VerifyError> {
    let fail = |reason: String| VerifyError::Proof {
        id: id.to_string(),
        reason,
    };
    let proof = fetch(id).map_err(fail)?;
    let mut last = "no trusted program key".to_string();
    for vk in vkeys {
        match check.check(vk, &proof, public_values) {
            Ok(()) => return Ok(()),
            Err(e) => last = e,
        }
    }
    Err(fail(last))
}

/// Contiguity and hash links across parts.
struct Chain {
    from: u64,
    chain_id: u64,
    match_all: [u8; 32],
    prev: Option<(u64, [u8; 32])>,
    first_parent: Option<(u64, [u8; 32])>,
    checkpoints: Vec<(u64, [u8; 32])>,
}

impl Chain {
    fn new(from: u64, chain_id: u64) -> Self {
        Self {
            from,
            chain_id,
            match_all: filter_hash(&[], &Default::default()),
            prev: None,
            first_parent: None,
            checkpoints: Vec::new(),
        }
    }

    fn link(&mut self, s: &LogSetStatement) -> Result<(), VerifyError> {
        let block = s.start_block;
        if s.chain_id != self.chain_id {
            return Err(VerifyError::Chain { got: s.chain_id });
        }
        if s.filter_hash != self.match_all {
            return Err(VerifyError::NotMatchAll { block });
        }
        if s.end_block < s.start_block {
            return Err(VerifyError::PublicValues);
        }
        match self.prev {
            None => {
                if block > self.from {
                    return Err(VerifyError::Gap { block: self.from });
                }
                self.first_parent = Some((block, s.start_parent_hash));
            }
            Some((end, hash)) => {
                if block != end + 1 {
                    return Err(VerifyError::Gap { block: end + 1 });
                }
                if s.start_parent_hash != hash {
                    return Err(VerifyError::Link { block });
                }
            }
        }
        self.prev = Some((s.end_block, s.end_block_hash));
        self.checkpoints.push((s.end_block, s.end_block_hash));
        Ok(())
    }
}

/// How an `eth_getLogs` answer compares with the verified logs. Checked
/// fields: block number, transaction index, log index, address, topics and
/// data. Block and transaction hashes are not in the records.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Comparison {
    pub matched: usize,
    /// Proven logs the answer lacks or reports differently.
    pub missing: Vec<Log>,
    /// Answer entries that are not a proven log (extra, altered, or unparsable).
    pub unproven: Vec<Value>,
}

impl Comparison {
    pub fn is_exact(&self) -> bool {
        self.missing.is_empty() && self.unproven.is_empty()
    }
}

pub fn compare(verified: &[Log], answer: &[Value]) -> Comparison {
    let proven: BTreeMap<(u64, u32), &Log> = verified
        .iter()
        .map(|l| ((l.block, l.log_index), l))
        .collect();
    let mut seen = BTreeMap::new();
    let mut out = Comparison::default();
    for v in answer {
        match answer_log(v) {
            Some(l) if proven.get(&(l.block, l.log_index)) == Some(&&l) => {
                if seen.insert((l.block, l.log_index), ()).is_none() {
                    out.matched += 1;
                } else {
                    out.unproven.push(v.clone());
                }
            }
            _ => out.unproven.push(v.clone()),
        }
    }
    out.missing = verified
        .iter()
        .filter(|l| !seen.contains_key(&(l.block, l.log_index)))
        .cloned()
        .collect();
    out
}

fn answer_log(v: &Value) -> Option<Log> {
    if v.get("removed").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let q = |k: &str| -> Option<u64> {
        u64::from_str_radix(v.get(k)?.as_str()?.strip_prefix("0x")?, 16).ok()
    };
    let b = |s: &str| hex::decode(s.strip_prefix("0x")?).ok();
    Some(Log {
        block: q("blockNumber")?,
        tx_index: u32::try_from(q("transactionIndex")?).ok()?,
        log_index: u32::try_from(q("logIndex")?).ok()?,
        address: b(v.get("address")?.as_str()?)?.try_into().ok()?,
        topics: v
            .get("topics")?
            .as_array()?
            .iter()
            .map(|t| b(t.as_str()?)?.try_into().ok())
            .collect::<Option<_>>()?,
        data: b(v.get("data")?.as_str()?)?,
    })
}
