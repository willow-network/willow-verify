//! **The log-set segment aggregator** — N contiguous, aligned segment proofs of
//! the range log-set guest folded into ONE `LogSetStatement` over the whole
//! range, byte-identical to the statement the leaf guest would have produced
//! had it walked the entire range in one run.
//!
//! ```text
//!   seg 0: [s, s+S)      root_0  (own tree, cap S)
//!   seg 1: [s+S, s+2S)   root_1
//!   ...
//!   seg M-1: [.., end]   root_{M-1} (own tree, cap c = nextpow2(N_last) <= S)
//!                           └─ lifted to height k with zero subtrees
//!   top tree over M roots at height k, padded with Z_k to nextpow2(M)
//!     == log_set_root(all N leaves)          (proved in `tests::fold_equals_flat`)
//! ```
//!
//! # What the aggregate statement means
//!
//! Each child is the leaf guest's public values, `abi.encode(LogSetStatement[1])`
//! (320 bytes), proven under ONE child program vkey. The aggregator:
//!
//! - **P1 (prove time only)** — calls `verify(child_vkey, sha256(child_pv))` for
//!   every child, in order. In SP1 that is `verify_sp1_proof`, a NO-OP at execute
//!   time; the binding happens in the recursion layer. See `zkvm/aggregator/README.md`.
//! - **P2 shape** — every child pv is exactly one statement, canonical ABI.
//! - **P3 uniform** — same `chainId`, same `filterHash` across children.
//! - **P4 contiguity** — `start[i+1] == end[i] + 1` and
//!   `startParentHash[i+1] == endBlockHash[i]` (the hash chain continues across
//!   the seam, so the leaf walk's induction extends over the whole range).
//! - **P5 alignment** — every child but the last spans exactly `S = 2^k` blocks;
//!   the last spans `1..=S`. This is what makes the subtree fold equal the flat
//!   ERC tree.
//! - **P6 root** — the ERC root over the union, computed by lifting and folding.
//! - **P7 count** — `logCount` = checked sum.
//!
//! The committed bytes are `abi.encode(LogSetStatement[1])`, the SAME shape the
//! leaf guest commits and `LogSetVerifier` decodes — so an aggregate verifies
//! through the existing contract with only the range-vkey pin changed. The child
//! program is bound by the aggregator's own vkey: the guest compiles the child
//! vkey in from a tracked pin file ([`vkey`]), so a different child program means
//! a different aggregator vkey.

use logset_core::erc::{abi_encode_statements, node_hash, LogSetStatement};

pub const AGG_MAGIC: u32 = 0x574C_4131; // "WLA1", little-endian on the wire
/// abi.encode(LogSetStatement[1]) = offset word + length word + 8 words.
pub const CHILD_PV_LEN: usize = 64 + 256;
pub const MAX_SEG_LOG2: u8 = 24;
pub const MAX_CHILDREN: u32 = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AggError {
    Short,
    Trailing,
    BadMagic,
    SegLog2(u8),
    NoChildren,
    TooManyChildren(u32),
    ChildShape(u32),
    ChainId(u32),
    FilterHash(u32),
    NotContiguous(u32),
    SeamHash(u32),
    ChildSpan(u32),
    Overflow,
}

impl core::fmt::Display for AggError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// The aggregator's stdin, one `read_vec`:
/// `magic u32 LE | seg_log2 u8 | n u32 LE | n × 320-byte child pv`.
pub fn encode_input(seg_log2: u8, children: &[Vec<u8>]) -> Vec<u8> {
    let mut o = Vec::with_capacity(9 + children.len() * CHILD_PV_LEN);
    o.extend_from_slice(&AGG_MAGIC.to_le_bytes());
    o.push(seg_log2);
    o.extend_from_slice(&(children.len() as u32).to_le_bytes());
    for c in children {
        o.extend_from_slice(c);
    }
    o
}

fn word(b: &[u8], off: usize) -> [u8; 32] {
    let mut w = [0u8; 32];
    w.copy_from_slice(&b[off..off + 32]);
    w
}

/// A uint64 ABI word: the high 24 bytes must be zero (canonical).
fn u64_word(b: &[u8], off: usize) -> Option<u64> {
    if b[off..off + 24].iter().any(|&x| x != 0) {
        return None;
    }
    Some(u64::from_be_bytes(
        b[off + 24..off + 32].try_into().unwrap(),
    ))
}

/// Decode one child pv: exactly `abi.encode(LogSetStatement[1])`, canonical.
pub fn decode_child(pv: &[u8]) -> Option<LogSetStatement> {
    if pv.len() != CHILD_PV_LEN {
        return None;
    }
    decode_statements(pv).map(|mut v| v.remove(0))
}

/// Decode `abi.encode(LogSetStatement[m])`, `m >= 1`, canonical and exact
/// (offset word 0x20, length word m, `64 + 256·m` bytes, no dirty high bytes).
pub fn decode_statements(pv: &[u8]) -> Option<Vec<LogSetStatement>> {
    if pv.len() < CHILD_PV_LEN || (pv.len() - 64) % 256 != 0 {
        return None;
    }
    let m = (pv.len() - 64) / 256;
    if u64_word(pv, 0)? != 0x20 || u64_word(pv, 32)? != m as u64 {
        return None;
    }
    let mut out = Vec::with_capacity(m);
    for j in 0..m {
        let b = 64 + 256 * j;
        out.push(LogSetStatement {
            chain_id: u64_word(pv, b)?,
            start_block: u64_word(pv, b + 32)?,
            end_block: u64_word(pv, b + 64)?,
            start_parent_hash: word(pv, b + 96),
            end_block_hash: word(pv, b + 128),
            filter_hash: word(pv, b + 160),
            log_set_root: word(pv, b + 192),
            log_count: u64_word(pv, b + 224)?,
        });
    }
    Some(out)
}

/// Zero-subtree roots: `z[0] = 0x00..00`, `z[h+1] = node(z[h], z[h])`.
fn zero_roots(max_h: usize) -> Vec<[u8; 32]> {
    let mut z = Vec::with_capacity(max_h + 1);
    z.push([0u8; 32]);
    for h in 0..max_h {
        let n = node_hash(&z[h], &z[h]);
        z.push(n);
    }
    z
}

fn ceil_log2(n: u64) -> usize {
    let mut h = 0usize;
    while (1u64 << h) < n {
        h += 1;
    }
    h
}

/// Fold the children's roots into the ERC root over the union of their ranges.
/// `spans[i]` is child i's block count; P5 must already hold.
pub fn fold_roots(seg_log2: u8, roots: &[[u8; 32]], spans: &[u64]) -> [u8; 32] {
    let m = roots.len();
    if m == 1 {
        // One child: its own tree IS the range's tree (cap = nextpow2(N)).
        return roots[0];
    }
    let k = seg_log2 as usize;
    let top = ceil_log2(m as u64);
    let z = zero_roots(k + top);
    // Lift every child root to height k (only the last can be short).
    let mut level: Vec<[u8; 32]> = Vec::with_capacity(1 << top);
    for (r, &n) in roots.iter().zip(spans) {
        let mut x = *r;
        for h in ceil_log2(n)..k {
            x = node_hash(&x, &z[h]);
        }
        level.push(x);
    }
    level.resize(1 << top, z[k]);
    let mut h = k;
    while level.len() > 1 {
        let half = level.len() / 2;
        for i in 0..half {
            let (l, r) = (level[2 * i], level[2 * i + 1]);
            level[i] = if l == z[h] && r == z[h] {
                z[h + 1]
            } else {
                node_hash(&l, &r)
            };
        }
        level.truncate(half);
        h += 1;
    }
    level[0]
}

/// Everything the aggregator decides. `verify` is the P1 sink (SP1's
/// `verify_sp1_proof(child_vkey, digest)` in the guest), called once per child
/// in child order with `sha256(child_pv)`.
pub fn aggregate(
    input: &[u8],
    sha256: &dyn Fn(&[u8]) -> [u8; 32],
    verify: &mut dyn FnMut(&[u8; 32]),
) -> Result<(LogSetStatement, Vec<u8>), AggError> {
    if input.len() < 9 {
        return Err(AggError::Short);
    }
    if u32::from_le_bytes(input[0..4].try_into().unwrap()) != AGG_MAGIC {
        return Err(AggError::BadMagic);
    }
    let seg_log2 = input[4];
    if seg_log2 > MAX_SEG_LOG2 {
        return Err(AggError::SegLog2(seg_log2));
    }
    let n = u32::from_le_bytes(input[5..9].try_into().unwrap());
    if n == 0 {
        return Err(AggError::NoChildren);
    }
    if n > MAX_CHILDREN {
        return Err(AggError::TooManyChildren(n));
    }
    let need = 9usize + n as usize * CHILD_PV_LEN;
    if input.len() < need {
        return Err(AggError::Short);
    }
    if input.len() > need {
        return Err(AggError::Trailing);
    }

    let mut kids: Vec<LogSetStatement> = Vec::with_capacity(n as usize);
    for i in 0..n {
        let off = 9 + i as usize * CHILD_PV_LEN;
        let pv = &input[off..off + CHILD_PV_LEN];
        // P1: the proof of THIS pv under the pinned child vkey (prove time).
        verify(&sha256(pv));
        kids.push(decode_child(pv).ok_or(AggError::ChildShape(i))?);
    }
    let out = fold_statements(seg_log2, &kids)?;
    let pv = abi_encode_statements(core::slice::from_ref(&out));
    Ok((out, pv))
}

/// The child span `2^k` of a fold, read off the children: with two or more
/// children the first one's span, which must be a power of two (P5 then holds
/// every child but the last to it); with one child, the smallest `k` that
/// covers it. Not an input and not committed: whatever `k` passes P5, the folded
/// root is the flat ERC root over the union (`tests::fold_equals_flat*`), so the
/// geometry is free per chain (Ethereum 64-block L1s, a fast chain's `2^k`-block
/// leaves and `Q`-block L1s) without a new program.
pub fn derive_seg_log2(kids: &[LogSetStatement]) -> Result<u8, AggError> {
    let first = kids.first().ok_or(AggError::NoChildren)?;
    if first.end_block < first.start_block {
        return Err(AggError::ChildShape(0));
    }
    let span = first.end_block - first.start_block + 1;
    let k = if kids.len() == 1 {
        ceil_log2(span) as u32
    } else if span.is_power_of_two() {
        span.trailing_zeros()
    } else {
        return Err(AggError::ChildSpan(0));
    };
    if k > MAX_SEG_LOG2 as u32 {
        return Err(AggError::SegLog2(k.min(255) as u8));
    }
    Ok(k as u8)
}

/// P2 range sanity and P3–P7 over already-bound child statements: the one fold
/// every aggregator level runs. `seg = 2^seg_log2` is the span every child but
/// the last must have.
pub fn fold_statements(
    seg_log2: u8,
    kids: &[LogSetStatement],
) -> Result<LogSetStatement, AggError> {
    if kids.is_empty() {
        return Err(AggError::NoChildren);
    }
    if seg_log2 > MAX_SEG_LOG2 {
        return Err(AggError::SegLog2(seg_log2));
    }
    let seg = 1u64 << seg_log2;
    for (i, s) in kids.iter().enumerate() {
        if s.end_block < s.start_block {
            return Err(AggError::ChildShape(i as u32));
        }
    }
    let first = &kids[0];
    let mut log_count: u64 = 0;
    let mut spans = Vec::with_capacity(kids.len());
    for (i, s) in kids.iter().enumerate() {
        let i32_ = i as u32;
        if s.chain_id != first.chain_id {
            return Err(AggError::ChainId(i32_));
        }
        if s.filter_hash != first.filter_hash {
            return Err(AggError::FilterHash(i32_));
        }
        if i > 0 {
            let p = &kids[i - 1];
            if p.end_block.checked_add(1) != Some(s.start_block) {
                return Err(AggError::NotContiguous(i32_));
            }
            if s.start_parent_hash != p.end_block_hash {
                return Err(AggError::SeamHash(i32_));
            }
        }
        let span = s.end_block - s.start_block + 1;
        let last = i + 1 == kids.len();
        if (!last && span != seg) || (last && span > seg) {
            return Err(AggError::ChildSpan(i32_));
        }
        spans.push(span);
        log_count = log_count
            .checked_add(s.log_count)
            .ok_or(AggError::Overflow)?;
    }
    let roots: Vec<[u8; 32]> = kids.iter().map(|s| s.log_set_root).collect();
    let last = kids.last().unwrap();
    Ok(LogSetStatement {
        chain_id: first.chain_id,
        start_block: first.start_block,
        end_block: last.end_block,
        start_parent_hash: first.start_parent_hash,
        end_block_hash: last.end_block_hash,
        filter_hash: first.filter_hash,
        log_set_root: fold_roots(seg_log2, &roots, &spans),
        log_count,
    })
}

#[cfg(feature = "index")]
pub mod index_agg;

#[cfg(feature = "index")]
pub mod fold;

pub mod vkey;

#[cfg(test)]
mod tests;
