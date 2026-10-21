//! Transactions: each one's canonical consensus bytes — its transactions-trie
//! value, whose keccak is its hash — from `eth_getBlockByNumber(n, true)` JSON.
//!
//! Legacy (EIP-155 or not), 2930, 1559, 4844 (the block form, without blobs) and
//! 7702 go through alloy's `TxEnvelope`. alloy has no arms for the OP-stack
//! `0x7E` deposit or the Nitro `0x64..=0x6A` types: those are hand-encoded from
//! their RPC fields, each gated per chain like their receipts. Nothing here is
//! taken on trust: the values must rebuild the header's transactionsRoot
//! ([`crate::block::verify_body`]), which proves the encoders right block by
//! block.

use crate::receipts::{deposits_allowed, nitro_allowed, DEPOSIT_TX_TYPE, NITRO_TX_TYPES};
use alloy_primitives::U256;
use alloy_rlp::Encodable;
use anyhow::{bail, ensure, Context, Result};
use serde_json::Value;

/// Nitro `ArbitrumDepositTx`: an ETH deposit from L1.
pub const ARB_DEPOSIT: u8 = 0x64;
/// Nitro `ArbitrumUnsignedTx`: an L1 contract's unsigned call, with a nonce.
pub const ARB_UNSIGNED: u8 = 0x65;
/// Nitro `ArbitrumContractTx`: an L1 contract's unsigned call, by request id.
pub const ARB_CONTRACT: u8 = 0x66;
/// Nitro `ArbitrumRetryTx`: a retryable ticket's redeem.
pub const ARB_RETRY: u8 = 0x68;
/// Nitro `ArbitrumSubmitRetryableTx`: a retryable ticket's creation.
pub const ARB_SUBMIT_RETRYABLE: u8 = 0x69;
/// Nitro `ArbitrumInternalTx`: ArbOS's own per-block call.
pub const ARB_INTERNAL: u8 = 0x6A;

/// A block's transactions: trie values and hashes (`keccak(value)`), in order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockTransactions {
    pub values: Vec<Vec<u8>>,
    pub hashes: Vec<[u8; 32]>,
}

/// The `type` of a transaction object; legacy when absent.
pub fn tx_type(tx: &Value) -> Result<u8> {
    match tx.get("type") {
        None | Some(Value::Null) => Ok(0),
        Some(v) => {
            let s = v.as_str().context("type is not a string")?;
            let s = s.strip_prefix("0x").context("type is not 0x-prefixed")?;
            u8::from_str_radix(s, 16).context("type is not a byte")
        }
    }
}

/// One transaction object's trie value. `0x7E` only on an OP-stack chain, the
/// Nitro types only on an Orbit chain.
pub fn value_from_rpc(tx: &Value, chain_id: u64) -> Result<Vec<u8>> {
    ensure!(tx.is_object(), "not a transaction object");
    let ty = tx_type(tx)?;
    if ty == DEPOSIT_TX_TYPE {
        ensure!(
            deposits_allowed(chain_id),
            "a 0x7E deposit but chain {chain_id} is not an OP-stack chain"
        );
        return deposit_value(tx);
    }
    if NITRO_TX_TYPES.contains(&ty) {
        ensure!(
            nitro_allowed(chain_id),
            "a Nitro 0x{ty:02x} transaction but chain {chain_id} is not an Orbit chain"
        );
        return nitro_value(tx, ty);
    }
    use alloy::eips::eip2718::Encodable2718;
    use serde::Deserialize;
    let parsed: alloy::rpc::types::Transaction =
        Deserialize::deserialize(tx).context("transaction JSON parse")?;
    let value = parsed.inner.inner().encoded_2718();
    ensure!(
        value
            .first()
            .copied()
            .map(|b| if b >= 0xc0 { 0 } else { b })
            == Some(ty),
        "type 0x{ty:02x} encoded as another type"
    );
    Ok(value)
}

/// A full block's `transactions` array → trie values, in order. Each object's
/// `transactionIndex` (when given) must be its position and its `hash` must be
/// the keccak of its value.
pub fn from_rpc(txs: &Value, chain_id: u64) -> Result<BlockTransactions> {
    let list = txs.as_array().context("transactions is not an array")?;
    let mut out = BlockTransactions {
        values: Vec::with_capacity(list.len()),
        hashes: Vec::with_capacity(list.len()),
    };
    for (i, tx) in list.iter().enumerate() {
        ensure!(
            tx.is_object(),
            "transaction {i} is not an object (a block fetched without full transactions?)"
        );
        if let Some(index) = tx.get("transactionIndex").filter(|v| !v.is_null()) {
            ensure!(
                quantity(index) == Some(U256::from(i)),
                "transaction at position {i} reports transactionIndex {index}"
            );
        }
        let value = value_from_rpc(tx, chain_id).with_context(|| format!("transaction {i}"))?;
        let hash = alloy_primitives::keccak256(&value).0;
        let reported = tx
            .get("hash")
            .and_then(|v| fixed_hex::<32>(v).ok())
            .with_context(|| format!("transaction {i} has no hash"))?;
        ensure!(
            reported == hash,
            "transaction {i}: its bytes hash to 0x{}, the node reports 0x{}",
            hex::encode(hash),
            hex::encode(reported)
        );
        out.values.push(value);
        out.hashes.push(hash);
    }
    Ok(out)
}

/// `0x7E ‖ rlp([sourceHash, from, to, mint, value, gas, isSystemTx, data])`
/// (op-geth `DepositTx`; `to` and `mint` encode nil as the empty string).
fn deposit_value(tx: &Value) -> Result<Vec<u8>> {
    let mut f = Fields::default();
    f.bytes(&fixed::<32>(tx, "sourceHash")?)
        .bytes(&fixed::<20>(tx, "from")?)
        .opt_address(opt_address(tx, "to")?)
        .uint(opt_uint(tx, "mint")?.unwrap_or_default())
        .uint(uint(tx, "value")?)
        .u64(u64_field(tx, "gas")?)
        .bool(opt_bool(tx, "isSystemTx")?.unwrap_or(false))
        .bytes(&input(tx)?);
    Ok(f.typed(DEPOSIT_TX_TYPE))
}

/// The Nitro types, field for field as `core/types/arb_types.go` in
/// OffchainLabs/go-ethereum declares them (RLP of the struct, `To`/`RetryTo`
/// nil as the empty string), from the fields its `newRPCTransaction` reports:
/// `GasFeeCap` is `maxFeePerGas`, `FeeRefundAddr` is `refundTo`.
fn nitro_value(tx: &Value, ty: u8) -> Result<Vec<u8>> {
    let mut f = Fields::default();
    f.uint(uint(tx, "chainId")?);
    match ty {
        ARB_DEPOSIT => {
            f.bytes(&fixed::<32>(tx, "requestId")?)
                .bytes(&fixed::<20>(tx, "from")?)
                .bytes(&fixed::<20>(tx, "to")?)
                .uint(uint(tx, "value")?);
        }
        ARB_UNSIGNED => {
            f.bytes(&fixed::<20>(tx, "from")?)
                .u64(u64_field(tx, "nonce")?)
                .uint(uint(tx, "maxFeePerGas")?)
                .u64(u64_field(tx, "gas")?)
                .opt_address(opt_address(tx, "to")?)
                .uint(uint(tx, "value")?)
                .bytes(&input(tx)?);
        }
        ARB_CONTRACT => {
            f.bytes(&fixed::<32>(tx, "requestId")?)
                .bytes(&fixed::<20>(tx, "from")?)
                .uint(uint(tx, "maxFeePerGas")?)
                .u64(u64_field(tx, "gas")?)
                .opt_address(opt_address(tx, "to")?)
                .uint(uint(tx, "value")?)
                .bytes(&input(tx)?);
        }
        ARB_RETRY => {
            f.u64(u64_field(tx, "nonce")?)
                .bytes(&fixed::<20>(tx, "from")?)
                .uint(uint(tx, "maxFeePerGas")?)
                .u64(u64_field(tx, "gas")?)
                .opt_address(opt_address(tx, "to")?)
                .uint(uint(tx, "value")?)
                .bytes(&input(tx)?)
                .bytes(&fixed::<32>(tx, "ticketId")?)
                .bytes(&fixed::<20>(tx, "refundTo")?)
                .uint(uint(tx, "maxRefund")?)
                .uint(uint(tx, "submissionFeeRefund")?);
        }
        ARB_SUBMIT_RETRYABLE => {
            f.bytes(&fixed::<32>(tx, "requestId")?)
                .bytes(&fixed::<20>(tx, "from")?)
                .uint(uint(tx, "l1BaseFee")?)
                .uint(uint(tx, "depositValue")?)
                .uint(uint(tx, "maxFeePerGas")?)
                .u64(u64_field(tx, "gas")?)
                .opt_address(opt_address(tx, "retryTo")?)
                .uint(uint(tx, "retryValue")?)
                .bytes(&fixed::<20>(tx, "beneficiary")?)
                .uint(uint(tx, "maxSubmissionFee")?)
                .bytes(&fixed::<20>(tx, "refundTo")?)
                .bytes(&bytes(tx, "retryData")?);
        }
        ARB_INTERNAL => {
            f.bytes(&input(tx)?);
        }
        _ => bail!("0x{ty:02x} is not a Nitro transaction type this encoder knows"),
    }
    Ok(f.typed(ty))
}

/// RLP list items, appended in order.
#[derive(Default)]
struct Fields(Vec<u8>);

impl Fields {
    fn bytes(&mut self, b: &[u8]) -> &mut Self {
        b.encode(&mut self.0);
        self
    }

    /// A scalar: minimal big-endian bytes, zero as the empty string.
    fn uint(&mut self, v: U256) -> &mut Self {
        self.bytes(&v.to_be_bytes_trimmed_vec())
    }

    fn u64(&mut self, v: u64) -> &mut Self {
        self.uint(U256::from(v))
    }

    fn bool(&mut self, v: bool) -> &mut Self {
        self.uint(U256::from(v as u8))
    }

    fn opt_address(&mut self, a: Option<[u8; 20]>) -> &mut Self {
        self.bytes(a.as_ref().map_or(&[][..], |a| &a[..]))
    }

    fn typed(&self, ty: u8) -> Vec<u8> {
        let mut out = vec![ty];
        alloy_rlp::Header {
            list: true,
            payload_length: self.0.len(),
        }
        .encode(&mut out);
        out.extend_from_slice(&self.0);
        out
    }
}

fn present<'a>(tx: &'a Value, key: &str) -> Option<&'a Value> {
    tx.get(key).filter(|v| !v.is_null())
}

fn hex_str<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .with_context(|| format!("{key} is not a 0x-prefixed string"))
}

fn quantity(v: &Value) -> Option<U256> {
    let s = v.as_str()?.strip_prefix("0x")?;
    if s.is_empty() || s.len() > 64 {
        return None;
    }
    U256::from_str_radix(s, 16).ok()
}

fn opt_uint(tx: &Value, key: &str) -> Result<Option<U256>> {
    present(tx, key)
        .map(|v| quantity(v).with_context(|| format!("{key} is not a quantity")))
        .transpose()
}

fn uint(tx: &Value, key: &str) -> Result<U256> {
    opt_uint(tx, key)?.with_context(|| format!("no {key}"))
}

fn u64_field(tx: &Value, key: &str) -> Result<u64> {
    u64::try_from(uint(tx, key)?).with_context(|| format!("{key} does not fit 64 bits"))
}

fn opt_bool(tx: &Value, key: &str) -> Result<Option<bool>> {
    present(tx, key)
        .map(|v| v.as_bool().with_context(|| format!("{key} is not a bool")))
        .transpose()
}

fn bytes(tx: &Value, key: &str) -> Result<Vec<u8>> {
    let v = present(tx, key).with_context(|| format!("no {key}"))?;
    hex::decode(hex_str(v, key)?).with_context(|| format!("{key} is not hex"))
}

/// The call data: `input`, or `data` from a node that names it so.
fn input(tx: &Value) -> Result<Vec<u8>> {
    match present(tx, "input") {
        Some(_) => bytes(tx, "input"),
        None => bytes(tx, "data"),
    }
}

fn fixed_hex<const N: usize>(v: &Value) -> Result<[u8; N]> {
    let b = hex::decode(hex_str(v, "value")?)?;
    b.try_into()
        .map_err(|b: Vec<u8>| anyhow::anyhow!("{} bytes, expected {N}", b.len()))
}

fn fixed<const N: usize>(tx: &Value, key: &str) -> Result<[u8; N]> {
    let v = present(tx, key).with_context(|| format!("no {key}"))?;
    fixed_hex(v).with_context(|| key.to_string())
}

fn opt_address(tx: &Value, key: &str) -> Result<Option<[u8; 20]>> {
    present(tx, key)
        .map(|v| fixed_hex::<20>(v).with_context(|| key.to_string()))
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn hx(b: &[u8]) -> String {
        format!("0x{}", hex::encode(b))
    }

    /// The deposit encoder's field layout, nil `to` and `mint`, and the chain
    /// gates. (Real deposits are proven by `block`'s fixture tests.)
    #[test]
    fn deposit_layout_and_chain_gates() {
        let base = json!({
            "type": "0x7e",
            "sourceHash": hx(&[0x11; 32]),
            "from": hx(&[0x22; 20]),
            "to": hx(&[0x33; 20]),
            "mint": "0x0",
            "value": "0x0",
            "gas": "0xf4240",
            "input": "0x3db6be2b",
        });
        let v = value_from_rpc(&base, 10).unwrap();
        assert_eq!(v[0], 0x7e);
        let (payload, _, _) = crate::receipts::rlp_item(&v[1..]).unwrap();
        let items = crate::receipts::rlp_list_items(payload).unwrap();
        assert_eq!(items.len(), 8);
        // nil `to`, nonzero mint, a system tx.
        let mut other = base.clone();
        other["to"] = Value::Null;
        other["mint"] = json!("0x2386f26fc10000");
        other["isSystemTx"] = json!(true);
        let w = value_from_rpc(&other, 10).unwrap();
        let (payload, _, _) = crate::receipts::rlp_item(&w[1..]).unwrap();
        let items = crate::receipts::rlp_list_items(payload).unwrap();
        assert_eq!(items[2].0, &[] as &[u8], "nil to");
        assert_eq!(items[3].0, &[0x23, 0x86, 0xf2, 0x6f, 0xc1, 0x00, 0x00]);
        assert_eq!(items[6].0, &[0x01], "isSystemTx");
        // Deposits are refused off the OP stack, Nitro types off Orbit chains.
        assert!(value_from_rpc(&base, 1).is_err());
        assert!(value_from_rpc(&base, 42161).is_err());
        let internal = json!({"type": "0x6a", "chainId": "0xa4b1", "input": "0x6bf6a42d"});
        assert!(value_from_rpc(&internal, 42161).is_ok());
        assert!(value_from_rpc(&internal, 10).is_err());
        assert!(value_from_rpc(&json!({"type": "0x67", "chainId": "0x1"}), 42161).is_err());
        assert!(value_from_rpc(&json!("0x1234"), 1).is_err());
    }

    /// Standard types through alloy: a hash list in place of objects, a
    /// position that disagrees with `transactionIndex`, or a wrong `hash` are
    /// refused.
    #[test]
    fn standard_types_and_their_hashes() {
        use alloy::consensus::{Signed, TxEip1559, TxEnvelope, TxLegacy};
        use alloy::eips::eip2718::Encodable2718;
        use alloy_primitives::{Address, Bytes, Signature, TxKind};
        let sig = Signature::new(U256::from(7u8) << 200, U256::from(9u8) << 200, true);
        let legacy = TxLegacy {
            chain_id: Some(1),
            nonce: 3,
            gas_price: 10,
            gas_limit: 21_000,
            to: TxKind::Call(Address::from([1; 20])),
            value: U256::from(5u8),
            input: Bytes::new(),
        };
        let dynamic = TxEip1559 {
            chain_id: 1,
            nonce: 4,
            gas_limit: 50_000,
            max_fee_per_gas: 20,
            max_priority_fee_per_gas: 2,
            to: TxKind::Create,
            value: U256::ZERO,
            access_list: Default::default(),
            input: Bytes::from(vec![0xab; 40]),
        };
        let envs = [
            TxEnvelope::Legacy(Signed::new_unhashed(legacy, sig)),
            TxEnvelope::Eip1559(Signed::new_unhashed(dynamic, sig)),
        ];
        let rpc: Vec<Value> = envs
            .iter()
            .enumerate()
            .map(|(i, env)| {
                let t = alloy::rpc::types::Transaction {
                    inner: alloy::consensus::transaction::Recovered::new_unchecked(
                        env.clone(),
                        Address::from([9; 20]),
                    ),
                    block_hash: None,
                    block_number: None,
                    transaction_index: Some(i as u64),
                    effective_gas_price: None,
                };
                serde_json::to_value(&t).unwrap()
            })
            .collect();
        let got = from_rpc(&Value::Array(rpc.clone()), 1).unwrap();
        for (i, env) in envs.iter().enumerate() {
            assert_eq!(got.values[i], env.encoded_2718());
            assert_eq!(got.hashes[i], env.tx_hash().0);
        }
        assert!(
            from_rpc(&json!([rpc[0]["hash"]]), 1).is_err(),
            "hashes only"
        );
        assert!(
            from_rpc(&Value::Array(vec![rpc[1].clone(), rpc[0].clone()]), 1).is_err(),
            "out of position"
        );
        let mut bad = rpc.clone();
        bad[1]["hash"] = json!(hx(&[0x44; 32]));
        assert!(from_rpc(&Value::Array(bad), 1).is_err(), "wrong hash");
        let mut bad = rpc;
        bad[0]["value"] = json!("0x6");
        bad[0]["hash"] = Value::Null;
        assert!(from_rpc(&Value::Array(bad), 1).is_err(), "no hash");
    }
}
