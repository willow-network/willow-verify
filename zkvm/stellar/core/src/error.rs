

use core::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {

    Frame(&'static str),

    Xdr(&'static str),

    BucketFrame,

    LedgerSeq { index: u32, got: u32 },

    ParentHash { index: u32 },

    Network,

    LastModified { index: u32, got: u32 },

    QuorumSetInsane(&'static str),

    LedgerVersion { index: u32, got: u32 },

    ResultSetHash { index: u32 },

    SorobanCount { index: u32 },

    SorobanHash { index: u32, tx: u32 },

    BucketListHash { index: u32 },


    BucketShape { index: u32, why: &'static str },

    PrevLedger(&'static str),

    Overflow,

    ScpStatement(&'static str),

    ScpSignature { envelope: u32 },


    NoQuorum,

    AnchorLink(&'static str),

    EmptyRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for Error {}
