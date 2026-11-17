














pub mod filter;

pub mod system;
pub mod verify;
pub mod wire;

pub use filter::{plan, Log, LogFilter};

pub use system::{openvm_body, ProofSystem, OPENVM_PROOF_MAGIC};
pub use verify::{
    block_statement, compare, leaf_statement, verify, BySystem, Comparison, Keys, ProofCheck,
    Trust, Verified, VerifyError, NOT_A_PROOF,
};
pub use wire::{
    BlockProof, LeafProof, LogsProof, ProofId, ProofPart, RangeProofWire, RecordWire, SegmentProof,
};

use logset_core::erc::block_digest_prefix;



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





