//! Receipts: the trie VALUE bytes, the rebuilt receipts root, and a
//! hand-written RLP reader that recovers the logs from those value bytes.
//!
//! Two paths meet here on purpose:
//!   * `values_and_logs_from_rpc` — the GENERATION path. alloy decodes the
//!     `eth_getBlockReceipts` JSON into `ReceiptEnvelope` and re-encodes it as
//!     EIP-2718, which is exactly the trie value.
//!   * `logs_from_value` — the VERIFY path. Nothing but the blob's bytes; a
//!     minimal RLP walker, the same shape the guest has to implement.
//!
//! Both must produce the same logs, and the value bytes must rebuild the
//! header's `receiptsRoot` under two independent trie implementations.

use anyhow::{bail, ensure, Context, Result};

/// A block's receipts as `(trie values, logs per receipt)`, both in tx order.
pub type BlockReceipts = (Vec<Vec<u8>>, Vec<Vec<Log>>);

/// One log, in the only form the ERC rules need.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

/// Chain-gated receipt types, from the one chain table (`logset_chains`): `0x7E`
/// deposits on the OP-stack ids, Nitro `0x64..=0x6A` on the Nitro ids — the
/// guest's `OP_STACK_CHAIN_IDS` / `NITRO_CHAIN_IDS`.
pub use logset_chains::{DEPOSIT_TX_TYPE, NITRO_TX_TYPES};

pub fn deposits_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).deposits
}

pub fn nitro_allowed(chain_id: u64) -> bool {
    logset_chains::receipt_gate(chain_id).nitro
}

fn json_qty(r: &serde_json::Value, key: &str) -> Result<Option<u64>> {
    match r.get(key) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(v) => {
            let s = v
                .as_str()
                .with_context(|| format!("{key} is not a string"))?;
            Ok(Some(
                u64::from_str_radix(s.trim_start_matches("0x"), 16)
                    .with_context(|| format!("{key} is not a quantity"))?,
            ))
        }
    }
}

fn json_bytes(v: &serde_json::Value, what: &str) -> Result<Vec<u8>> {
    let s = v
        .as_str()
        .with_context(|| format!("{what} is not a string"))?;
    hex::decode(s.trim_start_matches("0x")).with_context(|| format!("{what} is not hex"))
}

/// A `0x7E` deposit receipt's trie value, hand-encoded from its RPC JSON:
/// `0x7E | rlp([status, cumulativeGasUsed, logsBloom, logs])`, and from Canyon
/// (`depositReceiptVersion` set) `depositNonce, depositReceiptVersion` after
/// the logs. alloy's `ReceiptEnvelope` has no deposit arm; the header's
/// `receiptsRoot` is what proves this encoder right, block by block.
pub fn deposit_value_from_rpc(r: &serde_json::Value) -> Result<(Vec<u8>, Vec<Log>)> {
    hand_encoded_value(r, DEPOSIT_TX_TYPE, true)
}

/// A Nitro receipt (`0x64..=0x6A`) hand-encoded from its RPC JSON: the plain
/// `ty | rlp([status, cumulativeGasUsed, logsBloom, logs])`, no tail (alloy has
/// no Nitro arms). Proven right by the header's `receiptsRoot`, block by block.
pub fn nitro_value_from_rpc(r: &serde_json::Value, ty: u8) -> Result<(Vec<u8>, Vec<Log>)> {
    ensure!(
        NITRO_TX_TYPES.contains(&ty),
        "0x{ty:02x} is not a Nitro type"
    );
    hand_encoded_value(r, ty, false)
}

fn hand_encoded_value(
    r: &serde_json::Value,
    ty: u8,
    deposit_tail: bool,
) -> Result<(Vec<u8>, Vec<Log>)> {
    use alloy_rlp::Encodable;
    let status = json_qty(r, "status")?.context("deposit receipt without status")?;
    let cum = json_qty(r, "cumulativeGasUsed")?.context("no cumulativeGasUsed")?;
    let bloom = json_bytes(r.get("logsBloom").context("no logsBloom")?, "logsBloom")?;
    ensure!(bloom.len() == 256, "logsBloom is {} bytes", bloom.len());
    let mut logs = Vec::new();
    for l in r
        .get("logs")
        .and_then(|v| v.as_array())
        .context("no logs array")?
    {
        let address: [u8; 20] = json_bytes(l.get("address").context("log address")?, "address")?
            .try_into()
            .map_err(|_| anyhow::anyhow!("log address is not 20 bytes"))?;
        let mut topics = Vec::new();
        for t in l
            .get("topics")
            .and_then(|v| v.as_array())
            .context("log topics")?
        {
            topics.push(
                json_bytes(t, "topic")?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("topic is not 32 bytes"))?,
            );
        }
        ensure!(topics.len() <= 4, "log has {} topics", topics.len());
        let data = json_bytes(l.get("data").context("log data")?, "data")?;
        logs.push(Log {
            address,
            topics,
            data,
        });
    }
    // Before Canyon (no version) the trie value has no tail, though nodes
    // report the Regolith nonce: op-geth's `Receipts.EncodeIndex`.
    let version = if deposit_tail {
        json_qty(r, "depositReceiptVersion")?
    } else {
        None
    };
    let nonce = match version {
        Some(_) => Some(
            json_qty(r, "depositNonce")?.context("depositReceiptVersion without depositNonce")?,
        ),
        None => None,
    };

    let enc = |f: &dyn Fn(&mut Vec<u8>)| {
        let mut b = Vec::new();
        f(&mut b);
        b
    };
    let list = |items: &[Vec<u8>]| {
        let payload: Vec<u8> = items.concat();
        let mut out = Vec::new();
        alloy_rlp::Header {
            list: true,
            payload_length: payload.len(),
        }
        .encode(&mut out);
        out.extend_from_slice(&payload);
        out
    };
    let log_items: Vec<Vec<u8>> = logs
        .iter()
        .map(|l| {
            let topics: Vec<Vec<u8>> = l
                .topics
                .iter()
                .map(|t| enc(&|b| t.as_slice().encode(b)))
                .collect();
            list(&[
                enc(&|b| l.address.as_slice().encode(b)),
                list(&topics),
                enc(&|b| l.data.as_slice().encode(b)),
            ])
        })
        .collect();
    let mut items = vec![
        enc(&|b| status.encode(b)),
        enc(&|b| cum.encode(b)),
        enc(&|b| bloom.as_slice().encode(b)),
        list(&log_items),
    ];
    if let Some(n) = nonce {
        items.push(enc(&|b| n.encode(b)));
    }
    if let Some(v) = version {
        items.push(enc(&|b| v.encode(b)));
    }
    let mut value = vec![ty];
    value.extend(list(&items));
    Ok((value, logs))
}

/// Some endpoints answer `"logs": null` for a receipt without
/// logs. Read that as `[]` only when the receipt's bloom is all zero, which no
/// receipt with a log has; the rebuilt receiptsRoot checks the rest.
fn logs_null_as_empty(r: &serde_json::Value) -> std::borrow::Cow<'_, serde_json::Value> {
    let zero_bloom = r
        .get("logsBloom")
        .and_then(|b| b.as_str())
        .is_some_and(|b| b.trim_start_matches("0x").bytes().all(|c| c == b'0'));
    if r.get("logs").is_some_and(|l| l.is_null()) && zero_bloom {
        let mut c = r.clone();
        c["logs"] = serde_json::json!([]);
        return std::borrow::Cow::Owned(c);
    }
    std::borrow::Cow::Borrowed(r)
}

/// Decode an `eth_getBlockReceipts` result array into `(trie values, per-receipt logs)`,
/// both in transaction-index order. Also checks that the RPC's own
/// `transactionIndex` fields are `0..n` in order — if the endpoint served the
/// receipts out of order the trie key assignment would be silently wrong.
/// `0x7E` deposits are accepted only when `chain_id` is an OP-stack chain, Nitro
/// types only on an Orbit chain.
pub fn values_and_logs_from_rpc(
    result: &serde_json::Value,
    chain_id: u64,
) -> Result<BlockReceipts> {
    use alloy::eips::eip2718::Encodable2718;
    let arr = result
        .as_array()
        .context("eth_getBlockReceipts result is not an array")?;
    let mut values = Vec::with_capacity(arr.len());
    let mut logs = Vec::with_capacity(arr.len());
    for (i, r) in arr.iter().enumerate() {
        let fixed = logs_null_as_empty(r);
        let r: &serde_json::Value = &fixed;
        let reported = r
            .get("transactionIndex")
            .and_then(|v| v.as_str())
            .map(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16))
            .transpose()
            .context("transactionIndex is not a quantity")?;
        ensure!(
            reported == Some(i as u64),
            "eth_getBlockReceipts returned receipts out of order: position {i} reports transactionIndex {reported:?}"
        );
        let ty = r.get("type").and_then(|v| v.as_str()).unwrap_or("0x0");
        if u8::from_str_radix(ty.trim_start_matches("0x"), 16).ok() == Some(DEPOSIT_TX_TYPE) {
            ensure!(
                deposits_allowed(chain_id),
                "receipt {i} is a 0x7E deposit but chain {chain_id} is not an OP-stack chain"
            );
            let (v, l) =
                deposit_value_from_rpc(r).with_context(|| format!("deposit receipt {i}"))?;
            values.push(v);
            logs.push(l);
            continue;
        }
        if let Some(t) = u8::from_str_radix(ty.trim_start_matches("0x"), 16)
            .ok()
            .filter(|t| NITRO_TX_TYPES.contains(t))
        {
            ensure!(
                nitro_allowed(chain_id),
                "receipt {i} is a Nitro 0x{t:02x} receipt but chain {chain_id} is not an Orbit chain"
            );
            let (v, l) =
                nitro_value_from_rpc(r, t).with_context(|| format!("Nitro receipt {i}"))?;
            values.push(v);
            logs.push(l);
            continue;
        }
        let rcpt: alloy::rpc::types::TransactionReceipt =
            serde_json::from_value(r.clone()).with_context(|| format!("receipt {i} JSON parse"))?;
        let env = rcpt.inner.into_primitives_receipt();
        values.push(env.encoded_2718());
        logs.push(
            env.logs()
                .iter()
                .map(|l| Log {
                    address: l.address.into_array(),
                    topics: l.topics().iter().map(|t| t.0).collect(),
                    data: l.data.data.to_vec(),
                })
                .collect(),
        );
    }
    Ok((values, logs))
}

/// The receipts root over pre-encoded trie values, under two trie
/// implementations ([`crate::trie::ordered_root`]).
pub fn receipts_root(values: &[Vec<u8>]) -> Result<[u8; 32]> {
    crate::trie::ordered_root(values)
}

// ---------------------------------------------------------------------------
// The verify path: RLP straight off the blob bytes.
// ---------------------------------------------------------------------------

/// One RLP item: `(payload, total_len, is_list)`.
pub fn rlp_item(b: &[u8]) -> Result<(&[u8], usize, bool)> {
    let first = *b.first().context("RLP: empty input")?;
    let (hdr, len, is_list) = match first {
        0x00..=0x7f => (0usize, 1usize, false),
        0x80..=0xb7 => (1, (first - 0x80) as usize, false),
        0xb8..=0xbf => long(b, first - 0xb7, false)?,
        0xc0..=0xf7 => (1, (first - 0xc0) as usize, true),
        0xf8..=0xff => long(b, first - 0xf7, true)?,
    };
    ensure!(
        len <= b.len().saturating_sub(hdr),
        "RLP: item runs past the end"
    );
    Ok((&b[hdr..hdr + len], hdr + len, is_list))
}

fn long(b: &[u8], nlen: u8, is_list: bool) -> Result<(usize, usize, bool)> {
    let nlen = nlen as usize;
    ensure!(b.len() > nlen, "RLP: truncated length prefix");
    let lb = &b[1..1 + nlen];
    ensure!(lb[0] != 0, "RLP: non-minimal length prefix");
    let mut len = 0usize;
    for &x in lb {
        len = len
            .checked_mul(256)
            .and_then(|v| v.checked_add(x as usize))
            .context("RLP: length overflow")?;
    }
    ensure!(len >= 56, "RLP: non-minimal long form");
    Ok((1 + nlen, len, is_list))
}

/// Split an RLP list payload into its items.
pub fn rlp_list_items(payload: &[u8]) -> Result<Vec<(&[u8], bool)>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < payload.len() {
        let (p, total, is_list) = rlp_item(&payload[at..])?;
        out.push((p, is_list));
        at += total;
    }
    Ok(out)
}

fn fixed<const N: usize>(b: &[u8], what: &str) -> Result<[u8; N]> {
    ensure!(b.len() == N, "{what}: expected {N} bytes, got {}", b.len());
    Ok(b.try_into().expect("length checked"))
}

/// Recover a receipt's logs from its trie VALUE bytes.
///
/// `type ‖ rlp(receipt)` for EIP-2718 types `0x01..=0x04` (and `0x7E` on an
/// OP-stack chain, `0x64..=0x6A` on an Orbit chain), bare `rlp(receipt)` for legacy. The body is
/// `[status_or_postState, cumulativeGasUsed, logsBloom(256), logs]`, plus a
/// deposit's `depositNonce` / `depositReceiptVersion`; a log is
/// `[address(20), topics[](32 each, 0..4), data]`.
pub fn logs_from_value(value: &[u8], chain_id: u64) -> Result<Vec<Log>> {
    let first = *value.first().context("empty receipt value")?;
    let deposit = first == DEPOSIT_TX_TYPE && deposits_allowed(chain_id);
    let nitro = NITRO_TX_TYPES.contains(&first) && nitro_allowed(chain_id);
    let body = match first {
        0x01..=0x04 => &value[1..],
        _ if deposit || nitro => &value[1..],
        0x00 => bail!("receipt value starts with a 0x00 type byte"),
        0x05..=0x7f => bail!("unsupported EIP-2718 receipt type 0x{first:02x}"),
        _ => value,
    };
    let (payload, total, is_list) = rlp_item(body)?;
    ensure!(is_list, "receipt body is not an RLP list");
    ensure!(
        total == body.len(),
        "receipt body has {} trailing bytes",
        body.len() - total
    );
    let fields = rlp_list_items(payload)?;
    if deposit {
        ensure!(
            (4..=6).contains(&fields.len()),
            "deposit receipt has {} fields, expected 4..=6",
            fields.len()
        );
        for (f, is_list) in &fields[4..] {
            ensure!(
                !is_list && f.len() <= 8 && f.first() != Some(&0),
                "deposit nonce/version is not a canonical u64"
            );
        }
    } else {
        ensure!(
            fields.len() == 4,
            "receipt body has {} fields, expected 4",
            fields.len()
        );
    }
    let status = fields[0].0;
    // Post-Byzantium: a 0/1 status scalar. Pre-Byzantium: a 32-byte postState root.
    ensure!(
        (!fields[0].1 && status.len() <= 1) || status.len() == 32,
        "receipt field 0 is neither a status scalar nor a 32-byte postState"
    );
    let _ = fixed::<256>(fields[2].0, "logsBloom")?;
    ensure!(fields[3].1, "receipt logs field is not a list");

    let mut out = Vec::new();
    for (log_payload, is_list) in rlp_list_items(fields[3].0)? {
        ensure!(is_list, "log is not an RLP list");
        let l = rlp_list_items(log_payload)?;
        ensure!(l.len() == 3, "log has {} fields, expected 3", l.len());
        let address = fixed::<20>(l[0].0, "log address")?;
        ensure!(l[1].1, "log topics field is not a list");
        let mut topics = Vec::new();
        for (t, tl) in rlp_list_items(l[1].0)? {
            ensure!(!tl, "a topic is an RLP list");
            topics.push(fixed::<32>(t, "topic")?);
        }
        ensure!(
            topics.len() <= 4,
            "log has {} topics, at most 4 allowed",
            topics.len()
        );
        out.push(Log {
            address,
            topics,
            data: l[2].0.to_vec(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_logs_count_only_under_a_zero_bloom() {
        let z = format!("0x{}", "0".repeat(512));
        let r = serde_json::json!({"logs": null, "logsBloom": z});
        assert_eq!(logs_null_as_empty(&r)["logs"], serde_json::json!([]));
        let nz = format!("0x01{}", "0".repeat(510));
        let r = serde_json::json!({"logs": null, "logsBloom": nz});
        assert!(logs_null_as_empty(&r)["logs"].is_null());
    }

    /// Round-trip a synthetic receipt through alloy's 2718 encoder and our RLP
    /// reader, legacy and typed, empty logs and 0/1/4-topic logs.
    #[test]
    fn logs_from_value_matches_alloy() {
        use alloy::consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom};
        use alloy::eips::eip2718::Encodable2718;
        use alloy_primitives::{Address, Bytes, Log as PLog, LogData, B256};

        let mk = |n_topics: usize, data: &[u8]| PLog {
            address: Address::from([0x42u8; 20]),
            data: LogData::new(
                (0..n_topics)
                    .map(|i| B256::from([i as u8 + 1; 32]))
                    .collect(),
                Bytes::copy_from_slice(data),
            )
            .unwrap(),
        };
        let logs = vec![mk(0, b""), mk(1, b"hi"), mk(4, &[7u8; 100])];
        let r = Receipt {
            status: true.into(),
            cumulative_gas_used: 21_000,
            logs: logs.clone(),
        };
        let wb = ReceiptWithBloom::new(r, alloy_primitives::Bloom::default());
        for env in [
            ReceiptEnvelope::Legacy(wb.clone()),
            ReceiptEnvelope::Eip2930(wb.clone()),
            ReceiptEnvelope::Eip1559(wb.clone()),
            ReceiptEnvelope::Eip4844(wb.clone()),
            ReceiptEnvelope::Eip7702(wb.clone()),
        ] {
            let v = env.encoded_2718();
            let got = logs_from_value(&v, 1).unwrap();
            assert_eq!(got.len(), logs.len());
            for (g, l) in got.iter().zip(&logs) {
                assert_eq!(g.address, l.address.into_array());
                assert_eq!(g.topics, l.topics().iter().map(|t| t.0).collect::<Vec<_>>());
                assert_eq!(g.data, l.data.data.to_vec());
            }
        }
    }

    fn base_kat_values() -> Vec<Vec<u8>> {
        let p = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../fixtures/base/48932815.receipts.bin"
        );
        let raw = std::fs::read(p).expect("base fixture");
        let (mut out, mut at) = (Vec::new(), 0usize);
        while at < raw.len() {
            let n = u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
            out.push(raw[at + 4..at + 4 + n].to_vec());
            at += 4 + n;
        }
        out
    }

    /// The real Base block: the verify path reads all 115 values under chain 8453
    /// (378 logs), both trie implementations rebuild the header's receiptsRoot,
    /// and the same deposit bytes are refused under chain 1.
    #[test]
    fn base_real_block_verify_path() {
        let values = base_kat_values();
        assert_eq!(values.len(), 115);
        let n_logs: usize = values
            .iter()
            .map(|v| logs_from_value(v, 8453).unwrap().len())
            .sum();
        assert_eq!(n_logs, 378);
        assert_eq!(
            hex::encode(receipts_root(&values).unwrap()),
            "e4d359ee3d786cead168e809e945246cca6a5a5c9a7ac62c2edf171141882f72"
        );
        assert!(logs_from_value(&values[0], 1).is_err());
    }

    /// The generation path's hand encoder, fed RPC-shaped JSON derived from the
    /// real deposit receipt, reproduces its exact trie value.
    #[test]
    fn deposit_encoder_reproduces_the_real_base_deposit() {
        let v = &base_kat_values()[0];
        assert_eq!(v[0], 0x7e);
        let (payload, _, _) = rlp_item(&v[1..]).unwrap();
        let f = rlp_list_items(payload).unwrap();
        assert_eq!(f.len(), 6, "post-Canyon: nonce + version");
        let q = |b: &[u8]| format!("0x{:x}", b.iter().fold(0u64, |a, x| (a << 8) | *x as u64));
        let logs: Vec<serde_json::Value> = logs_from_value(v, 8453)
            .unwrap()
            .iter()
            .map(|l| {
                serde_json::json!({
                    "address": format!("0x{}", hex::encode(l.address)),
                    "topics": l.topics.iter().map(|t| format!("0x{}", hex::encode(t))).collect::<Vec<_>>(),
                    "data": format!("0x{}", hex::encode(&l.data)),
                })
            })
            .collect();
        let json = serde_json::json!({
            "type": "0x7e",
            "transactionIndex": "0x0",
            "status": q(f[0].0),
            "cumulativeGasUsed": q(f[1].0),
            "logsBloom": format!("0x{}", hex::encode(f[2].0)),
            "logs": logs,
            "depositNonce": q(f[4].0),
            "depositReceiptVersion": q(f[5].0),
        });
        let (enc, _) = deposit_value_from_rpc(&json).unwrap();
        assert_eq!(&enc, v, "hand encoder == Base's own bytes");
        let arr = serde_json::Value::Array(vec![json.clone()]);
        let (vals, _) = values_and_logs_from_rpc(&arr, 8453).unwrap();
        assert_eq!(&vals[0], v);
        assert!(values_and_logs_from_rpc(&arr, 1).is_err());
    }

    fn orbit_fixture(name: &str) -> (serde_json::Value, serde_json::Value) {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/orbit");
        let rd = |ext: &str| -> serde_json::Value {
            serde_json::from_slice(&std::fs::read(format!("{dir}/{name}.{ext}.json")).unwrap())
                .unwrap()
        };
        (rd("block"), rd("receipts"))
    }

    /// Real Nitro blocks: every receipt (internal 0x6A, submit-retryable 0x69,
    /// retry 0x68, legacy, 1559) encodes to values that rebuild the header's
    /// receiptsRoot under both tries, and the verify path reads back the same logs.
    #[test]
    fn nitro_real_blocks_rebuild_the_receipts_root() {
        for (name, chain, n_logs, types) in [
            (
                "robinhood-69562157",
                4663u64,
                2usize,
                vec![0x6a, 0x69, 0x68],
            ),
            ("arbitrum-one-507965087", 42161, 32, vec![0x6a, 0x00, 0x02]),
        ] {
            let (block, receipts) = orbit_fixture(name);
            let (header_rlp_number, header_rlp) =
                crate::header::header_rlp_from_block_json(&block).unwrap();
            let info = crate::header::decode_positional(&header_rlp).unwrap();
            assert_eq!(info.number, header_rlp_number);
            let (values, logs) = values_and_logs_from_rpc(&receipts, chain).unwrap();
            assert_eq!(
                receipts_root(&values).unwrap(),
                info.receipts_root,
                "{name}"
            );
            let got_types: Vec<u8> = values
                .iter()
                .map(|v| if v[0] >= 0xc0 { 0x00 } else { v[0] })
                .collect();
            for t in types {
                assert!(got_types.contains(&t), "{name}: type 0x{t:02x}");
            }
            assert_eq!(logs.iter().map(|l| l.len()).sum::<usize>(), n_logs);
            for (v, l) in values.iter().zip(&logs) {
                assert_eq!(&logs_from_value(v, chain).unwrap(), l);
            }
            // The same bytes are refused off the Orbit chains.
            let nitro = values.iter().find(|v| v[0] == 0x6a).unwrap();
            assert!(logs_from_value(nitro, 1).is_err());
            assert!(logs_from_value(nitro, 10).is_err());
            assert!(values_and_logs_from_rpc(&receipts, 1).is_err());
        }
    }

    /// Base before Canyon: a deposit receipt reports `depositNonce` (from
    /// Regolith) but its trie value is the plain 4 fields until Canyon adds
    /// `depositReceiptVersion`, so these blocks rebuild their receiptsRoot only
    /// without the nonce. Block 1 holds only its L1 info deposit; 9,101,526 is
    /// the last block before Canyon.
    #[test]
    fn deposits_before_canyon_rebuild_the_receipts_root() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/opstack");
        let rd = |name: &str, ext: &str| -> serde_json::Value {
            serde_json::from_slice(&std::fs::read(format!("{dir}/{name}.{ext}.json")).unwrap())
                .unwrap()
        };
        for name in ["base-1", "base-9101526"] {
            let (block, receipts) = (rd(name, "full"), rd(name, "receipts"));
            assert!(receipts[0].get("depositNonce").is_some(), "{name}");
            assert!(receipts[0].get("depositReceiptVersion").is_none(), "{name}");
            let (_, header_rlp) = crate::header::header_rlp_from_block_json(&block).unwrap();
            let info = crate::header::decode_positional(&header_rlp).unwrap();
            let (values, logs) = values_and_logs_from_rpc(&receipts, 8453).unwrap();
            assert_eq!(
                receipts_root(&values).unwrap(),
                info.receipts_root,
                "{name}"
            );
            let (payload, _, _) = rlp_item(&values[0][1..]).unwrap();
            assert_eq!(rlp_list_items(payload).unwrap().len(), 4, "{name}");
            for (v, l) in values.iter().zip(&logs) {
                assert_eq!(&logs_from_value(v, 8453).unwrap(), l);
            }
        }
    }

    /// A Nitro receipt never carries a deposit tail: a value with a 5th field is
    /// refused even on an Orbit chain.
    #[test]
    fn nitro_receipt_with_a_tail_is_refused() {
        let (_, receipts) = orbit_fixture("robinhood-69562157");
        let (values, _) = values_and_logs_from_rpc(&receipts, 4663).unwrap();
        let v = &values[0];
        assert_eq!(v[0], 0x6a);
        let (payload, _, _) = rlp_item(&v[1..]).unwrap();
        let mut items: Vec<u8> = payload.to_vec();
        items.push(0x01);
        let mut forged = vec![0x6a];
        alloy_rlp::Header {
            list: true,
            payload_length: items.len(),
        }
        .encode(&mut forged);
        forged.extend_from_slice(&items);
        assert!(logs_from_value(&forged, 4663).is_err());
    }

    /// A tampered byte must be caught by the root, and structural damage by the
    /// RLP reader.
    #[test]
    fn rejects_malformed_values() {
        assert!(logs_from_value(&[], 1).is_err());
        assert!(logs_from_value(&[0x00, 0xc0], 1).is_err());
        assert!(logs_from_value(&[0x7f, 0xc0], 1).is_err());
        // A list of 3, not 4.
        assert!(logs_from_value(&[0xc3, 0x01, 0x02, 0x03], 1).is_err());
        // Truncated.
        assert!(logs_from_value(&[0xf8, 0x40, 0x01], 1).is_err());
    }

    /// Both trie implementations on real receipt-shaped values, and the empty
    /// root. (Shorter values, down to the shortest transaction, are covered in
    /// `transactions`.)
    #[test]
    fn trie_implementations_agree() {
        use alloy::consensus::{Receipt, ReceiptEnvelope, ReceiptWithBloom};
        use alloy::eips::eip2718::Encodable2718;
        use alloy_primitives::{Address, Bytes, Log as PLog, LogData, B256};

        let values: Vec<Vec<u8>> = (0u16..300)
            .map(|i| {
                let logs: Vec<PLog> = (0..(i % 4))
                    .map(|j| PLog {
                        address: Address::from([(i as u8).wrapping_add(j as u8); 20]),
                        data: LogData::new(
                            (0..(j % 4))
                                .map(|k| B256::from([k as u8 + 1; 32]))
                                .collect(),
                            Bytes::from(vec![i as u8; 10 + (i as usize % 90)]),
                        )
                        .unwrap(),
                    })
                    .collect();
                let r = Receipt {
                    status: (i % 2 == 0).into(),
                    cumulative_gas_used: 21_000 * i as u64,
                    logs,
                };
                let wb = ReceiptWithBloom::new(r, alloy_primitives::Bloom::from([i as u8; 256]));
                match i % 3 {
                    0 => ReceiptEnvelope::Legacy(wb),
                    1 => ReceiptEnvelope::Eip1559(wb),
                    _ => ReceiptEnvelope::Eip4844(wb),
                }
                .encoded_2718()
            })
            .collect();
        assert!(
            values.iter().all(|v| v.len() >= 259),
            "a consensus receipt is always >= 259 bytes"
        );
        for n in [1usize, 2, 17, 128, 129, 255, 256, 300] {
            let r = receipts_root(&values[..n]).unwrap();
            assert_eq!(
                r,
                alloy_trie::root::ordered_trie_root_encoded(&values[..n]).0
            );
        }
        assert_eq!(
            alloy_trie::root::ordered_trie_root_encoded::<Vec<u8>>(&[]).0,
            willow_receipts_trie::empty_root()
        );
    }

    /// A long-form length near the top of `usize` is refused, never wrapped
    /// into a slice past the input (the archive's bytes reach this parser).
    #[test]
    fn an_rlp_length_past_the_input_is_refused() {
        for b in [
            vec![0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xf8],
            vec![0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0x01],
            vec![0xb8, 0x40, 0x01],
        ] {
            let e = rlp_item(&b).unwrap_err();
            assert!(e.to_string().contains("past the end"), "{e}");
        }
        assert!(rlp_list_items(&[0xbf, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xf8]).is_err());
    }
}
