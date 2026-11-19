




use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt,
};


const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlsFailure {
    InvalidPublicKey(usize),
    InvalidSignature,
    AggregateIsIdentity,
    VerificationFailed,
}


pub fn public_key(pk: &[u8; 48]) -> Option<G1Affine> {
    Option::<G1Affine>::from(G1Affine::from_compressed(pk)).filter(|p| !bool::from(p.is_identity()))
}


pub fn aggregate_verify_points(
    keys: &[G1Affine],
    signature: &[u8; 96],
    message: &[u8; 32],
) -> Result<(), BlsFailure> {
    aggregate_verify_with(keys, signature, message, DST)
}


pub fn aggregate_verify_with(
    keys: &[G1Affine],
    signature: &[u8; 96],
    message: &[u8],
    dst: &[u8],
) -> Result<(), BlsFailure> {
    let agg = G1Affine::from(
        keys.iter()
            .fold(G1Projective::identity(), |a, k| a + G1Projective::from(k)),
    );
    if keys.is_empty() || bool::from(agg.is_identity()) {
        return Err(BlsFailure::AggregateIsIdentity);
    }
    let sig = Option::<G2Affine>::from(G2Affine::from_compressed(signature))
        .ok_or(BlsFailure::InvalidSignature)?;
    let msg = G2Affine::from(
        <G2Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(message, dst),
    );
    let neg_g1 = G1Affine::from(-G1Projective::generator());
    let ok = multi_miller_loop(&[
        (&neg_g1, &G2Prepared::from(sig)),
        (&agg, &G2Prepared::from(msg)),
    ])
    .final_exponentiation()
        == Gt::identity();
    ok.then_some(()).ok_or(BlsFailure::VerificationFailed)
}


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
