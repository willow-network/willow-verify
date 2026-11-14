//! A block as `eth_getBlockByNumber` reports it, from verified parts: the
//! served header (the log store serves the same function's output), and the
//! parts of a block proof beyond its transactions (withdrawals, uncles),
//! verified against the header by the rules the store's ingest applies
//! (`logset_blockcheck::verify_body_with_uncles`).

use crate::fault::{check, reject, Fault, Tag};
use alloy::eips::eip4895::Withdrawal;
use alloy_rlp::Decodable;
use anyhow::Result;
use logset_blockcheck::receipts::{deposits_allowed, nitro_allowed};
use logset_blockcheck::{VerifiedBody, VerifiedHeader};
use serde_json::{json, Value};

/// The fields a block proof may leave unproven, together: a block with uncles
/// whose uncle headers the server does not hold (their hashes, and the size
/// they add to).
pub const UNPROVEN_UNCLES: [&str; 2] = ["size", "uncles"];

/// A block's uncles: their header RLPs and hashes, in order.
pub type Uncles = (Vec<Vec<u8>>, Vec<[u8; 32]>);

/// keccak256(rlp([])): the ommersHash of a block without uncles.
pub fn empty_ommers() -> [u8; 32] {
    alloy_primitives::keccak256([0xc0u8]).0
}

/// The millisecond part of a verified block's time, if its second transaction sets one.
fn millis(chain_id: u64, body: &VerifiedBody) -> Option<u64> {
    let txs: Vec<&[u8]> = body.tx_values.iter().map(Vec::as_slice).collect();
    crate::render::block_millis(chain_id, &txs)
}

/// The `eth_getBlockByNumber(n, false)` response for a verified block: geth's
/// `RPCMarshalBlock` fields, each from the verified header or body.
pub fn served_header(chain_id: u64, h: &VerifiedHeader, body: &VerifiedBody) -> Result<Value> {
    let x = &h.header;
    let hx = |b: &[u8]| format!("0x{}", hex::encode(b));
    let q = |n: u64| format!("0x{n:x}");
    let mut out = json!({
        "number": q(x.number),
        "hash": hx(&h.info.hash),
        "parentHash": hx(x.parent_hash.as_slice()),
        "nonce": hx(x.nonce.as_slice()),
        "mixHash": hx(x.mix_hash.as_slice()),
        "sha3Uncles": hx(x.ommers_hash.as_slice()),
        "logsBloom": hx(x.logs_bloom.as_slice()),
        "stateRoot": hx(x.state_root.as_slice()),
        "miner": hx(x.beneficiary.as_slice()),
        "difficulty": format!("0x{:x}", x.difficulty),
        "extraData": hx(&x.extra_data),
        "gasLimit": q(x.gas_limit),
        "gasUsed": q(x.gas_used),
        "timestamp": q(x.timestamp),
        "transactionsRoot": hx(x.transactions_root.as_slice()),
        "receiptsRoot": hx(x.receipts_root.as_slice()),
        "size": q(body.size),
        "transactions": body.tx_hashes.iter().map(|t| hx(t)).collect::<Vec<_>>(),
        "uncles": body.uncles.iter().map(|u| hx(u)).collect::<Vec<_>>(),
    });
    let o = out.as_object_mut().expect("an object");
    let mut put = |k: &str, v: Option<String>| {
        if let Some(v) = v {
            o.insert(k.into(), Value::String(v));
        }
    };
    put("baseFeePerGas", x.base_fee_per_gas.map(q));
    put(
        "withdrawalsRoot",
        x.withdrawals_root.map(|r| hx(r.as_slice())),
    );
    put("blobGasUsed", x.blob_gas_used.map(q));
    put("excessBlobGas", x.excess_blob_gas.map(q));
    put(
        "parentBeaconBlockRoot",
        x.parent_beacon_block_root.map(|r| hx(r.as_slice())),
    );
    put("requestsHash", x.requests_hash.map(|r| hx(r.as_slice())));
    put(
        "timestampMs",
        millis(chain_id, body).map(|ms| q(x.timestamp.saturating_mul(1_000).saturating_add(ms))),
    );
    if logset_blockcheck::header::parlia(chain_id) {
        put(
            "milliTimestamp",
            Some(q(logset_blockcheck::header::parlia_milli_timestamp(x))),
        );
    }
    if nitro_allowed(chain_id) {
        let info = logset_blockcheck::header::nitro_header_info(x)?;
        put("l1BlockNumber", Some(q(info.l1_block_number)));
        put("sendRoot", Some(hx(&info.send_root)));
        put("sendCount", Some(q(info.send_count)));
    }
    if let Some(w) = &body.withdrawals {
        let list: Vec<Value> = w
            .iter()
            .map(|w| {
                json!({
                    "index": q(w.index),
                    "validatorIndex": q(w.validator_index),
                    "address": hx(w.address.as_slice()),
                    "amount": q(w.amount),
                })
            })
            .collect();
        o.insert("withdrawals".into(), Value::Array(list));
    }
    Ok(out)
}

/// `rlp([items...])` of already-encoded items.
pub fn rlp_list(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: usize = items.iter().map(Vec::len).sum();
    let mut out = Vec::with_capacity(payload + 9);
    alloy_rlp::Header {
        list: true,
        payload_length: payload,
    }
    .encode(&mut out);
    for i in items {
        out.extend_from_slice(i);
    }
    out
}

/// A block's withdrawals as a block proof carries them (their RLP list),
/// verified against the header: absent without a withdrawalsRoot; empty on the
/// OP stack, whose withdrawalsRoot is the empty list's before Isthmus and the
/// L2ToL1MessagePasser's storage root from it; else rebuilding the root.
pub fn check_withdrawals(
    chain_id: u64,
    h: &alloy::consensus::Header,
    rlp: Option<&[u8]>,
) -> Result<Option<Vec<Withdrawal>>> {
    let (root, rlp) = match (h.withdrawals_root, rlp) {
        (None, None) => return Ok(None),
        (None, Some(_)) => {
            return Err(reject(
                Fault::Withdrawals,
                "withdrawals given, but the header has no withdrawalsRoot",
            ))
        }
        (Some(_), None) => {
            return Err(reject(
                Fault::Missing,
                "the header commits to withdrawals the proof does not carry",
            ))
        }
        (Some(root), Some(rlp)) => (root, rlp),
    };
    let mut buf = rlp;
    let list = Vec::<Withdrawal>::decode(&mut buf)
        .map_err(|e| reject(Fault::Withdrawals, format!("withdrawals RLP: {e}")))?;
    check!(
        buf.is_empty() && alloy_rlp::encode(&list) == rlp,
        Fault::Withdrawals,
        "the withdrawals RLP is not one list in canonical form"
    );
    if deposits_allowed(chain_id) {
        check!(
            list.is_empty(),
            Fault::Withdrawals,
            "an OP-stack block lists {} withdrawals",
            list.len()
        );
        return Ok(Some(list));
    }
    let values: Vec<Vec<u8>> = list.iter().map(alloy_rlp::encode).collect();
    let rebuilt = logset_blockcheck::trie::ordered_root(&values).tag(Fault::Withdrawals)?;
    check!(
        rebuilt == root.0,
        Fault::Withdrawals,
        "the withdrawals rebuild root 0x{}, not the header's 0x{}",
        hex::encode(rebuilt),
        hex::encode(root)
    );
    Ok(Some(list))
}

/// A block's uncles as a block proof carries them, verified against the
/// header: their RLPs (each a canonical header) rebuild ommersHash; absent
/// when the block has none; or left unproven ([`UNPROVEN_UNCLES`]) for a block
/// that has some. Their RLPs and hashes, or `None` when unproven.
pub fn check_uncles(
    h: &alloy::consensus::Header,
    uncles: Option<&[Vec<u8>]>,
    unproven: &[String],
) -> Result<Option<Uncles>> {
    let empty = empty_ommers();
    if !unproven.is_empty() {
        check!(
            unproven.iter().map(String::as_str).eq(UNPROVEN_UNCLES),
            Fault::Malformed,
            "a block proof may leave only {UNPROVEN_UNCLES:?} unproven, not {unproven:?}"
        );
        check!(
            uncles.is_none(),
            Fault::Malformed,
            "uncles both given and left unproven"
        );
        check!(
            h.ommers_hash.0 != empty,
            Fault::Uncles,
            "uncles left unproven on a block without uncles"
        );
        return Ok(None);
    }
    let Some(list) = uncles else {
        check!(
            h.ommers_hash.0 == empty,
            Fault::Uncles,
            "the header commits to uncles 0x{} the proof does not carry",
            hex::encode(h.ommers_hash)
        );
        return Ok(Some((vec![], vec![])));
    };
    check!(
        !list.is_empty(),
        Fault::Malformed,
        "an empty uncles list: a block without uncles carries none"
    );
    let mut hashes = Vec::with_capacity(list.len());
    for (i, u) in list.iter().enumerate() {
        crate::proof::decode_header(u)
            .map_err(|e| reject(Fault::Uncles, format!("uncle {i}: {e:#}")))?;
        let v = logset_blockcheck::header_from_rlp(u)
            .map_err(|e| reject(Fault::Uncles, format!("uncle {i}: {e:#}")))?;
        hashes.push(v.info.hash);
    }
    let ommers = alloy_primitives::keccak256(rlp_list(list)).0;
    check!(
        ommers == h.ommers_hash.0,
        Fault::Uncles,
        "the uncles rebuild ommersHash 0x{}, not the header's 0x{}",
        hex::encode(ommers),
        hex::encode(h.ommers_hash)
    );
    Ok(Some((list.to_vec(), hashes)))
}
