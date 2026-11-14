//! Block proofs (`willow_getBlockProof`, interface I1) on the committed
//! fixtures of all seven chains: each renders `eth_getBlockByNumber` with
//! transaction hashes and in full exactly as publicnode reported it, and every
//! tampered proof fails with its expected kind.

use crate::compare::compare;
use crate::fault::{fault, Fault};
use crate::proof::{decode_header, verify, Block, BlockProof, Subject};
use crate::tests_live::{read, CRATE};
use serde_json::Value;

pub(crate) struct Fixture {
    pub name: String,
    pub chain: u64,
    pub params: Value,
    pub result: Value,
    pub file: Value,
}

/// Every fixture of `dir` (under `fixtures/`) whose name ends with `suffix`, by name.
pub(crate) fn fixtures(dir: &str, suffix: &str) -> Vec<Fixture> {
    let dir = format!("{CRATE}/fixtures/{dir}");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir}: {e}"))
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(suffix))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no {suffix} fixtures in {dir}");
    names
        .into_iter()
        .map(|name| {
            let file = read(format!("{dir}/{name}"));
            Fixture {
                chain: file["chainId"].as_u64().unwrap(),
                params: file["request"]["params"].clone(),
                result: file["result"].clone(),
                name,
                file,
            }
        })
        .collect()
}

/// A block parameter: a quantity, or `{"blockHash"}`.
pub(crate) fn block_param(v: &Value) -> Block {
    if let Some(h) = v.get("blockHash") {
        return Block::Hash(crate::hex_fixed::<32>(h).unwrap());
    }
    Block::Number(crate::hex_u64(v).unwrap())
}

/// `p` fails as the response to `s` with kind `want`.
pub(crate) fn fails(p: &BlockProof, chain: u64, s: Subject, want: Fault, what: &str) {
    match verify(p, chain, &s) {
        Ok(_) => panic!("{what}: a tampered proof verified"),
        Err(e) => assert_eq!(fault(&e), Some(want), "{what}: {e:#}"),
    }
}

pub(crate) fn hexs(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

pub(crate) fn flip(s: &str, at: usize) -> String {
    let mut b = hex::decode(&s[2..]).unwrap();
    let at = at.min(b.len() - 1);
    b[at] ^= 0x01;
    hexs(&b)
}

fn proof(f: &Fixture) -> BlockProof {
    serde_json::from_value(f.result.clone()).unwrap()
}

/// The node's `eth_getBlockByNumber(n, false)`: its full block with each
/// transaction as its hash.
fn hashes_only(full: &Value) -> Value {
    let mut b = full.clone();
    b["transactions"] = Value::Array(
        full["transactions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["hash"].clone())
            .collect(),
    );
    b
}

#[test]
fn every_chains_block_proof_renders_as_publicnode_reports_it() {
    let all = fixtures("block", ".json");
    let chains: std::collections::BTreeSet<u64> = all.iter().map(|f| f.chain).collect();
    assert_eq!(chains.len(), 7, "fixtures of all seven chains: {chains:?}");
    for f in &all {
        let p = proof(f);
        let by = block_param(&f.params[0]);
        let v = verify(&p, f.chain, &Subject::Block(by))
            .unwrap_or_else(|e| panic!("{}: {e:#}", f.name));
        // The same proof by number.
        verify(&p, f.chain, &Subject::Block(Block::Number(v.block_number))).unwrap();
        let node = &f.file["node"]["eth_getBlockByNumber"];
        let unproven_uncles = f.name.contains("uncles-unproven");
        for full in [false, true] {
            let r = v.block(full).unwrap();
            let theirs = if full {
                node.clone()
            } else {
                hashes_only(node)
            };
            let c = compare(&r, &theirs);
            assert!(c.is_exact(), "{} full={full}: {:?}", f.name, c.differ);
            let expected: &[&str] = if unproven_uncles {
                &["size", "uncles"]
            } else {
                &[]
            };
            assert_eq!(r.unproven, expected, "{} full={full}", f.name);
            // What the node reports and the proof does not: totalDifficulty
            // (no block commits to it), and the unproven fields.
            for k in &c.unproven {
                assert!(
                    k == "totalDifficulty" || expected.contains(&k.as_str()),
                    "{} full={full}: the node reports {k}, which we leave out",
                    f.name
                );
            }
            assert!(c.matched > 15, "{}", f.name);
        }
        assert_eq!(v.answer().unwrap(), v.block(false).unwrap());
    }
}

/// Which fixture each tamper uses.
fn fixture(name: &str) -> (Fixture, BlockProof) {
    let f = fixtures("block", ".json")
        .into_iter()
        .find(|f| f.name.starts_with(name))
        .unwrap_or_else(|| panic!("no {name} fixture"));
    let p = proof(&f);
    (f, p)
}

/// The proof with its withdrawals list changed by `f`.
fn with_withdrawals(
    p: &BlockProof,
    f: impl Fn(&mut Vec<alloy::eips::eip4895::Withdrawal>),
) -> BlockProof {
    use alloy_rlp::Decodable;
    let raw = hex::decode(&p.withdrawals.as_ref().unwrap()[2..]).unwrap();
    let mut list = Vec::<alloy::eips::eip4895::Withdrawal>::decode(&mut raw.as_slice()).unwrap();
    f(&mut list);
    let mut q = p.clone();
    q.withdrawals = Some(hexs(&alloy_rlp::encode(&list)));
    q
}

#[test]
fn a_tampered_block_proof_fails_with_its_kind() {
    // Ethereum after Fusaka: transactions and withdrawals.
    let (f, good) = fixture("ethereum-26");
    let (chain, s) = (f.chain, Subject::Block(block_param(&f.params[0])));
    assert!(verify(&good, chain, &s).is_ok());
    let n = crate::proof::parse_qty(&good.block_number, "n").unwrap();

    let mut p = good.clone();
    p.header = flip(&p.header, 100);
    fails(&p, chain, s, Fault::Header, "a byte of the header");
    let mut p = good.clone();
    p.block_number = format!("0x{:x}", n + 1);
    fails(&p, chain, s, Fault::Header, "the block number");
    fails(
        &good,
        chain,
        Subject::Block(Block::Number(n + 1)),
        Fault::Block,
        "another block asked for",
    );
    fails(&good, 10, s, Fault::Chain, "another chain");

    let mut p = good.clone();
    let t = p.transactions.as_mut().unwrap();
    t[3] = flip(&t[3], 40);
    fails(&p, chain, s, Fault::Transactions, "a transaction changed");
    let mut p = good.clone();
    p.transactions.as_mut().unwrap().remove(5);
    fails(&p, chain, s, Fault::Transactions, "a transaction dropped");
    let mut p = good.clone();
    p.transactions.as_mut().unwrap().swap(1, 2);
    fails(&p, chain, s, Fault::Transactions, "transactions reordered");
    let mut p = good.clone();
    p.transactions = None;
    fails(&p, chain, s, Fault::Missing, "no transactions");
    let mut p = good.clone();
    p.transaction_index = Some("0x0".into());
    fails(&p, chain, s, Fault::Malformed, "a transactionIndex");

    let p = with_withdrawals(&good, |w| w[0].amount += 1);
    fails(&p, chain, s, Fault::Withdrawals, "a withdrawal amount");
    let p = with_withdrawals(&good, |w| {
        w.pop();
    });
    fails(&p, chain, s, Fault::Withdrawals, "a withdrawal dropped");
    let p = with_withdrawals(&good, |w| w.swap(0, 1));
    fails(&p, chain, s, Fault::Withdrawals, "withdrawals reordered");
    let mut p = good.clone();
    p.withdrawals = None;
    fails(&p, chain, s, Fault::Missing, "no withdrawals");
    let mut p = good.clone();
    p.withdrawals = Some(format!("{}00", p.withdrawals.unwrap()));
    fails(
        &p,
        chain,
        s,
        Fault::Withdrawals,
        "a byte after the withdrawals list",
    );

    let mut p = good.clone();
    p.unproven = vec!["size".into(), "uncles".into()];
    fails(
        &p,
        chain,
        s,
        Fault::Uncles,
        "uncles left unproven on a block without uncles",
    );
    let mut p = good.clone();
    p.unproven = vec!["stateRoot".into()];
    fails(
        &p,
        chain,
        s,
        Fault::Malformed,
        "another field left unproven",
    );
    let mut p = good.clone();
    p.uncles = Some(vec![]);
    fails(&p, chain, s, Fault::Malformed, "an empty uncles list");

    // Fields a block proof may not have elsewhere.
    fails(
        &good,
        chain,
        Subject::BlockReceipts(Block::Number(n)),
        Fault::Malformed,
        "a block proof as a block-receipts proof",
    );

    // Before the merge: an uncle.
    let (f, good) = fixture("ethereum-1500000.json");
    let s = Subject::Block(block_param(&f.params[0]));
    assert!(verify(&good, 1, &s).is_ok());
    let mut p = good.clone();
    let u = p.uncles.as_mut().unwrap();
    u[0] = flip(&u[0], 50);
    fails(&p, 1, s, Fault::Uncles, "an uncle changed");
    let mut p = good.clone();
    p.uncles = None;
    fails(&p, 1, s, Fault::Uncles, "the uncle dropped");
    let mut p = good.clone();
    p.unproven = vec!["size".into(), "uncles".into()];
    fails(
        &p,
        1,
        s,
        Fault::Malformed,
        "an uncle both given and unproven",
    );
    let mut p = good.clone();
    p.withdrawals = Some("0xc0".into());
    fails(&p, 1, s, Fault::Withdrawals, "withdrawals before Shanghai");
    let (_, unproven) = fixture("ethereum-1500000-uncles-unproven");
    assert!(verify(&unproven, 1, &s).is_ok());

    // The OP stack after Isthmus: no withdrawals, and its deposits' receipts.
    let (f, good) = fixture("op-");
    let s = Subject::Block(block_param(&f.params[0]));
    assert!(verify(&good, 10, &s).is_ok());
    assert_eq!(good.withdrawals.as_deref(), Some("0xc0"));
    let (_, eth) = fixture("ethereum-26");
    let mut p = good.clone();
    p.withdrawals = eth.withdrawals.clone();
    fails(
        &p,
        10,
        s,
        Fault::Withdrawals,
        "an OP-stack block listing withdrawals",
    );
    let mut p = good.clone();
    let k = p.receipt_proofs.len() - 1;
    p.receipt_proofs[k].value = flip(&p.receipt_proofs[k].value, 20);
    fails(&p, 10, s, Fault::Receipts, "a deposit's receipt changed");
    // Without its deposits' receipts, a deposit's nonce is unproven, not wrong.
    let mut p = good.clone();
    p.receipt_proofs.clear();
    let v = verify(&p, 10, &s).unwrap();
    assert!(v
        .block(true)
        .unwrap()
        .unproven
        .contains(&"transactions[].nonce".to_string()));
}

/// The v2 fields round-trip and are named as the wire names them.
#[test]
fn the_block_proof_wire_form_round_trips() {
    for f in fixtures("block", ".json") {
        let p = proof(&f);
        let back = serde_json::to_value(&p).unwrap();
        assert_eq!(back, f.result, "{}", f.name);
        let o = back.as_object().unwrap();
        for k in o.keys() {
            assert!(
                [
                    "chainId",
                    "blockNumber",
                    "blockHash",
                    "header",
                    "transactions",
                    "receiptProofs",
                    "withdrawals",
                    "uncles",
                    "unproven"
                ]
                .contains(&k.as_str()),
                "{}: {k}",
                f.name
            );
        }
        let h = decode_header(&hex::decode(&p.header[2..]).unwrap()).unwrap();
        assert_eq!(
            h.withdrawals_root.is_some(),
            p.withdrawals.is_some(),
            "{}",
            f.name
        );
    }
}

/// Each kind has its own name on the wire.
#[test]
fn fault_kinds_have_distinct_names() {
    let names: std::collections::BTreeSet<&str> = Fault::ALL.iter().map(|f| f.as_str()).collect();
    assert_eq!(names.len(), Fault::ALL.len());
    let e = crate::fault::reject(Fault::Uncles, "x");
    let e = e.context("outer");
    assert_eq!(fault(&e), Some(Fault::Uncles));
    assert_eq!(fault(&anyhow::anyhow!("untagged")), None);
}
