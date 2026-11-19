










use crate::call::{address_word, calldata};
use crate::{Failure, Kind};
use alloy_primitives::{keccak256, Address, Bytes, Signature, B256, U256};
use base64::Engine;
use serde_json::Value;


const FEED_DOMAIN: &[u8; 20] = b"Arbitrum Nitro Feed:";


const CURVE_ORDER: [u8; 32] = [
    0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xfe,
    0xba, 0xae, 0xdc, 0xe6, 0xaf, 0x48, 0xa0, 0x3b, 0xbf, 0xd2, 0x5e, 0x8c, 0xd0, 0x36, 0x41, 0x41,
];

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}



fn byte_field(v: Option<&Value>) -> Option<Vec<u8>> {
    match v {
        None | Some(Value::Null) => Some(Vec::new()),
        Some(Value::String(s)) => base64::engine::general_purpose::STANDARD.decode(s).ok(),
        Some(_) => None,
    }
}


fn integer(v: Option<&Value>) -> Option<u64> {
    v.and_then(Value::as_u64)
}


fn hex_value<const N: usize>(v: &Value) -> Option<[u8; N]> {
    let s = v.as_str()?;
    let digits = s
        .strip_prefix("0x")
        .or_else(|| s.strip_prefix("0X"))
        .unwrap_or(s);
    if digits.len() != 2 * N {
        return None;
    }
    hex::decode(digits).ok()?.try_into().ok()
}


fn object_member<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object()?.get(key)
}



pub fn feed_signature_hash(
    chain_id: u64,
    seq: u64,
    block_hash: &[u8; 32],
    m: &Value,
) -> Option<B256> {
    let outer = object_member(m, "message").filter(|v| v.is_object())?;
    let inner = object_member(outer, "message").filter(|v| v.is_object())?;
    let header = object_member(inner, "header").filter(|v| v.is_object())?;

    let metadata = byte_field(m.get("blockMetadata"))?;
    let delayed_read = integer(outer.get("delayedMessagesRead"))?;
    let kind = u8::try_from(integer(header.get("kind"))?).ok()?;
    let sender = hex_value::<20>(header.get("sender")?)?;
    let l1_block = integer(header.get("blockNumber"))?;
    let timestamp = integer(header.get("timestamp"))?;
    let request_id = match header.get("requestId") {
        None | Some(Value::Null) => None,
        Some(v) => Some(hex_value::<32>(v)?),
    };
    let base_fee = match header.get("baseFeeL1") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_u64()?),
    };
    let l2_message = byte_field(inner.get("l2Msg"))?;

    let mut pre = Vec::with_capacity(160 + metadata.len() + l2_message.len());
    pre.extend_from_slice(FEED_DOMAIN);
    pre.extend_from_slice(&chain_id.to_be_bytes());
    pre.extend_from_slice(&seq.to_be_bytes());
    pre.extend_from_slice(block_hash);
    pre.extend_from_slice(&metadata);
    pre.extend_from_slice(&delayed_read.to_be_bytes());
    pre.push(kind);
    pre.extend_from_slice(&sender);
    pre.extend_from_slice(&l1_block.to_be_bytes());
    pre.extend_from_slice(&timestamp.to_be_bytes());
    if let Some(id) = request_id {
        pre.extend_from_slice(&id);
    }
    if let Some(fee) = base_fee {

        let b = fee.to_be_bytes();
        let first = b.iter().position(|x| *x != 0).unwrap_or(b.len());
        pre.extend_from_slice(&b[first..]);
    }
    pre.extend_from_slice(&l2_message);
    Some(keccak256(&pre))
}


fn parity_of(v: u8) -> Option<bool> {
    match v {
        0 => Some(false),
        1 => Some(true),
        2..=26 | 29..=34 => None,
        _ => Some(v.is_multiple_of(2)),
    }
}



fn recover(digest: &B256, sig: &[u8; 65], odd: bool) -> Option<Address> {
    let n = U256::from_be_bytes(CURVE_ORDER);
    let r = U256::from_be_slice(&sig[..32]);
    let mut s = U256::from_be_slice(&sig[32..64]);
    let mut odd = odd;
    if r.is_zero() || s.is_zero() || r >= n || s >= n {
        return None;
    }
    if s > n >> 1usize {
        s = n - s;
        odd = !odd;
    }
    Signature::new(r, s, odd)
        .recover_address_from_prehash(digest)
        .ok()
}


pub fn nitro_signed_block(chain_id: u64, m: &Value) -> Result<(u64, [u8; 32], Address), Failure> {
    let seq = integer(m.get("sequenceNumber"))
        .ok_or_else(|| bad("a feed message without a sequence number"))?;
    let hash = m
        .get("blockHash")
        .and_then(hex_value::<32>)
        .ok_or_else(|| bad("a feed message without a block hash"))?;
    let sig: [u8; 65] = byte_field(m.get("signatureV2"))
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| bad("a feed message without a signature"))?;
    let odd = parity_of(sig[64]).ok_or_else(|| bad("a malformed signature"))?;
    let digest = feed_signature_hash(chain_id, seq, &hash, m)
        .ok_or_else(|| bad("a malformed feed message"))?;
    let signer = recover(&digest, &sig, odd)
        .ok_or_else(|| rejected("the sequencer's signature does not recover a key"))?;
    Ok((seq, hash, signer))
}



fn inbox_names(
    inbox: Address,
    signer: Address,
    call: &dyn Fn(Address, Bytes) -> Result<Bytes, Failure>,
) -> Result<bool, Failure> {
    let who = address_word(signer);
    for role in ["isBatchPoster(address)", "isSequencer(address)"] {
        if crate::l2::as_bool(&call(inbox, calldata(role, &[who]))?)? {
            return Ok(true);
        }
    }
    Ok(false)
}



pub(crate) fn nitro_sequenced_block(
    chain_id: u64,
    name: &str,
    inbox: Address,
    first_block: u64,
    signed: &Value,
    call: &dyn Fn(Address, Bytes) -> Result<Bytes, Failure>,
) -> Result<(u64, [u8; 32], Address), Failure> {
    let message = signed
        .get("message")
        .ok_or_else(|| bad("the signed block has no feed message"))?;
    let (seq, hash, signer) = nitro_signed_block(chain_id, message)?;
    if !inbox_names(inbox, signer, call)? {
        return Err(rejected(format!(
            "{signer} is not a sequencer or batch poster Ethereum names for {name}"
        )));
    }
    let number = first_block
        .checked_add(seq)
        .ok_or_else(|| bad("a sequence number out of range"))?;
    Ok((number, hash, signer))
}
