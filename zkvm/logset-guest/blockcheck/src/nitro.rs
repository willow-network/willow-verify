








use alloy_primitives::U256;

use anyhow::{ensure, Result};



pub const ARB_DEPOSIT: u8 = 0x64;

pub const ARB_UNSIGNED: u8 = 0x65;

pub const ARB_CONTRACT: u8 = 0x66;

pub const ARB_RETRY: u8 = 0x68;

pub const ARB_SUBMIT_RETRYABLE: u8 = 0x69;

pub const ARB_INTERNAL: u8 = 0x6A;


#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NitroHeaderInfo {
    pub send_root: [u8; 32],
    pub send_count: u64,
    pub l1_block_number: u64,
    pub arbos_version: u64,
}


const LAST_ARBOS_REFUSING_ZERO_BASE_FEE: u64 = 40;


fn mix_word(mix: &[u8; 32], i: usize) -> u64 {
    u64::from_be_bytes(mix[8 * i..8 * i + 8].try_into().expect("8 bytes"))
}




pub fn nitro_header_info(h: &alloy::consensus::Header) -> Result<NitroHeaderInfo> {
    let carries_info =
        h.base_fee_per_gas.is_some() && h.extra_data.len() == 32 && h.difficulty == U256::from(1u8);
    if !carries_info {
        return Ok(NitroHeaderInfo::default());
    }
    let mix = &h.mix_hash.0;
    let info = NitroHeaderInfo {
        send_root: <[u8; 32]>::try_from(&h.extra_data[..]).expect("32 bytes"),
        send_count: mix_word(mix, 0),
        l1_block_number: mix_word(mix, 1),
        arbos_version: mix_word(mix, 2),
    };
    ensure!(
        !(info.arbos_version <= LAST_ARBOS_REFUSING_ZERO_BASE_FEE && h.base_fee_per_gas == Some(0)),
        "ArbOS {} header with a zero base fee: its Nitro header info depends on node settings",
        info.arbos_version
    );
    Ok(info)
}


























































































































































































































































































