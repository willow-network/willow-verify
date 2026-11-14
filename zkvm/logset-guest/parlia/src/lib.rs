//! BNB Smart Chain's Parlia consensus, as a light client checks it: a header must hash to its
//! block hash, carry a seal by a validator of the set followed from a pinned checkpoint (the
//! in-turn rules, sign counts and epoch switches of bnb-chain/bsc), and any vote attestation it
//! carries must hold the aggregate BLS signature of 2/3+ of its target's validators. Anything
//! unexpected is an error: a later fork's field, a malleated seal, trailing bytes.
//!
//! - [`header`]: the header, its hash, seal hash and signer, and what `extraData` carries.
//! - [`vote`]: attestations and their BLS check.
//! - [`snapshot`]: the validator snapshot and [`snapshot::verify`], one header at a time.
//! - [`checkpoint`]: the pinned checkpoints.

pub mod checkpoint;
pub mod header;
pub mod snapshot;
pub mod spec;
pub mod vote;

#[cfg(test)]
mod tests;

pub use header::Header;
pub use snapshot::{verify, Checked, Snapshot, Validator, Vote};
pub use spec::{Spec, MAINNET};

/// Why a header was refused.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    /// Not a header this code reads: bad JSON or RLP, a hash that doesn't match, an unknown field.
    Malformed(String),
    /// A header field outside the rules (time, gas, fork fields, mixHash, uncles).
    Field(String),
    /// `extraData` without the vanity and seal, or with validators where none belong.
    BadExtra(String),
    /// The seal does not recover, or is a malleated copy.
    BadSeal(String),
    /// Sealed by someone other than the block's `miner`.
    CoinbaseMismatch(String),
    /// Sealed by an address outside the validator set.
    Unauthorized(String),
    /// `difficulty` doesn't match whether the sealer was in turn.
    WrongDifficulty(String),
    /// The sealer already sealed its share of recent blocks.
    RecentlySigned(String),
    /// The vote attestation is invalid.
    Attestation(String),
    /// Not the next block after the snapshot.
    NotChild(String),
    /// The checkpoint or an epoch switch can't be set up from what was given.
    Checkpoint(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (k, m) = match self {
            Error::Malformed(m) => ("malformed", m),
            Error::Field(m) => ("field", m),
            Error::BadExtra(m) => ("extraData", m),
            Error::BadSeal(m) => ("seal", m),
            Error::CoinbaseMismatch(m) => ("miner", m),
            Error::Unauthorized(m) => ("unauthorized", m),
            Error::WrongDifficulty(m) => ("difficulty", m),
            Error::RecentlySigned(m) => ("recently signed", m),
            Error::Attestation(m) => ("attestation", m),
            Error::NotChild(m) => ("not the next block", m),
            Error::Checkpoint(m) => ("checkpoint", m),
        };
        write!(f, "{k}: {m}")
    }
}

impl std::error::Error for Error {}
