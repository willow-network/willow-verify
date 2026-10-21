//! **The consensus-layer half of Willow's stage-2 exact-set proof, in-guest.**
//!
//! A pure RISC-V beacon balance + credential-membership scan that re-derives in
//! Rust every adversarial soundness property the (superseded) Binius64 circuit
//! crate it replaces carried. Measured in the SP1 guest
//! (864 cyc/SHA-256 compression; a Binius64 leaf is 41–63× larger) retired the
//! leaf *architecture*, not the theorems — so this crate ports the theorems.
//!
//! - [`ssz`] — the SSZ merkleization reference, byte-exact against the shipping
//!   `native.rs` (validators depth 40, balances depth 38, length mixin).
//! - [`fork`] — the per-fork generalized-index table with the E0080 compile-time
//!   guard (append a `BeaconState` field in `crates/network` and this crate stops
//!   compiling). Direct port.
//! - [`scan`] — the windowed credential scan with the **derived liveness selector**
//!   (`i < length`, never prover-supplied), the [`scan::Liveness`] policy that lets
//!   the false-exclusion attack be executed against the broken variant, and the
//!   `CredentialBranch` mode whose `siblings[0]` is the free join key.
//! - [`verify`] — the guest entry `verify_cl_scan`: window → list roots → anchor
//!   to `beacon_state_root`, everything DERIVED, the `Result` assertion path.
//!
//! The output interfaces the stage-2 join (`zkvm/stage2-join`): per-matched
//! `(pubkey_root, balance)`, `matched_total`, `matched_count`, the balances grand
//! total, and the anchored `validators_root`/`balances_root`.

pub mod fork;
pub mod scan;
pub mod ssz;
pub mod verify;

#[cfg(test)]
mod tests;
