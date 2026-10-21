//! The sync committee's aggregate signature: `anchor-core`'s check (aggregate the participants'
//! keys, hash the signing root to G2, one two-pairing product), on zkcrypto's crates.io
//! `bls12_381` with sha2 0.9 as Helios (the reference) uses it. The SP1 fork `anchor-core` builds
//! with does not compile for 32-bit targets.

use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt,
};

/// Eth2's signature ciphersuite.
const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlsFailure {
    InvalidPublicKey(usize),
    InvalidSignature,
    AggregateIsIdentity,
    VerificationFailed,
}

/// Verifies `signature` over `signing_root` by the keys whose bit is set; returns how many.
pub fn fast_aggregate_verify(
    pubkeys: &[[u8; 48]],
    bits: &[u8],
    signature: &[u8; 96],
    signing_root: &[u8; 32],
) -> Result<usize, BlsFailure> {
    let mut agg = G1Projective::identity();
    let mut count = 0;
    for (i, pk) in pubkeys.iter().enumerate() {
        if bits[i / 8] & (1 << (i % 8)) == 0 {
            continue;
        }
        // Checked decompression: on the curve and in the subgroup.
        let p = G1Affine::from_compressed(pk);
        if p.is_none().into() {
            return Err(BlsFailure::InvalidPublicKey(i));
        }
        agg += G1Projective::from(p.unwrap());
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
    let msg = G2Affine::from(
        <G2Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(signing_root, DST),
    );
    // e(-G1, sig) · e(agg, H(m)) == 1.
    let neg_g1 = G1Affine::from(-G1Projective::generator());
    let ok = multi_miller_loop(&[
        (&neg_g1, &G2Prepared::from(sig.unwrap())),
        (&agg, &G2Prepared::from(msg)),
    ])
    .final_exponentiation()
        == Gt::identity();
    if ok {
        Ok(count)
    } else {
        Err(BlsFailure::VerificationFailed)
    }
}
