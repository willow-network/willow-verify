//! A signed transaction as `eth_sendRawTransaction` sends it: its hash (keccak of its EIP-2718
//! bytes, a blob transaction's without its sidecar) and, when it decodes, its signer and nonce.

use alloy::consensus::transaction::SignerRecoverable;
use alloy::consensus::{Transaction, TxEnvelope};
use alloy::eips::eip2718::Decodable2718;

pub(crate) struct Sent {
    pub hash: String,
    /// The signer, recovered from the signature; `None` when it does not recover.
    pub from: Option<String>,
    pub nonce: u64,
    /// `None` for a legacy transaction without EIP-155 replay protection.
    pub chain_id: Option<u64>,
}

/// An RLP item's header at `b[at]`: (payload start, end), or `None` past the bytes.
fn item(b: &[u8], at: usize) -> Option<(bool, usize, usize)> {
    let p = *b.get(at)?;
    let be = |n: usize| -> Option<usize> {
        let s = b.get(at + 1..at + 1 + n)?;
        if n > 8 {
            return None;
        }
        Some(s.iter().fold(0usize, |v, x| (v << 8) | *x as usize))
    };
    let (list, start, len) = match p {
        0x00..=0x7f => return Some((false, at, at + 1)),
        0x80..=0xb7 => (false, at + 1, (p - 0x80) as usize),
        0xb8..=0xbf => {
            let n = (p - 0xb7) as usize;
            (false, at + 1 + n, be(n)?)
        }
        0xc0..=0xf7 => (true, at + 1, (p - 0xc0) as usize),
        _ => {
            let n = (p - 0xf7) as usize;
            (true, at + 1 + n, be(n)?)
        }
    };
    let end = start.checked_add(len)?;
    (end <= b.len()).then_some((list, start, end))
}

/// The canonical EIP-2718 bytes: a blob transaction's network form (`[tx, blobs, commitments,
/// proofs]`) without its sidecar; any other as it is.
fn canonical(raw: &[u8]) -> Option<Vec<u8>> {
    if raw.first() != Some(&3) {
        return Some(raw.to_vec());
    }
    let body = &raw[1..];
    let (list, start, end) = item(body, 0)?;
    if !list || end != body.len() {
        return None;
    }
    let (inner_list, _, inner_end) = item(body, start)?;
    if inner_list && body.get(start).is_some_and(|b| *b >= 0xc0) {
        let mut out = vec![3u8];
        out.extend_from_slice(&body[start..inner_end]);
        return Some(out);
    }
    Some(raw.to_vec())
}

/// Decodes a signed transaction's bytes; `None` when they are not one this client decodes.
pub(crate) fn decode_sent(raw: &[u8]) -> Option<Sent> {
    if raw.is_empty() {
        return None;
    }
    let bytes = canonical(raw)?;
    let mut slice = bytes.as_slice();
    let tx = TxEnvelope::decode_2718(&mut slice).ok()?;
    if !slice.is_empty() {
        return None;
    }
    let from = tx.recover_signer().ok().map(|a| format!("{a:#x}"));
    Some(Sent {
        hash: crate::hex::keccak_hex(&bytes),
        from,
        nonce: tx.nonce(),
        chain_id: tx.chain_id(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blob_transactions_network_form_hashes_without_its_sidecar() {
        // [tx, blobs, commitments, proofs] with a tiny stand-in tx list.
        let tx = [0xc3u8, 0x01, 0x02, 0x03];
        let mut net = vec![3u8, 0xc7];
        net.extend_from_slice(&tx);
        net.extend_from_slice(&[0xc0, 0xc0, 0xc0]);
        assert_eq!(canonical(&net).unwrap(), [&[3u8][..], &tx[..]].concat());
        let plain = [&[3u8][..], &tx[..]].concat();
        assert_eq!(canonical(&plain).unwrap(), plain);
        assert_eq!(
            canonical(&[2u8, 0xc1, 0x01]).unwrap(),
            vec![2u8, 0xc1, 0x01]
        );
    }
}
