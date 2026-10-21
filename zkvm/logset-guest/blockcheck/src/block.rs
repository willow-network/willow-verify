//! A block's header, body and receipts, checked against its block hash.

use crate::header::{self, HeaderInfo};
use crate::receipts::{self, Log};
use crate::transactions;
use alloy::eips::eip4895::Withdrawal;
use anyhow::{ensure, Context, Result};
use serde_json::Value;

/// A header whose RLP hashes to the block hash it was reported with.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedHeader {
    pub info: HeaderInfo,
    pub timestamp: u64,
    pub rlp: Vec<u8>,
    /// Every header field, decoded from `rlp`.
    pub header: alloy::consensus::Header,
}

/// Check an `eth_getBlockByNumber`/`ByHash` result: its header fields encode to
/// RLP whose keccak is its `hash`, and two decoders read that RLP the same way.
/// Fields outside the header RLP (`transactions`, `size`, chain extras) are
/// checked by [`verify_body`].
pub fn verify_header(block: &Value) -> Result<VerifiedHeader> {
    let (number, rlp) = header::header_rlp_from_block_json(block)?;
    let info = header::decode_alloy(&rlp)?;
    ensure!(
        info == header::decode_positional(&rlp)?,
        "the two header decoders disagree on block {number}"
    );
    ensure!(
        info.number == number,
        "block {number} decodes as block {}",
        info.number
    );
    let header = {
        use alloy_rlp::Decodable;
        alloy::consensus::Header::decode(&mut rlp.as_slice()).context("header RLP decode")?
    };
    Ok(VerifiedHeader {
        info,
        timestamp: header.timestamp,
        rlp,
        header,
    })
}

/// A block's body, checked against its verified header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifiedBody {
    /// Transaction hashes, in block order: keccak of the bytes that rebuild
    /// the header's transactionsRoot.
    pub tx_hashes: Vec<[u8; 32]>,
    /// Those bytes: each transaction's transactions-trie value, in block order.
    pub tx_values: Vec<Vec<u8>>,
    /// The withdrawals, when the header commits to a withdrawals list.
    pub withdrawals: Option<Vec<Withdrawal>>,
    /// Length of the block's RLP `[header, transactions, uncles, withdrawals?]`:
    /// the `size` nodes report.
    pub size: u64,
}

/// Check the body of an `eth_getBlockByNumber(n, true)` result against its
/// verified header:
///
/// - `transactions` (full objects) encode, with `chain_id`'s transaction types,
///   to values that rebuild transactionsRoot under two tries; each hash is the
///   keccak of its value and must be the one the node reports.
/// - `uncles` must be empty and ommersHash the empty list's: no chain served
///   here has uncles, and their hashes would not be checkable without their
///   headers.
/// - `withdrawals` must rebuild withdrawalsRoot. OP-stack blocks carry none
///   (Canyon), and from Isthmus on their withdrawalsRoot commits to the
///   L2ToL1MessagePasser storage instead, so there the list must be empty.
/// - `size` is computed from the verified parts, never taken from the node.
pub fn verify_body(chain_id: u64, h: &VerifiedHeader, block: &Value) -> Result<VerifiedBody> {
    if let Some(hash) = block.get("hash").filter(|v| !v.is_null()) {
        ensure!(
            hex32(hash) == Some(h.info.hash),
            "the block is {hash}, not 0x{}",
            hex::encode(h.info.hash)
        );
    }
    let txs = transactions::from_rpc(
        block.get("transactions").context("no transactions")?,
        chain_id,
    )?;
    let root = crate::trie::ordered_root(&txs.values)?;
    ensure!(
        root == h.header.transactions_root.0,
        "rebuilt transactions root 0x{} != header transactionsRoot 0x{}",
        hex::encode(root),
        hex::encode(h.header.transactions_root)
    );
    for (i, tx) in block["transactions"]
        .as_array()
        .expect("checked")
        .iter()
        .enumerate()
    {
        if let Some(b) = tx.get("blockHash").filter(|v| !v.is_null()) {
            ensure!(
                hex32(b) == Some(h.info.hash),
                "transaction {i} is for block hash {b}"
            );
        }
    }

    let uncles = block.get("uncles").filter(|v| !v.is_null());
    ensure!(
        uncles.is_none_or(|u| u.as_array().is_some_and(Vec::is_empty)),
        "the block lists uncles: not served"
    );
    ensure!(
        h.header.ommers_hash == alloy::consensus::EMPTY_OMMER_ROOT_HASH,
        "the header commits to uncles: not served"
    );

    let listed = block.get("withdrawals").filter(|v| !v.is_null());
    let withdrawals = match h.header.withdrawals_root {
        None => {
            ensure!(
                listed.is_none_or(|w| w.as_array().is_some_and(Vec::is_empty)),
                "withdrawals listed but the header has no withdrawalsRoot"
            );
            None
        }
        Some(_) if receipts::deposits_allowed(chain_id) => {
            ensure!(
                listed.is_none_or(|w| w.as_array().is_some_and(Vec::is_empty)),
                "an OP-stack block lists withdrawals"
            );
            Some(vec![])
        }
        Some(want) => {
            let list: Vec<Withdrawal> =
                serde_json::from_value(listed.context("no withdrawals")?.clone())
                    .context("withdrawals JSON parse")?;
            let values: Vec<Vec<u8>> = list.iter().map(alloy_rlp::encode).collect();
            let root = crate::trie::ordered_root(&values)?;
            ensure!(
                root == want.0,
                "rebuilt withdrawals root 0x{} != header withdrawalsRoot 0x{}",
                hex::encode(root),
                hex::encode(want)
            );
            Some(list)
        }
    };

    let size = block_size(&h.rlp, &txs.values, withdrawals.as_deref());
    Ok(VerifiedBody {
        tx_hashes: txs.hashes,
        tx_values: txs.values,
        withdrawals,
        size: size as u64,
    })
}

/// `len(rlp([header, transactions, [], withdrawals?]))`, typed transactions as
/// strings (geth's `Block.Size`).
fn block_size(header_rlp: &[u8], txs: &[Vec<u8>], withdrawals: Option<&[Withdrawal]>) -> usize {
    use alloy_rlp::Encodable;
    let list = |payload: usize| {
        alloy_rlp::Header {
            list: true,
            payload_length: payload,
        }
        .length()
            + payload
    };
    let txs: usize = txs
        .iter()
        .map(|v| match v.first() {
            Some(b) if *b >= 0xc0 => v.len(),
            _ => v.as_slice().length(),
        })
        .sum();
    let mut payload = header_rlp.len() + list(txs) + list(0);
    if let Some(w) = withdrawals {
        payload += list(w.iter().map(Encodable::length).sum());
    }
    list(payload)
}

/// Check an `eth_getBlockReceipts` result against the block `header` describes:
/// every receipt names this block, the receipts encode (with `chain_id`'s
/// receipt types) to trie values that rebuild the header's receiptsRoot, and the
/// logs read back from those value bytes equal the ones the encoder took from
/// the JSON. Returns each receipt's logs, in transaction order.
pub fn verify_receipts(
    chain_id: u64,
    header: &HeaderInfo,
    receipts: &Value,
) -> Result<Vec<Vec<Log>>> {
    let list = receipts
        .as_array()
        .context("eth_getBlockReceipts result is not an array")?;
    for (i, r) in list.iter().enumerate() {
        if let Some(h) = r.get("blockHash").filter(|v| !v.is_null()) {
            ensure!(
                hex32(h) == Some(header.hash),
                "receipt {i} is for block hash {h}, not 0x{}",
                hex::encode(header.hash)
            );
        }
        if let Some(n) = r.get("blockNumber").filter(|v| !v.is_null()) {
            ensure!(
                quantity(n) == Some(header.number),
                "receipt {i} is for block {n}, not {}",
                header.number
            );
        }
    }
    let (values, encoded) = receipts::values_and_logs_from_rpc(receipts, chain_id)?;
    let root = receipts::receipts_root(&values)?;
    ensure!(
        root == header.receipts_root,
        "rebuilt receipts root 0x{} != header receiptsRoot 0x{}",
        hex::encode(root),
        hex::encode(header.receipts_root)
    );
    values
        .iter()
        .zip(&encoded)
        .enumerate()
        .map(|(i, (value, logs))| {
            let read = receipts::logs_from_value(value, chain_id)
                .with_context(|| format!("receipt {i}"))?;
            ensure!(
                &read == logs,
                "receipt {i}: the logs read from its trie value differ from the encoded ones"
            );
            Ok(read)
        })
        .collect()
}

/// A log of a verified block at its position in the block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockLog {
    pub log: Log,
    /// Position of its receipt (= its transaction) in the block.
    pub transaction_index: u64,
    /// Position of the log among all the block's logs.
    pub log_index: u64,
}

/// Every log of a block, in block order, from its receipts' logs.
pub fn block_logs(receipts: &[Vec<Log>]) -> Vec<BlockLog> {
    let mut out = Vec::with_capacity(receipts.iter().map(Vec::len).sum());
    for (tx, logs) in receipts.iter().enumerate() {
        for log in logs {
            out.push(BlockLog {
                log: log.clone(),
                transaction_index: tx as u64,
                log_index: out.len() as u64,
            });
        }
    }
    out
}

/// Whether a logs bloom may hold `input` (an address or a topic). `false` means
/// no log in the block has it.
pub fn bloom_may_contain(bloom: &[u8; 256], input: &[u8]) -> bool {
    alloy_primitives::Bloom::new(*bloom).contains_input(alloy_primitives::BloomInput::Raw(input))
}

fn hex32(v: &Value) -> Option<[u8; 32]> {
    hex::decode(v.as_str()?.strip_prefix("0x")?)
        .ok()?
        .try_into()
        .ok()
}

fn quantity(v: &Value) -> Option<u64> {
    u64::from_str_radix(v.as_str()?.strip_prefix("0x")?, 16).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn orbit(name: &str) -> (Value, Value) {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/orbit");
        let rd = |ext: &str| -> Value {
            serde_json::from_slice(&std::fs::read(format!("{dir}/{name}.{ext}.json")).unwrap())
                .unwrap()
        };
        (rd("block"), rd("receipts"))
    }

    /// A real Robinhood block verifies end to end, and its logs carry block-wide
    /// log indexes and their receipt's position.
    #[test]
    fn real_nitro_block_verifies_with_positions() {
        let (block, receipts) = orbit("robinhood-69562157");
        let h = verify_header(&block).unwrap();
        assert_eq!(h.info.number, 69_562_157);
        assert_eq!(
            Some(h.timestamp),
            quantity(&block["timestamp"]),
            "timestamp from the verified RLP"
        );
        let per_receipt = verify_receipts(4663, &h.info, &receipts).unwrap();
        let logs = block_logs(&per_receipt);
        let json: Vec<&Value> = receipts
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|r| r["logs"].as_array().unwrap())
            .collect();
        assert_eq!(logs.len(), json.len());
        for (l, j) in logs.iter().zip(json) {
            assert_eq!(Some(l.log_index), quantity(&j["logIndex"]));
            assert_eq!(Some(l.transaction_index), quantity(&j["transactionIndex"]));
            assert!(bloom_may_contain(&h.info.logs_bloom, &l.log.address));
        }
        // The Nitro receipt types are refused off the Orbit chains.
        assert!(verify_receipts(1, &h.info, &receipts).is_err());
    }

    fn orbit_full(name: &str) -> Value {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures/orbit");
        serde_json::from_slice(&std::fs::read(format!("{dir}/{name}.full.json")).unwrap()).unwrap()
    }

    /// Real Nitro blocks with full transactions (internal 0x6A, submit-retryable
    /// 0x69, retry 0x68, legacy, 1559): the body rebuilds transactionsRoot, the
    /// hashes are the node's, `size` is the node's, and the Nitro JSON extras
    /// are what the verified header carries.
    #[test]
    fn real_nitro_bodies_verify() {
        for (name, chain, types) in [
            ("robinhood-69562157", 4663u64, vec!["0x6a", "0x69", "0x68"]),
            ("arbitrum-one-507965087", 42161, vec!["0x6a", "0x2", "0x0"]),
        ] {
            let full = orbit_full(name);
            let (block, receipts) = orbit(name);
            let h = verify_header(&full).unwrap();
            assert_eq!(h, verify_header(&block).unwrap(), "{name}");
            for t in types {
                assert!(
                    full["transactions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|x| x["type"] == t),
                    "{name}: {t}"
                );
            }
            let body = verify_body(chain, &h, &full).unwrap();
            let listed: Vec<[u8; 32]> = block["transactions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|t| hex32(t).unwrap())
                .collect();
            assert_eq!(body.tx_hashes, listed, "{name}");
            let in_receipts: Vec<[u8; 32]> = receipts
                .as_array()
                .unwrap()
                .iter()
                .map(|r| hex32(&r["transactionHash"]).unwrap())
                .collect();
            assert_eq!(body.tx_hashes, in_receipts, "{name}");
            assert_eq!(Some(body.size), quantity(&block["size"]), "{name}");
            assert_eq!(body.withdrawals, None);
            let info = header::nitro_header_info(&h.header).unwrap();
            assert_eq!(
                Some(info.l1_block_number),
                quantity(&block["l1BlockNumber"])
            );
            assert_eq!(Some(info.send_count), quantity(&block["sendCount"]));
            assert_eq!(Some(info.send_root), hex32(&block["sendRoot"]));
            // The Nitro types are refused off the Orbit chains, and a block
            // listing only hashes has no body to check.
            assert!(verify_body(1, &h, &full).is_err());
            assert!(verify_body(10, &h, &full).is_err());
            assert!(verify_body(chain, &h, &block).is_err());
        }
    }

    /// Every way a node can lie about a block's body is refused.
    #[test]
    fn tampered_bodies_are_refused() {
        let full = orbit_full("robinhood-69562157");
        let h = verify_header(&full).unwrap();
        assert!(verify_body(4663, &h, &full).is_ok());
        let renumber = |b: &mut Value| {
            for (i, t) in b["transactions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .enumerate()
            {
                t["transactionIndex"] = Value::from(format!("0x{i:x}"));
            }
        };
        let rehash = |b: &mut Value, i: usize| {
            let v = crate::transactions::value_from_rpc(&b["transactions"][i], 4663).unwrap();
            b["transactions"][i]["hash"] =
                Value::from(format!("0x{}", hex::encode(alloy_primitives::keccak256(v))));
        };
        type Edit = Box<dyn Fn(&mut Value)>;
        let cases: Vec<(&str, Edit)> = vec![
            (
                "retry value",
                Box::new(|b| b["transactions"][2]["value"] = Value::from("0x1")),
            ),
            (
                "retry value, rehashed",
                Box::new(move |b| {
                    b["transactions"][2]["value"] = Value::from("0x1");
                    rehash(b, 2);
                }),
            ),
            (
                "retryable's retryTo dropped, rehashed",
                Box::new(move |b| {
                    b["transactions"][1]
                        .as_object_mut()
                        .unwrap()
                        .remove("retryTo");
                    rehash(b, 1);
                }),
            ),
            (
                "internal call data, rehashed",
                Box::new(move |b| {
                    b["transactions"][0]["input"] = Value::from("0x6bf6a42d");
                    rehash(b, 0);
                }),
            ),
            (
                "wrong hash",
                Box::new(|b| {
                    b["transactions"][1]["hash"] = Value::from(format!("0x{}", "55".repeat(32)))
                }),
            ),
            (
                "swapped",
                Box::new(|b| b["transactions"].as_array_mut().unwrap().swap(1, 2)),
            ),
            (
                "swapped and renumbered",
                Box::new(move |b| {
                    b["transactions"].as_array_mut().unwrap().swap(1, 2);
                    renumber(b);
                }),
            ),
            (
                "dropped and renumbered",
                Box::new(move |b| {
                    b["transactions"].as_array_mut().unwrap().remove(2);
                    renumber(b);
                }),
            ),
            (
                "duplicated and renumbered",
                Box::new(move |b| {
                    let t = b["transactions"][2].clone();
                    b["transactions"].as_array_mut().unwrap().push(t);
                    renumber(b);
                }),
            ),
            (
                "an uncle",
                Box::new(|b| b["uncles"] = serde_json::json!([format!("0x{}", "66".repeat(32))])),
            ),
            (
                "withdrawals on a chain without them",
                Box::new(|b| {
                    b["withdrawals"] = serde_json::json!([{"index": "0x1",
                        "validatorIndex": "0x2", "address": format!("0x{}", "88".repeat(20)),
                        "amount": "0x3"}])
                }),
            ),
            (
                "another block's hash",
                Box::new(|b| b["hash"] = Value::from(format!("0x{}", "77".repeat(32)))),
            ),
        ];
        for (what, edit) in cases {
            let mut bad = full.clone();
            edit(&mut bad);
            assert!(verify_body(4663, &h, &bad).is_err(), "{what} was accepted");
        }
    }

    #[test]
    fn tampered_header_or_receipts_are_refused() {
        let (block, receipts) = orbit("arbitrum-one-507965087");
        let h = verify_header(&block).unwrap();
        let mut bad = block.clone();
        bad["timestamp"] = Value::from("0x1");
        assert!(verify_header(&bad).is_err());
        let mut bad = receipts.clone();
        let logs = bad[1]["logs"].as_array_mut().unwrap();
        if let Some(l) = logs.first_mut() {
            l["data"] = Value::from("0x01");
        } else {
            bad[1]["cumulativeGasUsed"] = Value::from("0x1");
        }
        assert!(verify_receipts(42161, &h.info, &bad).is_err());
        let mut other = receipts.clone();
        other[0]["blockHash"] = Value::from(format!("0x{}", hex::encode([7u8; 32])));
        assert!(verify_receipts(42161, &h.info, &other).is_err());
        assert!(!bloom_may_contain(&[0u8; 256], &[1u8; 20]));
    }
}
