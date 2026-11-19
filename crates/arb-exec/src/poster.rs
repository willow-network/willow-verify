





use alloy_primitives::{b256, Address, Bytes, B256, U256};
use alloy_rlp::{Encodable, Header};
use brotli::enc::encode::{
    BrotliEncoderOperation, BrotliEncoderParameter, BrotliEncoderStateStruct,
};
use brotli::enc::interface::{InputPair, PredictionModeContextMap, StaticCommand};
use brotli::enc::{InputReferenceMut, StandardAlloc};


#[derive(Clone, Debug)]
pub struct Message<'a> {
    pub nonce: u64,
    pub tip: U256,
    pub fee_cap: U256,
    pub gas: u64,
    pub to: Option<Address>,
    pub value: U256,
    pub data: &'a Bytes,
    pub access_list: &'a [(Address, Vec<B256>)],
}


const NONCE_FILLER: u64 = 12_125_040_168_863_184_969;

const TIP_FILLER: u64 = 968_523_046;

const FEE_CAP_FILLER: u64 = 4_054_464_639;

const GAS_FILLER: u64 = 3_091_729_570;

const SIG_V: u64 = 126_483;
const SIG_R: B256 = b256!("0xef22bddd350b943170a67d35191c27e310709a28c38b5762a152ff640108f5b2");
const SIG_S: B256 = b256!("0xa9463b19d1148abedba3d6925530d4465b271ce2cc61f80b1a0a80fd73eab881");


const WINDOW_BITS: u32 = 22;



pub fn stand_in(m: &Message<'_>, estimating: bool) -> Vec<u8> {
    let nonce = match m.nonce {
        0 => NONCE_FILLER,
        n => n,
    };
    let tip = match m.tip.is_zero() {
        true => U256::from(TIP_FILLER),
        false => m.tip,
    };
    let fee_cap = match m.fee_cap.is_zero() {
        true => U256::from(FEE_CAP_FILLER),
        false => m.fee_cap,
    };
    let gas = match m.gas == 0 || estimating {
        true => GAS_FILLER,
        false => m.gas,
    };
    let mut body = Vec::new();
    0u64.encode(&mut body);
    nonce.encode(&mut body);
    tip.encode(&mut body);
    fee_cap.encode(&mut body);
    gas.encode(&mut body);
    match m.to {
        Some(to) => to.encode(&mut body),

        None => body.push(alloy_rlp::EMPTY_STRING_CODE),
    }
    m.value.encode(&mut body);
    m.data.encode(&mut body);
    let mut entries = Vec::new();
    for (address, keys) in m.access_list {
        let mut keys_rlp = Vec::new();
        for k in keys {
            k.encode(&mut keys_rlp);
        }
        let mut entry = Vec::new();
        address.encode(&mut entry);
        list_header(keys_rlp.len(), &mut entry);
        entry.extend_from_slice(&keys_rlp);
        list_header(entry.len(), &mut entries);
        entries.extend_from_slice(&entry);
    }
    list_header(entries.len(), &mut body);
    body.extend_from_slice(&entries);
    SIG_V.encode(&mut body);
    U256::from_be_bytes(SIG_R.0).encode(&mut body);
    U256::from_be_bytes(SIG_S.0).encode(&mut body);
    let mut out = vec![0x02];
    list_header(body.len(), &mut out);
    out.extend_from_slice(&body);
    out
}

fn list_header(payload_length: usize, out: &mut Vec<u8>) {
    Header {
        list: true,
        payload_length,
    }
    .encode(out);
}



pub fn brotli_len(input: &[u8], level: u32) -> Option<usize> {
    let mut encoder = BrotliEncoderStateStruct::new(StandardAlloc::default());
    if !encoder.set_parameter(BrotliEncoderParameter::BROTLI_PARAM_QUALITY, level)
        || !encoder.set_parameter(BrotliEncoderParameter::BROTLI_PARAM_LGWIN, WINDOW_BITS)
    {
        return None;
    }
    let mut out = vec![0u8; input.len() + 8 * (input.len() / 1024) + 64];
    let (mut in_left, mut in_at) = (input.len(), 0usize);
    let (mut out_left, mut out_at) = (out.len(), 0usize);
    let mut total = None;
    let mut no_callback = |_: &mut PredictionModeContextMap<InputReferenceMut>,
                           _: &mut [StaticCommand],
                           _: InputPair,
                           _: &mut StandardAlloc| ();
    let ok = encoder.compress_stream(
        BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
        &mut in_left,
        input,
        &mut in_at,
        &mut out_left,
        &mut out,
        &mut out_at,
        &mut total,
        &mut no_callback,
    );
    ok.then_some(out_at)
}



pub fn reproducible(len: usize, level: u32) -> bool {
    level == 1 && !(16_385..=32_768).contains(&len)
}


pub fn data_cost(
    m: &Message<'_>,
    level: u32,
    price_per_unit: U256,
    estimating: bool,
) -> Result<(U256, u64), String> {
    let bytes = stand_in(m, estimating);
    let len = bytes.len();
    if !reproducible(len, level) {
        return Err(format!(
            "its L1 data's size after brotli ({len} bytes at level {level}) isn't reproduced here"
        ));
    }
    let size =
        brotli_len(&bytes, level).ok_or_else(|| format!("brotli failed at level {level}"))?;
    let units = (size as u64).saturating_mul(16);
    let padded = units.wrapping_add(256).wrapping_mul(10_100) / 10_000;
    Ok((price_per_unit.saturating_mul(U256::from(padded)), padded))
}



pub fn poster_gas(cost: U256, price: U256, estimating: Option<U256>) -> u64 {
    let (cost, price) = match estimating {
        None => (cost, price),
        Some(min) => {

            let (eighth, rest) = (price / U256::from(8), price % U256::from(8));
            let lowered = eighth * U256::from(7) + rest * U256::from(7) / U256::from(8);
            (cost.saturating_add(cost / U256::from(10)), lowered.max(min))
        }
    };
    match price.is_zero() {
        true => 0,
        false => (cost / price).saturating_to(),
    }
}
