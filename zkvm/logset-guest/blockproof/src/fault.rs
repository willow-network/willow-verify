//! Why a proof did not verify, as a kind a client decides by.
//!
//! [`crate::proof::verify`], [`crate::fees::verify_fees`] and
//! [`crate::fees::verify_fee_history`] fail with an `anyhow::Error` that
//! carries a [`Rejected`]: its [`Fault`] names what failed, and [`fault`]
//! reads it back. A client (the page's `kind`, the drop-in client's `data.kind`)
//! chooses what to do by the kind alone, never by the message, which can carry
//! text the server chose.

use std::fmt;

/// What part of a proof failed to verify.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fault {
    /// Not the expected shape: bad hex or quantity, a field missing or present
    /// where it may not be, a request the proof can't serve.
    Malformed,
    /// The proof names a formula this verifier does not know.
    Formula,
    /// The proof is for another chain.
    Chain,
    /// A header does not hash to its block hash, is not in canonical form, or
    /// its number is not the one stated.
    Header,
    /// The proof is for another block than the one asked for.
    Block,
    /// The proof is for another transaction than the one asked for.
    Item,
    /// The transactions do not rebuild the header's transactionsRoot, or a
    /// transaction proof does not walk to it.
    Transactions,
    /// The receipts do not rebuild the header's receiptsRoot, or a receipt
    /// proof does not walk to it.
    Receipts,
    /// The withdrawals do not rebuild the header's withdrawalsRoot, or an
    /// OP-stack block lists some.
    Withdrawals,
    /// The uncles do not rebuild the header's ommersHash, or the proof leaves
    /// uncles unproven where there are none.
    Uncles,
    /// An item the request needs is not in the proof.
    Missing,
    /// A header of a fee or fee-history proof is not its predecessor's child.
    Link,
    /// The headers are not the range the request asked for.
    Range,
    /// A body is for another block than the header at its place.
    Body,
}

impl Fault {
    /// The kind's name on the wire (the page's `kind`, the client's `data.kind`).
    pub fn as_str(self) -> &'static str {
        match self {
            Fault::Malformed => "malformed",
            Fault::Formula => "unknown-formula",
            Fault::Chain => "wrong-chain",
            Fault::Header => "header",
            Fault::Block => "wrong-block",
            Fault::Item => "wrong-item",
            Fault::Transactions => "transactions",
            Fault::Receipts => "receipts",
            Fault::Withdrawals => "withdrawals",
            Fault::Uncles => "uncles",
            Fault::Missing => "missing",
            Fault::Link => "broken-link",
            Fault::Range => "wrong-range",
            Fault::Body => "body",
        }
    }

    /// Every kind, for tables and tests.
    pub const ALL: [Fault; 14] = [
        Fault::Malformed,
        Fault::Formula,
        Fault::Chain,
        Fault::Header,
        Fault::Block,
        Fault::Item,
        Fault::Transactions,
        Fault::Receipts,
        Fault::Withdrawals,
        Fault::Uncles,
        Fault::Missing,
        Fault::Link,
        Fault::Range,
        Fault::Body,
    ];
}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A failed verification: its kind and what did not match.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejected {
    pub fault: Fault,
    pub message: String,
}

impl fmt::Display for Rejected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Rejected {}

/// An error of kind `fault`.
pub fn reject(fault: Fault, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Rejected {
        fault,
        message: message.into(),
    })
}

/// `e` as an error of kind `fault`, unless it already has a kind.
pub fn tag(fault: Fault, e: anyhow::Error) -> anyhow::Error {
    if self::fault(&e).is_some() {
        e
    } else {
        reject(fault, format!("{e:#}"))
    }
}

/// The kind an error carries, through any context added to it.
pub fn fault(e: &anyhow::Error) -> Option<Fault> {
    e.downcast_ref::<Rejected>()
        .or_else(|| e.chain().find_map(|c| c.downcast_ref::<Rejected>()))
        .map(|r| r.fault)
}

/// `Ok(())`, or an error of kind `$fault`.
macro_rules! check {
    ($cond:expr, $fault:expr, $($arg:tt)+) => {
        if !$cond {
            return Err($crate::fault::reject($fault, format!($($arg)+)));
        }
    };
}
pub(crate) use check;

/// Tags a result's error with a kind (keeping one it already has).
pub(crate) trait Tag<T> {
    fn tag(self, fault: Fault) -> anyhow::Result<T>;
}

impl<T> Tag<T> for anyhow::Result<T> {
    fn tag(self, fault: Fault) -> anyhow::Result<T> {
        self.map_err(|e| tag(fault, e))
    }
}
