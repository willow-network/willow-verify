//! JSON-RPC hex: quantities, hashes, addresses, storage keys.

use serde_json::Value;

pub(crate) const ZERO_HASH: &str =
    "0x0000000000000000000000000000000000000000000000000000000000000000";

/// The most a number may be and still be exact as a JSON number (2^53 - 1), as the npm client.
const MAX_SAFE: u64 = (1 << 53) - 1;

/// A `0x` quantity (at most 14 hex digits), a decimal string (at most 15 digits) or a safe
/// non-negative integer; `None` when it is none of them.
pub(crate) fn to_number(x: &Value) -> Option<u64> {
    match x {
        Value::Number(n) => n.as_u64().filter(|n| *n <= MAX_SAFE).or_else(|| {
            n.as_f64()
                .filter(|f| f.fract() == 0.0 && *f >= 0.0 && *f <= MAX_SAFE as f64)
                .map(|f| f as u64)
        }),
        Value::String(s) => str_number(s),
        _ => None,
    }
}

pub(crate) fn str_number(s: &str) -> Option<u64> {
    if let Some(h) = s.strip_prefix("0x") {
        if !h.is_empty() && h.len() <= 14 && h.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u64::from_str_radix(h, 16).ok();
        }
        return None;
    }
    if !s.is_empty() && s.len() <= 15 && s.bytes().all(|b| b.is_ascii_digit()) {
        return s.parse().ok();
    }
    None
}

pub(crate) fn quantity(n: u64) -> String {
    format!("0x{n:x}")
}

pub(crate) fn quantity_u128(n: u128) -> String {
    format!("0x{n:x}")
}

fn is_hex_of(x: &str, digits: usize) -> bool {
    x.len() == digits + 2 && x.starts_with("0x") && x[2..].bytes().all(|b| b.is_ascii_hexdigit())
}

pub(crate) fn is_hash(x: &Value) -> bool {
    x.as_str().is_some_and(|s| is_hex_of(s, 64))
}

pub(crate) fn is_hash_str(s: &str) -> bool {
    is_hex_of(s, 64)
}

pub(crate) fn is_address(x: &Value) -> bool {
    x.as_str().is_some_and(|s| is_hex_of(s, 40))
}

pub(crate) fn same_hex(a: &Value, b: &Value) -> bool {
    match (a.as_str(), b.as_str()) {
        (Some(a), Some(b)) => a.eq_ignore_ascii_case(b),
        _ => false,
    }
}

/// `0x` bytes, an even number of hex digits.
pub(crate) fn to_bytes(h: &str) -> Option<Vec<u8>> {
    let d = h.strip_prefix("0x")?;
    if d.len() % 2 != 0 {
        return None;
    }
    hex::decode(d).ok()
}

pub(crate) fn keccak_hex(b: &[u8]) -> String {
    format!("{}", alloy_primitives::keccak256(b))
}

/// A storage key as 32 bytes, lowercase, for comparing keys written differently.
pub(crate) fn slot_key(k: &str) -> Option<String> {
    let d = k.strip_prefix("0x")?;
    if d.len() > 64 || !d.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some(format!("0x{:0>64}", d.to_ascii_lowercase()))
}

/// At most `n` characters of a value's JSON, for messages.
pub(crate) fn shown(x: &Value, n: usize) -> String {
    let s = x.to_string();
    if s.chars().count() <= n {
        s
    } else {
        s.chars().take(n).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_as_the_npm_client_reads_them() {
        assert_eq!(to_number(&json!("0x10")), Some(16));
        assert_eq!(to_number(&json!("16")), Some(16));
        assert_eq!(to_number(&json!(16)), Some(16));
        assert_eq!(to_number(&json!("0x")), None);
        assert_eq!(to_number(&json!("0x123456789012345")), None);
        assert_eq!(to_number(&json!("-1")), None);
        assert_eq!(to_number(&json!(1.5)), None);
        assert_eq!(to_number(&json!(null)), None);
        assert_eq!(slot_key("0x1").unwrap(), format!("0x{}1", "0".repeat(63)));
        assert!(slot_key("0xg").is_none());
        assert!(is_hash(&json!(format!("0x{}", "ab".repeat(32)))));
        assert!(!is_hash(&json!("0xab")));
    }
}
