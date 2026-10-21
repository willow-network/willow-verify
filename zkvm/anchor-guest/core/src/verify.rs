//! The whole anchor child: parse the witness blob, verify the sync-committee
//! signature over the attested header, walk the finality branch to the finalized
//! header, (optionally) verify the next-committee handoff branch, and emit the
//! [`AnchorStatement`]. [`run`] is the SAME code the SP1 program executes; every
//! adversarial rejection is a real `Err` from this exact path.

use beacon_cl_core::fork::{depth_for_gindex, verify_branch, ForkVersion};
use beacon_cl_core::ssz::Bytes32;

use crate::bls::{fast_aggregate_verify, BlsFailure};
use crate::gindex::{finalized_root_gindex, next_sync_committee_gindex};
use crate::header::{
    beacon_block_header_root, compute_domain, compute_signing_root, sync_committee_hash_tree_root,
    DOMAIN_SYNC_COMMITTEE, SYNC_COMMITTEE_SIZE,
};
use crate::schedule::{
    fork_at_slot, fork_to_u64, fork_version_bytes, SLOTS_PER_SYNC_COMMITTEE_PERIOD,
};
use crate::wire::{AnchorStatement, ANCHOR_KIND};

/// Supermajority floor: `3 · participants ≥ 2 · 512` (the light-client spec's
/// finality-confirmation threshold), enforced IN circuit — the native verifier
/// deliberately leaves it to the caller; here there is no caller to lean on.
pub const MIN_PARTICIPANTS: usize = SYNC_COMMITTEE_SIZE * 2 / 3 + 1; // 342

/// Why the anchor child rejected a witness blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorError {
    /// The blob ran out of bytes.
    Truncated,
    /// A slot is pre-Capella — out of scope, refused loudly.
    PreCapellaSlot(u64),
    /// Spec ordering: the signature slot must be strictly after the attested slot
    /// (the committee signs a PAST header).
    SignatureSlotNotAfterAttested {
        signature_slot: u64,
        attested_slot: u64,
    },
    /// Spec ordering: the finalized header cannot be after the attested header.
    FinalizedAfterAttested {
        finalized_slot: u64,
        attested_slot: u64,
    },
    /// Handoff attribution: the signature slot must be in the SAME sync-committee
    /// period as the attested slot — the contract attributes the handoff to
    /// `period(signature_slot)+1`, but it is proven under the ATTESTED state.
    SignatureNotInAttestedPeriod {
        signature_slot: u64,
        attested_slot: u64,
    },
    /// Fewer than [`MIN_PARTICIPANTS`] participation bits set.
    LowParticipation { got: usize, need: usize },
    /// A participant's compressed pubkey failed checked decompression.
    InvalidPublicKey(usize),
    /// The signature bytes failed checked decompression.
    InvalidSignature,
    /// All-zero participation (aggregate pubkey is the identity).
    AggregateIsIdentity,
    /// The BLS pairing equation did not hold for the recomputed signing root.
    SignatureVerificationFailed,
    /// The finality branch does not commit the recomputed finalized header root
    /// under the attested `state_root` at the derived gindex.
    FinalityBranchInvalid,
    /// The next-committee branch does not commit the witnessed next committee
    /// under the attested `state_root` at the derived gindex.
    NextCommitteeBranchInvalid,
}

impl From<BlsFailure> for AnchorError {
    fn from(f: BlsFailure) -> Self {
        match f {
            BlsFailure::InvalidPublicKey(i) => AnchorError::InvalidPublicKey(i),
            BlsFailure::InvalidSignature => AnchorError::InvalidSignature,
            BlsFailure::AggregateIsIdentity => AnchorError::AggregateIsIdentity,
            BlsFailure::VerificationFailed => AnchorError::SignatureVerificationFailed,
        }
    }
}

/// The five witnessed fields of a `BeaconBlockHeader`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderFields {
    pub slot: u64,
    pub proposer_index: u64,
    pub parent_root: Bytes32,
    pub state_root: Bytes32,
    pub body_root: Bytes32,
}

impl HeaderFields {
    pub fn root(&self) -> Bytes32 {
        beacon_block_header_root(
            self.slot,
            self.proposer_index,
            &self.parent_root,
            &self.state_root,
            &self.body_root,
        )
    }
}

/// A witnessed sync committee: 512 compressed pubkeys + the aggregate pubkey.
#[derive(Clone)]
pub struct CommitteeWitness {
    pub pubkeys: Box<[[u8; 48]; SYNC_COMMITTEE_SIZE]>,
    pub aggregate_pubkey: [u8; 48],
}

impl CommitteeWitness {
    pub fn root(&self) -> Bytes32 {
        sync_committee_hash_tree_root(&self.pubkeys, &self.aggregate_pubkey)
    }
}

/// The parsed anchor witness — see [`decode_blob`] for the wire layout.
pub struct AnchorWitness {
    pub attested: HeaderFields,
    pub finalized: HeaderFields,
    pub signature_slot: u64,
    pub genesis_validators_root: Bytes32,
    pub participation_bits: [u8; SYNC_COMMITTEE_SIZE / 8],
    pub signature: [u8; 96],
    pub committee: CommitteeWitness,
    pub finality_branch: Vec<Bytes32>,
    /// The optional period-handoff section: the NEXT committee + its branch under
    /// the attested state.
    pub next_committee: Option<(CommitteeWitness, Vec<Bytes32>)>,
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], AnchorError> {
        if self.i + n > self.b.len() {
            return Err(AnchorError::Truncated);
        }
        let s = &self.b[self.i..self.i + n];
        self.i += n;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, AnchorError> {
        Ok(self.take(1)?[0])
    }
    fn u64(&mut self) -> Result<u64, AnchorError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn b32(&mut self) -> Result<Bytes32, AnchorError> {
        Ok(self.take(32)?.try_into().unwrap())
    }
    fn b48(&mut self) -> Result<[u8; 48], AnchorError> {
        Ok(self.take(48)?.try_into().unwrap())
    }
    fn header(&mut self) -> Result<HeaderFields, AnchorError> {
        Ok(HeaderFields {
            slot: self.u64()?,
            proposer_index: self.u64()?,
            parent_root: self.b32()?,
            state_root: self.b32()?,
            body_root: self.b32()?,
        })
    }
    fn committee(&mut self) -> Result<CommitteeWitness, AnchorError> {
        let mut pubkeys = Box::new([[0u8; 48]; SYNC_COMMITTEE_SIZE]);
        for pk in pubkeys.iter_mut() {
            *pk = self.b48()?;
        }
        let aggregate_pubkey = self.b48()?;
        Ok(CommitteeWitness {
            pubkeys,
            aggregate_pubkey,
        })
    }
    fn branch(&mut self, depth: usize) -> Result<Vec<Bytes32>, AnchorError> {
        let mut v = Vec::with_capacity(depth);
        for _ in 0..depth {
            v.push(self.b32()?);
        }
        Ok(v)
    }
}

/// Parse the witness blob. Branch lengths are DERIVED from the fork the attested
/// slot implies — a wrong-depth branch has no wire form. Layout (little-endian):
///
/// ```text
/// attested  header      5 fields (u64, u64, b32, b32, b32)
/// finalized header      5 fields
/// signature_slot        u64
/// genesis_validators_root  b32
/// participation_bits    64 bytes (SSZ Bitvector[512])
/// signature             96 bytes
/// committee             512 × b48 pubkeys + b48 aggregate_pubkey
/// finality_branch       depth(finalized_root_gindex(fork_att)) × b32
/// next_flag             u8; if 1:
///   next committee      512 × b48 + b48
///   next_branch         depth(next_sync_committee_gindex(fork_att)) × b32
/// ```
pub fn decode_blob(blob: &[u8]) -> Result<(AnchorWitness, ForkVersion), AnchorError> {
    let mut r = Reader { b: blob, i: 0 };
    let attested = r.header()?;
    let finalized = r.header()?;
    let signature_slot = r.u64()?;
    let genesis_validators_root = r.b32()?;
    let participation_bits: [u8; SYNC_COMMITTEE_SIZE / 8] =
        r.take(SYNC_COMMITTEE_SIZE / 8)?.try_into().unwrap();
    let signature: [u8; 96] = r.take(96)?.try_into().unwrap();
    let committee = r.committee()?;
    let fork_att = fork_at_slot(attested.slot).ok_or(AnchorError::PreCapellaSlot(attested.slot))?;
    let finality_branch = r.branch(depth_for_gindex(finalized_root_gindex(fork_att)))?;
    let next_committee = if r.u8()? == 1 {
        let next = r.committee()?;
        let branch = r.branch(depth_for_gindex(next_sync_committee_gindex(fork_att)))?;
        Some((next, branch))
    } else {
        None
    };
    Ok((
        AnchorWitness {
            attested,
            finalized,
            signature_slot,
            genesis_validators_root,
            participation_bits,
            signature,
            committee,
            finality_branch,
            next_committee,
        },
        fork_att,
    ))
}

/// The whole anchor child over a parsed witness. Split from [`run`] so tests can
/// mutate structured witnesses without re-serializing.
pub fn verify_witness(
    w: &AnchorWitness,
    fork_att: ForkVersion,
) -> Result<AnchorStatement, AnchorError> {
    // --- slot ordering (spec: committee signs a PAST header; finality is behind it) ---
    if w.signature_slot <= w.attested.slot {
        return Err(AnchorError::SignatureSlotNotAfterAttested {
            signature_slot: w.signature_slot,
            attested_slot: w.attested.slot,
        });
    }
    if w.finalized.slot > w.attested.slot {
        return Err(AnchorError::FinalizedAfterAttested {
            finalized_slot: w.finalized.slot,
            attested_slot: w.attested.slot,
        });
    }
    // Same period: the handoff is proven under the ATTESTED state (period
    // `attested/8192 + 1`'s committee) but attributed on chain to period
    // `signature_slot/8192 + 1` — a boundary-straddling signature would shift
    // the attribution off by one.
    if w.signature_slot / SLOTS_PER_SYNC_COMMITTEE_PERIOD
        != w.attested.slot / SLOTS_PER_SYNC_COMMITTEE_PERIOD
    {
        return Err(AnchorError::SignatureNotInAttestedPeriod {
            signature_slot: w.signature_slot,
            attested_slot: w.attested.slot,
        });
    }
    let fork_fin =
        fork_at_slot(w.finalized.slot).ok_or(AnchorError::PreCapellaSlot(w.finalized.slot))?;
    // Domain fork: the version at `signature_slot − 1`, NOT the attested slot —
    // the two can straddle a fork boundary (helios `saturating_sub(1)`; the
    // native bls.rs documents the same).
    let fork_sig = fork_at_slot(w.signature_slot.saturating_sub(1)).ok_or(
        AnchorError::PreCapellaSlot(w.signature_slot.saturating_sub(1)),
    )?;

    // --- supermajority, counted from the bitfield BEFORE any curve work ---
    let participation = (0..SYNC_COMMITTEE_SIZE)
        .filter(|&i| crate::header::bit_set(&w.participation_bits, i))
        .count();
    if participation < MIN_PARTICIPANTS {
        return Err(AnchorError::LowParticipation {
            got: participation,
            need: MIN_PARTICIPANTS,
        });
    }

    // --- the signature: committee → aggregate pubkey → signing root → pairing ---
    let attested_root = w.attested.root();
    let domain = compute_domain(
        DOMAIN_SYNC_COMMITTEE,
        fork_version_bytes(fork_sig),
        &w.genesis_validators_root,
    );
    let signing_root = compute_signing_root(&attested_root, &domain);
    let verified = fast_aggregate_verify(
        &w.committee.pubkeys,
        &w.participation_bits,
        &w.signature,
        &signing_root,
    )?;
    debug_assert_eq!(verified, participation);

    // --- finality: the finalized header root under the attested state ---
    let finalized_root = w.finalized.root();
    let fin_gindex = finalized_root_gindex(fork_att);
    if !verify_branch(
        &finalized_root,
        &w.finality_branch,
        fin_gindex,
        &w.attested.state_root,
    ) {
        return Err(AnchorError::FinalityBranchInvalid);
    }

    // --- optional period handoff: the NEXT committee under the attested state ---
    let next_sync_committee_root = match &w.next_committee {
        Some((next, branch)) => {
            let next_root = next.root();
            if !verify_branch(
                &next_root,
                branch,
                next_sync_committee_gindex(fork_att),
                &w.attested.state_root,
            ) {
                return Err(AnchorError::NextCommitteeBranchInvalid);
            }
            next_root
        }
        None => [0u8; 32],
    };

    Ok(AnchorStatement {
        kind: ANCHOR_KIND,
        fork: fork_to_u64(fork_fin),
        beacon_state_root: w.finalized.state_root,
        slot: w.finalized.slot,
        attested_slot: w.attested.slot,
        signature_slot: w.signature_slot,
        participation: participation as u64,
        genesis_validators_root: w.genesis_validators_root,
        sync_committee_root: w.committee.root(),
        next_sync_committee_root,
    })
}

/// The whole guest: parse + verify. The SP1 program commits `encode()` of the
/// returned statement.
pub fn run(blob: &[u8]) -> Result<AnchorStatement, AnchorError> {
    let (w, fork_att) = decode_blob(blob)?;
    verify_witness(&w, fork_att)
}

/// Serialize a witness to the exact blob [`decode_blob`] reads (the assembler /
/// genblob / roundtrip-test side).
pub fn encode_blob(w: &AnchorWitness) -> Vec<u8> {
    let mut b = Vec::new();
    let u = |b: &mut Vec<u8>, v: u64| b.extend_from_slice(&v.to_le_bytes());
    let header = |b: &mut Vec<u8>, h: &HeaderFields| {
        u(b, h.slot);
        u(b, h.proposer_index);
        b.extend_from_slice(&h.parent_root);
        b.extend_from_slice(&h.state_root);
        b.extend_from_slice(&h.body_root);
    };
    let committee = |b: &mut Vec<u8>, c: &CommitteeWitness| {
        for pk in c.pubkeys.iter() {
            b.extend_from_slice(pk);
        }
        b.extend_from_slice(&c.aggregate_pubkey);
    };
    header(&mut b, &w.attested);
    header(&mut b, &w.finalized);
    u(&mut b, w.signature_slot);
    b.extend_from_slice(&w.genesis_validators_root);
    b.extend_from_slice(&w.participation_bits);
    b.extend_from_slice(&w.signature);
    committee(&mut b, &w.committee);
    for n in &w.finality_branch {
        b.extend_from_slice(n);
    }
    match &w.next_committee {
        Some((next, branch)) => {
            b.push(1);
            committee(&mut b, next);
            for n in branch {
                b.extend_from_slice(n);
            }
        }
        None => b.push(0),
    }
    b
}
