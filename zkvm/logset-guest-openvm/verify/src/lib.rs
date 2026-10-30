//! The OpenVM side of a log query check: [`OvmCheck`] verifies a stored OpenVM STARK proof
//! under a pinned program. SP1 proofs are `willow-logs-verify`'s `Sp1Check`, in another
//! workspace (see Cargo.toml); [`logset_query::ProofSystem`] tells which one a proof needs.
//!
//! An OpenVM program is pinned by its verifying key: the internal-recursive layer's key, which
//! every program shares, plus the program's baseline (its exe commit, the VM's verifier-key
//! commits and its memory layout). Its 32-byte program key, the one an L1 commits for its
//! leaves and a client lists in [`Trust`](logset_query::Trust), is the baseline's exe commit
//! as eight BabyBear limbs, each a canonical big-endian u32: the bytes agg-core commits from the
//! `[u32; 8]` the OpenVM L1 checks each child's exe commit against.
//!
//! Without the `verifier` feature only [`pinned`]'s minted keys build: the SP1 side's
//! [`Trust`](logset_query::Trust) takes its OpenVM keys from there, with no OpenVM dependency.

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

/// The internal-recursive layer's verifying key (`cargo openvm setup`'s `internal_recursive.vk`).
#[cfg(feature = "verifier")]
pub type AggVk = MultiStarkVerifyingKey<BabyBearPoseidon2Config>;

/// A program's 32-byte key: its exe commit, limb by limb, big-endian.
#[cfg(feature = "verifier")]
pub fn program_key(baseline: &VerificationBaseline) -> [u8; 32] {
    let mut key = [0u8; 32];
    for (i, limb) in baseline.app_exe_commit.iter().enumerate() {
        key[4 * i..4 * i + 4].copy_from_slice(&limb.as_canonical_u32().to_be_bytes());
    }
    key
}

#[cfg(feature = "verifier")]
/// Verifies OpenVM `.proof`s (magic, then zstd of `VmStarkProof`) under pinned programs.
#[derive(Default)]
pub struct OvmCheck {
    programs: Vec<([u8; 32], VmStarkVerifyingKey)>,
}

#[cfg(feature = "verifier")]
impl OvmCheck {
    /// One entry per program, all under the shared internal-recursive key.
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

    /// The program keys this check can verify under.
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
        // Checks the proof, its program and VM against the baseline, and the user public
        // values against the final memory root. What those values say is checked below.
        verify_vm_stark_proof_decoded(vk, &proof).map_err(|e| format!("{e:?}"))?;
        // The guest reveals sha256(pv), one byte per field element. An element over 255 is not
        // a byte the guest wrote, so it is refused rather than truncated.
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

    // `tests/fixtures/openvm`: `echo-guest.rs` (reveals sha256 of its input) proven with
    // `cargo openvm prove stark` v2.0.2 on input `echo.pv`; `agg.vk` is that setup's
    // internal-recursive key, `echo.baseline` the program's baseline (bitcode), and
    // `echo.proof` the proof as the tip worker stores it.
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
