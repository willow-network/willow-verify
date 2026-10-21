//! BLS12-381 fast-aggregate-verify on zkcrypto `bls12_381` — the in-circuit
//! backend (sp1-patched in the guest; plain pure-Rust on the host, so the same
//! path serves the native adversarial tests). Structure mirrors the repo's native
//! blst verifier (`crates/network/src/beacon/bls.rs`) and helios-consensus-core's
//! `types/bls.rs` (aggregate pubkey → hash-to-curve → 2-pairing ate evaluation).

use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, Gt};

use crate::header::{bit_set, SYNC_COMMITTEE_SIZE};
use beacon_cl_core::ssz::Bytes32;

/// Eth2 BLS ciphersuite (min-pubkey-size, proof-of-possession) signature DST.
pub const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// Why the aggregate-signature check failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlsFailure {
    /// A participant's compressed pubkey is not a valid subgroup point.
    /// (`G1Affine::from_compressed` is the CHECKED decompression: on-curve and
    /// torsion-free.)
    InvalidPublicKey(usize),
    /// The 96-byte signature is not a valid G2 subgroup point.
    InvalidSignature,
    /// No participation bits set / the aggregate pubkey is the identity.
    AggregateIsIdentity,
    /// The pairing equation did not hold.
    VerificationFailed,
}

/// Hash `msg` to G2 with the Eth2 DST (SSWU, ExpandMsgXmd over the digest-0.10
/// sha2 line — the patched bls12_381's hash-to-curve API). `pub(crate)` so the
/// tests can sign synthetic messages with the same map.
pub(crate) fn hash_to_g2(msg: &[u8]) -> G2Affine {
    G2Affine::from(<bls12_381::G2Projective as HashToCurve<
        ExpandMsgXmd<sha2_v010::Sha256>,
    >>::hash_to_curve([msg], DST))
}

/// `e(-G1, sig) · e(pk_agg, H(m)) == 1` — helios's ate2 evaluation of
/// `FastAggregateVerify`.
fn ate2_evaluation(sig: &G2Affine, msg_hash: &G2Affine, agg_pk: &G1Affine) -> bool {
    let neg_g1 = G1Affine::from(-G1Projective::generator());
    let pairing = multi_miller_loop(&[
        (&neg_g1, &G2Prepared::from(*sig)),
        (agg_pk, &G2Prepared::from(*msg_hash)),
    ]);
    pairing.final_exponentiation() == Gt::identity()
}

/// Verify a sync-committee aggregate signature over `signing_root`.
///
/// Decompresses ONLY the participants (checked decompression — subgroup
/// membership included), aggregates them in G1, and checks the 2-pairing
/// equation. On success returns the participant count; the CALLER must enforce
/// the supermajority threshold on that count (a single honest signer verifies).
pub fn fast_aggregate_verify(
    committee_pubkeys: &[[u8; 48]; SYNC_COMMITTEE_SIZE],
    participation_bits: &[u8; SYNC_COMMITTEE_SIZE / 8],
    signature: &[u8; 96],
    signing_root: &Bytes32,
) -> Result<usize, BlsFailure> {
    let mut agg = G1Projective::identity();
    let mut count = 0usize;
    for (i, pk_bytes) in committee_pubkeys.iter().enumerate() {
        if !bit_set(participation_bits, i) {
            continue;
        }
        let pk = G1Affine::from_compressed(pk_bytes);
        if pk.is_none().into() {
            return Err(BlsFailure::InvalidPublicKey(i));
        }
        agg += G1Projective::from(pk.unwrap());
        count += 1;
    }
    let agg = G1Affine::from(agg);
    if agg.is_identity().into() {
        return Err(BlsFailure::AggregateIsIdentity);
    }

    let sig = G2Affine::from_compressed(signature);
    if sig.is_none().into() {
        return Err(BlsFailure::InvalidSignature);
    }

    let msg_hash = hash_to_g2(signing_root);
    if !ate2_evaluation(&sig.unwrap(), &msg_hash, &agg) {
        return Err(BlsFailure::VerificationFailed);
    }
    Ok(count)
}
