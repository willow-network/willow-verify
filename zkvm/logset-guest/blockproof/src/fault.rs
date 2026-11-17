








use std::fmt;


#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Fault {


    Malformed,

    Formula,

    Chain,


    Header,

    Block,

    Item,


    Transactions,


    Receipts,


    Withdrawals,


    Uncles,

    Missing,

    Link,

    Range,

    Body,
}

impl Fault {

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


















}

impl fmt::Display for Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}


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


pub fn reject(fault: Fault, message: impl Into<String>) -> anyhow::Error {
    anyhow::Error::new(Rejected {
        fault,
        message: message.into(),
    })
}


pub fn tag(fault: Fault, e: anyhow::Error) -> anyhow::Error {
    if self::fault(&e).is_some() {
        e
    } else {
        reject(fault, format!("{e:#}"))
    }
}


pub fn fault(e: &anyhow::Error) -> Option<Fault> {
    e.downcast_ref::<Rejected>()
        .or_else(|| e.chain().find_map(|c| c.downcast_ref::<Rejected>()))
        .map(|r| r.fault)
}


macro_rules! check {
    ($cond:expr, $fault:expr, $($arg:tt)+) => {
        if !$cond {
            return Err($crate::fault::reject($fault, format!($($arg)+)));
        }
    };
}
pub(crate) use check;


pub(crate) trait Tag<T> {
    fn tag(self, fault: Fault) -> anyhow::Result<T>;
}

impl<T> Tag<T> for anyhow::Result<T> {
    fn tag(self, fault: Fault) -> anyhow::Result<T> {
        self.map_err(|e| tag(fault, e))
    }
}
