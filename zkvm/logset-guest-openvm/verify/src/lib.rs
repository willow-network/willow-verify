













pub mod pinned;
#[cfg(feature = "verifier")]
pub mod serve;

#[cfg(feature = "verifier")]
use logset_query::{openvm_body, ProofCheck};
#[cfg(feature = "verifier")]
use openvm_stark_backend::{
    codec::Decode, keygen::types::MultiStarkVerifyingKey, p3_field::PrimeField32,
};
#[cfg(feature = "verifier")]
use openvm_stark_sdk::config::baby_bear_poseidon2::BabyBearPoseidon2Config;
#[cfg(feature = "verifier")]
use openvm_verify_stark_host::{
    verify_vm_stark_proof_decoded,
    vk::{VerificationBaseline, VmStarkVerifyingKey},
    VmStarkProof,
};
#[cfg(feature = "verifier")]
use sha2::{Digest, Sha256};


#[cfg(feature = "verifier")]
pub type AggVk = MultiStarkVerifyingKey<BabyBearPoseidon2Config>;


#[cfg(feature = "verifier")]
pub fn program_key(baseline: &VerificationBaseline) -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, limb) in baseline.app_exe_commit.iter().enumerate() {
        key[4 * i..4 * i + 4].copy_from_slice(&limb.as_canonical_u32().to_be_bytes());
    }
    key
}

#[cfg(feature = "verifier")]

#[derive(Default)]
pub struct OvmCheck {
    programs: Vec<([u8; 32], VmStarkVerifyingKey)>,
}

#[cfg(feature = "verifier")]
impl OvmCheck {

    pub fn new(agg_vk: &AggVk, baselines: impl IntoIterator<Item = VerificationBaseline>) -> Self {
        let programs = baselines
            .into_iter()
            .map(|baseline| {
                let key = program_key(&baseline);
                let vk = VmStarkVerifyingKey {
                    mvk: agg_vk.clone(),
                    baseline,
                };
                (key, vk)
            })
            .collect();
        Self { programs }
    }


    pub fn keys(&self) -> Vec<[u8; 32]> {
        self.programs.iter().map(|(k, _)| *k).collect()
    }
}

#[cfg(feature = "verifier")]
impl ProofCheck for OvmCheck {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        let body = openvm_body(proof).ok_or("not an OpenVM proof")?;
        let (_, vk) = self
            .programs
            .iter()
            .find(|(k, _)| k == vkey)
            .ok_or_else(|| format!("no OpenVM program with key 0x{}", hex::encode(vkey)))?;
        let raw = zstd::decode_all(body).map_err(|e| format!("proof bytes: {e}"))?;
        let proof = VmStarkProof::decode_from_bytes(&raw).map_err(|e| format!("proof bytes: {e}"))?;


        verify_vm_stark_proof_decoded(vk, &proof).map_err(|e| format!("{e:?}"))?;


        let user = &proof.user_pvs_proof.public_values;
        let mut got = Vec::with_capacity(user.len());
        for f in user {
            let v = f.as_canonical_u32();
            if v > 0xff {
                return Err(format!("public value {v} is not a byte"));
            }
            got.push(v as u8);
        }
        if got.as_slice() != Sha256::digest(public_values).as_slice() {
            return Err("proof is for other public values".into());
        }
        Ok(())
    }
}

#[cfg(all(test, feature = "verifier"))]
mod tests {
    use super::*;





    const AGG_VK: &[u8] = include_bytes!("../tests/fixtures/openvm/agg.vk");
    const BASELINE: &[u8] = include_bytes!("../tests/fixtures/openvm/echo.baseline");
    const PROOF: &[u8] = include_bytes!("../tests/fixtures/openvm/echo.proof");
    const PV: &[u8] = include_bytes!("../tests/fixtures/openvm/echo.pv");

    fn fixture() -> (OvmCheck, [u8; 32]) {
        let agg: AggVk = bitcode::deserialize(AGG_VK).unwrap();
        let baseline: VerificationBaseline = bitcode::deserialize(BASELINE).unwrap();
        let key = program_key(&baseline);
        (OvmCheck::new(&agg, [baseline]), key)
    }

    #[test]
    fn verifies_the_proof_of_exactly_its_public_values() {
        let (mut c, key) = fixture();
        assert_eq!(c.keys(), vec![key]);
        c.check(&key, PROOF, PV).unwrap();
        let mut other = PV.to_vec();
        other[0] ^= 1;
        assert_eq!(c.check(&key, PROOF, &other).unwrap_err(), "proof is for other public values");
    }

    #[test]
    fn refuses_a_changed_proof_another_program_and_non_openvm_bytes() {
        let (mut c, key) = fixture();
        let raw = zstd::decode_all(openvm_body(PROOF).unwrap()).unwrap();
        for at in [raw.len() / 2, raw.len() - 40] {
            let mut bad = raw.clone();
            bad[at] ^= 1;
            let stored = [&logset_query::OPENVM_PROOF_MAGIC[..], &zstd::encode_all(&bad[..], 0).unwrap()].concat();
            assert!(c.check(&key, &stored, PV).is_err(), "byte {at} changed and still verified");
        }
        let mut other = key;
        other[31] ^= 1;
        assert!(c.check(&other, PROOF, PV).unwrap_err().starts_with("no OpenVM program"));
        assert_eq!(c.check(&key, &PROOF[4..], PV).unwrap_err(), "not an OpenVM proof");
        assert_eq!(logset_query::ProofSystem::of(PROOF), Some(logset_query::ProofSystem::OpenVm));
    }

    #[test]
    fn the_program_key_is_the_exe_commit_limbs_big_endian() {
        let baseline: VerificationBaseline = bitcode::deserialize(BASELINE).unwrap();
        let key = program_key(&baseline);
        for (i, limb) in baseline.app_exe_commit.iter().enumerate() {
            assert_eq!(&key[4 * i..4 * i + 4], &limb.as_canonical_u32().to_be_bytes());
        }
    }
}
