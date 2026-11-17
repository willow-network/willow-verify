











pub mod header;

pub mod spec;
pub mod vote;




pub use header::Header;

pub use spec::{Spec, MAINNET};


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {

    Malformed(String),

    Field(String),

    BadExtra(String),

    BadSeal(String),









    Attestation(String),




}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (k, m) = match self {
            Error::Malformed(m) => ("malformed", m),
            Error::Field(m) => ("field", m),
            Error::BadExtra(m) => ("extraData", m),
            Error::BadSeal(m) => ("seal", m),




            Error::Attestation(m) => ("attestation", m),


        };
        write!(f, "{k}: {m}")
    }
}

impl std::error::Error for Error {}
