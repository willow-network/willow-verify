//! alloy through `WillowTransport` (viem.test.ts / ethers.test.ts, as alloy flows): a provider's
//! typed calls against the mock server, every response verified by the client.

#![cfg(feature = "alloy")]

use super::support::*;
use crate::alloy::WillowTransport;
use serde_json::{json, Value};

/// A provider's flows: alloy-provider, in the release check's graph (`--cfg willow_provider_tests`).
#[cfg(willow_provider_tests)]
mod provider {
    use super::*;
    use alloy_primitives::{Address, Bytes, U256};
    use alloy_provider::{Provider, ProviderBuilder};
    use alloy_rpc_client::RpcClient;
    use alloy_rpc_types_eth::{BlockNumberOrTag, TransactionRequest};
    use std::str::FromStr;

    async fn provider_on(chain_id: u64, world: std::sync::Arc<World>) -> (Harness, impl Provider) {
        let h = harness_with(chain_id, world, "test-key", |c| c).await;
        let t = WillowTransport::from_verifier(h.client.clone());
        let p = ProviderBuilder::new().connect_client(RpcClient::new(t, false));
        (h, p)
    }

    fn addr(v: &Value) -> Address {
        Address::from_str(v.as_str().unwrap()).unwrap()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alloy_balance_nonce_call_and_estimate_on_ethereum() {
        let w = full_world();
        let b = w2_block();
        w.head(1, b.0, &b.1, "signed", "head");
        let (h, p) = provider_on(1, w).await;
        let eoa = fixture("state/eoa.json");
        let a = addr(&eoa["request"]["params"][0]);
        assert_eq!(p.get_block_number().await.unwrap(), b.0);
        assert_eq!(
            p.get_balance(a).await.unwrap(),
            U256::from_str(eoa["expect"]["eth_getBalance"].as_str().unwrap()).unwrap()
        );
        assert_eq!(
            p.get_transaction_count(a).pending().await.unwrap(),
            n(&eoa["expect"]["eth_getTransactionCount"])
        );
        let c = fixture("call/call-usdc-balance.json");
        let cp = &c["request"]["params"][0];
        let tx = TransactionRequest::default().to(addr(&cp["to"])).input(
            Bytes::from_str(cp["data"].as_str().or(cp["input"].as_str()).unwrap())
                .unwrap()
                .into(),
        );
        let out = p.call(tx).await.unwrap();
        assert_eq!(json!(out), c["expect"]["result"]);
        let est = fixture("call/estimate-transfer.json");
        let ep = &est["request"]["params"][0];
        let mut tx = TransactionRequest::default()
            .from(addr(&ep["from"]))
            .to(addr(&ep["to"]));
        if let Some(v) = ep["value"].as_str() {
            tx = tx.value(U256::from_str(v).unwrap());
        }
        assert_eq!(p.estimate_gas(tx).await.unwrap(), 21000);
        // A revert reaches alloy as the call's own JSON-RPC error (code 3, with its data).
        let r = fixture("call/call-revert.json");
        let rp = &r["request"]["params"][0];
        let tx = TransactionRequest::default()
            .from(addr(&rp["from"]))
            .to(addr(&rp["to"]))
            .input(
                Bytes::from_str(rp["data"].as_str().unwrap())
                    .unwrap()
                    .into(),
            );
        let e = p.call(tx).await.unwrap_err();
        let payload = e.as_error_resp().expect("a JSON-RPC error");
        assert_eq!(payload.code, r["expect"]["error"]["code"].as_i64().unwrap());
        assert!(h.events().iter().all(|e| e.status != crate::Status::Failed));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alloy_blocks_and_fees_on_ethereum() {
        let w = full_world();
        let b = block_of("ethereum-26391511.json");
        w.head_at(1, &b);
        let (_h, p) = provider_on(1, w).await;
        let f = fixture("block/ethereum-26391511.json");
        let blk = p
            .get_block_by_number(BlockNumberOrTag::Latest)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(format!("{:#x}", blk.header.hash), b.1);
        let n_txs = f["node"]["eth_getBlockByNumber"]["transactions"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!(blk.transactions.len(), n_txs);
        let full = p
            .get_block_by_number(BlockNumberOrTag::Latest)
            .full()
            .await
            .unwrap()
            .unwrap();
        assert_eq!(full.transactions.len(), n_txs);
        let fee = fixture("fees/ethereum-26391511.fee.json");
        assert_eq!(
            p.get_gas_price().await.unwrap(),
            n(&fee["verified"]["eth_gasPrice"]) as u128
        );
        assert_eq!(p.get_chain_id().await.unwrap(), 1);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn alloy_gets_the_clients_errors_as_json_rpc_errors() {
        let w = full_world();
        w.head_at(8453, &block_of("base-56049971.json"));
        let (_h, p) = provider_on(8453, w).await;
        // An L2 "safe" tag can't be verified: -32099 with its kind.
        let e = p
            .get_block_by_number(BlockNumberOrTag::Safe)
            .await
            .unwrap_err();
        let payload = e.as_error_resp().expect("a JSON-RPC error");
        assert_eq!(payload.code, -32099);
        assert!(payload
            .data
            .as_ref()
            .unwrap()
            .get()
            .contains("\"kind\":\"tag\""));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_batch_through_the_transport_gets_one_response_per_item() {
    use alloy_json_rpc::{Id, Request, RequestPacket, ResponsePacket, ResponsePayload};
    use tower::Service;
    let w = full_world();
    w.head_at(8453, &block_of("base-56049971.json"));
    let h = harness_with(8453, w, "test-key", |c| c).await;
    let mut t = WillowTransport::from_verifier(h.client.clone());
    let req = |id: u64, m: &'static str, p: Value| {
        Request::new(m, Id::Number(id), p).serialize().unwrap()
    };
    let packet = RequestPacket::Batch(vec![
        req(1, "eth_chainId", json!([])),
        req(2, "eth_coinbase", json!([])),
    ]);
    let ResponsePacket::Batch(out) = t.call(packet).await.unwrap() else {
        panic!("a batch")
    };
    assert_eq!(out.len(), 2);
    assert!(matches!(&out[0].payload, ResponsePayload::Success(v) if v.get() == "\"0x2105\""));
    assert!(
        matches!(&out[1].payload, ResponsePayload::Failure(e) if e.code == -32099 || e.code == -32601)
    );
}
