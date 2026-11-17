






use logset_query::{Keys, ProofCheck, Trust};
use slop_algebra::AbstractField;
use sp1_primitives::SP1Field;
use sp1_verifier::compressed::SP1CompressedVerifier;
use sp1_verifier::SP1Proof;

#[cfg(feature = "cli")]
mod openvm;
#[cfg(feature = "cli")]
pub use openvm::OvmProcess;


pub const SEGMENT_VKEY: &str = include_str!("../../vkeys/logset-idxagg-program.koalabear");
pub const LEAF_VKEY: &str = include_str!("../../vkeys/logset-program.koalabear");


pub fn sp1_keys() -> Keys {
    Keys {
        segment: vec![parse_vkey(SEGMENT_VKEY).expect("pinned key")],
        leaf: vec![parse_vkey(LEAF_VKEY).expect("pinned key")],
    }
}





pub fn openvm_keys() -> Keys {
    willow_logs_verify_openvm::pinned::client_keys()
        .expect("pins/program-keys.txt has the minted keys")
}








pub const OPENVM_FROM: &[(u64, u64)] = &[(1, 26_361_856)];


pub fn openvm_from(chain_id: u64) -> Option<u64> {
    OPENVM_FROM
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, b)| *b)
}


pub fn parse_vkey(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    let mut out = [0u8; 32];
    hex::decode_to_slice(s.strip_prefix("0x").unwrap_or(s), &mut out)
        .map_err(|e| format!("program key {s:?}: {e}"))?;
    Ok(out)
}




pub const PINNED_CHAINS: [(u64, &str); 7] = [
    (1, "ethereum"),
    (8453, "base"),
    (10, "op"),
    (130, "unichain"),
    (57073, "ink"),
    (42161, "arbitrum-one"),
    (4663, "robinhood"),
];


pub fn pinned_chain(chain_id: u64) -> Option<&'static str> {
    PINNED_CHAINS
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, name)| *name)
}



pub fn pinned_trust(chain_id: u64) -> Result<Trust, String> {
    if pinned_chain(chain_id).is_none() {
        return Err(format!(
            "no pinned program keys for chain {chain_id}; pass both the segment and the leaf keys"
        ));
    }
    Ok(Trust {
        chain_id,
        sp1: sp1_keys(),
        openvm: openvm_keys(),
        openvm_from: openvm_from(chain_id),
    })
}





pub fn trust_with(
    chain_id: u64,
    segment_vkeys: Vec<[u8; 32]>,
    leaf_vkeys: Vec<[u8; 32]>,
) -> Result<Trust, String> {
    let mut t = if segment_vkeys.is_empty() || leaf_vkeys.is_empty() {
        pinned_trust(chain_id)?
    } else {
        Trust {
            chain_id,
            sp1: Keys::default(),
            openvm: Keys::default(),
            openvm_from: None,
        }
    };
    if !segment_vkeys.is_empty() {
        t.sp1.segment = segment_vkeys;
    }
    if !leaf_vkeys.is_empty() {
        t.sp1.leaf = leaf_vkeys;
    }
    Ok(t)
}



pub struct Sp1Check(SP1CompressedVerifier);

impl Default for Sp1Check {
    fn default() -> Self {
        Self(SP1CompressedVerifier::new())
    }
}

impl ProofCheck for Sp1Check {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
        if logset_query::ProofSystem::of(proof) != Some(logset_query::ProofSystem::Sp1) {
            return Err("not an SP1 proof".into());
        }
        let proof: SP1Proof =
            bincode::deserialize(proof).map_err(|e| format!("proof bytes: {e}"))?;
        let SP1Proof::Compressed(proof) = proof else {
            return Err("not a compressed proof".into());
        };
        let words: [SP1Field; 8] = std::array::from_fn(|i| {
            SP1Field::from_canonical_u32(u32::from_be_bytes(
                vkey[4 * i..4 * i + 4].try_into().unwrap(),
            ))
        });
        self.0
            .verify_compressed_with_public_values(&proof, public_values, &words)
            .map_err(|e| e.to_string())
    }
}





























































































































































