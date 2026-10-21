//! **L2 segment fold** — `n` contiguous L1 index aggregates (64-block segments)
//! folded into ONE `LogSetStatement` over the union, carrying the LIST of the
//! children's index commitments by hash rather than merging them (DESIGN §2.4:
//! a merge costs ~2,660+ cycles/log per level and 64-block trees are already
//! compact for queries).
//!
//! Checks, in order:
//! - **P1** — `verify(sha256(child_pv))` per child, in order, under the L1 vkey
//!   compiled into the guest from `vkeys/logset-idxagg-program.koalabear`;
//! - **P2 shape** — each child pv is exactly [`L1_PV_LEN`] bytes, canonical;
//! - **one leaf program** — every child committed the SAME leaf vkey, which this
//!   fold commits in turn (the contract checks it against its key table);
//! - **index count** — `entryCount == logCount` per child;
//! - **P3–P7** — the same statement fold as every level, with the L1 span
//!   `2^k` read off the children ([`crate::derive_seg_log2`]: 64 blocks on
//!   Ethereum, the chain's index scope `Q` on a fast chain): uniform chain and
//!   filter (and the filter must be match-all), contiguity, seam hashes,
//!   alignment, root, count.
//!
//! Public values ([`FOLD_PV_LEN`] = 384 bytes):
//! `abi.encode(LogSetStatement[1]) | indexListHash (32) | leafVKey (32)` where
//!
//! ```text
//! indexListHash = keccak256(INDEX_LIST_TAG | n u32 BE |
//!                 n × (startBlock u64 BE | endBlock u64 BE | entryCount u64 BE | rootE | rootT))
//! ```
//! (`IndexListHashing.hash` in `contracts/.../LogSetVerifierV2.sol` is the same
//! bytes). A query client checks one range proof per listed segment.
//!
//! Input (one `read_vec`): `magic "WLF1" u32 LE | n u32 LE | n × L1 pv`.

use crate::index_agg::{decode_l1_pv, L1_PV_LEN};
use crate::{derive_seg_log2, fold_statements, AggError, CHILD_PV_LEN};
use logset_core::erc::{abi_encode_statements, filter_hash, LogSetStatement};
use logset_core::index::IndexCommitment;

pub const FOLD_MAGIC: u32 = 0x3146_4C57; // "WLF1" little-endian
/// Ethereum's L1 segments span 64 blocks (the last child of a fold may be
/// shorter). The fold itself reads the span off its children.
pub const L1_SEG_LOG2: u8 = 6;
pub const MAX_FOLD_CHILDREN: u32 = 4096;
/// `abi.encode(LogSetStatement[1])` + `indexListHash` + the leaf vkey.
pub const FOLD_PV_LEN: usize = CHILD_PV_LEN + 32 + 32;
/// `bytes32("COMPLETE_LOG_INDEX_LIST_V1")`.
pub const INDEX_LIST_TAG: [u8; 32] = {
    let s = b"COMPLETE_LOG_INDEX_LIST_V1";
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < s.len() {
        out[i] = s[i];
        i += 1;
    }
    out
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoldError {
    Short,
    Trailing,
    BadMagic,
    NoChildren,
    TooManyChildren(u32),
    ChildShape(u32),
    LeafKey(u32),
    EntryCount(u32),
    NotMatchAll,
    Agg(AggError),
}

/// One listed segment: its block range and its index commitment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexSegment {
    pub start_block: u64,
    pub end_block: u64,
    pub index: IndexCommitment,
}

pub fn encode_fold_input(children: &[Vec<u8>]) -> Vec<u8> {
    let mut o = Vec::with_capacity(8 + children.len() * L1_PV_LEN);
    o.extend_from_slice(&FOLD_MAGIC.to_le_bytes());
    o.extend_from_slice(&(children.len() as u32).to_le_bytes());
    for c in children {
        o.extend_from_slice(c);
    }
    o
}

/// The `indexListHash` preimage, hashed with logset-core's keccak.
pub fn index_list_hash(segs: &[IndexSegment]) -> [u8; 32] {
    let mut p = Vec::with_capacity(36 + segs.len() * 88);
    p.extend_from_slice(&INDEX_LIST_TAG);
    p.extend_from_slice(&(segs.len() as u32).to_be_bytes());
    for s in segs {
        p.extend_from_slice(&s.start_block.to_be_bytes());
        p.extend_from_slice(&s.end_block.to_be_bytes());
        p.extend_from_slice(&s.index.entry_count.to_be_bytes());
        p.extend_from_slice(&s.index.root_by_emitter);
        p.extend_from_slice(&s.index.root_by_topic0);
    }
    logset_core::keccak256(&p)
}

/// The whole argument. Returns the folded statement, the listed segments, the
/// `indexListHash`, the leaf vkey and the public values.
#[allow(clippy::type_complexity)]
pub fn fold(
    input: &[u8],
    sha256: &dyn Fn(&[u8]) -> [u8; 32],
    verify: &mut dyn FnMut(&[u8; 32]),
) -> Result<
    (
        LogSetStatement,
        Vec<IndexSegment>,
        [u8; 32],
        [u8; 32],
        Vec<u8>,
    ),
    FoldError,
> {
    if input.len() < 8 {
        return Err(FoldError::Short);
    }
    if u32::from_le_bytes(input[0..4].try_into().unwrap()) != FOLD_MAGIC {
        return Err(FoldError::BadMagic);
    }
    let n = u32::from_le_bytes(input[4..8].try_into().unwrap());
    if n == 0 {
        return Err(FoldError::NoChildren);
    }
    if n > MAX_FOLD_CHILDREN {
        return Err(FoldError::TooManyChildren(n));
    }
    let need = 8 + n as usize * L1_PV_LEN;
    if input.len() < need {
        return Err(FoldError::Short);
    }
    if input.len() > need {
        return Err(FoldError::Trailing);
    }
    let mut stmts = Vec::with_capacity(n as usize);
    let mut segs = Vec::with_capacity(n as usize);
    let mut leaf_key: Option<[u8; 32]> = None;
    for i in 0..n {
        let off = 8 + i as usize * L1_PV_LEN;
        let pv = &input[off..off + L1_PV_LEN];
        verify(&sha256(pv));
        let (s, c, k) = decode_l1_pv(pv).ok_or(FoldError::ChildShape(i))?;
        if *leaf_key.get_or_insert(k) != k {
            return Err(FoldError::LeafKey(i));
        }
        if c.entry_count != s.log_count {
            return Err(FoldError::EntryCount(i));
        }
        segs.push(IndexSegment {
            start_block: s.start_block,
            end_block: s.end_block,
            index: c,
        });
        stmts.push(s);
    }
    let k = derive_seg_log2(&stmts).map_err(FoldError::Agg)?;
    let out = fold_statements(k, &stmts).map_err(FoldError::Agg)?;
    let empty: [Vec<[u8; 32]>; 4] = Default::default();
    if out.filter_hash != filter_hash(&[], &empty) {
        return Err(FoldError::NotMatchAll);
    }
    let ilh = index_list_hash(&segs);
    let key = leaf_key.expect("n >= 1");
    let mut pv = abi_encode_statements(core::slice::from_ref(&out));
    pv.extend_from_slice(&ilh);
    pv.extend_from_slice(&key);
    Ok((out, segs, ilh, key, pv))
}

/// Split a fold's public values (canonical only).
pub fn decode_fold_pv(pv: &[u8]) -> Option<(LogSetStatement, [u8; 32], [u8; 32])> {
    if pv.len() != FOLD_PV_LEN {
        return None;
    }
    let s = crate::decode_child(&pv[..CHILD_PV_LEN])?;
    let h: [u8; 32] = pv[CHILD_PV_LEN..CHILD_PV_LEN + 32].try_into().unwrap();
    let k: [u8; 32] = pv[CHILD_PV_LEN + 32..].try_into().unwrap();
    Some((s, h, k))
}
