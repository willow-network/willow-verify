//! The SP1 side of a log query check: [`Sp1Check`] verifies a stored compressed
//! proof under a pinned program key. Everything else is `logset_query::verify`.

use logset_query::{ProofCheck, Trust};
use slop_algebra::AbstractField;
use sp1_primitives::SP1Field;
use sp1_verifier::compressed::SP1CompressedVerifier;
use sp1_verifier::SP1Proof;

/// The L1 index aggregator and leaf program keys minted for SP1 6.8.1 (`PINS/MINT.txt`).
pub const SEGMENT_VKEY: &str = include_str!("../../vkeys/logset-idxagg-program.koalabear");
pub const LEAF_VKEY: &str = include_str!("../../vkeys/logset-program.koalabear");

/// A program key from its `.koalabear` text (64 hex digits).
pub fn parse_vkey(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    let mut out = [0u8; 32];
    hex::decode_to_slice(s.strip_prefix("0x").unwrap_or(s), &mut out)
        .map_err(|e| format!("program key {s:?}: {e}"))?;
    Ok(out)
}

/// The chains the pinned keys are for. One leaf program and one L1 program serve
/// all of them: the leaf gates receipt types by the statement's own chain id
/// (`core/src/receipt.rs`) and the L1 reads each child's span off the child.
pub const PINNED_CHAINS: [(u64, &str); 7] = [
    (1, "ethereum"),
    (8453, "base"),
    (10, "op"),
    (130, "unichain"),
    (57073, "ink"),
    (42161, "arbitrum-one"),
    (4663, "robinhood"),
];

/// The name of a chain with pinned keys.
pub fn pinned_chain(chain_id: u64) -> Option<&'static str> {
    PINNED_CHAINS
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, name)| *name)
}

/// Trust in the pinned program keys on `chain_id`; refused for a chain they are not for.
pub fn pinned_trust(chain_id: u64) -> Result<Trust, String> {
    if pinned_chain(chain_id).is_none() {
        return Err(format!(
            "no pinned program keys for chain {chain_id}; pass both the segment and the leaf keys"
        ));
    }
    Ok(Trust {
        chain_id,
        segment_vkeys: vec![parse_vkey(SEGMENT_VKEY).expect("pinned key")],
        leaf_vkeys: vec![parse_vkey(LEAF_VKEY).expect("pinned key")],
    })
}

/// Trust on `chain_id`: the pinned keys, each tier replaced by its override when
/// one is given. A chain with no pinned keys needs both overrides.
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
            segment_vkeys: vec![],
            leaf_vkeys: vec![],
        }
    };
    if !segment_vkeys.is_empty() {
        t.segment_vkeys = segment_vkeys;
    }
    if !leaf_vkeys.is_empty() {
        t.leaf_vkeys = leaf_vkeys;
    }
    Ok(t)
}

/// Verifies compressed SP1 proofs as the segment tier stores them: the bincode
/// of `SP1ProofWithPublicValues`, whose leading field is the `SP1Proof`.
pub struct Sp1Check(SP1CompressedVerifier);

impl Default for Sp1Check {
    fn default() -> Self {
        Self(SP1CompressedVerifier::new())
    }
}

impl ProofCheck for Sp1Check {
    fn check(&mut self, vkey: &[u8; 32], proof: &[u8], public_values: &[u8]) -> Result<(), String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_served_chain_has_the_minted_keys_and_others_need_their_own() {
        let leaf = parse_vkey(LEAF_VKEY).unwrap();
        let seg = parse_vkey(SEGMENT_VKEY).unwrap();
        assert_eq!(hex::encode(&leaf[..4]), "687c1233");
        assert_eq!(hex::encode(&seg[..4]), "0b5dffe0");
        for (id, _) in PINNED_CHAINS {
            let t = pinned_trust(id).unwrap();
            assert_eq!(
                (t.chain_id, t.segment_vkeys, t.leaf_vkeys),
                (id, vec![seg], vec![leaf])
            );
        }
        assert_eq!(pinned_chain(42161), Some("arbitrum-one"));
        assert!(pinned_trust(56).is_err());
        assert!(trust_with(56, vec![[1; 32]], vec![]).is_err());
        let t = trust_with(56, vec![[1; 32]], vec![[2; 32]]).unwrap();
        assert_eq!(
            (t.segment_vkeys, t.leaf_vkeys),
            (vec![[1; 32]], vec![[2; 32]])
        );
        let t = trust_with(4663, vec![], vec![[2; 32]]).unwrap();
        assert_eq!((t.segment_vkeys, t.leaf_vkeys), (vec![seg], vec![[2; 32]]));
    }
}
