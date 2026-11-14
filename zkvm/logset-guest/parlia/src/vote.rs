//! Fast finality: a header's vote attestation (`types.VoteAttestation`) and its aggregate BLS
//! signature, over Ethereum's BLS ciphersuite as BNB's validators sign with it (prysm's `bls`).

use alloy_primitives::{keccak256, B256};
use alloy_rlp::{Decodable, Encodable, Header as RlpHeader};
use bls12_381::hash_to_curve::{ExpandMsgXmd, HashToCurve};
use bls12_381::{
    multi_miller_loop, G1Affine, G1Projective, G2Affine, G2Prepared, G2Projective, Gt,
};

/// Longest `Extra` an attestation may carry.
pub const MAX_EXTRA: usize = 256;

const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_POP_";

/// What the validators vote for: the highest justified block (source) and the block to
/// justify (target).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VoteData {
    pub source_number: u64,
    pub source_hash: B256,
    pub target_number: u64,
    pub target_hash: B256,
}

impl VoteData {
    fn rlp(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(84);
        self.source_number.encode(&mut payload);
        self.source_hash.encode(&mut payload);
        self.target_number.encode(&mut payload);
        self.target_hash.encode(&mut payload);
        let mut out = Vec::with_capacity(payload.len() + 2);
        RlpHeader {
            list: true,
            payload_length: payload.len(),
        }
        .encode(&mut out);
        out.extend_from_slice(&payload);
        out
    }

    /// What each validator signs.
    pub fn hash(&self) -> B256 {
        keccak256(self.rlp())
    }
}

/// The votes of 2/3+ of the validators on one [`VoteData`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attestation {
    /// Bit `i`: the `i`-th validator by address voted.
    pub voters: u64,
    pub signature: [u8; 96],
    pub data: VoteData,
    pub extra: Vec<u8>,
}

fn take_list<'a>(buf: &mut &'a [u8]) -> Result<&'a [u8], String> {
    let h = RlpHeader::decode(buf).map_err(|e| e.to_string())?;
    if !h.list || h.payload_length > buf.len() {
        return Err("not a list".into());
    }
    let whole: &'a [u8] = buf;
    let (inner, rest) = whole.split_at(h.payload_length);
    *buf = rest;
    Ok(inner)
}

impl Attestation {
    /// Exactly one RLP `[voters, signature, [source, sourceHash, target, targetHash], extra]`.
    pub fn decode(bytes: &[u8]) -> Result<Attestation, String> {
        let mut buf = bytes;
        let mut p = take_list(&mut buf)?;
        if !buf.is_empty() {
            return Err(format!("{} bytes after it", buf.len()));
        }
        let voters = u64::decode(&mut p).map_err(|e| e.to_string())?;
        let sig = alloy_rlp::Bytes::decode(&mut p).map_err(|e| e.to_string())?;
        let signature: [u8; 96] = sig[..]
            .try_into()
            .map_err(|_| format!("signature is {} bytes", sig.len()))?;
        let mut d = take_list(&mut p)?;
        let data = VoteData {
            source_number: u64::decode(&mut d).map_err(|e| e.to_string())?,
            source_hash: B256::decode(&mut d).map_err(|e| e.to_string())?,
            target_number: u64::decode(&mut d).map_err(|e| e.to_string())?,
            target_hash: B256::decode(&mut d).map_err(|e| e.to_string())?,
        };
        if !d.is_empty() {
            return Err("vote data has more than 4 fields".into());
        }
        let extra = alloy_rlp::Bytes::decode(&mut p)
            .map_err(|e| e.to_string())?
            .to_vec();
        if !p.is_empty() {
            return Err("more than 4 fields".into());
        }
        Ok(Attestation {
            voters,
            signature,
            data,
            extra,
        })
    }
}

/// Whether `signature` is the aggregate of `keys`' signatures over `message` (`FastAggregateVerify`).
pub fn fast_aggregate_verify(keys: &[[u8; 48]], signature: &[u8; 96], message: &B256) -> bool {
    let mut agg = G1Projective::identity();
    for k in keys {
        // Checked decompression: on the curve and in the subgroup.
        let p = G1Affine::from_compressed(k);
        if p.is_none().into() {
            return false;
        }
        let p = p.unwrap();
        if p.is_identity().into() {
            return false;
        }
        agg += G1Projective::from(p);
    }
    let agg = G1Affine::from(agg);
    if keys.is_empty() || bool::from(agg.is_identity()) {
        return false;
    }
    let sig = G2Affine::from_compressed(signature);
    if sig.is_none().into() {
        return false;
    }
    let msg = G2Affine::from(
        <G2Projective as HashToCurve<ExpandMsgXmd<sha2::Sha256>>>::hash_to_curve(
            message.as_slice(),
            DST,
        ),
    );
    // e(-G1, sig) · e(agg, H(m)) == 1.
    let neg_g1 = G1Affine::from(-G1Projective::generator());
    multi_miller_loop(&[
        (&neg_g1, &G2Prepared::from(sig.unwrap())),
        (&agg, &G2Prepared::from(msg)),
    ])
    .final_exponentiation()
        == Gt::identity()
}
