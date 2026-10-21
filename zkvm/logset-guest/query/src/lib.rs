//! **Complete log queries over proven segments** (DESIGN §2.7).
//!
//! A final segment's L1 proof commits the sorted index of every log in its
//! blocks (`IndexCommitment`). Everything a filter can match inside the segment
//! lies in a few contiguous runs of that index, so a query proof is the segment
//! proof plus one [`RangeProof`](logset_core::index::RangeProof) per run: no new
//! SP1 proof per query. Blocks not yet in a final segment are served as their
//! leaf proof (one block on Ethereum, an aligned `S`-block segment on a fast
//! chain) with every block's full record stream, which the client filters itself.
//!
//! [`serve`] builds proofs on the host from what the segment tier stores;
//! [`verify`] checks them on the client. Both derive the key ranges from the
//! filter with [`plan`]: a verifier never takes a range from the server, so a
//! server can only choose which bytes to send, never what counts as complete.

pub mod filter;
pub mod serve;
pub mod verify;
pub mod wire;

pub use filter::{plan, Log, LogFilter, MAX_RANGES};
pub use serve::{SegmentIndex, ServeError};
pub use verify::{
    block_statement, compare, leaf_statement, verify, Comparison, ProofCheck, Trust, Verified,
    VerifyError,
};
pub use wire::{
    BlockProof, LeafProof, LogsProof, ProofId, ProofPart, RangeProofWire, RecordWire, SegmentProof,
};

use logset_core::erc::block_digest_prefix;

/// A block's `blockLogsHash` preimage (`k` still zero) and the offset of each
/// record in it, or `None` if the stream does not split into whole records.
pub fn frame(number: u64, records: &[u8]) -> Option<(Vec<u8>, Vec<u32>)> {
    let mut buf = block_digest_prefix(number);
    let base = buf.len();
    let mut starts = Vec::new();
    let mut p = 0usize;
    while p < records.len() {
        starts.push(u32::try_from(base + p).ok()?);
        p += record_len(&records[p..])?;
    }
    buf.extend_from_slice(records);
    Some((buf, starts))
}

/// A multi-block leaf's `.rec` file: `n × (u32 LE length | stream)`, one
/// match-all record stream per block in block order (the L1 input's per-block
/// framing).
pub fn pack_streams(streams: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::with_capacity(streams.iter().map(|s| 4 + s.len()).sum());
    for s in streams {
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(s);
    }
    out
}

/// The inverse of [`pack_streams`]: exactly `n` streams and nothing after them.
pub fn unpack_streams(bytes: &[u8], n: u64) -> Option<Vec<Vec<u8>>> {
    let mut out = Vec::with_capacity(usize::try_from(n).ok()?.min(1 << 16));
    let mut p = 0usize;
    for _ in 0..n {
        let len = u32::from_le_bytes(bytes.get(p..p + 4)?.try_into().ok()?) as usize;
        let end = (p + 4).checked_add(len)?;
        out.push(bytes.get(p + 4..end)?.to_vec());
        p = end;
    }
    (p == bytes.len()).then_some(out)
}

/// Length of the record at the start of `r`: `tx 4 | log 4 | emitter 20 |
/// nTopics 1 | topics 32·n | dataLen 4 | data`.
fn record_len(r: &[u8]) -> Option<usize> {
    let nt = *r.get(28)? as usize;
    if nt > logset_core::erc::MAX_TOPICS {
        return None;
    }
    let dl_off = 29 + 32 * nt;
    let dl = u32::from_be_bytes(r.get(dl_off..dl_off + 4)?.try_into().ok()?) as usize;
    let end = (dl_off + 4).checked_add(dl)?;
    (end <= r.len()).then_some(end)
}

#[cfg(any(test, feature = "fixtures"))]
pub mod fixtures;
#[cfg(test)]
mod tests;
