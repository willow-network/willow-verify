//! eth_sendRawTransaction: the hash is computed here from the bytes, the sender and
//! nonce recorded for the pending-nonce rule (tx.test.ts). Transactions signed here with a test key.

use super::support::*;
use crate::config::Status;
use alloy::consensus::{
    SignableTransaction, TxEip1559, TxEip2930, TxEip4844, TxEnvelope, TxLegacy,
};
use alloy::eips::eip2718::Encodable2718;
use alloy_primitives::{Address, Bytes, Signature, TxKind, B256, U256};
use serde_json::json;

const KEY: &str = "59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d";

fn to() -> Address {
    "0x4b1d5e7c9a3f2e6d8c0b1a2f3e4d5c6b7a8f9e01"
        .parse()
        .unwrap()
}

fn sign(hash: B256) -> Signature {
    let k = k256::ecdsa::SigningKey::from_slice(&hex::decode(KEY).unwrap()).unwrap();
    let (sig, recid) = k.sign_prehash_recoverable(hash.as_slice()).unwrap();
    Signature::new(
        U256::from_be_slice(&sig.r().to_bytes()),
        U256::from_be_slice(&sig.s().to_bytes()),
        recid.is_y_odd(),
    )
}

fn address() -> String {
    let k = k256::ecdsa::SigningKey::from_slice(&hex::decode(KEY).unwrap()).unwrap();
    format!("{:#x}", Address::from_public_key(k.verifying_key()))
}

fn env<T: SignableTransaction<Signature>>(tx: T) -> Vec<u8>
where
    alloy::consensus::Signed<T>: Into<TxEnvelope>,
{
    let s = sign(tx.signature_hash());
    let e: TxEnvelope = tx.into_signed(s).into();
    e.encoded_2718()
}

fn signed() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "legacy, no replay protection",
            env(TxLegacy {
                chain_id: None,
                nonce: 7,
                gas_price: 2,
                gas_limit: 21000,
                to: TxKind::Call(to()),
                value: U256::from(1),
                input: Bytes::new(),
            }),
        ),
        (
            "legacy, EIP-155",
            env(TxLegacy {
                chain_id: Some(1),
                nonce: 7,
                gas_price: 2,
                gas_limit: 21000,
                to: TxKind::Call(to()),
                value: U256::from(1),
                input: Bytes::new(),
            }),
        ),
        (
            "EIP-2930",
            env(TxEip2930 {
                chain_id: 1,
                nonce: 7,
                gas_price: 2,
                gas_limit: 21000,
                to: TxKind::Call(to()),
                value: U256::from(1),
                access_list: Default::default(),
                input: Bytes::new(),
            }),
        ),
        ("EIP-1559", eip1559()),
        (
            "EIP-4844",
            env(TxEip4844 {
                chain_id: 1,
                nonce: 7,
                gas_limit: 21000,
                max_fee_per_gas: 3,
                max_priority_fee_per_gas: 1,
                to: to(),
                value: U256::from(1),
                access_list: Default::default(),
                blob_versioned_hashes: vec![B256::from_slice(
                    &[[1u8].as_slice(), &[0xab; 31]].concat(),
                )],
                max_fee_per_blob_gas: 5,
                input: Bytes::new(),
            }),
        ),
    ]
}

fn eip1559() -> Vec<u8> {
    env(TxEip1559 {
        chain_id: 1,
        nonce: 7,
        gas_limit: 21000,
        max_fee_per_gas: 3,
        max_priority_fee_per_gas: 1,
        to: TxKind::Call(to()),
        value: U256::from(1),
        access_list: Default::default(),
        input: Bytes::from(vec![0x12, 0x34]),
    })
}

/// A signed EIP-1559 transfer, as `0x` hex.
pub(super) fn raw_1559() -> String {
    format!("0x{}", hex::encode(eip1559()))
}

fn rlp(payload: &[u8], list: bool) -> Vec<u8> {
    let mut out = vec![];
    alloy_rlp::Header {
        list,
        payload_length: payload.len(),
    }
    .encode(&mut out);
    out.extend_from_slice(payload);
    out
}

#[test]
fn every_type_hashes_as_its_bytes_and_its_sender_and_nonce_recover() {
    for (name, raw) in signed() {
        let t = crate::tx::decode_sent(&raw).unwrap_or_else(|| panic!("{name}"));
        assert_eq!(t.hash, crate::hex::keccak_hex(&raw), "{name}");
        assert_eq!(t.from.as_deref(), Some(address().as_str()), "{name}");
        assert_eq!(t.nonce, 7, "{name}");
        assert_eq!(
            t.chain_id,
            if name.contains("no replay") {
                None
            } else {
                Some(1)
            },
            "{name}"
        );
    }
}

#[test]
fn a_blob_transactions_network_form_hashes_without_its_sidecar() {
    let raw = signed()
        .into_iter()
        .find(|(n, _)| *n == "EIP-4844")
        .unwrap()
        .1;
    let body = &raw[1..];
    let mut items = body.to_vec();
    items.extend(rlp(&rlp(&[7u8; 64], false), true));
    items.extend(rlp(&rlp(&[1u8; 48], false), true));
    items.extend(rlp(&rlp(&[2u8; 48], false), true));
    let network = [&[3u8][..], &rlp(&items, true)].concat();
    let t = crate::tx::decode_sent(&network).unwrap();
    assert_eq!(t.hash, crate::hex::keccak_hex(&raw));
    assert_eq!(t.from.as_deref(), Some(address().as_str()));
}

#[test]
fn bytes_that_are_no_transaction_do_not_decode() {
    assert!(crate::tx::decode_sent(&[0x05, 0xc0]).is_none());
    assert!(crate::tx::decode_sent(&[0xc1, 0x80]).is_none());
    assert!(crate::tx::decode_sent(&[]).is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn the_hash_returned_is_the_clients_own_and_the_next_pending_nonce_is_past_it() {
    let h = harness(1, |c| c).await;
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    let raw = format!("0x{}", hex::encode(eip1559()));
    let hash = crate::hex::keccak_hex(&eip1559());
    h.world
        .standard("1|eth_sendRawTransaction|*", json!({"result": hash}));
    assert_eq!(
        h.ask("eth_sendRawTransaction", json!([raw])).await.unwrap(),
        json!(hash)
    );
    assert_eq!(h.last().status, Status::Local);
    assert_eq!(h.client.inner.sent_nonce(&address()), Some(7));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_server_that_returns_another_hash_fails_verification() {
    let h = harness(1, |c| c).await;
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    h.world.standard(
        "1|eth_sendRawTransaction|*",
        json!({"result": format!("0x{}", "00".repeat(32))}),
    );
    let e = h
        .ask(
            "eth_sendRawTransaction",
            json!([format!("0x{}", hex::encode(eip1559()))]),
        )
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("tx-hash".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_providers_refusal_passes_through_marked() {
    let h = harness(1, |c| c).await;
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    h.world.standard(
        "1|eth_sendRawTransaction|*",
        json!({"error": {"code": -32000, "message": "nonce too low"}}),
    );
    let e = h
        .ask(
            "eth_sendRawTransaction",
            json!([format!("0x{}", hex::encode(eip1559()))]),
        )
        .await
        .unwrap_err();
    assert_eq!(e, crate::Error::rpc(-32000, "nonce too low"));
    let ev = h.last();
    assert_eq!(
        (ev.status, ev.kind.as_deref()),
        (Status::Unverifiable, Some("provider-error"))
    );
}

/// The pending nonce past this client's own last send: the eoa fixture's proven nonce, then a
/// send recorded for that address one above it.
#[tokio::test(flavor = "multi_thread")]
async fn the_pending_nonce_is_one_past_this_clients_own_send_when_that_is_higher() {
    let h = harness(1, |c| c).await;
    let b = w2_block();
    h.world.head(1, b.0, &b.1, "signed", "head");
    let f = fixture("state/eoa.json");
    let a = f["request"]["params"][0].as_str().unwrap().to_string();
    let proven = n(&f["expect"]["eth_getTransactionCount"]);
    h.client.inner.record_sent(&a, proven as u128 + 4);
    assert_eq!(
        h.ask("eth_getTransactionCount", json!([a, "pending"]))
            .await
            .unwrap(),
        json!(q(proven + 5))
    );
    assert_eq!(
        h.ask("eth_getTransactionCount", json!([a, "latest"]))
            .await
            .unwrap(),
        json!(q(proven))
    );
    h.client.inner.record_sent(&a, 0);
    assert_eq!(
        h.ask("eth_getTransactionCount", json!([a, "pending"]))
            .await
            .unwrap(),
        json!(q(proven + 5))
    );
}
