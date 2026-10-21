//! A minimal, fail-closed RLP walker.
//!
//! Hand-written rather than `alloy-rlp`/`alloy-consensus` on purpose: the guest
//! only ever needs "split this list into its top-level items", it must survive a
//! header shape it has never seen (a future fork adding a field), and it must
//! never panic on witness bytes. A typed `Header` decoder would reject a
//! post-Prague header the moment a field is appended, which is exactly the wrong
//! failure mode for a guest that hashes the bytes it was given.
//!
//! Encodings are checked for CANONICAL form (minimal length prefixes, no leading
//! zero in a length-of-length). Non-canonical RLP cannot forge anything here —
//! headers are pinned by the hash chain and receipt values by the trie root — but
//! rejecting it keeps "one byte string, one meaning" true inside the guest.

use crate::error::Error;
use alloc::vec::Vec;

/// One top-level RLP item: its payload and whether it was a list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Item<'a> {
    pub is_list: bool,
    /// The item's contents. For a single self-encoded byte (`0x00..=0x7f`) this is
    /// the one-byte slice holding that byte.
    pub payload: &'a [u8],
}

fn need(buf: &[u8], want: usize) -> Result<(), Error> {
    if buf.len() < want {
        return Err(Error::Rlp {
            what: "item runs past the end of its input",
        });
    }
    Ok(())
}

fn be_len(bytes: &[u8]) -> Result<usize, Error> {
    if bytes.is_empty() || bytes.len() > 8 {
        return Err(Error::Rlp {
            what: "length-of-length out of range",
        });
    }
    if bytes[0] == 0 {
        return Err(Error::Rlp {
            what: "leading zero in a length-of-length",
        });
    }
    let mut v: u64 = 0;
    for &b in bytes {
        v = (v << 8) | b as u64;
    }
    usize::try_from(v).map_err(|_| Error::Rlp {
        what: "length does not fit usize",
    })
}

/// Decode one RLP head at the start of `buf`. Returns `(is_list, payload, consumed)`.
pub fn head(buf: &[u8]) -> Result<(bool, &[u8], usize), Error> {
    let b = *buf.first().ok_or(Error::Rlp {
        what: "empty input",
    })?;
    if b <= 0x7f {
        Ok((false, &buf[0..1], 1))
    } else if b <= 0xb7 {
        let len = (b - 0x80) as usize;
        need(buf, 1 + len)?;
        if len == 1 && buf[1] <= 0x7f {
            return Err(Error::Rlp {
                what: "single byte below 0x80 must be self-encoded",
            });
        }
        Ok((false, &buf[1..1 + len], 1 + len))
    } else if b <= 0xbf {
        let ll = (b - 0xb7) as usize;
        need(buf, 1 + ll)?;
        let len = be_len(&buf[1..1 + ll])?;
        if len <= 55 {
            return Err(Error::Rlp {
                what: "long form used for a string of 55 bytes or fewer",
            });
        }
        let end = 1usize
            .checked_add(ll)
            .and_then(|x| x.checked_add(len))
            .ok_or(Error::Rlp {
                what: "string length overflows usize",
            })?;
        need(buf, end)?;
        Ok((false, &buf[1 + ll..end], end))
    } else if b <= 0xf7 {
        let len = (b - 0xc0) as usize;
        need(buf, 1 + len)?;
        Ok((true, &buf[1..1 + len], 1 + len))
    } else {
        let ll = (b - 0xf7) as usize;
        need(buf, 1 + ll)?;
        let len = be_len(&buf[1..1 + ll])?;
        if len <= 55 {
            return Err(Error::Rlp {
                what: "long form used for a list of 55 bytes or fewer",
            });
        }
        let end = 1usize
            .checked_add(ll)
            .and_then(|x| x.checked_add(len))
            .ok_or(Error::Rlp {
                what: "list length overflows usize",
            })?;
        need(buf, end)?;
        Ok((true, &buf[1 + ll..end], end))
    }
}

/// Split an RLP list PAYLOAD into its top-level items.
pub fn items(payload: &[u8]) -> Result<Vec<Item<'_>>, Error> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < payload.len() {
        let (is_list, p, consumed) = head(&payload[i..])?;
        out.push(Item {
            is_list,
            payload: p,
        });
        i += consumed;
    }
    Ok(out)
}

/// Decode `input` as ONE RLP list that spans the whole slice, returning its items.
pub fn list(input: &[u8]) -> Result<Vec<Item<'_>>, Error> {
    let (is_list, payload, consumed) = head(input)?;
    if !is_list {
        return Err(Error::Rlp {
            what: "expected a list",
        });
    }
    if consumed != input.len() {
        return Err(Error::Rlp {
            what: "bytes remain after the rlp list",
        });
    }
    items(payload)
}

/// Read a minimal-length big-endian RLP integer as `u64`. The empty string is 0.
pub fn as_u64(payload: &[u8]) -> Option<u64> {
    if payload.is_empty() {
        return Some(0);
    }
    if payload.len() > 8 || payload[0] == 0 {
        return None;
    }
    let mut v = 0u64;
    for &b in payload {
        v = (v << 8) | b as u64;
    }
    Some(v)
}
