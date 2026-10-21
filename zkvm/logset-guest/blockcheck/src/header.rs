//! Execution-header fields, two ways.
//!
//! `decode_alloy` is the generation path (alloy's `Header` codec, the same one
//! `tools/window-verify` uses). `decode_positional` is the verify path: a
//! positional walk of the RLP list that reads only the four fields the
//! algorithm needs, with no header struct in between — the shape the guest has
//! to implement. `HeaderInfo` from either must agree, and both hash the EXACT
//! input bytes, so a header that decodes but re-encodes differently cannot
//! smuggle a field past the block hash.

use anyhow::{ensure, Context, Result};

/// The header fields the range walk consumes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderInfo {
    pub number: u64,
    /// keccak-256 of the raw header RLP: the block hash.
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub receipts_root: [u8; 32],
    pub logs_bloom: [u8; 256],
}

pub fn decode_alloy(rlp: &[u8]) -> Result<HeaderInfo> {
    use alloy_rlp::Decodable;
    let mut buf = rlp;
    let h = alloy::consensus::Header::decode(&mut buf).context("header RLP decode")?;
    ensure!(
        buf.is_empty(),
        "header RLP has {} trailing bytes",
        buf.len()
    );
    Ok(HeaderInfo {
        number: h.number,
        hash: alloy_primitives::keccak256(rlp).0,
        parent_hash: h.parent_hash.0,
        receipts_root: h.receipts_root.0,
        logs_bloom: h.logs_bloom.0 .0,
    })
}

/// Item positions in the execution-header RLP list.
const PARENT_HASH: usize = 0;
const RECEIPTS_ROOT: usize = 5;
const LOGS_BLOOM: usize = 6;
const NUMBER: usize = 8;

/// Positional RLP walk — no `Header` struct, no re-encode.
pub fn decode_positional(rlp: &[u8]) -> Result<HeaderInfo> {
    let (payload, total, is_list) = crate::receipts::rlp_item(rlp)?;
    ensure!(is_list, "header is not an RLP list");
    ensure!(
        total == rlp.len(),
        "header RLP has {} trailing bytes",
        rlp.len() - total
    );
    let items = crate::receipts::rlp_list_items(payload)?;
    ensure!(
        items.len() > NUMBER,
        "header has only {} fields",
        items.len()
    );
    let f = |i: usize| -> Result<&[u8]> {
        ensure!(!items[i].1, "header field {i} is a list");
        Ok(items[i].0)
    };
    let n = f(NUMBER)?;
    ensure!(n.len() <= 8, "block number is {} bytes", n.len());
    ensure!(
        n.is_empty() || n[0] != 0,
        "block number has a leading zero byte"
    );
    let mut number = 0u64;
    for &b in n {
        number = (number << 8) | b as u64;
    }
    Ok(HeaderInfo {
        number,
        hash: alloy_primitives::keccak256(rlp).0,
        parent_hash: f(PARENT_HASH)?
            .try_into()
            .context("parentHash is not 32 bytes")?,
        receipts_root: f(RECEIPTS_ROOT)?
            .try_into()
            .context("receiptsRoot is not 32 bytes")?,
        logs_bloom: f(LOGS_BLOOM)?
            .try_into()
            .context("logsBloom is not 256 bytes")?,
    })
}

/// Build the raw header RLP from an `eth_getBlockByNumber` result (with or
/// without full transactions, which it does not read) and SELF-CHECK it:
/// `keccak256(rlp)` must equal the response's reported `hash`. Mirrors
/// `tools/window-verify/src/export.rs::header_rlp_from_block_json`.
pub fn header_rlp_from_block_json(result: &serde_json::Value) -> Result<(u64, Vec<u8>)> {
    use serde::Deserialize;
    let header: alloy::rpc::types::Header =
        Deserialize::deserialize(result).context("block JSON parse")?;
    let number = header.inner.number;
    let rlp = alloy_rlp::encode(&header.inner);
    let hash = alloy_primitives::keccak256(&rlp);
    ensure!(
        hash == header.hash,
        "block {number} header self-check: computed 0x{}, endpoint reported 0x{}",
        hex::encode(hash),
        hex::encode(header.hash)
    );
    Ok((number, rlp))
}

/// What a Nitro header carries in `extraData` and `mixHash`, which Nitro nodes
/// also report as the `sendRoot`, `sendCount` and `l1BlockNumber` JSON fields.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NitroHeaderInfo {
    pub send_root: [u8; 32],
    pub send_count: u64,
    pub l1_block_number: u64,
    pub arbos_version: u64,
}

/// `types.DeserializeHeaderExtraInformation` (OffchainLabs/go-ethereum): zeros
/// for a header without a base fee, a 32-byte extraData and difficulty 1. ArbOS
/// <= 40 headers with a zero base fee depend on a node setting
/// (`legacyZeroBaseFeeUntil`) and are refused.
pub fn nitro_header_info(h: &alloy::consensus::Header) -> Result<NitroHeaderInfo> {
    if h.base_fee_per_gas.is_none()
        || h.extra_data.len() != 32
        || h.difficulty != alloy_primitives::U256::from(1u8)
    {
        return Ok(NitroHeaderInfo::default());
    }
    let mix = h.mix_hash.0;
    let word = |i: usize| u64::from_be_bytes(mix[i..i + 8].try_into().expect("8 bytes"));
    let info = NitroHeaderInfo {
        send_root: h.extra_data[..].try_into().expect("32 bytes"),
        send_count: word(0),
        l1_block_number: word(8),
        arbos_version: word(16),
    };
    ensure!(
        info.arbos_version > 40 || h.base_fee_per_gas != Some(0),
        "ArbOS {} header with a zero base fee: its Nitro header info depends on node settings",
        info.arbos_version
    );
    Ok(info)
}
