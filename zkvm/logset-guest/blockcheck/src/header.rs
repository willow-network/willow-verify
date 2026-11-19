









use anyhow::{ensure, Context, Result};


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HeaderInfo {
    pub number: u64,

    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub receipts_root: [u8; 32],
    pub logs_bloom: [u8; 256],
}

pub fn decode_alloy(rlp: &[u8]) -> Result<HeaderInfo> {
    let h = alloy_header(rlp)?;
    Ok(HeaderInfo {
        number: h.number,
        hash: alloy_primitives::keccak256(rlp).0,
        parent_hash: h.parent_hash.0,
        receipts_root: h.receipts_root.0,
        logs_bloom: h.logs_bloom.0 .0,
    })
}




pub fn alloy_header(rlp: &[u8]) -> Result<alloy::consensus::Header> {
    use alloy_rlp::Decodable;
    let core = core_rlp(rlp)?;
    let mut buf = core.as_slice();
    let h = alloy::consensus::Header::decode(&mut buf).context("header RLP decode")?;
    ensure!(
        buf.is_empty(),
        "header RLP has {} trailing bytes",
        buf.len()
    );
    Ok(h)
}


const ALLOY_FIELDS: usize = 21;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailingKind {

    Hash,

    Quantity,

    Data,
}






pub const TRAILING: [(&str, TrailingKind); 3] = [
    ("blockAccessListHash", TrailingKind::Hash),
    ("slotNumber", TrailingKind::Quantity),
    ("parentProposerPubkey", TrailingKind::Data),
];


pub fn trailing_fields(chain_id: u64) -> &'static [(&'static str, TrailingKind)] {
    if chain_id == logset_chains::BERACHAIN {
        &TRAILING[2..]
    } else {
        &TRAILING[..2]
    }
}



pub fn trailing_items(rlp: &[u8]) -> Result<Vec<Vec<u8>>> {
    let (payload, total, is_list) = crate::receipts::rlp_item(rlp)?;
    ensure!(is_list && total == rlp.len(), "header is not one RLP list");
    let items = crate::receipts::rlp_list_items(payload)?;
    items
        .iter()
        .skip(ALLOY_FIELDS)
        .map(|(b, is_list)| {
            ensure!(!is_list, "a header field after requestsHash is a list");
            Ok(b.to_vec())
        })
        .collect()
}



pub fn encode_with_trailing(h: &alloy::consensus::Header, trailing: &[Vec<u8>]) -> Result<Vec<u8>> {
    let rlp = alloy_rlp::encode(h);
    if trailing.is_empty() {
        return Ok(rlp);
    }
    let items: Vec<Vec<u8>> = trailing
        .iter()
        .map(|b| alloy_rlp::encode(b.as_slice()))
        .collect();
    append_items(&rlp, &items)
}


fn core_rlp(rlp: &[u8]) -> Result<Vec<u8>> {
    let (payload, total, is_list) = crate::receipts::rlp_item(rlp)?;
    ensure!(is_list, "header is not an RLP list");
    ensure!(
        total == rlp.len(),
        "header RLP has {} trailing bytes",
        rlp.len() - total
    );
    let mut at = 0usize;
    let mut n = 0usize;
    while at < payload.len() && n < ALLOY_FIELDS {
        let (_, len, _) = crate::receipts::rlp_item(&payload[at..])?;
        at += len;
        n += 1;
    }
    if at == payload.len() {
        return Ok(rlp.to_vec());
    }
    let mut out = Vec::with_capacity(at + 9);
    alloy_rlp::Header {
        list: true,
        payload_length: at,
    }
    .encode(&mut out);
    out.extend_from_slice(&payload[..at]);
    Ok(out)
}


fn append_items(rlp: &[u8], items: &[Vec<u8>]) -> Result<Vec<u8>> {
    let (payload, _, is_list) = crate::receipts::rlp_item(rlp)?;
    ensure!(is_list, "header is not an RLP list");
    let extra: usize = items.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(payload.len() + extra + 9);
    alloy_rlp::Header {
        list: true,
        payload_length: payload.len() + extra,
    }
    .encode(&mut out);
    out.extend_from_slice(payload);
    for i in items {
        out.extend_from_slice(i);
    }
    Ok(out)
}


const PARENT_HASH: usize = 0;
const RECEIPTS_ROOT: usize = 5;
const LOGS_BLOOM: usize = 6;
const NUMBER: usize = 8;


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

























































pub fn parlia(chain_id: u64) -> bool {
    chain_id == 56
}



pub fn monad(chain_id: u64) -> bool {
    chain_id == logset_chains::MONAD || chain_id == logset_chains::MONAD_TESTNET
}



pub fn opbnb(chain_id: u64) -> bool {
    chain_id == logset_chains::OPBNB
}



pub fn mantle(chain_id: u64) -> bool {
    chain_id == logset_chains::MANTLE
}















pub fn zero_total_difficulty(chain_id: u64) -> bool {
    monad(chain_id) || opbnb(chain_id)
}




pub fn opbnb_milli_timestamp(h: &alloy::consensus::Header) -> Option<u64> {
    let ms = u64::from(u16::from_be_bytes([h.mix_hash.0[0], h.mix_hash.0[1]]));
    (ms < 1_000).then(|| h.timestamp * 1_000 + ms)
}


pub fn parlia_milli_timestamp(h: &alloy::consensus::Header) -> u64 {
    let mix = u64::from_be_bytes(h.mix_hash.0[24..].try_into().expect("8 bytes"));
    h.timestamp * 1_000 + mix
}


pub use crate::nitro::{nitro_header_info, NitroHeaderInfo};
