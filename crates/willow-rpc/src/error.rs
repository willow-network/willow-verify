//! The client's errors: a proof that failed (-32098), a value it could not verify (-32099), or a
//! JSON-RPC error passed on as the server sent it (or a verified revert the client computed).

use serde_json::{json, Value};
use std::fmt;

/// Code of [`Error::Verification`]: a proof failed verification.
pub const VERIFICATION_FAILED: i64 = -32098;
/// Code of [`Error::Unverifiable`]: a value the client could not verify.
pub const UNVERIFIABLE: i64 = -32099;

/// What the client's own errors carry: decide by `kind`, never by `detail`, which can carry text
/// the server chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ErrorData {
    /// What failed, by name (README "Errors").
    pub kind: String,
    /// What exactly.
    pub detail: Option<String>,
    /// The app's method.
    pub method: Option<String>,
}

/// An error from [`Verifier::request`](crate::Verifier::request).
#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    /// Code -32098: a proof failed, so the server lied or Willow has a bug. No value is returned,
    /// also under [`Unverifiable::Flag`](crate::Unverifiable::Flag).
    Verification(ErrorData),
    /// Code -32099: a value the client could not verify, under
    /// [`Unverifiable::Throw`](crate::Unverifiable::Throw) (the default).
    Unverifiable(ErrorData),
    /// Any other code: a server refusal passed on as sent, or a verified revert (code 3, with its
    /// data) the client computed itself.
    Rpc {
        code: i64,
        message: String,
        data: Option<Value>,
    },
}

impl Error {
    pub(crate) fn verification(
        kind: &str,
        detail: impl Into<String>,
        method: Option<&str>,
    ) -> Error {
        Error::Verification(ErrorData {
            kind: kind.into(),
            detail: Some(detail.into()),
            method: method.map(Into::into),
        })
    }

    pub(crate) fn unverifiable(
        kind: &str,
        detail: impl Into<String>,
        method: Option<&str>,
    ) -> Error {
        Error::Unverifiable(ErrorData {
            kind: kind.into(),
            detail: Some(detail.into()),
            method: method.map(Into::into),
        })
    }

    pub(crate) fn rpc(code: i64, message: impl Into<String>) -> Error {
        Error::Rpc {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// A server's JSON-RPC error object, as it sent it (a code that is not a number is -32603).
    pub(crate) fn from_object(e: &Value) -> Error {
        let code = e.get("code").and_then(code_of).unwrap_or(-32603);
        let message = match e.get("message") {
            Some(Value::String(s)) => s.clone(),
            None | Some(Value::Null) => String::new(),
            Some(v) => v.to_string(),
        };
        Error::Rpc {
            code,
            message,
            data: e.get("data").cloned(),
        }
    }

    /// The JSON-RPC error code.
    pub fn code(&self) -> i64 {
        match self {
            Error::Verification(_) => VERIFICATION_FAILED,
            Error::Unverifiable(_) => UNVERIFIABLE,
            Error::Rpc { code, .. } => *code,
        }
    }

    /// `data.kind` of the client's own errors.
    pub fn kind(&self) -> Option<&str> {
        match self {
            Error::Verification(d) | Error::Unverifiable(d) => Some(&d.kind),
            Error::Rpc { .. } => None,
        }
    }

    /// The client's own error data.
    pub fn data(&self) -> Option<&ErrorData> {
        match self {
            Error::Verification(d) | Error::Unverifiable(d) => Some(d),
            Error::Rpc { .. } => None,
        }
    }

    /// The error's `message`.
    pub fn message(&self) -> String {
        match self {
            Error::Verification(d) => with_detail("Willow's response failed verification", d),
            Error::Unverifiable(d) => with_detail("The response could not be verified", d),
            Error::Rpc { message, .. } => message.clone(),
        }
    }

    /// The JSON-RPC error object: `{code, message, data?}`.
    pub fn to_json(&self) -> Value {
        match self {
            Error::Verification(d) | Error::Unverifiable(d) => {
                let mut data = json!({"kind": d.kind});
                if let Some(x) = &d.detail {
                    data["detail"] = json!(x);
                }
                if let Some(m) = &d.method {
                    data["method"] = json!(m);
                }
                json!({"code": self.code(), "message": self.message(), "data": data})
            }
            Error::Rpc {
                code,
                message,
                data,
            } => match data {
                Some(d) => json!({"code": code, "message": message, "data": d}),
                None => json!({"code": code, "message": message}),
            },
        }
    }

    /// The client's own error with its method set, when it has none.
    pub(crate) fn with_method(self, method: Option<&str>) -> Error {
        match (self, method) {
            (Error::Verification(mut d), Some(m)) if d.method.is_none() => {
                d.method = Some(m.into());
                Error::Verification(d)
            }
            (Error::Unverifiable(mut d), Some(m)) if d.method.is_none() => {
                d.method = Some(m.into());
                Error::Unverifiable(d)
            }
            (e, _) => e,
        }
    }

    /// The same error raised for `method` (a merged proof's failure is each request's own).
    pub(crate) fn for_method(self, method: &str) -> Error {
        match self {
            Error::Verification(mut d) if d.method.as_deref() != Some(method) => {
                d.method = Some(method.into());
                Error::Verification(d)
            }
            e => e,
        }
    }
}

fn with_detail(head: &str, d: &ErrorData) -> String {
    match &d.detail {
        Some(x) if !x.is_empty() => format!("{head} ({}): {x}", d.kind),
        _ => format!("{head} ({})", d.kind),
    }
}

/// A JSON-RPC code: an integer, as JSON writes it.
fn code_of(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n
            .as_i64()
            .or_else(|| n.as_f64().filter(|f| f.fract() == 0.0).map(|f| f as i64))
            .filter(|c| *c != 0),
        Value::String(s) => s.trim().parse().ok().filter(|c: &i64| *c != 0),
        _ => None,
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.message(), self.code())
    }
}

impl std::error::Error for Error {}
