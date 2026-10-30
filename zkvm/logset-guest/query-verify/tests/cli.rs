//! The CLI offline on real answers (`web/verify/test/logs`): SP1 (Ethereum 26226600-26226601,
//! two one-block leaves, as before) and OpenVM (the 10/28 GPU run's Ethereum leaf and L1 and Base
//! L1). OpenVM parts need `willow-logs-verify-openvm`: with `WILLOW_LOGS_VERIFY_OPENVM` naming it,
//! they must verify; without, they must fail closed. Either way nothing else changes.

use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn logs(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web/verify/test/logs")
        .join(rel)
}

fn helper() -> Option<String> {
    std::env::var("WILLOW_LOGS_VERIFY_OPENVM")
        .ok()
        .filter(|s| !s.is_empty())
}

/// Runs the CLI on `answer` (a `willow_getLogsProof` result) with proofs from `store`.
fn run(answer: &Value, store: &Path, filter: &Value, extra: &[&str]) -> (bool, Value, String) {
    let dir = std::env::temp_dir().join(format!("wlv-cli-{}-{}", std::process::id(), rand_tag()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("answer.json");
    std::fs::write(&file, answer.to_string()).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_willow-logs-verify"))
        .args([
            "--proof",
            file.to_str().unwrap(),
            "--proofs-dir",
            store.to_str().unwrap(),
        ])
        .args(["--filter", &filter.to_string()])
        .args(extra)
        .output()
        .unwrap();
    std::fs::remove_dir_all(&dir).unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let report = serde_json::from_str(&stdout).unwrap_or(Value::Null);
    (
        out.status.success(),
        report,
        String::from_utf8_lossy(&out.stderr).to_string(),
    )
}

fn rand_tag() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

fn openvm_answer(rel: &str) -> (Value, Value, u64, PathBuf) {
    let v: Value = serde_json::from_slice(&std::fs::read(logs(rel)).unwrap()).unwrap();
    let store = logs(rel).parent().unwrap().join("store");
    (
        v["result"].clone(),
        v["filter"].clone(),
        v["logs"].as_u64().unwrap(),
        store,
    )
}

#[test]
fn a_real_sp1_answer_verifies_as_before() {
    let answer: Value =
        serde_json::from_slice(&std::fs::read(logs("eth-26226600/answer.json")).unwrap()).unwrap();
    let filter =
        serde_json::json!({"fromBlock": answer["fromBlock"], "toBlock": answer["toBlock"]});
    let store = logs("eth-26226600/store");
    // No OpenVM verifier is needed for an all-SP1 answer.
    let (ok, r, err) = run(
        &answer,
        &store,
        &filter,
        &["--chain-id", "1", "--openvm-verifier", "/nonexistent"],
    );
    assert!(ok, "{err} {r}");
    assert_eq!(r["proofs"], serde_json::json!({"sp1": 2, "openvm": 0}));
    assert_eq!(r["openvmFrom"], Value::Null);
    assert_eq!(r["parts"]["blocks"], 2);
    // A cutover at or before these blocks makes SP1 the wrong system for them.
    let (ok, _, err) = run(
        &answer,
        &store,
        &filter,
        &["--chain-id", "1", "--openvm-from", "26226601"],
    );
    assert!(
        !ok && err.contains("is proven with SP1, but this chain uses OpenVM there"),
        "{err}"
    );
    let (ok, _, err) = run(
        &answer,
        &store,
        &filter,
        &["--chain-id", "1", "--openvm-from", "26226602"],
    );
    assert!(ok, "{err}");
}

#[test]
fn real_openvm_answers_verify_with_the_openvm_verifier_and_fail_closed_without_it() {
    for rel in [
        "eth-26069632/segment.json",
        "eth-26069632/block.json",
        "base-52969856/segment.json",
    ] {
        let (answer, filter, want, store) = openvm_answer(rel);
        let chain = answer["chainId"].as_str().unwrap().to_string();
        let chain = u64::from_str_radix(chain.trim_start_matches("0x"), 16)
            .unwrap()
            .to_string();
        // Ethereum's pinned cutover (26,361,856) comes after these 10/28 proofs, so under it an
        // OpenVM part here is the wrong system. Every Ethereum run below therefore sets a test
        // cutover at the answer's first block; Base is not pinned.
        let first = u64::from_str_radix(
            answer["fromBlock"]
                .as_str()
                .unwrap()
                .trim_start_matches("0x"),
            16,
        )
        .unwrap()
        .to_string();
        let pin: Vec<&str> = if chain == "1" {
            vec!["--openvm-from", first.as_str()]
        } else {
            vec![]
        };
        let args = |rest: &[&str]| -> Vec<String> {
            ["--chain-id", chain.as_str()]
                .iter()
                .chain(pin.iter())
                .chain(rest.iter())
                .map(|s| s.to_string())
                .collect()
        };
        let run_ = |a: &Value, rest: &[&str]| {
            let v = args(rest);
            let v: Vec<&str> = v.iter().map(String::as_str).collect();
            run(a, &store, &filter, &v)
        };
        if chain == "1" {
            let (ok, _, err) = run(
                &answer,
                &store,
                &filter,
                &["--chain-id", "1", "--openvm-verifier", "/nonexistent/ovm"],
            );
            assert!(
                !ok && err.contains("is proven with OpenVM, but this chain uses SP1 there"),
                "{rel}: under the pinned cutover: {err}"
            );
        }
        let (ok, _, err) = run_(&answer, &["--openvm-verifier", "/nonexistent/ovm"]);
        assert!(
            !ok && err.contains("no OpenVM verifier (/nonexistent/ovm"),
            "{rel}: {err}"
        );
        let Some(h) = helper() else {
            eprintln!(
                "{rel}: WILLOW_LOGS_VERIFY_OPENVM is not set, so only the fail-closed case ran"
            );
            continue;
        };
        let (ok, r, err) = run_(&answer, &["--openvm-verifier", h.as_str()]);
        assert!(ok, "{rel}: {err} {r}");
        assert_eq!(r["logs"], want, "{rel}");
        assert_eq!(
            r["proofs"],
            serde_json::json!({"sp1": 0, "openvm": 1}),
            "{rel}"
        );
        assert_eq!(r["openvmFrom"], answer["fromBlock"], "{rel}");
        assert_eq!(r["cutover"]["checked"], chain == "1", "{rel}");
        // A changed public value byte (here the statement's end block hash) is refused by
        // the OpenVM verifier, through the helper.
        let mut bad = answer.clone();
        let pv = bad["parts"][0]["publicValues"]
            .as_str()
            .unwrap()
            .to_string();
        let i = 2 + 2 * 200;
        let flipped = if &pv[i..i + 1] == "0" { "1" } else { "0" };
        bad["parts"][0]["publicValues"] = format!("{}{flipped}{}", &pv[..i], &pv[i + 1..]).into();
        let (ok, _, err) = run_(&bad, &["--openvm-verifier", h.as_str()]);
        assert!(!ok && err.contains("rejected"), "{rel}: {err}");
        // The chain's cutover after these blocks: OpenVM is the wrong system for them (the
        // last --openvm-from wins).
        let after = (u64::from_str_radix(
            answer["toBlock"].as_str().unwrap().trim_start_matches("0x"),
            16,
        )
        .unwrap()
            + 1)
        .to_string();
        let (ok, _, err) = run_(
            &answer,
            &[
                "--openvm-verifier",
                h.as_str(),
                "--openvm-from",
                after.as_str(),
            ],
        );
        assert!(
            !ok && err.contains("is proven with OpenVM, but this chain uses SP1 there"),
            "{rel}: {err}"
        );
        // A helper that is not the OpenVM verifier is refused before any proof.
        let (ok, _, err) = run_(&answer, &["--openvm-verifier", "/usr/bin/true"]);
        assert!(!ok && err.contains("stopped"), "{rel}: {err}");
    }
}
