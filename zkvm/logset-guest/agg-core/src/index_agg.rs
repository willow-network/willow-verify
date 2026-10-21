//! **L1 index aggregator** — the sorted log index at SEGMENT scope, built from
//! leaf proofs.
//!
//! A leaf proves a run of `span` blocks (header links, receipts tries, every
//! log) and commits the ERC `logSetRoot` over their match-all `blockLogsHash`es.
//! On Ethereum a leaf is ONE block; on a fast chain it is a `2^k`-block segment
//! sized by cycles. This aggregator takes
//! `n ≤ 64` such children, every block's record bytes and a permutation hint, and
//!
//! 1. **P1** — calls `verify(leaf_vkey, sha256(child_pv))` for every child, in
//!    order, over the child's FULL public values. `leaf_vkey` is an INPUT and is
//!    COMMITTED: the contract's program-key table decides
//!    which leaf programs it accepts from which block, so a receipt-format fork
//!    is one table append, not a new aggregator;
//! 2. decodes each child pv as `abi.encode(LogSetStatement[m])`, `m ≥ 1`, and
//!    uses statement `[0]`, which must be match-all (any further statements,
//!    e.g. per-filter ones, are covered by the P1 digest and otherwise ignored);
//! 3. runs the ordinary statement fold over those `[0]`s with the child span
//!    `2^k` read off the children ([`crate::derive_seg_log2`]): chain/filter
//!    uniformity, contiguity, seam hashes, alignment, root fold, count;
//! 4. for every block of every child re-hashes `BLOCK_TAG | number | k_b |
//!    records_b`, rebuilds the child's ERC root over those digests and requires
//!    it to equal the child's `logSetRoot`, with `Σ k_b == logCount`. Each
//!    block's stream must parse into exactly `k_b` whole records, so the parse
//!    is unique and no record can move between blocks (the digests commit the
//!    block number): the index entries are exactly the logs the leaf proved;
//! 5. feeds each block to the SAME `IndexBuilder` the index leaf guest uses and
//!    checks the host's permutation hint.
//!
//! The committed index is byte-identical to what the index leaf guest commits
//! over the same range, for one-block and multi-block children alike
//! (`tests::index_agg_tests::l1_equals_index_leaf*`).
//!
//! Input (first `read_vec`): `magic "WLI2" u32 LE | leaf_vkey 32 B (8 × u32 BE) |
//! n u32 LE | n × { pv_len u32 LE | pv | span × { rec_len u32 LE | records } }`,
//! `span` = the child statement's block count (one stream per block, so a
//! one-block child is the original one-stream frame byte for byte).
//! Hint: second `read_vec`. Public values ([`L1_PV_LEN`] = 448 bytes):
//! `abi.encode(LogSetStatement[1]) | IndexCommitment (96) | leaf_vkey (32)`.

use crate::vkey::{vkey_bytes_to_words, vkey_words_to_bytes};
use crate::{decode_statements, derive_seg_log2, fold_statements, AggError, CHILD_PV_LEN};
use logset_core::erc::{
    abi_encode_statements, block_digest_prefix, filter_hash, finish_block_digest, log_set_root,
    LogSetStatement,
};
use logset_core::index::{build_hint, IndexBuilder, IndexCommitment};

pub const IDX_AGG_MAGIC: u32 = 0x3249_4C57; // "WLI2" little-endian
/// Children per L1 aggregate: the 64-block index scope (DESIGN §1.1).
pub const MAX_L1_CHILDREN: u32 = 64;
/// Blocks per child at most (a fast-chain leaf segment is ≤ 2,048 today).
pub const MAX_CHILD_SPAN: u64 = 1 << 16;
/// `abi.encode(LogSetStatement[1])` + `IndexCommitment` + the leaf vkey.
pub const L1_PV_LEN: usize = CHILD_PV_LEN + 96 + 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdxAggError {
    Agg(AggError),
    Frame(&'static str),
    ChildCount(u32),
    ChildShape(u32),
    ChildSpanTooLong(u32),
    NotMatchAll,
    RecordParse(u32),
    CountMismatch(u32),
    DigestMismatch(u32),
    Index(logset_core::Error),
}

/// The L1 stdin for ONE-block children. `children[i] = (leaf pv, block records)`.
pub fn encode_idx_input(leaf_vkey: &[u32; 8], children: &[(Vec<u8>, Vec<u8>)]) -> Vec<u8> {
    let kids: Vec<(Vec<u8>, Vec<Vec<u8>>)> = children
        .iter()
        .map(|(pv, rec)| (pv.clone(), vec![rec.clone()]))
        .collect();
    encode_idx_input_blocks(leaf_vkey, &kids)
}

/// The L1 stdin for children of any span. `children[i] = (leaf pv, one record
/// stream per block of the child, in block order)`.
pub fn encode_idx_input_blocks(
    leaf_vkey: &[u32; 8],
    children: &[(Vec<u8>, Vec<Vec<u8>>)],
) -> Vec<u8> {
    let mut o = Vec::new();
    o.extend_from_slice(&IDX_AGG_MAGIC.to_le_bytes());
    o.extend_from_slice(&vkey_words_to_bytes(leaf_vkey));
    o.extend_from_slice(&(children.len() as u32).to_le_bytes());
    for (pv, blocks) in children {
        o.extend_from_slice(&(pv.len() as u32).to_le_bytes());
        o.extend_from_slice(pv);
        for rec in blocks {
            o.extend_from_slice(&(rec.len() as u32).to_le_bytes());
            o.extend_from_slice(rec);
        }
    }
    o
}

struct Rd<'a> {
    b: &'a [u8],
    p: usize,
}
impl<'a> Rd<'a> {
    fn u32(&mut self) -> Result<u32, IdxAggError> {
        let s = self.take(4)?;
        Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
    }
    fn take(&mut self, n: usize) -> Result<&'a [u8], IdxAggError> {
        let e = self
            .p
            .checked_add(n)
            .ok_or(IdxAggError::Frame("overflow"))?;
        if e > self.b.len() {
            return Err(IdxAggError::Frame("short"));
        }
        let s = &self.b[self.p..e];
        self.p = e;
        Ok(s)
    }
}

/// Offsets of each record in `recs` (relative to `base`), or `None` if the
/// stream does not parse into whole records exactly.
fn record_starts(recs: &[u8], base: usize) -> Option<Vec<u32>> {
    let mut starts = Vec::new();
    let mut p = 0usize;
    while p < recs.len() {
        // tx 4 | log 4 | emitter 20 | nTopics 1
        if recs.len() - p < 29 {
            return None;
        }
        let nt = recs[p + 28] as usize;
        if nt > 4 {
            return None;
        }
        let dl_off = p + 29 + 32 * nt;
        if recs.len() < dl_off + 4 {
            return None;
        }
        let dl = u32::from_be_bytes([
            recs[dl_off],
            recs[dl_off + 1],
            recs[dl_off + 2],
            recs[dl_off + 3],
        ]) as usize;
        let end = (dl_off + 4).checked_add(dl)?;
        if end > recs.len() {
            return None;
        }
        starts.push((base + p) as u32);
        p = end;
    }
    Some(starts)
}

/// One decoded child: its full pv, its `[0]` statement, one record stream per block.
type Child<'a> = (&'a [u8], LogSetStatement, Vec<&'a [u8]>);
/// A decoded L1 input: `(leaf vkey words, children)`.
type Children<'a> = ([u32; 8], Vec<Child<'a>>);

fn decode_input(input: &[u8]) -> Result<Children<'_>, IdxAggError> {
    let mut r = Rd { b: input, p: 0 };
    if r.u32()? != IDX_AGG_MAGIC {
        return Err(IdxAggError::Frame("magic"));
    }
    let vk: [u8; 32] = r.take(32)?.try_into().unwrap();
    let n = r.u32()?;
    if n == 0 || n > MAX_L1_CHILDREN {
        return Err(IdxAggError::ChildCount(n));
    }
    let mut kids = Vec::with_capacity(n as usize);
    for i in 0..n {
        let pl = r.u32()? as usize;
        let pv = r.take(pl)?;
        let s = decode_statements(pv)
            .ok_or(IdxAggError::ChildShape(i))?
            .swap_remove(0);
        if s.end_block < s.start_block {
            return Err(IdxAggError::ChildShape(i));
        }
        let span = s.end_block - s.start_block + 1;
        if span > MAX_CHILD_SPAN {
            return Err(IdxAggError::ChildSpanTooLong(i));
        }
        let mut blocks = Vec::with_capacity(span as usize);
        for _ in 0..span {
            let rl = r.u32()? as usize;
            blocks.push(r.take(rl)?);
        }
        kids.push((pv, s, blocks));
    }
    if r.p != input.len() {
        return Err(IdxAggError::Frame("trailing"));
    }
    Ok((vkey_bytes_to_words(&vk), kids))
}

/// Bind every block's records to its `blockLogsHash`, every child's digests to
/// its proven `logSetRoot`, and feed the blocks to `ix` in order. Shared by the
/// guest path and [`host_hint`], so the host's entry list is built by the code
/// the guest runs.
fn index_children(kids: &[Child<'_>], ix: &mut IndexBuilder) -> Result<(), IdxAggError> {
    for (i, (_, s, blocks)) in kids.iter().enumerate() {
        let i = i as u32;
        let mut digests = Vec::with_capacity(blocks.len());
        let mut count: u64 = 0;
        for (b, rec) in blocks.iter().enumerate() {
            let number = s.start_block + b as u64;
            let mut buf = block_digest_prefix(number);
            let base = buf.len();
            let starts = record_starts(rec, base).ok_or(IdxAggError::RecordParse(i))?;
            buf.extend_from_slice(rec);
            if !starts.is_empty() {
                ix.push_keys(number, &buf, &starts)
                    .map_err(IdxAggError::Index)?;
                ix.fill_leaves(number, &buf, &starts);
            }
            count += starts.len() as u64;
            digests.push(finish_block_digest(&mut buf, starts.len() as u32));
        }
        if count != s.log_count {
            return Err(IdxAggError::CountMismatch(i));
        }
        // One block: the root IS its blockLogsHash (a one-leaf ERC tree).
        let root = log_set_root(&digests).map_err(IdxAggError::Index)?;
        if root != s.log_set_root {
            return Err(IdxAggError::DigestMismatch(i));
        }
    }
    Ok(())
}

/// The whole argument. `verify(leaf_vkey, digest)` is the P1 sink (SP1's
/// `verify_sp1_proof` in the guest). Returns the segment statement, its index
/// commitment, the committed leaf vkey bytes and the public values.
pub fn aggregate_index(
    input: &[u8],
    hint: &[u8],
    sha256: &dyn Fn(&[u8]) -> [u8; 32],
    verify: &mut dyn FnMut(&[u32; 8], &[u8; 32]),
) -> Result<(LogSetStatement, IndexCommitment, [u8; 32], Vec<u8>), IdxAggError> {
    let (vk, kids) = decode_input(input)?;
    // 1. P1 over each FULL child pv, in order, under the input leaf vkey.
    for (pv, _, _) in &kids {
        verify(&vk, &sha256(pv));
    }
    // 2 + 3. The fold over the [0] statements; match-all only.
    let stmts: Vec<LogSetStatement> = kids.iter().map(|k| k.1.clone()).collect();
    let seg_log2 = derive_seg_log2(&stmts).map_err(IdxAggError::Agg)?;
    let stmt = fold_statements(seg_log2, &stmts).map_err(IdxAggError::Agg)?;
    let empty: [Vec<[u8; 32]>; 4] = Default::default();
    if stmt.filter_hash != filter_hash(&[], &empty) {
        return Err(IdxAggError::NotMatchAll);
    }
    // 4 + 5. Records bound to each proven blockLogsHash; the index over them.
    let mut ix = IndexBuilder::new();
    index_children(&kids, &mut ix)?;
    if ix.entries.len() as u64 != stmt.log_count {
        return Err(IdxAggError::CountMismatch(u32::MAX));
    }
    let com = ix.finish(hint).map_err(IdxAggError::Index)?;
    let leaf_key = vkey_words_to_bytes(&vk);
    let mut pv = abi_encode_statements(core::slice::from_ref(&stmt));
    pv.extend_from_slice(&com.encode());
    pv.extend_from_slice(&leaf_key);
    Ok((stmt, com, leaf_key, pv))
}

/// HOST: the permutation hint for an L1 input, from the same record binding
/// the guest runs (a child that would be rejected is rejected here too).
pub fn host_hint(input: &[u8]) -> Result<Vec<u8>, IdxAggError> {
    let (_, kids) = decode_input(input)?;
    let mut ix = IndexBuilder::new();
    index_children(&kids, &mut ix)?;
    Ok(build_hint(&ix.entries))
}

/// Split an L1 public-values blob into its three parts (canonical only).
pub fn decode_l1_pv(pv: &[u8]) -> Option<(LogSetStatement, IndexCommitment, [u8; 32])> {
    if pv.len() != L1_PV_LEN {
        return None;
    }
    let s = crate::decode_child(&pv[..CHILD_PV_LEN])?;
    let c = IndexCommitment::decode(&pv[CHILD_PV_LEN..CHILD_PV_LEN + 96])?;
    let k: [u8; 32] = pv[CHILD_PV_LEN + 96..].try_into().unwrap();
    Some((s, c, k))
}

/// HOST: every block's match-all record stream from a witness blob, in block
/// order (empty for a block with no logs) — what the segment tier stores next
/// to each per-block proof. The blob's filter list must be the single
/// match-all filter.
pub fn host_block_records(blob: &[u8]) -> Result<Vec<Vec<u8>>, logset_core::Error> {
    let w = logset_core::decode_blob(blob)?;
    let mut ix = IndexBuilder::new();
    ix.keep_records = true;
    logset_core::index::run_statements_core(blob, Some(&mut ix))?;
    let mut out = vec![Vec::new(); w.n_blocks as usize];
    for (b, r) in &ix.records {
        out[(b - w.start_block) as usize].extend_from_slice(r);
    }
    Ok(out)
}
