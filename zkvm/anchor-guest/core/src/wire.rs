//! The kind-`0x0014` ANCHOR child statement — the ONE definition (the guest
//! commits `encode()`, the aggregate decodes those exact bytes and `pv_digest`s
//! them). Wire discipline identical to the sibling children: little-endian
//! `u64`s, raw 32-byte roots, byte concatenation, `kind` read first, exact
//! length.

use beacon_cl_core::ssz::Bytes32;

/// `kind` of the anchor child (child namespace `< 0x1000`; `0x0010` = CL window,
/// `0x0013` = EL operator-range).
pub const ANCHOR_KIND: u64 = 0x0014;

/// Encoded length of an [`AnchorStatement`]: 6×u64 + 4×32 = 176 bytes. Distinct
/// from every reserved Willow statement length (128, 150, 152, 158, 184, 208,
/// 224, 232, 280, 392, 464), so exact-length discipline keeps the families apart.
pub const ANCHOR_LEN: usize = 6 * 8 + 4 * 32;

/// The anchor child's committed public values.
///
/// `(beacon_state_root, slot)` are the FINALIZED header's fields — the pair the
/// aggregate must match. `sync_committee_root` names the signing committee; the
/// verifier contract's stored committee chain decides whether that root is THE
/// period-`signature_slot/8192` committee (the weak-subjectivity leg — a carried
/// "trusted committee" input would be prover-chosen). `next_sync_committee_root`
/// is the period-handoff output: the period-P+1 committee root proven under the
/// attested state, or all-zeros when the witness carried no handoff section.
/// The circuit enforces `period(signature_slot) == period(attested_slot)`
/// (period = `slot/8192`), so the handoff — proven under the ATTESTED state —
/// is unambiguously the period-`signature_slot/8192 + 1` committee the contract
/// attributes it to; a boundary-straddling signature would shift it by one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnchorStatement {
    pub kind: u64,
    /// Fork of the FINALIZED (anchored) state, in the aggregate's numbering.
    pub fork: u64,
    /// The finalized header's `state_root` — the canonical beacon anchor.
    pub beacon_state_root: Bytes32,
    /// The finalized header's `slot`.
    pub slot: u64,
    pub attested_slot: u64,
    pub signature_slot: u64,
    /// Number of committee participation bits set — re-checkable on chain against
    /// the in-circuit supermajority floor.
    pub participation: u64,
    pub genesis_validators_root: Bytes32,
    pub sync_committee_root: Bytes32,
    pub next_sync_committee_root: Bytes32,
}

impl AnchorStatement {
    pub fn encode(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(ANCHOR_LEN);
        let u = |b: &mut Vec<u8>, v: u64| b.extend_from_slice(&v.to_le_bytes());
        u(&mut b, self.kind);
        u(&mut b, self.fork);
        b.extend_from_slice(&self.beacon_state_root);
        u(&mut b, self.slot);
        u(&mut b, self.attested_slot);
        u(&mut b, self.signature_slot);
        u(&mut b, self.participation);
        b.extend_from_slice(&self.genesis_validators_root);
        b.extend_from_slice(&self.sync_committee_root);
        b.extend_from_slice(&self.next_sync_committee_root);
        debug_assert_eq!(b.len(), ANCHOR_LEN);
        b
    }

    /// Parse exactly [`ANCHOR_LEN`] bytes; `None` on any other length.
    pub fn decode_bytes(b: &[u8]) -> Option<Self> {
        if b.len() != ANCHOR_LEN {
            return None;
        }
        let u = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        let r = |o: usize| -> Bytes32 { b[o..o + 32].try_into().unwrap() };
        Some(Self {
            kind: u(0),
            fork: u(8),
            beacon_state_root: r(16),
            slot: u(48),
            attested_slot: u(56),
            signature_slot: u(64),
            participation: u(72),
            genesis_validators_root: r(80),
            sync_committee_root: r(112),
            next_sync_committee_root: r(144),
        })
    }
}
