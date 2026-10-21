//! Transaction and receipt proofs from the live log RPC (web/verify/test/items:
//! Ethereum, Base, Arbitrum One and Unichain), through `ItemVerifier` as the page
//! calls it, and every way a tampered answer must fail.

use super::*;

const ITEMS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../web/verify/test/items"
);

/// A saved case: the page's request, Willow's proof and its standard answers.
fn case(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(format!("{ITEMS}/{name}.json")).unwrap()).unwrap()
}

fn body(result: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string()
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// `ItemVerifier` with the case's request made, as the page makes it.
fn asked(c: &Value) -> ItemVerifier {
    let mut v = ItemVerifier::new(c["chainId"].as_u64().unwrap() as u32);
    let r = parse(&v.request(c["kind"].as_str().unwrap(), c["target"].as_str().unwrap()));
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!(r["method"], c["proof"]["method"]);
    assert_eq!(r["params"], c["proof"]["params"]);
    v
}

/// The case's proof, changed by `f`, verified: the failure's kind and message.
fn tampered(name: &str, f: impl FnOnce(&mut Value)) -> (String, String) {
    let c = case(name);
    let mut v = asked(&c);
    let mut proof = c["proof"]["result"].clone();
    f(&mut proof);
    let r = parse(&v.verify(&body(&proof)));
    assert_eq!(r["ok"], json!(false), "{name}: a changed proof verified");
    (
        r["kind"].as_str().unwrap().to_string(),
        r["message"].as_str().unwrap().to_string(),
    )
}

/// One hex digit of a `0x` string flipped.
fn flip(s: &mut Value, at: usize) {
    let mut b = s.as_str().unwrap().as_bytes().to_vec();
    let i = 2 + at % (b.len() - 2);
    b[i] = if b[i] == b'0' { b'1' } else { b'0' };
    *s = json!(String::from_utf8(b).unwrap());
}

#[test]
fn every_saved_proof_verifies_and_matches_willows_answers() {
    for name in [
        "eth-tx",
        "eth-receipt",
        "eth-receipt-1",
        "base-swap-receipt",
        "base-reverted-receipt",
        "arb-tx",
        "arb-receipt",
        "arb-block",
        "uni-receipt",
        "uni-deposit",
        "uni-block",
    ] {
        let c = case(name);
        let mut v = asked(&c);
        let r = parse(&v.verify(&body(&c["proof"]["result"])));
        assert_eq!(r["ok"], json!(true), "{name}: {r}");
        let rep = &r["report"];
        assert_eq!(rep["block"]["hash"], c["block"]["hash"], "{name}");
        assert_eq!(rep["block"]["number"], c["block"]["number"], "{name}");
        assert_eq!(rep["chainId"], c["chainId"], "{name}");
        if c["kind"] == "blockReceipts" {
            assert!(r["provenTransactions"].is_array(), "{name}");
            assert_eq!(
                rep["transactions"],
                json!(r["proven"].as_array().unwrap().len())
            );
        } else {
            let want = c["answers"][0]["params"][0].as_str().unwrap();
            assert_eq!(rep["transactionHash"], json!(want), "{name}");
            // A receipt's own transaction comes with it, rendered from the proven bytes.
            let tx = &r["provenTransaction"];
            match c["kind"].as_str().unwrap() {
                "receipt" => {
                    assert_eq!(tx["hash"], json!(want), "{name}");
                    assert_eq!(tx["from"], r["proven"]["from"], "{name}");
                    assert_eq!(tx["blockHash"], c["block"]["hash"], "{name}");
                }
                _ => assert!(tx.is_null(), "{name}"),
            }
        }
        for (i, a) in c["answers"].as_array().unwrap().iter().enumerate() {
            let which = if i == 0 { "answer" } else { "transactions" };
            let out = parse(&v.compare(which, &body(&a["result"])));
            assert_eq!(out["ok"], json!(true), "{name} {which}: {out}");
            let res = &out["result"];
            assert_eq!(res["exact"], json!(true), "{name} {which}: {res}");
            assert!(
                res["matched"].as_u64().unwrap() > 5,
                "{name} {which}: {res}"
            );
        }
    }
}

#[test]
fn the_fields_a_proof_does_not_cover_are_named() {
    let unproven = |name: &str| {
        let c = case(name);
        let mut v = asked(&c);
        parse(&v.verify(&body(&c["proof"]["result"])))["report"]["unproven"].clone()
    };
    // Nitro's passed-through receipt fields.
    assert_eq!(
        unproven("arb-receipt"),
        json!(["gasUsedForL1", "timeboosted"])
    );
    assert_eq!(
        unproven("arb-block"),
        json!(["[].gasUsedForL1", "[].timeboosted"])
    );
    // A receipt past the block's second transaction: its logs' block-wide indexes.
    assert_eq!(unproven("eth-receipt"), json!(["logs[].logIndex"]));
    assert_eq!(unproven("eth-receipt-1"), json!([]));
    assert_eq!(unproven("eth-tx"), json!([]));
    // An OP-stack receipt carries the L1 info deposit: its L1 fee fields are proven.
    assert_eq!(unproven("uni-receipt"), json!(["logs[].logIndex"]));
    assert_eq!(unproven("uni-block"), json!([]));
    // And Willow's answer reports the uncovered fields, which compare lists as unproven.
    let c = case("arb-receipt");
    let mut v = asked(&c);
    v.verify(&body(&c["proof"]["result"]));
    let out = parse(&v.compare("answer", &body(&c["answers"][0]["result"])));
    assert_eq!(
        out["result"]["unproven"],
        json!(["gasUsedForL1", "timeboosted"])
    );
}

#[test]
fn a_tampered_proof_fails() {
    // A trie node on the receipt's path.
    let (k, m) = tampered("eth-receipt", |p| {
        flip(&mut p["receiptProofs"][1]["proof"][0], 40)
    });
    assert_eq!(k, "rejected", "{m}");
    // The receipt's value, and the transaction's.
    let (k, m) = tampered("eth-receipt", |p| {
        flip(&mut p["receiptProofs"][1]["value"], 30)
    });
    assert_eq!(k, "rejected", "{m}");
    let (k, _) = tampered("eth-tx", |p| {
        flip(&mut p["transactionProofs"][0]["value"], 60)
    });
    assert_eq!(k, "rejected");
    // The header: its hash is no longer the block hash.
    let (k, m) = tampered("arb-receipt", |p| flip(&mut p["header"], 100));
    assert_eq!(k, "rejected", "{m}");
    // A whole block's receipt.
    let (k, m) = tampered("uni-block", |p| flip(&mut p["receipts"][3], 20));
    assert_eq!(k, "rejected", "{m}");
    assert!(m.contains("rebuild root"), "{m}");
    // A receipt left out of a whole block.
    let (k, _) = tampered("arb-block", |p| {
        p["receipts"].as_array_mut().unwrap().pop();
    });
    assert_eq!(k, "rejected");
    // Another position than the one asked for.
    let (k, m) = tampered("eth-receipt-1", |p| p["transactionIndex"] = json!("0x0"));
    assert_eq!(k, "rejected", "{m}");
    // The proof's own block hash, changed to match nothing.
    let (k, m) = tampered("uni-deposit", |p| flip(&mut p["blockHash"], 5));
    assert_eq!(k, "rejected", "{m}");
}

#[test]
fn a_proof_of_something_else_fails() {
    // Another transaction's valid proof, for a hash the page asked about.
    let asked_for = case("eth-tx");
    let other = case("eth-receipt-1");
    let mut v = ItemVerifier::new(1);
    v.request("transaction", asked_for["target"].as_str().unwrap());
    let mut wrong = other["proof"]["result"].clone();
    wrong["transactionIndex"] = json!("0x1");
    let r = parse(&v.verify(&body(&wrong)));
    assert_eq!(
        (r["ok"].clone(), r["kind"].clone()),
        (json!(false), json!("rejected")),
        "{r}"
    );
    // A valid Unichain proof read as Ethereum's.
    let c = case("uni-receipt");
    let mut v = ItemVerifier::new(1);
    v.request("receipt", c["target"].as_str().unwrap());
    let r = parse(&v.verify(&body(&c["proof"]["result"])));
    assert_eq!(r["kind"], json!("rejected"));
    assert!(r["message"].as_str().unwrap().contains("chain 130"), "{r}");
    // Another block than the number asked for.
    let c = case("arb-block");
    let mut v = ItemVerifier::new(42161);
    v.request("blockReceipts", "511868573");
    let r = parse(&v.verify(&body(&c["proof"]["result"])));
    assert_eq!(r["kind"], json!("rejected"), "{r}");
    // Single items where a whole block was asked for.
    let c = case("arb-receipt");
    let mut v = ItemVerifier::new(42161);
    v.request("blockReceipts", "latest");
    let r = parse(&v.verify(&body(&c["proof"]["result"])));
    assert_eq!(r["kind"], json!("rejected"), "{r}");
}

#[test]
fn requests_are_read_from_what_is_typed() {
    let mut v = ItemVerifier::new(1);
    let req = |v: &mut ItemVerifier, kind: &str, t: &str| parse(&v.request(kind, t));
    let h = format!("0x{}", "ab".repeat(32));
    let r = req(&mut v, "receipt", &format!("  {h} "));
    assert_eq!(
        (r["method"].clone(), r["params"].clone()),
        (json!("willow_getReceiptProof"), json!([h]))
    );
    let r = req(&mut v, "transaction", "26124863:5");
    assert_eq!(r["params"], json!(["0x18ea23f", "0x5"]));
    let r = req(&mut v, "transaction", &format!("{h}:0x10"));
    assert_eq!(r["params"], json!([h, "0x10"]));
    let r = req(&mut v, "transaction", "latest:0");
    assert_eq!(r["params"], json!(["latest", "0x0"]));
    let r = req(&mut v, "blockReceipts", "0x18ea23f");
    assert_eq!(
        (r["method"].clone(), r["params"].clone()),
        (json!("willow_getBlockReceiptsProof"), json!(["0x18ea23f"]))
    );
    let r = req(&mut v, "blockReceipts", "latest");
    assert_eq!(r["params"], json!(["latest"]));
    for (kind, t) in [
        ("transaction", "26124863"),
        ("transaction", "0x1234"),
        ("receipt", "26124863:x"),
        ("receipt", ""),
        ("blockReceipts", "finalized"),
        ("blockReceipts", "-1"),
        ("blockReceipts", "99999999999999999999999"),
        ("blockReceipts", "0x"),
    ] {
        let r = req(&mut v, kind, t);
        assert_eq!(
            (r["ok"].clone(), r["kind"].clone()),
            (json!(false), json!("invalid-request")),
            "{kind} {t:?}: {r}"
        );
    }
    // Out of order: a fault in the page, never a verdict.
    let mut fresh = ItemVerifier::new(1);
    assert_eq!(
        parse(&fresh.verify(&body(&json!({}))))["kind"],
        json!("internal")
    );
    assert_eq!(
        parse(&fresh.compare("answer", &body(&json!({}))))["kind"],
        json!("internal")
    );
    let r = req(&mut v, "receipt", "26124863:5");
    assert_eq!(r["ok"], json!(true));
    assert_eq!(
        parse(&v.compare("answer", &body(&json!({}))))["kind"],
        json!("internal")
    );
}

#[test]
fn unreadable_answers_are_malformed_input() {
    let c = case("eth-tx");
    let mut v = asked(&c);
    for bad in [
        "<html>502</html>".to_string(),
        body(&json!(null)),
        body(&json!({"chainId": "0x1"})),
        format!(
            r#"{{"jsonrpc":"2.0","id":1,"result":{}{}}}"#,
            "[".repeat(300),
            "]".repeat(300)
        ),
    ] {
        let r = parse(&v.verify(&bad));
        assert_eq!(r["kind"], json!("malformed-input"), "{r}");
    }
    let mut p = c["proof"]["result"].clone();
    p["header"] = json!("0xzz");
    assert_eq!(parse(&v.verify(&body(&p)))["kind"], json!("rejected"));
}

#[test]
fn another_answer_is_compared_field_by_field() {
    let c = case("arb-receipt");
    let mut v = asked(&c);
    v.verify(&body(&c["proof"]["result"]));
    let mut answer = c["answers"][0]["result"].clone();
    answer["status"] = json!("0x0");
    answer["logs"][0]["data"] = json!("0x00");
    let r = parse(&v.compare("answer", &body(&answer)))["result"].clone();
    assert_eq!(r["exact"], json!(false));
    assert_eq!(r["differCount"], json!(2), "{r}");
    assert!(
        r["differ"][0].as_str().unwrap().starts_with("logs[0].data"),
        "{r}"
    );
    // A provider that does not have it.
    let r = parse(&v.compare("answer", &body(&json!(null))))["result"].clone();
    assert_eq!(
        (r["exact"].clone(), r["absent"].clone()),
        (json!(false), json!(true))
    );
    // Unreadable: malformed input, never a crash.
    let r = parse(&v.compare("answer", "not json"));
    assert_eq!(r["kind"], json!("malformed-input"));
    // A block's transactions: the block object's list, field by field.
    let c = case("uni-block");
    let mut v = asked(&c);
    v.verify(&body(&c["proof"]["result"]));
    let mut block = c["answers"][1]["result"].clone();
    block["transactions"][2]["value"] = json!("0x1");
    let r = parse(&v.compare("transactions", &body(&block)))["result"].clone();
    assert_eq!(r["differCount"], json!(1), "{r}");
    let r = parse(&v.compare("transactions", &body(&json!({"number": "0x1"}))));
    assert_eq!(r["kind"], json!("malformed-input"));
    // A long difference is clipped.
    let mut receipts = c["answers"][0]["result"].clone();
    receipts[0]["logsBloom"] = json!(format!("0x{}", "f".repeat(512)));
    let r = parse(&v.compare("answer", &body(&receipts)))["result"].clone();
    assert!(
        r["differ"][0].as_str().unwrap().chars().count() <= 301,
        "{r}"
    );
}
