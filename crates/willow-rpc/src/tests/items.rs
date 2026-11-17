//! Transactions, receipts, block receipts and eth_getLogs by receipts
//! with the real items verifier on the shipped proofs in web/verify/test/items (items.real.test.ts).

use super::support::*;
use crate::config::{Level, Status};
use serde_json::{json, Value};

/// Every proven field equals the server's own standard response; what is left out is listed.
fn proven_subset(proven: &Value, standard: &Value, path: &str) -> Vec<String> {
    match proven {
        Value::Object(o) => o
            .iter()
            .flat_map(|(k, v)| {
                proven_subset(
                    v,
                    standard.get(k).unwrap_or(&Value::Null),
                    &format!("{path}.{k}"),
                )
            })
            .collect(),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .flat_map(|(i, v)| {
                proven_subset(
                    v,
                    standard.get(i).unwrap_or(&Value::Null),
                    &format!("{path}.{i}"),
                )
            })
            .collect(),
        v => {
            if v.to_string().eq_ignore_ascii_case(&standard.to_string()) {
                vec![]
            } else {
                vec![path.to_string()]
            }
        }
    }
}

fn method_of(kind: &str) -> &'static str {
    match kind {
        "transaction" => "eth_getTransactionByHash",
        "receipt" => "eth_getTransactionReceipt",
        _ => "eth_getBlockReceipts",
    }
}

fn items() -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = std::fs::read_dir(items_dir())
        .unwrap()
        .filter_map(|e| {
            let p = e.ok()?.path();
            (p.extension()? == "json").then(|| {
                (
                    p.file_name().unwrap().to_string_lossy().to_string(),
                    json_file(&p),
                )
            })
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

#[tokio::test(flavor = "multi_thread")]
async fn every_shipped_item_proof_verifies_and_is_the_servers_own_response() {
    let mut ran = 0;
    for (file, c) in items() {
        // A receipt named by <block>:<index> has no standard method to come from.
        if c["kind"] == "receipt" && c["target"].as_str().unwrap().contains(':') {
            continue;
        }
        let w = World::new();
        w.add_item(&c, true);
        let h = harness_with(n(&c["chainId"]), w, "test-key", |x| x).await;
        let out = h
            .ask(method_of(c["kind"].as_str().unwrap()), json!([c["target"]]))
            .await
            .unwrap();
        assert_eq!(
            proven_subset(&out, &c["answers"][0]["result"], ""),
            Vec::<String>::new(),
            "{file}"
        );
        let e = h.last();
        assert_eq!(e.status, Status::Verified, "{file}");
        assert_eq!(
            e.block.unwrap().hash,
            c["block"]["hash"].as_str().unwrap().to_ascii_lowercase(),
            "{file}"
        );
        assert_eq!(e.level, Some(Level::Final), "{file}");
        ran += 1;
    }
    assert!(ran >= 5, "{ran} item fixtures");
}

#[tokio::test(flavor = "multi_thread")]
async fn eth_get_logs_over_one_block_goes_by_its_block_receipts_proof() {
    let c = json_file(&items_dir().join("uni-block.json"));
    let w = World::new();
    w.add_item(&c, true);
    let number = n(&c["block"]["number"]);
    w.head(
        130,
        number,
        c["block"]["hash"].as_str().unwrap(),
        "signed",
        "sequencer",
    );
    let h = harness_with(130, w, "test-key", |x| x).await;
    let all: Vec<Value> = c["answers"][0]["result"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|r| r["logs"].as_array().cloned().unwrap_or_default())
        .collect();
    let address = all[0]["address"].as_str().unwrap().to_string();
    let logs = h
        .ask(
            "eth_getLogs",
            json!([{"fromBlock": q(number), "toBlock": "latest", "address": address}]),
        )
        .await
        .unwrap();
    let want: Vec<Value> = all
        .iter()
        .filter(|l| {
            l["address"]
                .as_str()
                .unwrap()
                .eq_ignore_ascii_case(&address)
        })
        .cloned()
        .collect();
    assert!(!want.is_empty());
    assert_eq!(logs, Value::Array(want));
    let by_hash = h
        .ask("eth_getLogs", json!([{"blockHash": c["block"]["hash"]}]))
        .await
        .unwrap();
    assert_eq!(by_hash, Value::Array(all));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_tampered_receipt_proof_fails_verification() {
    let mut c = json_file(&items_dir().join("eth-receipt.json"));
    let proofs = c["proof"]["result"]["receiptProofs"]
        .as_array_mut()
        .unwrap();
    let last = proofs.last_mut().unwrap();
    let p0 = last["proof"][0].as_str().unwrap().to_string();
    last["proof"][0] = json!(flip(&p0, 40));
    let w = World::new();
    w.add_item(&c, true);
    let h = harness_with(1, w, "test-key", |x| x).await;
    let e = h
        .ask("eth_getTransactionReceipt", json!([c["target"]]))
        .await
        .unwrap_err();
    assert_eq!(code_kind(&e), (-32098, Some("receipts".into())));
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unknown_transaction_hash_is_null() {
    let w = World::new();
    w.head(1, 1, &format!("0x{}", "aa".repeat(32)), "signed", "head");
    let h = harness_with(1, w, "test-key", |x| x).await;
    let unknown = format!("0x{}", "12".repeat(32));
    assert_eq!(
        h.ask("eth_getTransactionReceipt", json!([unknown]))
            .await
            .unwrap(),
        Value::Null
    );
    assert_eq!(
        h.ask("eth_getTransactionByHash", json!([unknown]))
            .await
            .unwrap(),
        Value::Null
    );
    let st: Vec<Status> = h.events().iter().map(|e| e.status).collect();
    assert_eq!(st, vec![Status::Absent, Status::Absent]);
}
