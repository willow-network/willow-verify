//! The fixtures (I2, I3) verify and come to the public node's own responses at the same block;
//! every mutation of them fails with its kind.

use super::*;
use crate::call::{Exec, Missing, Witness};
use crate::witness::Builder;
use std::path::PathBuf;

fn dir(d: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(d)
}

fn fixture(d: &str, name: &str) -> Value {
    let path = dir(d).join(name);
    serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap()
}

/// The fixtures of `d` whose names start with `prefix`, by name.
fn fixtures(d: &str, prefix: &str) -> Vec<(String, Value)> {
    let mut names: Vec<String> = std::fs::read_dir(dir(d))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.starts_with(prefix) && n.ends_with(".json"))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no fixtures {d}/{prefix}*");
    names
        .into_iter()
        .map(|n| (n.clone(), fixture(d, &n)))
        .collect()
}

/// The chain the client asks on: the fixture's own, unless a mutation names another.
fn chain_of(f: &Value) -> u64 {
    f["askedOn"].as_u64().unwrap_or_else(|| {
        proof::quantity(&f["result"]["chainId"])
            .unwrap()
            .to::<u64>()
    })
}

fn state(f: &Value) -> Result<ProvenState, Failure> {
    verify_state(chain_of(f), &f["request"]["params"], &f["result"])
}

fn call_of(f: &Value) -> Result<ProvenCall, Failure> {
    verify_call(1, &f["request"]["params"], &f["result"])
}

fn estimate_of(f: &Value) -> Result<ProvenEstimate, Failure> {
    verify_estimate(1, &f["request"]["params"], &f["result"])
}

#[test]
fn state_proofs_verify_and_derive_the_nodes_responses() {
    for (name, f) in fixtures("state", "") {
        let p = state(&f).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(p.responses, f["expect"], "{name}");
    }
}

#[test]
fn calls_verify_and_come_to_geths_responses() {
    for (name, f) in fixtures("call", "call-") {
        let p = call_of(&f).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(p.response, f["expect"], "{name}");
        assert_eq!(p.response, f["reference"]["eth_call"], "{name}");
    }
}

#[test]
fn estimates_equal_geths_at_the_same_block() {
    for (name, f) in fixtures("call", "estimate-") {
        let geth = &f["reference"]["eth_estimateGas"];
        assert!(f["reference"]["clientVersion"]
            .as_str()
            .unwrap()
            .starts_with("Geth/v1.17."));
        let p = estimate_of(&f).unwrap_or_else(|e| panic!("{name}: {e}"));
        eprintln!("{name}: Willow {} geth {geth}", p.response);
        assert_eq!(&p.response, geth, "{name}");
        assert_eq!(p.response, f["expect"], "{name}");
    }
}

/// The witness, environment and call of a call or estimate fixture.
fn parts(f: &Value) -> (Witness, call::Block, call::Call, Named) {
    let r = &f["result"];
    let at = At::parse(&f["request"]["params"][1]).unwrap();
    let block = named(
        1,
        at,
        (
            r["chainId"].as_str().unwrap(),
            r["blockNumber"].as_str().unwrap(),
            r["blockHash"].as_str().unwrap(),
            r["header"].as_str().unwrap(),
        ),
    )
    .unwrap();
    let w: wire::Witness = serde_json::from_value(r["witness"].clone()).unwrap();
    let w = witness::verify(block.header.state_root, &w).unwrap();
    let env = call::Block::from_header(
        &block.header,
        &logset_blockproof::derive::BlobSchedule::builtin(1),
    );
    let c = call::parse(&f["request"]["params"][0]).unwrap();
    (w, env, c, block)
}

/// geth's optimistic limit is (MaxUsedGas + 2300) * 64 / 63, and the search goes on from it, so
/// where a call has refunds the estimate depends on reading MaxUsedGas right. revm's gas before
/// refunds is it: with it the swap's estimate is geth's, with the gas used after refunds it is not.
#[test]
fn the_peak_is_geths_max_used_gas() {
    let f = fixture("call", "estimate-uniswap-swap.json");
    let geth = f["reference"]["eth_estimateGas"]["result"]
        .as_str()
        .unwrap();
    let geth = proof::quantity(&json!(geth)).unwrap().to::<u64>();
    let (w, env, c, _) = parts(&f);
    let none = |_: u64| None;
    let hi = env.gas_limit.min(1 << 24);
    let x = call::execute(
        &w,
        &env,
        &c,
        hi,
        &none,
        Mode::Strict,
        &mut Reads::default(),
        true,
    );
    assert!(matches!(x.ran, Ran::Returned(_)), "{:?}", x.ran);
    assert!(x.peak > x.used, "the swap has no refund");
    let with = |peak: &dyn Fn(&Exec) -> u64| {
        let found = estimate::search(
            &w,
            &env,
            &c,
            &none,
            Mode::Strict,
            &mut Reads::default(),
            peak,
        );
        match found.unwrap_or_else(|e| e) {
            Estimated::Gas(g) => g,
            other => panic!("{other:?}"),
        }
    };
    let peak = with(&|x: &Exec| x.peak);
    let used = with(&|x: &Exec| x.used);
    eprintln!(
        "swap: used {} peak {}; estimate {peak}, with used {used}, geth {geth}",
        x.used, x.peak
    );
    assert_eq!(peak, geth);
    assert_ne!(used, geth);
}

#[test]
fn blockhash_through_eip2935_equals_the_headers() {
    let f = fixture("call", "call-blockhash.json");
    let p = call_of(&f).unwrap();
    let wire::Outcome::Returned(out) = &p.outcome else {
        panic!("{:?}", p.outcome)
    };
    let out = proof::data(&json!(out)).unwrap();
    let (w, _, _, block) = parts(&f);
    let n = block.number;
    for (i, back) in [(0usize, 1u64), (1, 256)] {
        let header = &f["reference"]["headers"][(n - back).to_string()];
        let stored = logset_blockcheck::verify_header(header).unwrap();
        let hash = keccak256(&stored.rlp);
        assert_eq!(
            &out[1 + 32 * i..33 + 32 * i],
            hash.as_slice(),
            "block {}",
            n - back
        );
        assert_eq!(history::block_hash(&w, n, n - back), Some(hash));
    }
    // Outside the ring, or not before the block: no hash.
    assert_eq!(history::block_hash(&w, n, n), None);
    assert_eq!(history::block_hash(&w, n, n - 8_192), None);
}

/// The server's way: gather what a run lacks, run again, then prove exactly what the final
/// search read. The pruned witness verifies.
#[test]
fn a_witness_built_from_the_reads_verifies() {
    for (name, f) in fixtures("call", "estimate-") {
        let (full, env, c, block) = parts(&f);
        let r: wire::Witness = serde_json::from_value(f["result"]["witness"].clone()).unwrap();
        let mut builder = Builder::default();
        for a in witness::expand(&r).unwrap() {
            let address = proof::address(&a["address"]).unwrap();
            let keys: Vec<B256> = a["storageProof"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| proof::slot(&s["key"]).unwrap())
                .collect();
            let p = proof::check(block.header.state_root, address, &keys, &a).unwrap();
            builder.add(&p, &a, &code_of(&r, &p));
        }
        // Gathering from the server's seed (sender, recipient, beneficiary) without its access-list
        // hint: each round adds what the last one lacked.
        let mut gathered = Witness::default();
        for a in [Some(c.from), c.to, Some(env.beneficiary)]
            .into_iter()
            .flatten()
        {
            if let Some(p) = full.account(&a) {
                let mut p = p.clone();
                p.slots.clear();
                let code = code_of(&r, &p);
                gathered.add(p, code);
            }
        }
        let mut stray = vec![];
        let mut reads;
        let hash_of = |_: u64| None;
        let mut rounds = 0;
        let found = loop {
            rounds += 1;
            assert!(rounds <= 16, "{name}: more than 16 rounds");
            reads = Reads::default();
            match estimate::estimate(&gathered, &env, &c, &hash_of, Mode::Gather, &mut reads) {
                Estimated::Missing(m) => {
                    for (a, slots) in m.accounts.iter().chain(&m.slots) {
                        // Read while the sender was still empty (a creation's address from nonce
                        // 0, say): the final search does not read it, so the proof leaves it out.
                        let Some(p) = full.account(a) else {
                            stray.push(*a);
                            gathered.add(absent(*a), Bytes::new());
                            continue;
                        };
                        let mut p = p.clone();
                        p.slots = match p.exists {
                            true => slots
                                .iter()
                                .map(|k| {
                                    let v = full.slot(a, *k);
                                    (*k, v.unwrap_or_else(|| panic!("{name}: {a} slot {k}")))
                                })
                                .collect(),
                            false => vec![],
                        };
                        let code = code_of(&r, &p);
                        gathered.add(p, code);
                    }
                }
                other => break other,
            }
        };
        for a in &stray {
            assert!(
                !reads.accounts.contains_key(a),
                "{name}: the final search read {a}"
            );
        }
        let built = builder.build(&reads).unwrap();
        let mut result = f["result"].clone();
        result["witness"] = serde_json::to_value(&built).unwrap();
        let p = verify_estimate(1, &f["request"]["params"], &result)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(wire::Estimate::of(&found), Some(p.estimate), "{name}");
        assert!(built.accounts.len() <= r.accounts.len(), "{name}");
        eprintln!(
            "{name}: {rounds} rounds, {} accounts read of {}",
            built.accounts.len(),
            r.accounts.len()
        );
    }
}

/// An account as a provider might prove it absent (only the gathering's working set holds it).
fn absent(address: alloy_primitives::Address) -> Proven {
    Proven {
        address,
        exists: false,
        nonce: 0,
        balance: U256::ZERO,
        storage_root: alloy_trie::EMPTY_ROOT_HASH,
        code_hash: alloy_trie::KECCAK_EMPTY,
        slots: vec![],
    }
}

/// The code the fixture's witness holds for `p`.
fn code_of(w: &wire::Witness, p: &Proven) -> Bytes {
    match p.has_code() {
        true => proof::data(&json!(w.codes[&hx(p.code_hash.as_slice())])).unwrap(),
        false => Bytes::new(),
    }
}

#[test]
fn a_gathering_run_names_what_it_lacks() {
    let f = fixture("call", "estimate-weth-deposit.json");
    let (_, env, c, _) = parts(&f);
    let none = |_: u64| None;
    let ran = call::run_with(
        &Witness::default(),
        &env,
        &c,
        &none,
        Mode::Gather,
        &mut Reads::default(),
    );
    // Read as empty, the sender can't pay the deposit: the run stops there, naming it.
    let Ran::Missing(Missing { accounts, .. }) = ran else {
        panic!("{ran:?}")
    };
    assert!(accounts.contains_key(&c.from));
}

// The tamper suite: each mutation of a fixture must fail with its kind.

fn mutated(f: &Value, change: impl FnOnce(&mut Value)) -> Value {
    let mut f = f.clone();
    change(&mut f);
    f
}

/// The witness account of `address` in a fixture.
fn account_mut<'a>(f: &'a mut Value, address: &str) -> &'a mut Value {
    f["result"]["witness"]["accounts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["address"] == address)
        .unwrap_or_else(|| panic!("no account {address}"))
}

const USDC: &str = "0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48";
const HISTORY: &str = "0x0000f90827f1c53a10cb7a02335b175320002935";
const NOBODY2: &str = "0x4b1d5e7c9a3f2e6d8c0b1a2f3e4d5c6b7a8f9e01";
const WETH: &str = "0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2";

fn flip_last_hex(s: &str) -> String {
    let mut c: Vec<char> = s.chars().collect();
    let last = c.len() - 1;
    c[last] = if c[last] == '0' { '1' } else { '0' };
    c.into_iter().collect()
}

fn kind<T: std::fmt::Debug>(r: Result<T, Failure>) -> Kind {
    match r {
        Ok(v) => panic!("verified: {v:?}"),
        Err(f) => f.kind,
    }
}

#[test]
fn every_state_mutation_fails_with_its_kind() {
    let usdc = fixture("state", "usdc-slots-code.json");
    let absent = fixture("state", "absent.json");
    let eoa = fixture("state", "eoa.json");
    let cases: Vec<(&str, Value, Kind)> = vec![
        (
            "a byte of the header",
            mutated(&usdc, |f| {
                let h = f["result"]["header"].as_str().unwrap().to_string();
                f["result"]["header"] = json!(flip_last_hex(&h));
            }),
            Kind::Header,
        ),
        (
            "the hash of another block",
            mutated(&usdc, |f| {
                let h = f["result"]["blockHash"].as_str().unwrap().to_string();
                f["result"]["blockHash"] = json!(flip_last_hex(&h));
            }),
            Kind::Header,
        ),
        (
            "asked for another block",
            mutated(&usdc, |f| {
                let h = f["request"]["params"][2]["blockHash"]
                    .as_str()
                    .unwrap()
                    .to_string();
                f["request"]["params"][2]["blockHash"] = json!(flip_last_hex(&h));
            }),
            Kind::WrongBlock,
        ),
        (
            "asked for another number",
            mutated(&eoa, |f| {
                let n = proof::quantity(&f["request"]["params"][2])
                    .unwrap()
                    .to::<u64>();
                f["request"]["params"][2] = json!(format!("0x{:x}", n - 1));
            }),
            Kind::WrongBlock,
        ),
        (
            "another chain",
            mutated(&usdc, |f| {
                f["askedOn"] = json!(1);
                f["result"]["chainId"] = json!("0x2");
            }),
            Kind::WrongChain,
        ),
        (
            "a proof of another account",
            mutated(&usdc, |f| {
                f["request"]["params"][0] = json!(WETH);
            }),
            Kind::Account,
        ),
        (
            "a node of the account proof",
            mutated(&usdc, |f| {
                let n = f["result"]["account"]["accountProof"][1]
                    .as_str()
                    .unwrap()
                    .to_string();
                f["result"]["account"]["accountProof"][1] = json!(flip_last_hex(&n));
            }),
            Kind::Account,
        ),
        (
            "a balance",
            mutated(&usdc, |f| f["result"]["account"]["balance"] = json!("0x1")),
            Kind::Account,
        ),
        (
            "a slot value",
            mutated(&usdc, |f| {
                f["result"]["account"]["storageProof"][0]["value"] = json!("0x1");
            }),
            Kind::Slot,
        ),
        (
            "a slot proof dropped",
            mutated(&usdc, |f| {
                f["result"]["account"]["storageProof"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }),
            Kind::Slot,
        ),
        (
            "the proof of another slot",
            mutated(&usdc, |f| {
                f["result"]["account"]["storageProof"][1]["key"] = json!("0x2");
            }),
            Kind::Slot,
        ),
        (
            "code swapped (same length, other hash)",
            mutated(&usdc, |f| {
                let c = f["result"]["code"].as_str().unwrap().to_string();
                f["result"]["code"] = json!(flip_last_hex(&c));
            }),
            Kind::Code,
        ),
        (
            "code left out",
            mutated(&usdc, |f| {
                f["result"].as_object_mut().unwrap().remove("code");
            }),
            Kind::Code,
        ),
        (
            "an absent account given a balance",
            mutated(&absent, |f| {
                f["result"]["account"]["balance"] = json!("0x1");
            }),
            Kind::Account,
        ),
        (
            "an absent account given a nonce",
            mutated(&absent, |f| {
                f["result"]["account"]["nonce"] = json!("0x1");
            }),
            Kind::Account,
        ),
        (
            "an absent account given code",
            mutated(&absent, |f| {
                f["result"]["code"] = json!("0x00");
            }),
            Kind::Code,
        ),
        (
            "a header that is not hex",
            mutated(&usdc, |f| f["result"]["header"] = json!("0xzz")),
            Kind::Malformed,
        ),
        (
            "too many keys asked for",
            mutated(&usdc, |f| {
                f["request"]["params"][1] = json!(vec!["0x1"; MAX_PROOF_SLOTS + 1]);
            }),
            Kind::InvalidRequest,
        ),
    ];
    for (what, f, want) in cases {
        assert_eq!(kind(state(&f)), want, "{what}");
    }
}

#[test]
fn every_call_mutation_fails_with_its_kind() {
    let balance = fixture("call", "call-usdc-balance.json");
    let hashes = fixture("call", "call-blockhash.json");
    let swap = fixture("call", "estimate-uniswap-swap.json");
    let transfer = fixture("call", "estimate-transfer.json");
    let weth = fixture("call", "estimate-weth-deposit.json");
    let calls: Vec<(&str, Value, Kind)> = vec![
        (
            "an account dropped",
            mutated(&balance, |f| {
                let accounts = f["result"]["witness"]["accounts"].as_array_mut().unwrap();
                accounts.retain(|a| a["address"] != USDC);
            }),
            Kind::MissingRead,
        ),
        (
            "a slot dropped",
            mutated(&balance, |f| {
                account_mut(f, USDC)["storageProof"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }),
            Kind::MissingRead,
        ),
        (
            "a slot value",
            mutated(&balance, |f| {
                account_mut(f, USDC)["storageProof"][0]["value"] = json!("0x1234");
            }),
            Kind::Slot,
        ),
        (
            "a shared proof node",
            mutated(&balance, |f| {
                let n = f["result"]["witness"]["nodes"][0]
                    .as_str()
                    .unwrap()
                    .to_string();
                f["result"]["witness"]["nodes"][0] = json!(flip_last_hex(&n));
            }),
            Kind::Account,
        ),
        (
            "code swapped (same length, other hash)",
            mutated(&balance, |f| {
                let codes = f["result"]["witness"]["codes"].as_object_mut().unwrap();
                for c in codes.values_mut() {
                    *c = json!(flip_last_hex(c.as_str().unwrap()));
                }
            }),
            Kind::Code,
        ),
        (
            "code left out",
            mutated(&balance, |f| {
                f["result"]["witness"]["codes"] = json!({});
            }),
            Kind::Code,
        ),
        (
            "a different result",
            mutated(&balance, |f| {
                let r = f["result"]["result"]["returned"]
                    .as_str()
                    .unwrap()
                    .to_string();
                f["result"]["result"]["returned"] = json!(flip_last_hex(&r));
            }),
            Kind::Outcome,
        ),
        (
            "a revert for a result",
            mutated(&balance, |f| {
                f["result"]["result"] = json!({"reverted": "0x"});
            }),
            Kind::Outcome,
        ),
        (
            "the history account with another code hash",
            mutated(&hashes, |f| {
                account_mut(f, HISTORY)["codeHash"] = json!(format!("0x{}", "11".repeat(32)));
            }),
            Kind::History,
        ),
        (
            "a history slot dropped",
            mutated(&hashes, |f| {
                account_mut(f, HISTORY)["storageProof"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }),
            Kind::MissingRead,
        ),
        (
            "the history account dropped",
            mutated(&hashes, |f| {
                let accounts = f["result"]["witness"]["accounts"].as_array_mut().unwrap();
                accounts.retain(|a| a["address"] != HISTORY);
            }),
            Kind::MissingRead,
        ),
        (
            "a header byte",
            mutated(&balance, |f| {
                let h = f["result"]["header"].as_str().unwrap().to_string();
                f["result"]["header"] = json!(flip_last_hex(&h));
            }),
            Kind::Header,
        ),
        (
            "asked for another block",
            mutated(&balance, |f| {
                let h = f["request"]["params"][1]["blockHash"]
                    .as_str()
                    .unwrap()
                    .to_string();
                f["request"]["params"][1]["blockHash"] = json!(flip_last_hex(&h));
            }),
            Kind::WrongBlock,
        ),
        (
            "a node longer than 1,100 bytes",
            mutated(&balance, |f| {
                f["result"]["witness"]["nodes"][0] = json!(format!("0x{}", "ab".repeat(1_101)));
            }),
            Kind::Limits,
        ),
        (
            "a proof of 65 nodes",
            mutated(&balance, |f| {
                account_mut(f, USDC)["accountProof"] = json!(vec!["#0"; 65]);
            }),
            Kind::Limits,
        ),
        (
            "a node that refers to nothing",
            mutated(&balance, |f| {
                // Written so the source holds no "#" and digits (publish.sh's issue-number scan).
                account_mut(f, USDC)["accountProof"][0] = json!(format!("#{}", 999));
            }),
            Kind::Malformed,
        ),
        (
            "an account twice",
            mutated(&balance, |f| {
                let a = account_mut(f, USDC).clone();
                f["result"]["witness"]["accounts"]
                    .as_array_mut()
                    .unwrap()
                    .push(a);
            }),
            Kind::Malformed,
        ),
        (
            "overrides",
            mutated(&balance, |f| {
                let params = f["request"]["params"].as_array_mut().unwrap();
                params.push(json!({USDC: {"balance": "0x1"}}));
            }),
            Kind::InvalidRequest,
        ),
    ];
    for (what, f, want) in calls {
        assert_eq!(kind(call_of(&f)), want, "{what}");
    }
    assert_eq!(
        kind(verify_call(
            8453,
            &balance["request"]["params"],
            &balance["result"]
        )),
        Kind::Unsupported
    );

    let estimates: Vec<(&str, Value, Kind)> = vec![
        (
            "estimate + 1",
            mutated(&swap, |f| {
                let g = proof::quantity(&f["result"]["estimate"]["gas"])
                    .unwrap()
                    .to::<u64>();
                f["result"]["estimate"]["gas"] = json!(format!("0x{:x}", g + 1));
            }),
            Kind::Estimate,
        ),
        (
            "estimate - 1",
            mutated(&swap, |f| {
                let g = proof::quantity(&f["result"]["estimate"]["gas"])
                    .unwrap()
                    .to::<u64>();
                f["result"]["estimate"]["gas"] = json!(format!("0x{:x}", g - 1));
            }),
            Kind::Estimate,
        ),
        (
            "an error for an estimate",
            mutated(&swap, |f| {
                f["result"]["estimate"] =
                    json!({"error": "gas required exceeds allowance (16777216)"});
            }),
            Kind::Estimate,
        ),
        (
            "another algorithm",
            mutated(&swap, |f| {
                f["result"]["algorithm"] = json!("geth-v1.17.11-estimate");
            }),
            Kind::Algorithm,
        ),
        (
            "an absent account given a balance",
            mutated(&transfer, |f| {
                account_mut(f, NOBODY2)["balance"] = json!("0x1");
            }),
            Kind::Account,
        ),
        (
            "the sender dropped",
            mutated(&transfer, |f| {
                let from = f["request"]["params"][0]["from"].clone();
                let accounts = f["result"]["witness"]["accounts"].as_array_mut().unwrap();
                accounts.retain(|a| a["address"] != from);
            }),
            Kind::MissingRead,
        ),
        (
            "a slot of the deposit dropped",
            mutated(&weth, |f| {
                account_mut(f, WETH)["storageProof"]
                    .as_array_mut()
                    .unwrap()
                    .pop();
            }),
            Kind::MissingRead,
        ),
    ];
    for (what, f, want) in estimates {
        assert_eq!(kind(estimate_of(&f)), want, "{what}");
    }
}

/// An account of one fixture proven at the same block is an extra, unrelated account in
/// another's witness: allowed and ignored.
#[test]
fn an_extra_unrelated_account_is_ignored() {
    let balance = fixture("call", "call-usdc-balance.json");
    let weth = fixture("call", "estimate-weth-deposit.json");
    assert_eq!(balance["result"]["blockHash"], weth["result"]["blockHash"]);
    let w: wire::Witness = serde_json::from_value(weth["result"]["witness"].clone()).unwrap();
    let extra = witness::expand(&w)
        .unwrap()
        .into_iter()
        .find(|a| a["address"] == WETH)
        .unwrap();
    let with = mutated(&balance, |f| {
        f["result"]["witness"]["accounts"]
            .as_array_mut()
            .unwrap()
            .push(extra);
        let code_hash = w.accounts.iter().find(|a| a["address"] == WETH).unwrap()["codeHash"]
            .as_str()
            .unwrap()
            .to_string();
        f["result"]["witness"]["codes"][&code_hash] = json!(w.codes[&code_hash]);
    });
    let p = call_of(&with).unwrap();
    assert_eq!(p.response, balance["expect"]);
}

/// Malformed input fails with a kind; it never panics (a panic traps the wasm).
#[test]
fn malformed_witnesses_fail_without_panicking() {
    let balance = fixture("call", "call-usdc-balance.json");
    type Change = Box<dyn Fn(&mut Value)>;
    let changes: Vec<Change> = vec![
        Box::new(|f| f["result"]["witness"]["nodes"][0] = json!("0x0")),
        Box::new(|f| f["result"]["witness"]["nodes"][0] = json!("zz")),
        Box::new(|f| account_mut(f, USDC)["balance"] = json!(format!("0x{}", "f".repeat(65)))),
        Box::new(|f| account_mut(f, USDC)["codeHash"] = json!("0x12")),
        Box::new(|f| account_mut(f, USDC)["storageProof"] = json!(7)),
        Box::new(|f| account_mut(f, USDC)["storageProof"][0] = json!("x")),
        Box::new(|f| f["result"]["witness"]["accounts"][0] = json!(1)),
        Box::new(|f| f["result"]["witness"]["codes"] = json!({"0x12": "0x00"})),
        Box::new(|f| f["result"]["witness"] = json!(null)),
        Box::new(|f| f["result"]["result"] = json!({"returned": 5})),
        Box::new(|f| f["result"]["blockNumber"] = json!("0x")),
        Box::new(|f| f["request"]["params"][0] = json!({"to": "0x12"})),
        Box::new(|f| f["request"]["params"][1] = json!({"blockHash": 5})),
    ];
    for (i, change) in changes.iter().enumerate() {
        let mut f = balance.clone();
        change(&mut f);
        assert!(call_of(&f).is_err(), "change {i} verified");
    }
}
