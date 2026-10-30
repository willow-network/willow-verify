use super::*;
use logset_query::fixtures::{
    addr, chain, fake_openvm_proof, fake_proof, topic, Chain, FakeCheck, L1_VK,
};
use logset_query::{BlockProof, SegmentIndex, SegmentProof};
use std::collections::HashMap;

const START: u64 = 26_065_000;

/// A segment over blocks 0..6 and one-block parts for 6 and 7, as the server sends them.
fn served(c: &Chain, f: &LogFilter) -> (String, HashMap<String, Vec<u8>>) {
    let mut store = HashMap::new();
    let pv = c.l1(0, 6);
    let ix = SegmentIndex::build(&pv, &c.records[0..6]).unwrap();
    let id = c.segment_id(0, 6).to_string();
    store.insert(id.clone(), fake_proof(&L1_VK, &pv));
    let mut parts = vec![ProofPart::Segment(SegmentProof {
        ranges: ix.prove(f).unwrap().into_iter().map(Into::into).collect(),
        public_values: pv,
        proof: id,
    })];
    for i in 6..8 {
        let id = c.block_id(i).to_string();
        store.insert(id.clone(), fake_proof(&c.leaf_vkey(), &c.leaf_pvs[i]));
        parts.push(ProofPart::Block(BlockProof {
            public_values: c.leaf_pvs[i].clone(),
            proof: id,
            records: c.records[i].clone(),
        }));
    }
    let p = LogsProof {
        chain_id: 1,
        from_block: f.from_block,
        to_block: f.to_block,
        parts,
    };
    (serde_json::to_string(&p).unwrap(), store)
}

/// A JSON-RPC response body carrying `result`, as the page hands it over.
fn rpc(result: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":1,"result":{result}}}"#)
}

fn eth_filter(from: u64, to: u64) -> Value {
    json!({
        "fromBlock": format!("0x{from:x}"),
        "toBlock": format!("0x{to:x}"),
        "address": format!("0x{}", hex::encode(addr(2))),
        "topics": [format!("0x{}", hex::encode(topic(1)))],
    })
}

fn run(
    c: &Chain,
    answer: &str,
    filter: &Value,
    store: &HashMap<String, Vec<u8>>,
) -> Result<Outcome, Failure> {
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    check_answer(
        &c.trust(),
        &rpc(answer),
        &filter.to_string(),
        &mut fetch,
        &mut FakeCheck,
    )
}

#[test]
fn segment_and_block_parts_verify_and_compare() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let out = run(&c, &answer, &ef, &store).unwrap();
    let want: Vec<Log> = c.all_logs().into_iter().filter(|l| f.matches(l)).collect();
    assert!(!want.is_empty());
    assert_eq!(out.verified.logs, want);
    assert_eq!((out.segments, out.blocks), (1, 2));
    let r = out.report();
    assert_eq!(r["logs"], want.len());
    assert_eq!(r["lastProvenBlock"]["number"], format!("0x{:x}", START + 7));

    let plain: Vec<Value> = want.iter().map(log_json).collect();
    let cmp = out
        .compare(&rpc(&Value::Array(plain.clone()).to_string()))
        .unwrap();
    assert_eq!(
        (cmp["exact"].clone(), cmp["matched"].clone()),
        (json!(true), json!(want.len()))
    );
    let cmp = out
        .compare(&rpc(&Value::Array(plain[1..].to_vec()).to_string()))
        .unwrap();
    assert_eq!(cmp["exact"], json!(false));
    assert_eq!(cmp["missing"].as_array().unwrap().len(), 1);
}

#[test]
fn the_range_comes_from_the_client() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    // An answer for a narrower range than asked is refused before any proof is checked.
    let wider = eth_filter(START + 1, START + 9);
    let e = run(&c, &answer, &wider, &store).err().unwrap();
    assert_eq!(e.kind, Kind::Rejected);
    assert!(e.message.contains("not the requested"), "{e:?}");
    let mut tagged = ef.clone();
    tagged["toBlock"] = json!("latest");
    let e = run(&c, &answer, &tagged, &store).err().unwrap();
    assert_eq!(e.kind, Kind::InvalidFilter);
    assert!(e.message.contains("resolved"), "{e:?}");
}

#[test]
fn a_tampered_answer_fails() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let mut p: LogsProof = serde_json::from_str(&answer).unwrap();
    // Block 6's records (block 7 is empty).
    let ProofPart::Block(b) = &mut p.parts[1] else {
        panic!()
    };
    let last = b.records.len() - 1;
    b.records[last] ^= 1;
    let e = run(&c, &serde_json::to_string(&p).unwrap(), &ef, &store)
        .err()
        .unwrap();
    assert_eq!(e.kind, Kind::Rejected);
    assert!(e.message.contains("records do not match"), "{e:?}");
}

#[test]
fn key_overrides_replace_the_pinned_sp1_keys_only() {
    use willow_logs_verify::{LEAF_VKEY, SEGMENT_VKEY};
    let t = trust(1, "", "").unwrap();
    assert_eq!(t.sp1.leaf, vec![parse_vkey(LEAF_VKEY).unwrap()]);
    assert_eq!(t.sp1.segment, vec![parse_vkey(SEGMENT_VKEY).unwrap()]);
    assert_eq!(t.openvm, openvm_keys());
    let a = "11".repeat(32);
    let b = format!("0x{}", "22".repeat(32));
    let t = trust(1, "", &format!("{a}, {b}\n")).unwrap();
    assert_eq!(t.sp1.leaf, vec![[0x11; 32], [0x22; 32]]);
    assert_eq!(t.sp1.segment, vec![parse_vkey(SEGMENT_VKEY).unwrap()]);
    assert_eq!(t.openvm, openvm_keys());
    assert_eq!(trust(1, "zz", "").unwrap_err().kind, Kind::InvalidKeys);
    // keys() names both systems, in force and pinned.
    let k = keys_json(&trust(1, "", &a));
    assert_eq!(k["sp1"]["leaf"], json!([a]));
    assert_eq!(k["pinned"]["sp1"]["leaf"], json!([LEAF_VKEY.trim()]));
    assert_eq!(k["openvm"], k["pinned"]["openvm"]);
    assert_eq!(
        k["pinned"]["openvm"]["leaf"],
        json!(["28b819642cb4cbd26da3b3762408c71656423fd4267bb71e12a0c76e08490119"])
    );
    assert_eq!(
        k["pinned"]["openvm"]["segment"],
        json!(["3b585e4468e56c4e4ee629bb67c85e26328301473b4e0d611dde318e47585789"])
    );
    // Ethereum's pinned first OpenVM block, which an SP1 key override leaves as it is.
    assert_eq!(k["openvmFrom"], json!(format!("0x{:x}", 26_361_856)));
}

/// The page decides "browser failure" or "Willow's answer failed a check" by `kind`
/// alone. A server can put any text in a proof id, and that text lands in the message.
#[test]
fn a_rejection_is_rejected_whatever_its_text_says() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let mut p: LogsProof = serde_json::from_str(&answer).unwrap();
    let ProofPart::Block(b) = &mut p.parts[1] else {
        panic!()
    };
    b.proof = "out of memory: Array buffer allocation failed, Maximum memory size exceeded".into();
    let e = run(&c, &serde_json::to_string(&p).unwrap(), &ef, &store)
        .err()
        .unwrap();
    assert!(e.message.contains("out of memory"), "{e:?}");
    assert_eq!(e.kind, Kind::Rejected);
    let j = verified_json(&Err(e));
    assert_eq!(
        (j["ok"].clone(), j["kind"].clone()),
        (json!(false), json!("rejected"))
    );
}

#[test]
fn every_failure_has_a_kind() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let kind = |answer: &str, filter: &Value| run(&c, answer, filter, &store).err().unwrap().kind;
    // Willow's answer: not JSON, or JSON of the wrong shape.
    assert_eq!(kind("<html>502</html>", &ef), Kind::MalformedInput);
    assert_eq!(kind("null", &ef), Kind::MalformedInput);
    assert_eq!(kind(r#"{"chainId": 1}"#, &ef), Kind::MalformedInput);
    let mut p: Value = serde_json::from_str(&answer).unwrap();
    p["parts"][1]["records"] = json!("0xzz");
    assert_eq!(kind(&p.to_string(), &ef), Kind::MalformedInput);
    // The page's own filter.
    assert_eq!(
        kind(&answer, &json!({"fromBlock": "0x1"})),
        Kind::InvalidFilter
    );
    let mut bad_addr = ef.clone();
    bad_addr["address"] = json!("0x1234");
    assert_eq!(kind(&answer, &bad_addr), Kind::InvalidFilter);
    // A proof the store does not hold is a rejection, not a fault in the page.
    let mut fetch = |_: &str| Err::<Vec<u8>, _>("proof was not fetched".to_string());
    let e = check_answer(
        &c.trust(),
        &rpc(&answer),
        &ef.to_string(),
        &mut fetch,
        &mut FakeCheck,
    )
    .err()
    .unwrap();
    assert_eq!(e.kind, Kind::Rejected);

    let out = run(&c, &answer, &ef, &store).unwrap();
    let ok = verified_json(&Ok(out));
    assert_eq!(ok["ok"], json!(true));
    assert!(ok["logs"].as_array().is_some_and(|l| !l.is_empty()));
    assert_eq!(ok["report"]["toBlock"], json!(format!("0x{:x}", START + 7)));
}

#[test]
fn compare_and_keys_answer_with_a_kind() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let out = run(&c, &answer, &ef, &store).unwrap();
    for body in [
        rpc(r#"{"not": "an array"}"#),
        "[]".into(),
        r#"{"id": 1}"#.into(),
    ] {
        let j = compare_json(Some(&out), &body);
        assert_eq!(
            (j["ok"].clone(), j["kind"].clone()),
            (json!(false), json!("malformed-input")),
            "{body}"
        );
    }
    let j = compare_json(Some(&out), &rpc("[]"));
    assert_eq!(
        (j["ok"].clone(), j["result"]["exact"].clone()),
        (json!(true), json!(false))
    );
    let j = compare_json(None, &rpc("[]"));
    assert_eq!(j["kind"], json!("internal"));

    let k = keys_json(&trust(1, "", "zz"));
    assert_eq!(
        (k["ok"].clone(), k["kind"].clone()),
        (json!(false), json!("invalid-keys"))
    );
    let k = keys_json(&Ok(trust(1, "", "").unwrap()));
    assert_eq!(k["ok"], json!(true));
    assert_eq!(
        k["sp1"]["leaf"][0],
        json!(willow_logs_verify::LEAF_VKEY.trim())
    );
}

/// The page passes response bodies as the server sent them. Nesting deep enough to break a
/// browser's JSON.stringify is skipped where the verifier reads nothing, and is
/// `malformed-input` where it reads: never a crash, never a verdict on the browser.
#[test]
fn deep_nesting_is_skipped_or_malformed_input() {
    let c = chain(1, START, 8, 7);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, store) = served(&c, &f);
    let deep = format!("{}{}", "[".repeat(300_000), "]".repeat(300_000));
    let check = |body: &str| {
        let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
        check_answer(
            &c.trust(),
            body,
            &ef.to_string(),
            &mut fetch,
            &mut FakeCheck,
        )
    };
    // Fields the verifier does not read are skipped without recursion.
    let with_x = |a: &str| rpc(&format!(r#"{{"x":{deep},{}"#, &a[1..]));
    assert!(check(&with_x(&answer)).is_ok());
    let deep_id = format!(r#"{{"jsonrpc":"2.0","id":{deep},"result":{answer}}}"#);
    assert!(check(&deep_id).is_ok());
    // A tampered answer is still rejected with the deep field in it.
    let mut p: LogsProof = serde_json::from_str(&answer).unwrap();
    let ProofPart::Block(b) = &mut p.parts[1] else {
        panic!()
    };
    let last = b.records.len() - 1;
    b.records[last] ^= 1;
    let tampered = serde_json::to_string(&p).unwrap();
    assert_eq!(
        check(&with_x(&tampered)).err().unwrap().kind,
        Kind::Rejected
    );
    // Inside a part (buffered to read its "type" tag) the depth limit applies.
    assert!(answer.contains(r#"{"type":"block","#));
    let in_part = answer.replacen(
        r#"{"type":"block","#,
        &format!(r#"{{"type":"block","x":{deep},"#),
        1,
    );
    let e = check(&rpc(&in_part)).err().unwrap();
    assert_eq!(e.kind, Kind::MalformedInput);
    assert!(e.message.contains("recursion limit"), "{e:?}");

    let out = check(&rpc(&answer)).unwrap();
    let plain = Value::Array(out.verified.logs.iter().map(log_json).collect()).to_string();
    let j = compare_json(Some(&out), &rpc(&format!("[{deep},{}", &plain[1..])));
    assert_eq!(
        (j["ok"].clone(), j["kind"].clone()),
        (json!(false), json!("malformed-input"))
    );
    assert!(
        j["message"].as_str().unwrap().contains("recursion limit"),
        "{j}"
    );
    let j = compare_json(
        Some(&out),
        &format!(r#"{{"jsonrpc":"2.0","id":{deep},"result":{plain}}}"#),
    );
    assert_eq!(j["result"]["exact"], json!(true), "{j}");
}

/// Every chain the log RPC serves has the minted keys; any other needs both tiers entered.
#[test]
fn each_served_chain_is_pinned_and_others_are_named_unknown() {
    let chains = pinned_chains_json();
    let ids: Vec<u64> = chains
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["chainId"].as_u64().unwrap())
        .collect();
    assert_eq!(ids, vec![1, 8453, 10, 130, 57073, 42161, 4663]);
    for id in ids {
        let t = trust(id, "", "").unwrap();
        assert_eq!(t.chain_id, id);
        assert_eq!(t.sp1, sp1_keys());
        assert_eq!(t.openvm, openvm_keys());
    }
    let k = keys_json(&trust(4663, "", ""));
    assert_eq!(
        (k["chainId"].clone(), k["chain"].clone()),
        (json!(4663), json!("robinhood"))
    );
    let e = trust(56, "", "").unwrap_err();
    assert_eq!(e.kind, Kind::UnknownChain);
    assert_eq!(keys_json(&Err(e))["kind"], json!("unknown-chain"));
    let (a, b) = ("11".repeat(32), "22".repeat(32));
    assert!(trust(56, &a, "").is_err());
    let t = trust(56, &a, &b).unwrap();
    assert_eq!(
        (t.sp1.segment, t.sp1.leaf),
        (vec![[0x11; 32]], vec![[0x22; 32]])
    );
    assert_eq!(t.openvm, Keys::default());
}

/// An answer across the cutover: an SP1 segment, then OpenVM blocks. The report counts each
/// system's proofs and says where OpenVM starts; a pinned cutover elsewhere is refused.
#[test]
fn an_answer_across_the_cutover_reports_each_systems_proofs() {
    let c = chain(1, START, 8, 11);
    let ef = eth_filter(START + 1, START + 7);
    let f = LogFilter::from_eth_filter(&ef, START + 1, START + 7).unwrap();
    let (answer, mut store) = served(&c, &f);
    for i in 6..8 {
        store.insert(
            c.block_id(i).to_string(),
            fake_openvm_proof(&c.ovm_leaf_vkey(), &c.leaf_pvs[i]),
        );
    }
    let out = run(&c, &answer, &ef, &store).unwrap();
    let r = out.report();
    assert_eq!(r["proofs"], json!({"sp1": 1, "openvm": 2}));
    assert_eq!(r["openvmFrom"], json!(format!("0x{:x}", START + 6)));
    assert_eq!(r["cutover"], Value::Null);
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    let mut t = c.trust();
    t.openvm_from = Some(START + 6);
    let out = check_answer(
        &t,
        &rpc(&answer),
        &ef.to_string(),
        &mut fetch,
        &mut FakeCheck,
    )
    .unwrap();
    assert_eq!(out.report()["cutover"], json!(format!("0x{:x}", START + 6)));
    t.openvm_from = Some(START + 7);
    let e = check_answer(
        &t,
        &rpc(&answer),
        &ef.to_string(),
        &mut fetch,
        &mut FakeCheck,
    )
    .err()
    .unwrap();
    assert_eq!(e.kind, Kind::Rejected);
    assert!(
        e.message
            .contains("is proven with OpenVM, but this chain uses SP1"),
        "{e:?}"
    );
}

/// A fast chain's answer: an L1 over two 4-block leaves, then a 4-block leaf part.
#[test]
fn a_fast_chain_answer_with_multi_block_leaves_verifies() {
    use logset_query::LeafProof;
    const F: u64 = 510_656_512;
    let c = chain(42161, F, 12, 9);
    let ef = eth_filter(F + 1, F + 11);
    let f = LogFilter::from_eth_filter(&ef, F + 1, F + 11).unwrap();
    let mut store = HashMap::new();
    let pv = c.l1_over(4, 0, 8);
    let ix = SegmentIndex::build(&pv, &c.records[0..8]).unwrap();
    let seg_id = c.segment_id(0, 8).to_string();
    store.insert(seg_id.clone(), fake_proof(&L1_VK, &pv));
    let (leaf_pv, leaf_proof, _) = c.leaf_files(8, 12);
    let leaf_id = c.leaf_id(8, 12).to_string();
    store.insert(leaf_id.clone(), leaf_proof);
    let p = LogsProof {
        chain_id: 42161,
        from_block: F + 1,
        to_block: F + 11,
        parts: vec![
            ProofPart::Segment(SegmentProof {
                ranges: ix.prove(&f).unwrap().into_iter().map(Into::into).collect(),
                public_values: pv,
                proof: seg_id,
            }),
            ProofPart::Leaf(LeafProof {
                public_values: leaf_pv,
                proof: leaf_id,
                records: c.records[8..12].to_vec(),
            }),
        ],
    };
    let answer = serde_json::to_string(&p).unwrap();
    let out = run(&c, &answer, &ef, &store).unwrap();
    let want: Vec<Log> = c.all_logs().into_iter().filter(|l| f.matches(l)).collect();
    assert_eq!(out.verified.logs, want);
    assert_eq!(
        out.report()["parts"],
        json!({"segments": 1, "blocks": 0, "leaves": 1})
    );
    // The client's chain, not the server's: an Arbitrum answer is not Ethereum's.
    let mut fetch = |id: &str| store.get(id).cloned().ok_or_else(|| "missing".to_string());
    let e = check_answer(
        &trust(1, "", "").unwrap(),
        &rpc(&answer),
        &ef.to_string(),
        &mut fetch,
        &mut FakeCheck,
    )
    .err()
    .unwrap();
    assert_eq!(e.kind, Kind::Rejected);
}

/// The page's test build (feature `test-cutover`) can replace the chain's pinned first OpenVM
/// block, either way; build.sh checks that no other build has `setTestCutover` at all.
#[cfg(feature = "test-cutover")]
#[test]
fn the_test_build_replaces_the_pinned_cutover() {
    let keys = |v: &Verifier| serde_json::from_str::<Value>(&v.keys()).unwrap();
    let mut v = Verifier::new(1, "", "");
    assert_eq!(keys(&v)["openvmFrom"], json!(format!("0x{:x}", 26_361_856)));
    v.set_test_cutover(Some(START));
    assert_eq!(keys(&v)["openvmFrom"], json!(format!("0x{START:x}")));
    v.set_test_cutover(None);
    assert_eq!(keys(&v)["openvmFrom"], Value::Null);
    // A verifier whose keys were refused stays refused.
    let mut bad = Verifier::new(1, "", "zz");
    bad.set_test_cutover(Some(START));
    assert_eq!(keys(&bad)["kind"], json!("invalid-keys"));
}
