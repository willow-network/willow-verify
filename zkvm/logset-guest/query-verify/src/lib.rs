//! The SP1 side of a log query check: [`Sp1Check`] verifies a stored compressed
//! proof under a pinned program key, and [`pinned_trust`] holds both systems'
//! pinned keys. OpenVM proofs are verified by `willow-logs-verify-openvm`, which
//! cannot share this crate's dependency graph: the CLI runs it ([`OvmProcess`]),
//! the browser loads it as a second wasm module. Everything else is
//! `logset_query::verify`.

use logset_query::{Keys, ProofCheck, Trust};
use slop_algebra::AbstractField;
use sp1_primitives::SP1Field;
use sp1_verifier::compressed::SP1CompressedVerifier;
use sp1_verifier::SP1Proof;

#[cfg(feature = "cli")]
mod openvm;
#[cfg(feature = "cli")]
pub use openvm::OvmProcess;

/// The L1 index aggregator and leaf program keys minted for SP1 6.8.1 (`PINS/MINT.txt`).
pub const SEGMENT_VKEY: &str = include_str!("../../vkeys/logset-idxagg-program.koalabear");
pub const LEAF_VKEY: &str = include_str!("../../vkeys/logset-program.koalabear");

/// The pinned SP1 keys: the L1 and the leaf above.
pub fn sp1_keys() -> Keys {
    Keys {
        segment: vec![parse_vkey(SEGMENT_VKEY).expect("pinned key")],
        leaf: vec![parse_vkey(LEAF_VKEY).expect("pinned key")],
    }
}

/// The pinned OpenVM keys: the leaf and L1 of the canonical OpenVM 2.0.2 mint, read from
/// `zkvm/logset-guest-openvm/pins/program-keys.txt` by `willow_logs_verify_openvm::pinned`, the one
/// place OpenVM trust comes from. The OpenVM verifier checks each proof under its pinned
/// verifying keys for exactly these.
pub fn openvm_keys() -> Keys {
    willow_logs_verify_openvm::pinned::client_keys()
        .expect("pins/program-keys.txt has the minted keys")
}

/// Each pinned chain's first OpenVM block ([`Trust::openvm_from`]), once its cutover F is set.
/// Ethereum's entry is F itself (a block number, a 64-block window boundary): 26,361,856, the
/// `fromBlock` of the OpenVM entry proposed on `LogSetVerifierV2` on 2026-10-29 and the runner's
/// F. An L2's F is a timestamp (1,794,038,400, 2026-11-07 08:00 UTC, in all six 2026-10-29 L2
/// registries), and its entry is the first block whose timestamp is at or after F, so it can be
/// added only once that block exists: the L2s are pinned after 2026-11-07. Until a chain has an
/// entry, an answer may move from SP1 to OpenVM once, at any block, and the report says where.
pub const OPENVM_FROM: &[(u64, u64)] = &[(1, 26_361_856)];

/// The first OpenVM block of `chain_id`, if pinned.
pub fn openvm_from(chain_id: u64) -> Option<u64> {
    OPENVM_FROM
        .iter()
        .find(|(id, _)| *id == chain_id)
        .map(|(_, b)| *b)
}

/// A program key from its `.koalabear` text (64 hex digits).
pub fn parse_vkey(s: &str) -> Result<[u8; 32], String> {
    let s = s.trim();
    let mut out = [0u8; 32];
    hex::decode_to_slice(s.strip_prefix("0x").unwrap_or(s), &mut out)
        .map_err(|e| format!("program key {s:?}: {e}"))?;
    Ok(out)
}

/// The chains the pinned keys are for. In each system one leaf program and one L1
/// program serve all of them: the leaf gates receipt types by the statement's own
/// chain id (`core/src/receipt.rs`) and the L1 reads each child's span off the child.
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

/// Trust in both systems' pinned program keys on `chain_id`, with its pinned first
/// OpenVM block if any; refused for a chain they are not for.
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

/// Trust on `chain_id`: the pinned keys, each SP1 tier replaced by its override when
/// one is given. A chain with no pinned keys needs both overrides, and then trusts no
/// OpenVM program: an OpenVM program is pinned by its full verifying key, which only
/// the pinned ones have here.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_served_chain_has_the_minted_keys_and_others_need_their_own() {
        let leaf = parse_vkey(LEAF_VKEY).unwrap();
        let seg = parse_vkey(SEGMENT_VKEY).unwrap();
        assert_eq!(hex::encode(&leaf[..4]), "687c1233");
        assert_eq!(hex::encode(&seg[..4]), "0b5dffe0");
        // The OpenVM mint's leaf and L1 (pins/program-keys.txt), never SP1's.
        let ovm = openvm_keys();
        assert_eq!(
            hex::encode(ovm.leaf[0]),
            "28b819642cb4cbd26da3b3762408c71656423fd4267bb71e12a0c76e08490119"
        );
        assert_eq!(
            hex::encode(ovm.segment[0]),
            "3b585e4468e56c4e4ee629bb67c85e26328301473b4e0d611dde318e47585789"
        );
        for (id, name) in PINNED_CHAINS {
            let t = pinned_trust(id).unwrap();
            assert_eq!(t.chain_id, id);
            assert_eq!(
                t.sp1,
                Keys {
                    segment: vec![seg],
                    leaf: vec![leaf]
                }
            );
            assert_eq!(t.openvm, ovm);
            // Only Ethereum's cutover is pinned (V2's OpenVM entry from 26,361,856); the L2s'
            // first blocks at or after F_L2 do not exist yet.
            let want = if id == 1 { Some(26_361_856) } else { None };
            assert_eq!(t.openvm_from, want, "{name}");
        }
        assert_eq!(pinned_chain(42161), Some("arbitrum-one"));
        assert!(pinned_trust(56).is_err());
        assert!(trust_with(56, vec![[1; 32]], vec![]).is_err());
        let t = trust_with(56, vec![[1; 32]], vec![[2; 32]]).unwrap();
        assert_eq!((t.sp1.segment, t.sp1.leaf), (vec![[1; 32]], vec![[2; 32]]));
        assert_eq!(t.openvm, Keys::default());
        // An override replaces SP1's keys only; OpenVM's stay pinned.
        let t = trust_with(4663, vec![], vec![[2; 32]]).unwrap();
        assert_eq!((t.sp1.segment, t.sp1.leaf), (vec![seg], vec![[2; 32]]));
        assert_eq!(t.openvm, ovm);
    }

    /// Ethereum's pinned first OpenVM block (`OPENVM_FROM`, as `pinned_trust` carries it) on
    /// stand-in proofs of blocks 26,361,855 and 26,361,856: OpenVM from 26,361,856, SP1 before.
    #[test]
    fn ethereums_pinned_cutover_is_openvm_from_26361856_and_sp1_before() {
        use logset_query::fixtures::{chain, fake_openvm_proof, fake_proof, FakeCheck};
        use logset_query::{
            verify, BlockProof, LogFilter, LogsProof, ProofPart, ProofSystem, Verified, VerifyError,
        };
        use std::collections::HashMap;
        use ProofSystem::{OpenVm, Sp1};
        const F: u64 = 26_361_856;
        let pinned = pinned_trust(1).unwrap().openvm_from;
        assert_eq!(pinned, Some(F));
        let c = chain(1, F - 1, 2, 61);
        // The stand-in keys, under the pinned cutover.
        let t = Trust {
            openvm_from: pinned,
            ..c.trust()
        };
        // An answer of one-block parts, block `F - 1 + i` in system `s` for each `(i, s)`.
        let answer = |layout: &[(usize, ProofSystem)]| -> Result<Verified, VerifyError> {
            let mut store = HashMap::new();
            let from = F - 1 + layout[0].0 as u64;
            let to = from + layout.len() as u64 - 1;
            let parts: Vec<ProofPart> = layout
                .iter()
                .map(|&(i, s)| {
                    let pv = c.leaf_pvs[i].clone();
                    let id = c.block_id(i).to_string();
                    let proof = match s {
                        Sp1 => fake_proof(&c.leaf_vkey(), &pv),
                        OpenVm => fake_openvm_proof(&c.ovm_leaf_vkey(), &pv),
                    };
                    store.insert(id.clone(), proof);
                    ProofPart::Block(BlockProof {
                        public_values: pv,
                        proof: id,
                        records: c.records[i].clone(),
                    })
                })
                .collect();
            let f = LogFilter::from_eth_filter(&serde_json::json!({}), from, to).unwrap();
            let p = LogsProof {
                chain_id: 1,
                from_block: from,
                to_block: to,
                parts,
            };
            let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
            verify(&p, &f, &t, &mut fetch, &mut FakeCheck)
        };
        // At 26,361,856: OpenVM accepted, SP1 refused.
        let got = answer(&[(1, OpenVm)]).unwrap();
        assert_eq!((got.systems, got.openvm_from), (vec![OpenVm], Some(F)));
        assert_eq!(
            answer(&[(1, Sp1)]).unwrap_err(),
            VerifyError::System {
                block: F,
                got: Sp1,
                want: OpenVm
            }
        );
        // At 26,361,855: SP1 accepted, OpenVM refused.
        let got = answer(&[(0, Sp1)]).unwrap();
        assert_eq!((got.systems, got.openvm_from), (vec![Sp1], None));
        assert_eq!(
            answer(&[(0, OpenVm)]).unwrap_err(),
            VerifyError::System {
                block: F - 1,
                got: OpenVm,
                want: Sp1
            }
        );
        // Across it: SP1 then OpenVM, and nothing else.
        let got = answer(&[(0, Sp1), (1, OpenVm)]).unwrap();
        assert_eq!((got.systems, got.openvm_from), (vec![Sp1, OpenVm], Some(F)));
        assert_eq!(
            answer(&[(0, Sp1), (1, Sp1)]).unwrap_err(),
            VerifyError::System {
                block: F,
                got: Sp1,
                want: OpenVm
            }
        );
        assert_eq!(
            answer(&[(0, OpenVm), (1, OpenVm)]).unwrap_err(),
            VerifyError::System {
                block: F - 1,
                got: OpenVm,
                want: Sp1
            }
        );
    }

    #[test]
    fn sp1_check_refuses_openvm_bytes_before_decoding() {
        let mut c = Sp1Check::default();
        let ovm = [&logset_query::OPENVM_PROOF_MAGIC[..], &[0u8; 64]].concat();
        assert_eq!(
            c.check(&[0; 32], &ovm, b"").unwrap_err(),
            "not an SP1 proof"
        );
        assert_eq!(
            c.check(&[0; 32], &[9, 0, 0, 0], b"").unwrap_err(),
            "not an SP1 proof"
        );
    }
}
