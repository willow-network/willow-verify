








use crate::render::{hx, q, qu};
use crate::txs::{
    Decoded, Field, Kind, ARB_CONTRACT, ARB_DEPOSIT, ARB_INTERNAL, ARB_RETRY, ARB_SUBMIT_RETRYABLE,
    ARB_UNSIGNED,
};
use crate::ReceiptExtras;
use alloy::primitives::U256;
use anyhow::Context;
use logset_blockcheck::receipts::NITRO_TX_TYPES;
use serde_json::{Map, Value};


pub const ARBOS_ADDRESS: [u8; 20] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x0a, 0x4b, 0x05,
];


pub const ARB_RETRYABLE_ADDRESS: [u8; 20] = [
    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x6e,
];



const SUBMIT_RETRYABLE_SELECTOR: [u8; 4] = [0xc9, 0xf9, 0x5d, 0x32];


const ARBOS_ALWAYS_TIPPING: u64 = 9;

const TIP_FLAG_BYTE: usize = 25;


const COMMON_KEYS: [&str; 7] = ["chainId", "from", "to", "value", "gas", "nonce", "input"];

use Kind::{Address, Bytes, Hash, OptAddress, Uint};

const DEPOSIT_TABLE: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("requestId", Hash),
    ("from", Address),
    ("to", Address),
    ("value", Uint),
];

const UNSIGNED_TABLE: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("from", Address),
    ("nonce", Uint),
    ("maxFeePerGas", Uint),
    ("gas", Uint),
    ("to", OptAddress),
    ("value", Uint),
    ("input", Bytes),
];

const CONTRACT_TABLE: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("requestId", Hash),
    ("from", Address),
    ("maxFeePerGas", Uint),
    ("gas", Uint),
    ("to", OptAddress),
    ("value", Uint),
    ("input", Bytes),
];

const RETRY_TABLE: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("nonce", Uint),
    ("from", Address),
    ("maxFeePerGas", Uint),
    ("gas", Uint),
    ("to", OptAddress),
    ("value", Uint),
    ("input", Bytes),
    ("ticketId", Hash),
    ("refundTo", Address),
    ("maxRefund", Uint),
    ("submissionFeeRefund", Uint),
];

const SUBMIT_RETRYABLE_TABLE: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("requestId", Hash),
    ("from", Address),
    ("l1BaseFee", Uint),
    ("depositValue", Uint),
    ("maxFeePerGas", Uint),
    ("gas", Uint),
    ("retryTo", OptAddress),
    ("retryValue", Uint),
    ("beneficiary", Address),
    ("maxSubmissionFee", Uint),
    ("refundTo", Address),
    ("retryData", Bytes),
];

const INTERNAL_TABLE: &[(&str, Kind)] = &[("chainId", Uint), ("input", Bytes)];


pub fn layout(ty: u8) -> Option<&'static [(&'static str, Kind)]> {
    match ty {
        ARB_DEPOSIT => Some(DEPOSIT_TABLE),
        ARB_UNSIGNED => Some(UNSIGNED_TABLE),
        ARB_CONTRACT => Some(CONTRACT_TABLE),
        ARB_RETRY => Some(RETRY_TABLE),
        ARB_SUBMIT_RETRYABLE => Some(SUBMIT_RETRYABLE_TABLE),
        ARB_INTERNAL => Some(INTERNAL_TABLE),
        _ => None,
    }
}



pub fn to(tx: &Decoded) -> Option<[u8; 20]> {
    match tx.ty() {
        ARB_INTERNAL => Some(ARBOS_ADDRESS),
        ARB_SUBMIT_RETRYABLE => Some(ARB_RETRYABLE_ADDRESS),
        _ => tx.address("to"),
    }
}



pub fn value(tx: &Decoded) -> U256 {
    match tx.ty() {
        ARB_SUBMIT_RETRYABLE | ARB_INTERNAL => U256::ZERO,
        _ => tx.uint("value").unwrap_or_default(),
    }
}


pub fn input(tx: &Decoded) -> Vec<u8> {
    match tx.ty() {
        ARB_DEPOSIT => Vec::new(),
        ARB_SUBMIT_RETRYABLE => submit_retryable_call(tx),
        _ => match tx.field("input") {
            Some(Field::Bytes(b)) => b.clone(),
            _ => Vec::new(),
        },
    }
}




fn submit_retryable_call(tx: &Decoded) -> Vec<u8> {
    let scalar = |name: &str| tx.uint(name).unwrap_or_default().to_be_bytes::<32>();
    let address = |name: &str| {
        let mut w = [0u8; 32];
        if let Some(a) = tx.address(name) {
            w[12..].copy_from_slice(&a);
        }
        w
    };
    let request = match tx.field("requestId") {
        Some(Field::Hash(h)) => *h,
        _ => [0; 32],
    };
    let data: &[u8] = match tx.field("retryData") {
        Some(Field::Bytes(b)) => b.as_slice(),
        _ => &[],
    };
    let padded = data.len().div_ceil(32) * 32;
    let mut out = Vec::with_capacity(4 + 12 * 32 + padded);
    out.extend_from_slice(&SUBMIT_RETRYABLE_SELECTOR);
    out.extend_from_slice(&request);
    for name in [
        "l1BaseFee",
        "depositValue",
        "retryValue",
        "maxFeePerGas",
        "gas",
        "maxSubmissionFee",
    ] {
        out.extend_from_slice(&scalar(name));
    }
    for name in ["refundTo", "beneficiary", "retryTo"] {
        out.extend_from_slice(&address(name));
    }

    out.extend_from_slice(&U256::from(11 * 32).to_be_bytes::<32>());
    out.extend_from_slice(&U256::from(data.len()).to_be_bytes::<32>());
    out.extend_from_slice(data);
    out.resize(out.len() + (padded - data.len()), 0);
    out
}


pub fn sender(tx: &Decoded) -> anyhow::Result<[u8; 20]> {
    if tx.ty() == ARB_INTERNAL {
        return Ok(ARBOS_ADDRESS);
    }
    tx.address("from").context("no from")
}


fn arbos_version(mix_hash: &[u8; 32]) -> u64 {
    u64::from_be_bytes(mix_hash[16..24].try_into().expect("8 bytes"))
}


pub fn nitro_collects_tips(mix_hash: &[u8; 32]) -> bool {
    arbos_version(mix_hash) == ARBOS_ALWAYS_TIPPING || mix_hash[TIP_FLAG_BYTE] & 1 == 1
}


pub fn tx_gas_price(tx: &Decoded) -> U256 {
    tx.uint("maxFeePerGas").unwrap_or_default()
}


pub fn effective_gas_price(tx: &Decoded, base_fee: Option<U256>, collects_tips: bool) -> U256 {
    if !collects_tips {
        return base_fee.unwrap_or_default();
    }
    match tx {
        Decoded::Table { ty, pol: false, .. } if NITRO_TX_TYPES.contains(ty) => match *ty {
            ARB_DEPOSIT | ARB_INTERNAL => U256::ZERO,
            _ => base_fee.unwrap_or_else(|| tx_gas_price(tx)),
        },
        _ => crate::derive::tx_gas_price(tx, base_fee),
    }
}


pub fn l1_block_number(mix_hash: &[u8; 32]) -> u64 {
    u64::from_be_bytes(mix_hash[8..16].try_into().expect("8 bytes"))
}



pub fn tx_fields(tx: &Decoded, o: &mut Map<String, Value>) {
    let put = |o: &mut Map<String, Value>, k: &str, v: String| {
        o.insert(k.to_string(), Value::String(v));
    };
    put(o, "chainId", qu(tx.uint("chainId").unwrap_or_default()));
    for k in ["v", "r", "s"] {
        put(o, k, "0x0".into());
    }
    let Some(table) = layout(tx.ty()) else {
        return;
    };
    for (name, _) in table {
        if COMMON_KEYS.contains(name) {
            continue;
        }
        match tx.field(name) {
            Some(Field::Uint(v)) => put(o, name, qu(*v)),
            Some(Field::Address(Some(a))) => put(o, name, hx(a)),
            Some(Field::Hash(h)) => put(o, name, hx(h)),
            Some(Field::Bytes(b)) => put(o, name, hx(b)),
            Some(Field::Bool(b)) => {
                o.insert(name.to_string(), Value::Bool(*b));
            }
            Some(Field::Address(None)) | None => {}
        }
    }
}



pub fn receipt_fields(
    l1_block_number: u64,
    extras: Option<ReceiptExtras>,
    o: &mut Map<String, Value>,
    missing: &mut dyn FnMut(&[&str]),
) {
    o.insert(
        "l1BlockNumber".to_string(),
        Value::String(q(l1_block_number)),
    );
    match extras {
        Some(x) => {
            if let Some(g) = x.gas_used_for_l1 {
                o.insert("gasUsedForL1".to_string(), Value::String(q(g)));
            }
            if let Some(t) = x.timeboosted {
                o.insert("timeboosted".to_string(), Value::Bool(t));
            }
        }
        None => missing(&["gasUsedForL1", "timeboosted"]),
    }
}
