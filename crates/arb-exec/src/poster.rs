//! What Nitro charges a call for its L1 data, as gas (`GasChargingHook`, `RPCPostingGasHook`): the
//! size after brotli of a stand-in transaction made from the call, priced at the L1 price per
//! unit ArbOS holds and turned into gas at the L2 price.

use alloy_primitives::{keccak256, Address, Bytes, B256, U256, U512};
use alloy_rlp::{Encodable, Header};
use brotli::enc::encode::{
    BrotliEncoderOperation, BrotliEncoderParameter, BrotliEncoderStateStruct,
};
use brotli::enc::interface::PredictionModeContextMap;
use brotli::enc::{InputPair, InputReferenceMut, StandardAlloc, StaticCommand};
use std::sync::LazyLock;

/// Calldata gas per non-zero byte (EIP-2028): a unit of L1 data.
const UNITS_PER_BYTE: u64 = 16;
/// The padding a stand-in transaction's units get (`estimationPaddingUnits`, `...BasisPoints`).
const PADDING_UNITS: u64 = 16 * UNITS_PER_BYTE;
const PADDING_BIPS: u64 = 10_100;
/// What a gas estimate pads the L1 cost by (`GasEstimationL1PricePadding`).
const ESTIMATE_BIPS: u64 = 11_000;
/// Brotli's window (`arbcompress.WINDOW_SIZE`).
const WINDOW: u32 = 22;

/// The stand-in's fields where the call leaves them zero (`makeFakeTxForMessage`).
static FILLERS: LazyLock<Fillers> = LazyLock::new(|| {
    let k = |s: &str| keccak256(s.as_bytes());
    let word4 = |s: &str| U256::from_be_slice(&k(s)[..4]);
    Fillers {
        nonce: u64::from_be_bytes(k("Nonce")[..8].try_into().unwrap()),
        tip: word4("GasTipCap"),
        fee_cap: word4("GasFeeCap"),
        gas: u64::from(u32::from_be_bytes(k("Gas")[..4].try_into().unwrap())),
        // Arbitrum One's chain id times three: the v of every stand-in.
        v: U256::from(42_161u64 * 3),
        r: U256::from_be_bytes(k("R").0),
        s: U256::from_be_bytes(k("S").0),
    }
});

struct Fillers {
    nonce: u64,
    tip: U256,
    fee_cap: U256,
    gas: u64,
    v: U256,
    r: U256,
    s: U256,
}

/// The fields of a call Nitro's stand-in transaction is made of.
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

/// The stand-in transaction's bytes: an EIP-1559 transaction with chain id 0 and a fixed
/// signature, zero fields filled in (`makeFakeTxForMessage`, then `MarshalBinary`).
pub fn stand_in(m: &Message<'_>, estimating: bool) -> Vec<u8> {
    let f = &*FILLERS;
    let or = |v: U256, filler: U256| if v.is_zero() { filler } else { v };
    let nonce = if m.nonce == 0 { f.nonce } else { m.nonce };
    let gas = if m.gas == 0 || estimating {
        f.gas
    } else {
        m.gas
    };
    let (tip, fee_cap) = (or(m.tip, f.tip), or(m.fee_cap, f.fee_cap));
    let items: Vec<(Address, &[B256])> = m
        .access_list
        .iter()
        .map(|(a, keys)| (*a, keys.as_slice()))
        .collect();
    let item_len = |(a, keys): &(Address, &[B256])| {
        let keys_len: usize = keys.iter().map(Encodable::length).sum();
        let inner = a.length()
            + Header {
                list: true,
                payload_length: keys_len,
            }
            .length()
            + keys_len;
        Header {
            list: true,
            payload_length: inner,
        }
        .length()
            + inner
    };
    let list_len: usize = items.iter().map(item_len).sum();
    let to_len = m.to.map_or(1, |a| a.length());
    let payload = U256::ZERO.length()
        + nonce.length()
        + tip.length()
        + fee_cap.length()
        + gas.length()
        + to_len
        + m.value.length()
        + m.data.length()
        + Header {
            list: true,
            payload_length: list_len,
        }
        .length()
        + list_len
        + f.v.length()
        + f.r.length()
        + f.s.length();
    let mut out = Vec::with_capacity(payload + 8);
    out.push(2);
    Header {
        list: true,
        payload_length: payload,
    }
    .encode(&mut out);
    U256::ZERO.encode(&mut out);
    nonce.encode(&mut out);
    tip.encode(&mut out);
    fee_cap.encode(&mut out);
    gas.encode(&mut out);
    match m.to {
        Some(a) => a.encode(&mut out),
        None => out.push(alloy_rlp::EMPTY_STRING_CODE),
    }
    m.value.encode(&mut out);
    m.data.encode(&mut out);
    Header {
        list: true,
        payload_length: list_len,
    }
    .encode(&mut out);
    for item in &items {
        let (a, keys) = item;
        let keys_len: usize = keys.iter().map(Encodable::length).sum();
        let inner = a.length()
            + Header {
                list: true,
                payload_length: keys_len,
            }
            .length()
            + keys_len;
        Header {
            list: true,
            payload_length: inner,
        }
        .encode(&mut out);
        a.encode(&mut out);
        Header {
            list: true,
            payload_length: keys_len,
        }
        .encode(&mut out);
        for k in *keys {
            k.encode(&mut out);
        }
    }
    f.v.encode(&mut out);
    f.r.encode(&mut out);
    f.s.encode(&mut out);
    out
}

/// `input`'s length after brotli at `level` with a 22-bit window, the whole input given at once
/// and finished in one step, as Nitro calls Google's encoder (`arbcompress.CompressLevel`).
pub fn brotli_len(input: &[u8], level: u32) -> Option<usize> {
    let mut s = BrotliEncoderStateStruct::new(StandardAlloc::default());
    if !s.set_parameter(BrotliEncoderParameter::BROTLI_PARAM_QUALITY, level)
        || !s.set_parameter(BrotliEncoderParameter::BROTLI_PARAM_LGWIN, WINDOW)
    {
        return None;
    }
    let mut out = vec![0u8; input.len() + (input.len() >> 10) * 8 + 64];
    let (mut available_in, mut in_offset) = (input.len(), 0usize);
    let (mut available_out, mut out_offset) = (out.len(), 0usize);
    let mut total = Some(0usize);
    let mut nop = |_: &mut PredictionModeContextMap<InputReferenceMut<'_>>,
                   _: &mut [StaticCommand],
                   _: InputPair<'_>,
                   _: &mut StandardAlloc| ();
    let ok = s.compress_stream(
        BrotliEncoderOperation::BROTLI_OPERATION_FINISH,
        &mut available_in,
        input,
        &mut in_offset,
        &mut available_out,
        &mut out,
        &mut out_offset,
        &mut total,
        &mut nop,
    );
    (ok && s.is_finished()).then_some(out_offset)
}

/// Whether this port of brotli compresses `len` bytes at `level` to Google's size: checked at
/// level 1 (ArbOS's), where it hashes 6 bytes a match for inputs of 16–32 KiB and Google's
/// encoder 4 (`compress_fragment_two_pass.rs`: `table_bits < 15`; Nitro's C: `B <= 15`).
pub fn reproducible(len: usize, level: u32) -> bool {
    level == 1 && !(16_385..=32_768).contains(&len)
}

/// The L1 cost of a call made of `m` (wei) and its units: a stand-in's units after brotli, padded
/// (`PosterDataCost` for a message with no transaction behind it). Refused (why) where this
/// brotli's size isn't Nitro's.
pub fn data_cost(
    m: &Message<'_>,
    level: u32,
    price_per_unit: U256,
    estimating: bool,
) -> Result<(U256, u64), String> {
    let tx = stand_in(m, estimating);
    if !reproducible(tx.len(), level) {
        return Err(format!(
            "its L1 data's size after brotli ({} bytes at level {level}) isn't reproduced here",
            tx.len()
        ));
    }
    let len =
        brotli_len(&tx, level).ok_or_else(|| format!("brotli failed at level {level}"))? as u64;
    let units = UNITS_PER_BYTE.saturating_mul(len);
    let units = units.wrapping_add(PADDING_UNITS).wrapping_mul(PADDING_BIPS) / 10_000;
    let cost = U512::from(price_per_unit) * U512::from(units);
    Ok((narrow(cost), units))
}

/// The gas a call's L1 cost comes to at `price` (`GetPosterGas`); a gas estimate's at 7/8 of it
/// (at least the minimum base fee) with the cost padded 10%.
pub fn poster_gas(cost: U256, price: U256, estimating: Option<U256>) -> u64 {
    let (cost, price) = match estimating {
        None => (cost, price),
        Some(min_base_fee) => {
            let adjusted = (U512::from(price) * U512::from(7u64)) / U512::from(8u64);
            let adjusted = narrow(adjusted).max(min_base_fee);
            let padded = U512::from(cost) * U512::from(ESTIMATE_BIPS) / U512::from(10_000u64);
            (narrow(padded), adjusted)
        }
    };
    if price.is_zero() {
        return 0;
    }
    (cost / price).saturating_to()
}

/// A 512-bit product as 256 bits, saturating.
pub(crate) fn narrow(v: U512) -> U256 {
    match v.bit_len() <= 256 {
        true => U256::from_limbs_slice(&v.as_limbs()[..4]),
        false => U256::MAX,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stand_in_is_an_eip1559_transaction_with_its_fillers() {
        let data = Bytes::new();
        let m = Message {
            nonce: 0,
            tip: U256::ZERO,
            fee_cap: U256::ZERO,
            gas: 1_000_000,
            to: Some(Address::repeat_byte(0x11)),
            value: U256::ZERO,
            data: &data,
            access_list: &[],
        };
        let tx = stand_in(&m, false);
        assert_eq!(tx[0], 2);
        // The fillers as Nitro derives them.
        assert_eq!(FILLERS.v, U256::from(126_483u64));
        let decoded = alloy_rlp::Header::decode(&mut &tx[1..]).unwrap();
        assert!(decoded.list);
        assert_eq!(decoded.payload_length + 1 + decoded.length(), tx.len());
    }

    #[test]
    fn only_sizes_this_brotli_shares_with_nitros_are_priced() {
        assert!(reproducible(16_384, 1) && reproducible(32_769, 1));
        assert!(!reproducible(16_385, 1) && !reproducible(32_768, 1));
        assert!(!reproducible(100, 0) && !reproducible(100, 11));
    }

    #[test]
    fn brotli_compresses_in_one_finishing_step() {
        let input = vec![7u8; 1000];
        let short = brotli_len(&input, 1).unwrap();
        assert!(short < 40, "{short}");
        assert!(brotli_len(&[], 1).unwrap() > 0);
    }
}
