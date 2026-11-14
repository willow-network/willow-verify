//! `StateVerifier` as the page and the client call it, on W2's fixtures (state-verify/testdata).

use super::*;

const DATA: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../state-verify/testdata");

fn fixtures(dir: &str) -> Vec<(String, Value)> {
    let mut out: Vec<(String, Value)> = std::fs::read_dir(format!("{DATA}/{dir}"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json"))
        .map(|n| {
            let v = serde_json::from_slice(&std::fs::read(format!("{DATA}/{dir}/{n}")).unwrap())
                .unwrap();
            (n, v)
        })
        .collect();
    out.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!out.is_empty());
    out
}

fn chain_of(name: &str) -> u32 {
    if name.starts_with("base-") {
        8453
    } else {
        1
    }
}

fn body(v: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": v}).to_string()
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

fn kind_of(name: &str) -> &'static str {
    if name.starts_with("call-") {
        "call"
    } else if name.starts_with("estimate-") {
        "estimate"
    } else {
        "state"
    }
}

/// The fixture's request made, its result (changed by `f`) verified.
fn run(name: &str, f: &Value, change: impl FnOnce(&mut Value)) -> (StateVerifier, Value) {
    let mut v = StateVerifier::new(chain_of(name));
    let params = &f["request"]["params"];
    let r = parse(&v.request(kind_of(name), &params.to_string()));
    assert_eq!(r["ok"], json!(true), "{name}: {r}");
    assert_eq!(r["method"], f["request"]["method"]);
    assert_eq!(&r["params"], params);
    let mut result = f["result"].clone();
    change(&mut result);
    let out = parse(&v.verify(&body(&result)));
    (v, out)
}

fn flip(v: &mut Value) {
    let s = v.as_str().unwrap();
    let last = s.chars().last().unwrap();
    *v = json!(format!(
        "{}{}",
        &s[..s.len() - 1],
        if last == '0' { '1' } else { '0' }
    ));
}

#[test]
fn every_state_proof_verifies_and_compares() {
    for (name, f) in fixtures("state") {
        let (v, out) = run(&name, &f, |_| {});
        assert_eq!(out["ok"], json!(true), "{name}: {out}");
        assert_eq!(out["proven"], f["expect"], "{name}");
        let b = &out["report"]["block"];
        assert_eq!(
            b["hash"],
            json!(f["result"]["blockHash"].as_str().unwrap().to_lowercase())
        );
        assert_eq!(b["parentHash"].as_str().unwrap().len(), 66);
        assert!(b["timestamp"].as_u64().unwrap() > 1_700_000_000);
        let e = &f["expect"];
        let mut methods = vec!["eth_getBalance", "eth_getTransactionCount", "eth_getProof"];
        if !e["eth_getCode"].is_null() {
            methods.push("eth_getCode");
        }
        for m in methods {
            let c = parse(&v.compare(m, &body(&e[m])));
            assert_eq!(c["result"]["exact"], json!(true), "{name} {m}: {c}");
        }
        for (i, slot) in e["eth_getStorageAt"].as_array().unwrap().iter().enumerate() {
            let c = parse(&v.compare(&format!("eth_getStorageAt:{i}"), &body(slot)));
            assert_eq!(c["result"]["exact"], json!(true), "{name} slot {i}: {c}");
        }
        // Another balance, an error and an unreadable body.
        let c = parse(&v.compare("eth_getBalance", &body(&json!("0x123456789"))));
        assert_eq!(c["result"]["exact"], json!(false), "{name}: {c}");
        let err = json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "missing trie node"}}).to_string();
        assert_eq!(
            parse(&v.compare("eth_getProof", &err))["result"]["exact"],
            json!(false)
        );
        assert_eq!(
            parse(&v.compare("eth_getProof", "<html>"))["kind"],
            json!("malformed")
        );
        assert_eq!(
            parse(&v.compare("eth_call", &body(&json!("0x"))))["kind"],
            json!("internal")
        );
    }
}

#[test]
fn every_call_and_estimate_proof_verifies_as_geth_responds() {
    for (name, f) in fixtures("call") {
        let (v, out) = run(&name, &f, |_| {});
        assert_eq!(out["ok"], json!(true), "{name}: {out}");
        assert_eq!(out["proven"], f["expect"], "{name}");
        let method = if kind_of(&name) == "call" {
            "eth_call"
        } else {
            "eth_estimateGas"
        };
        if kind_of(&name) == "estimate" {
            assert_eq!(out["report"]["algorithm"], json!("geth-v1.17.10-estimate"));
        }
        let mut reference = f["reference"][method].clone();
        reference["jsonrpc"] = json!("2.0");
        reference["id"] = json!(1);
        let c = parse(&v.compare(method, &reference.to_string()));
        assert_eq!(c["result"]["exact"], json!(true), "{name}: {c}");
    }
}

#[test]
fn a_tampered_proof_fails_with_its_kind() {
    let state = |file: &str| {
        fixtures("state")
            .into_iter()
            .find(|(n, _)| n == file)
            .unwrap()
    };
    let call = |file: &str| {
        fixtures("call")
            .into_iter()
            .find(|(n, _)| n == file)
            .unwrap()
    };
    let kind = |(name, f): (String, Value), change: &dyn Fn(&mut Value)| {
        let (v, out) = run(&name, &f, |r| change(r));
        assert_eq!(out["ok"], json!(false), "{name}: {out}");
        assert_eq!(
            parse(&v.compare("eth_getBalance", &body(&json!("0x0"))))["kind"],
            json!("internal")
        );
        out["kind"].as_str().unwrap().to_string()
    };
    assert_eq!(
        kind(state("eoa.json"), &|r| r["account"]["balance"] =
            json!("0x1")),
        "account"
    );
    assert_eq!(
        kind(state("eoa.json"), &|r| flip(&mut r["header"])),
        "header"
    );
    assert_eq!(
        kind(state("eoa.json"), &|r| r["chainId"] = json!("0x2")),
        "wrong-chain"
    );
    assert_eq!(
        kind(state("usdc-slots-code.json"), &|r| r["account"]
            ["storageProof"][0]["value"] =
            json!("0x1")),
        "slot"
    );
    assert_eq!(
        kind(state("usdc-slots-code.json"), &|r| flip(&mut r["code"])),
        "code"
    );
    assert_eq!(
        kind(state("usdc-slots-code.json"), &|r| {
            r.as_object_mut().unwrap().remove("code");
        }),
        "code"
    );
    assert_eq!(
        kind(state("absent.json"), &|r| r["account"]["balance"] =
            json!("0x1")),
        "account"
    );
    assert_eq!(kind(state("eoa.json"), &|r| *r = json!(null)), "malformed");
    assert_eq!(
        kind(call("call-usdc-balance.json"), &|r| r["result"] =
            json!({"returned": "0x01"})),
        "outcome"
    );
    assert_eq!(
        kind(call("call-usdc-balance.json"), &|r| {
            r["witness"]["accounts"].as_array_mut().unwrap().pop();
        }),
        "missing-read"
    );
    assert_eq!(
        kind(call("estimate-transfer.json"), &|r| r["algorithm"] =
            json!("geth-v1.16.0-estimate")),
        "algorithm"
    );
    assert_eq!(
        kind(call("estimate-transfer.json"), &|r| r["estimate"] =
            json!({"gas": "0x5209"})),
        "estimate"
    );
    assert_eq!(
        kind(call("estimate-transfer.json"), &|r| r["estimate"] =
            json!({"gas": "0x5207"})),
        "estimate"
    );
    // Another block than the one asked for.
    let (name, f) = state("eoa.json");
    let other = fixtures("state")
        .into_iter()
        .find(|(n, _)| n == "base-usdc.json")
        .unwrap()
        .1;
    let mut v = StateVerifier::new(1);
    v.request("state", &f["request"]["params"].to_string());
    let mut r = f["result"].clone();
    for k in ["blockNumber", "blockHash", "header"] {
        r[k] = other["result"][k].clone();
    }
    r["chainId"] = json!("0x1");
    assert_eq!(
        parse(&v.verify(&body(&r)))["kind"],
        json!("wrong-block"),
        "{name}"
    );
}

#[test]
fn requests_are_read_as_the_proof_methods_read_them() {
    let mut v = StateVerifier::new(1);
    let a = "0x28c6c06298d514db089934071355e5743bf21d60";
    let ok = |v: &mut StateVerifier, kind: &str, t: Value| {
        let r = parse(&v.request(kind, &t.to_string()));
        assert_eq!(r["ok"], json!(true), "{t}: {r}");
        assert_eq!(r["params"], t);
    };
    ok(&mut v, "state", json!([a, [], "latest"]));
    ok(
        &mut v,
        "state",
        json!([a, ["0x0", "0x1"], {"blockHash": format!("0x{}", "ab".repeat(32))}, {"code": true}]),
    );
    ok(&mut v, "call", json!([{"to": a, "data": "0x"}, "0x1"]));
    ok(
        &mut v,
        "estimate",
        json!([{"from": a, "to": a, "value": "0x1"}, {"blockNumber": "0x10"}]),
    );
    for (kind, t) in [
        ("state", json!(["0x12", [], "latest"])),
        ("state", json!([a, "0x0", "latest"])),
        ("state", json!([a, ["0xzz"], "latest"])),
        ("state", json!([a, vec!["0x1"; 257], "latest"])),
        ("state", json!([a, [], "yesterday"])),
        ("state", json!([a, [], "latest", {"code": "yes"}])),
        ("state", json!([a, []])),
        (
            "call",
            json!([{"to": a}, "latest", {a: {"balance": "0x1"}}]),
        ),
        ("call", json!(["0x", "latest"])),
        (
            "estimate",
            json!([{"to": a, "gasPrice": "0x1", "maxFeePerGas": "0x1"}, "latest"]),
        ),
        ("state", json!({"address": a})),
    ] {
        let r = parse(&v.request(kind, &t.to_string()));
        assert_eq!(r["kind"], json!("invalid-request"), "{kind} {t}: {r}");
    }
    assert_eq!(
        parse(&v.request("state", "not json"))["kind"],
        json!("invalid-request")
    );
    assert_eq!(parse(&v.request("logs", "[]"))["kind"], json!("internal"));
    let mut base = StateVerifier::new(8453);
    assert_eq!(
        parse(&base.request("call", &json!([{"to": a}, "latest"]).to_string()))["kind"],
        json!("unsupported")
    );
    ok(&mut base, "state", json!([a, [], "latest"]));
    // Out of order: a fault of the caller, never a verdict.
    let mut fresh = StateVerifier::new(1);
    assert_eq!(
        parse(&fresh.verify(&body(&json!({}))))["kind"],
        json!("internal")
    );
    assert_eq!(
        parse(&fresh.compare("eth_getBalance", &body(&json!("0x0"))))["kind"],
        json!("internal")
    );
}
