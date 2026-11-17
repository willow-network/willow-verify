









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


pub fn parlia_milli_timestamp(h: &alloy::consensus::Header) -> u64 {
    let mix = u64::from_be_bytes(h.mix_hash.0[24..].try_into().expect("8 bytes"));
    h.timestamp * 1_000 + mix
}



#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NitroHeaderInfo {
    pub send_root: [u8; 32],
    pub send_count: u64,
    pub l1_block_number: u64,
    pub arbos_version: u64,
}





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
