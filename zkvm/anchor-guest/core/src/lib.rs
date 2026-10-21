//! **The ANCHOR child core** — in-circuit sync-committee provenance for the
//! stage-2 beacon anchor.
//!
//! The stage-2 aggregate binds everything under one `beacon_state_root` and
//! reports a `slot`, but until this child neither was PROVEN canonical: the
//! contract took `(beaconStateRoot, slot)` as given, and a consumer had to match
//! the pair against an authenticated anchor out of band. This child closes that
//! gap in-circuit:
//!
//! 1. **Signature.** A sync committee (512 witnessed pubkeys, committed by their
//!    SSZ `hash_tree_root`) signed the attested `BeaconBlockHeader` — verified as
//!    a real BLS fast-aggregate-verify over the participation bitfield, with the
//!    supermajority threshold (`3·n ≥ 2·512`) enforced HERE, in circuit.
//! 2. **Header binding.** `state_root` and `slot` are direct fields of the signed
//!    header — recomputing the 5-field header root from witnessed fields IS the
//!    binding; no generalized-index walk is needed to reach them.
//! 3. **Finality.** The `finality_branch` (depth derived from the fork, gindex
//!    169 at Electra/Fulu) proves `finalized_checkpoint.root` under the attested
//!    state; the finalized header's recomputed root must equal that leaf, and the
//!    statement's `(beacon_state_root, slot)` are the FINALIZED header's fields.
//! 4. **Committee provenance.** The signing committee's root is a public output —
//!    who says it is THE period-P committee is the verifier contract's stored
//!    committee chain (advanced by the optional next-sync-committee branch, also
//!    verified here), bootstrapped from one weak-subjectivity checkpoint. The
//!    circuit enforces `period(signature_slot) == period(attested_slot)` (period
//!    = `slot/8192`), so the handoff — proven under the ATTESTED state — is
//!    unambiguously the period-`signature_slot/8192 + 1` committee the contract
//!    attributes it to. A carried "trusted committee" input would be
//!    prover-chosen, a fabricated prior the proof would
//!    have to trust.
//!
//! The verification structure mirrors the repo's NATIVE light client
//! (`crates/network/src/beacon/{bls,light_client}.rs`, blst) and sp1-helios /
//! helios-consensus-core (`verify_generic_update` + `types/bls.rs`); the BLS
//! backend is zkcrypto `bls12_381` (sp1-patched in the guest), mirroring helios.

pub mod gindex;
pub mod header;
pub mod schedule;
pub mod wire;

#[cfg(feature = "bls")]
pub mod bls;
#[cfg(feature = "bls")]
pub mod verify;

#[cfg(all(test, feature = "bls"))]
mod tests;
