







pub mod header;

pub mod spec;
pub mod vote;

mod codec;





pub use header::Header;

pub use spec::{Spec, MAINNET};


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Malformed(String),

    BadExtra(String),
    BadSeal(String),







}

impl Error {
    fn parts(&self) -> (&'static str, &str) {
        match self {
            Error::Malformed(m) => ("malformed", m),

            Error::BadExtra(m) => ("extraData", m),
            Error::BadSeal(m) => ("seal", m),







        }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (prefix, message) = self.parts();
        write!(f, "{prefix}: {message}")
    }
}

impl std::error::Error for Error {}
