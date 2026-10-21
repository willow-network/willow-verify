//! Proofs over real blocks (a Base block's receipts trie; Robinhood Chain and
//! Arbitrum One blocks with Nitro and signed Ethereum types), and every way a
//! tampered proof must fail. The log store's tests cover the seven chains'
//! served fixture blocks end to end.

use crate::compare::compare;
use crate::mpt;
use crate::proof::{
    build_block, build_item, decode_header, verify, Block, BlockProof, Item, Subject,
};
use logset_blockcheck::receipts::values_and_logs_from_rpc;
use serde_json::Value;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../fixtures");

/// A real block: chain, header RLP, transaction and receipt values, and the
/// node's block (full transactions) and receipts JSON.
struct Real {
    chain: u64,
    header: Vec<u8>,
    txs: Vec<Vec<u8>>,
    receipts: Vec<Vec<u8>>,
    full: Value,
    receipts_json: Value,
}

impl Real {
    fn tx_refs(&self) -> Vec<&[u8]> {
        self.txs.iter().map(Vec::as_slice).collect()
    }

    fn receipt_refs(&self) -> Vec<&[u8]> {
        self.receipts.iter().map(Vec::as_slice).collect()
    }

    fn item(&self, i: usize, receipt: bool) -> BlockProof {
        build_item(
            self.chain,
            &self.header,
            &self.tx_refs(),
            &self.receipt_refs(),
            i,
            receipt,
        )
        .unwrap()
    }

    fn block(&self) -> BlockProof {
        build_block(
            self.chain,
            &self.header,
            &self.tx_refs(),
            &self.receipt_refs(),
        )
        .unwrap()
    }

    fn hash(&self) -> [u8; 32] {
        alloy_primitives::keccak256(&self.header).0
    }

    fn tx_hash(&self, i: usize) -> [u8; 32] {
        alloy_primitives::keccak256(&self.txs[i]).0
    }
}

fn orbit(name: &str, chain: u64) -> Real {
    let read = |ext: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(format!("{FIXTURES}/orbit/{name}.{ext}.json")).unwrap(),
        )
        .unwrap()
    };
    let full = read("full");
    let receipts_json = read("receipts");
    let h = logset_blockcheck::verify_header(&full).unwrap();
    let body = logset_blockcheck::verify_body(chain, &h, &full).unwrap();
    let (receipts, _) = values_and_logs_from_rpc(&receipts_json, chain).unwrap();
    Real {
        chain,
        header: h.rlp,
        txs: body.tx_values,
        receipts,
        full,
        receipts_json,
    }
}

/// Nitro blocks of every effectiveGasPrice rule: Arbitrum One's first Nitro
/// block under ArbOS 6, one before tip collection (507,965,087) and the first
/// block that collects tips (508,190,474); Robinhood Chain, which never does.
fn nitro_blocks() -> Vec<Real> {
    vec![
        orbit("robinhood-69562157", 4663),
        orbit("arbitrum-one-22207818", 42161),
        orbit("arbitrum-one-507965087", 42161),
        orbit("arbitrum-one-508190474", 42161),
    ]
}

/// The 115 receipt values of Base block 48,932,815 and its header RLP.
fn base_receipts() -> (Vec<u8>, Vec<Vec<u8>>) {
    let raw = std::fs::read(format!("{FIXTURES}/base/48932815.receipts.bin")).unwrap();
    let (mut out, mut at) = (Vec::new(), 0usize);
    while at < raw.len() {
        let n = u32::from_le_bytes(raw[at..at + 4].try_into().unwrap()) as usize;
        out.push(raw[at + 4..at + 4 + n].to_vec());
        at += 4 + n;
    }
    let header = std::fs::read(format!("{FIXTURES}/base/48932815.header.rlp")).unwrap();
    (header, out)
}

/// Values long enough that every leaf is hash-referenced, as real ones are.
fn synthetic(n: usize) -> Vec<Vec<u8>> {
    (0..n)
        .map(|i| {
            let mut v = vec![0xf8, 0x40];
            v.extend((0..64).map(|k| (i * 31 + k) as u8));
            v
        })
        .collect()
}

#[test]
fn every_item_of_real_and_synthetic_tries_proves_and_verifies() {
    let (header, receipts) = base_receipts();
    let root = decode_header(&header).unwrap().receipts_root.0;
    let all: Vec<usize> = (0..receipts.len()).collect();
    let (built, proofs) = mpt::prove(&receipts, &all).unwrap();
    assert_eq!(built, root, "the proof builder's root is the header's");
    for (i, p) in proofs.iter().enumerate() {
        assert_eq!(
            mpt::verify(&root, i, p).unwrap(),
            receipts[i],
            "receipt {i}"
        );
    }
    // Sizes around the keys' encoding edges: rlp(0) = 0x80, 0x7f, 0x80 = 0x8180.
    for n in [1, 2, 3, 127, 128, 129, 130, 300] {
        let values = synthetic(n);
        let root = logset_blockcheck::trie::ordered_root(&values).unwrap();
        let all: Vec<usize> = (0..n).collect();
        let (built, proofs) = mpt::prove(&values, &all).unwrap();
        assert_eq!(built, root, "{n} values");
        for (i, p) in proofs.iter().enumerate() {
            assert_eq!(
                mpt::verify(&root, i, p).unwrap(),
                values[i],
                "{n} values, item {i}"
            );
        }
    }
}

#[test]
fn a_tampered_trie_proof_fails() {
    let (header, receipts) = base_receipts();
    let root = decode_header(&header).unwrap().receipts_root.0;
    let (_, proofs) = mpt::prove(&receipts, &[5, 6, 114]).unwrap();
    let p = &proofs[0];
    assert!(mpt::verify(&root, 5, p).is_ok());
    // Another item's proof, or this proof read as another item.
    assert!(mpt::verify(&root, 6, p).is_err());
    assert!(mpt::verify(&root, 5, &proofs[1]).is_err());
    // A wrong root.
    let mut other = root;
    other[0] ^= 1;
    assert!(mpt::verify(&other, 5, p).is_err());
    // Any byte of any node.
    for n in 0..p.len() {
        for at in [0, p[n].len() / 2, p[n].len() - 1] {
            let mut t = p.clone();
            t[n][at] ^= 0x01;
            assert!(mpt::verify(&root, 5, &t).is_err(), "node {n} byte {at}");
        }
    }
    // A node dropped, a node added, the proof cut short.
    for n in 0..p.len() {
        let mut t = p.clone();
        t.remove(n);
        assert!(mpt::verify(&root, 5, &t).is_err(), "without node {n}");
    }
    let mut t = p.clone();
    t.push(proofs[2].last().unwrap().clone());
    assert!(mpt::verify(&root, 5, &t).is_err());
    assert!(mpt::verify(&root, 5, &[]).is_err());
    // An index past the trie: no leaf for it.
    let (_, last) = mpt::prove(&receipts, &[114]).unwrap();
    assert!(mpt::verify(&root, 115, &last[0]).is_err());
    assert!(mpt::verify(&root, usize::MAX, &last[0]).is_err());
}

/// Nitro blocks (internal 0x6A, submit-retryable 0x69, retry 0x68, legacy and
/// 0x2): each transaction and receipt proof verifies by hash and by position,
/// and renders the node's own objects, field for field; only the two
/// passed-through Nitro fields are left unproven.
#[test]
fn real_transaction_and_receipt_proofs_render_the_nodes_objects() {
    for b in nitro_blocks() {
        let n = decode_header(&b.header).unwrap().number;
        let node_txs = b.full["transactions"].as_array().unwrap();
        let node_receipts = b.receipts_json.as_array().unwrap();
        for i in 0..b.txs.len() {
            let at = format!("chain {} block {n} item {i}", b.chain);
            let by = [
                Item::Hash(b.tx_hash(i)),
                Item::At(Block::Number(n), i as u64),
                Item::At(Block::Hash(b.hash()), i as u64),
            ];
            for item in by {
                let tx = verify(&b.item(i, false), b.chain, &Subject::Transaction(item)).unwrap();
                assert_eq!(tx.block_hash, b.hash());
                let r = tx.answer().unwrap();
                assert!(r.unproven.is_empty(), "{at}: {:?}", r.unproven);
                let c = compare(&r, &node_txs[i]);
                assert!(c.is_exact(), "{at}: {:?}", c.differ);
                assert!(c.matched > 10, "{at}");

                let rc = verify(&b.item(i, true), b.chain, &Subject::Receipt(item)).unwrap();
                let r = rc.answer().unwrap();
                let c = compare(&r, &node_receipts[i]);
                assert!(c.differ.is_empty(), "{at}: {:?}", c.differ);
                for f in &c.unproven {
                    assert!(
                        ["gasUsedForL1", "timeboosted"].contains(&f.as_str())
                            || f.ends_with(".logIndex"),
                        "{at}: {f} unproven"
                    );
                }
                assert!(r.unproven.contains(&"gasUsedForL1".to_string()));
            }
        }
        // The whole block: every receipt, with every log index.
        let all = verify(
            &b.block(),
            b.chain,
            &Subject::BlockReceipts(Block::Number(n)),
        )
        .unwrap();
        assert!(all.complete);
        let r = all.answer().unwrap();
        assert_eq!(r.unproven, vec!["[].gasUsedForL1", "[].timeboosted"]);
        let c = compare(&r, &b.receipts_json);
        assert!(c.differ.is_empty(), "{:?}", c.differ);
        assert!(c
            .unproven
            .iter()
            .all(|f| f.ends_with(".gasUsedForL1") || f.ends_with(".timeboosted")));
        let t = compare(&all.transactions_json().unwrap(), &b.full["transactions"]);
        assert!(t.is_exact(), "{:?}", t.differ);
        // A changed value in the answer is caught.
        let mut wrong = b.receipts_json.clone();
        wrong[0]["status"] = "0x0".into();
        assert_eq!(compare(&r, &wrong).differ.len(), 1);
        // Arbitrum One's first Nitro block emitted no logs.
        let with_logs = node_receipts
            .iter()
            .position(|r| !r["logs"].as_array().unwrap().is_empty());
        if let Some(k) = with_logs {
            let mut wrong = b.receipts_json.clone();
            wrong[k]["logs"] = Value::Array(vec![]);
            assert!(!compare(&r, &wrong).differ.is_empty());
        }
    }
}

/// A receipt proof carries the previous receipt, not every earlier one: its
/// logs' block-wide `logIndex` is proven for the first two receipts only, and
/// otherwise left to the block proof.
#[test]
fn a_receipt_proof_leaves_later_log_indexes_to_the_block_proof() {
    let b = orbit("arbitrum-one-507965087", 42161);
    for i in [1, 3] {
        let s = Subject::Receipt(Item::Hash(b.tx_hash(i)));
        let r = verify(&b.item(i, true), b.chain, &s)
            .unwrap()
            .answer()
            .unwrap();
        let logs = r.json["logs"].as_array().unwrap();
        assert!(!logs.is_empty());
        assert!(logs[0].get("transactionHash").is_some());
        let proven = i == 1;
        assert_eq!(logs[0].get("logIndex").is_some(), proven, "receipt {i}");
        assert_eq!(
            !r.unproven.contains(&"logs[].logIndex".to_string()),
            proven,
            "receipt {i}"
        );
        let c = compare(&r, &b.receipts_json[i]);
        assert!(c.differ.is_empty(), "{:?}", c.differ);
        assert_eq!(c.unproven.iter().any(|f| f.ends_with("logIndex")), !proven);
    }
}

fn rejects(p: &BlockProof, chain: u64, s: Subject, what: &str) {
    match verify(p, chain, &s) {
        Err(_) => {}
        Ok(v) => match v.answer() {
            Err(_) => {}
            Ok(_) => panic!("{what}: a tampered proof verified"),
        },
    }
}

fn hexs(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

fn flip(s: &str, at: usize) -> String {
    let mut b = hex::decode(&s[2..]).unwrap();
    let at = at.min(b.len() - 1);
    b[at] ^= 0x01;
    hexs(&b)
}

#[test]
fn a_tampered_proof_fails() {
    let b = orbit("arbitrum-one-507965087", 42161);
    let other = orbit("robinhood-69562157", 4663);
    let n = decode_header(&b.header).unwrap().number;
    let i = 1usize;
    let by_hash = Subject::Receipt(Item::Hash(b.tx_hash(i)));
    let good = b.item(i, true);
    assert!(verify(&good, b.chain, &by_hash).is_ok());

    // A changed item: the receipt, the transaction, a proof node.
    let mut p = good.clone();
    let k = p.receipt_proofs.len() - 1;
    p.receipt_proofs[k].value = flip(&p.receipt_proofs[k].value, 40);
    rejects(&p, b.chain, by_hash, "changed receipt");
    let mut p = good.clone();
    p.transaction_proofs[0].value = flip(&p.transaction_proofs[0].value, 10);
    rejects(&p, b.chain, by_hash, "changed transaction");
    let mut p = good.clone();
    p.receipt_proofs[k].proof[0] = flip(&p.receipt_proofs[k].proof[0], 3);
    rejects(&p, b.chain, by_hash, "changed node");

    // A wrong index: the item's, or the subject's.
    let mut p = good.clone();
    p.receipt_proofs[k].index = "0x2".into();
    rejects(&p, b.chain, by_hash, "item moved");
    let mut p = good.clone();
    p.transaction_index = Some("0x2".into());
    rejects(&p, b.chain, by_hash, "subject moved");
    rejects(
        &good,
        b.chain,
        Subject::Receipt(Item::At(Block::Number(n), 2)),
        "asked for another position",
    );
    rejects(
        &good,
        b.chain,
        Subject::Receipt(Item::Hash(b.tx_hash(2))),
        "asked for another transaction",
    );

    // A wrong root: a header changed anywhere no longer hashes to blockHash,
    // and with blockHash recomputed it is another block (and its roots don't
    // hold the items).
    let mut p = good.clone();
    p.header = flip(&p.header, 200);
    rejects(&p, b.chain, by_hash, "changed header");
    let mut h = decode_header(&b.header).unwrap();
    h.receipts_root.0[0] ^= 1;
    let forged = alloy_rlp::encode(&h);
    let mut p = good.clone();
    p.header = hexs(&forged);
    p.block_hash = hexs(&alloy_primitives::keccak256(&forged).0);
    rejects(&p, b.chain, by_hash, "wrong receipts root");

    // A forged header with items to match verifies only as another block: the
    // block hash is the client's to confirm, and asking by hash refuses it.
    let txs = b.tx_refs();
    let mut fake = b.receipts.clone();
    fake[i] = b.receipts[0].clone();
    let fake_refs: Vec<&[u8]> = fake.iter().map(Vec::as_slice).collect();
    let mut h = decode_header(&b.header).unwrap();
    h.receipts_root.0 = logset_blockcheck::trie::ordered_root(&fake).unwrap();
    let forged = alloy_rlp::encode(&h);
    let p = build_item(b.chain, &forged, &txs, &fake_refs, i, true).unwrap();
    let v = verify(&p, b.chain, &by_hash).unwrap();
    assert_ne!(v.block_hash, b.hash(), "a forged header is another block");
    rejects(
        &p,
        b.chain,
        Subject::Receipt(Item::At(Block::Hash(b.hash()), i as u64)),
        "forged header, asked by block hash",
    );

    // A proof for a different block: its items under this header, this
    // header's items under its.
    let mut p = good.clone();
    p.header = hexs(&other.header);
    p.block_hash = hexs(&other.hash());
    p.block_number = format!("0x{:x}", decode_header(&other.header).unwrap().number);
    rejects(&p, b.chain, by_hash, "another block's header");
    let mut p = good.clone();
    p.receipt_proofs = other.item(1, true).receipt_proofs;
    rejects(&p, b.chain, by_hash, "another block's receipt proofs");

    // The redundant fields must agree; the chain must be the one asked about.
    let mut p = good.clone();
    p.block_number = "0x1".into();
    rejects(&p, b.chain, by_hash, "blockNumber");
    let mut p = good.clone();
    p.block_hash = flip(&p.block_hash, 0);
    rejects(&p, b.chain, by_hash, "blockHash");
    rejects(&good, 1, by_hash, "another chain");

    // Items the subject needs.
    let mut p = good.clone();
    p.transaction_proofs.clear();
    rejects(&p, b.chain, by_hash, "no transaction");
    let mut p = good.clone();
    p.receipt_proofs.retain(|r| r.index != "0x1");
    rejects(&p, b.chain, by_hash, "no receipt");
    let mut p = good.clone();
    p.transaction_index = None;
    rejects(&p, b.chain, by_hash, "no transactionIndex");
    let mut p = good.clone();
    let dup = p.receipt_proofs[k].clone();
    p.receipt_proofs.push(dup);
    rejects(&p, b.chain, by_hash, "an item twice");
    let mut p = good.clone();
    p.transaction_index = Some("0xffffffffffffffff".into());
    rejects(&p, b.chain, by_hash, "an index past any block");
    let mut p = good.clone();
    p.receipt_proofs[k].index = "0x100000".into();
    rejects(&p, b.chain, by_hash, "an item index past any block");

    // A block proof: a changed, dropped, added or reordered value fails to
    // rebuild the root; item proofs don't prove a whole block.
    let all = Subject::BlockReceipts(Block::Number(n));
    let good = b.block();
    assert!(verify(&good, b.chain, &all).is_ok());
    let mut p = good.clone();
    let list = p.receipts.as_mut().unwrap();
    list[2] = flip(&list[2], 30);
    rejects(&p, b.chain, all, "changed receipt in the list");
    let mut p = good.clone();
    p.receipts.as_mut().unwrap().pop();
    rejects(&p, b.chain, all, "a receipt dropped");
    let mut p = good.clone();
    p.transactions.as_mut().unwrap().swap(1, 2);
    rejects(&p, b.chain, all, "transactions reordered");
    let mut p = good.clone();
    let extra = p.receipts.as_ref().unwrap()[0].clone();
    p.receipts.as_mut().unwrap().push(extra);
    rejects(&p, b.chain, all, "a receipt added");
    rejects(
        &b.item(1, true),
        b.chain,
        all,
        "an item proof as a block proof",
    );
    rejects(
        &good,
        b.chain,
        Subject::BlockReceipts(Block::Number(n + 1)),
        "another block number",
    );
    let mut p = good.clone();
    p.receipt_proofs = b.item(1, true).receipt_proofs;
    rejects(&p, b.chain, all, "a list and item proofs");
}

/// Non-canonical header RLP decodes but doesn't re-encode: refused, so no
/// field can hide from the block hash.
#[test]
fn a_non_canonical_header_is_refused() {
    let b = orbit("robinhood-69562157", 4663);
    assert!(decode_header(&b.header).is_ok());
    // The same list with its length in a longer form than needed.
    let (payload, _, _) = logset_blockcheck::receipts::rlp_item(&b.header).unwrap();
    let len = payload.len();
    let mut long = vec![0xfa];
    long.extend_from_slice(&(len as u32).to_be_bytes()[1..]);
    long.extend_from_slice(payload);
    assert!(decode_header(&long).is_err());
    let mut trailing = b.header.clone();
    trailing.push(0x80);
    assert!(decode_header(&trailing).is_err());
}

/// An empty block's lists rebuild the empty root.
#[test]
fn an_empty_block_proves_no_receipts() {
    let b = orbit("robinhood-69562157", 4663);
    let mut h = decode_header(&b.header).unwrap();
    h.transactions_root = alloy_trie::EMPTY_ROOT_HASH;
    h.receipts_root = alloy_trie::EMPTY_ROOT_HASH;
    let header = alloy_rlp::encode(&h);
    let p = build_block(4663, &header, &[], &[]).unwrap();
    let v = verify(&p, 4663, &Subject::BlockReceipts(Block::Tag)).unwrap();
    assert_eq!(v.answer().unwrap().json, Value::Array(vec![]));
}

/// The wire form round-trips through JSON.
#[test]
fn the_wire_form_round_trips() {
    let b = orbit("robinhood-69562157", 4663);
    for p in [b.item(2, true), b.item(0, false), b.block()] {
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(
            serde_json::from_value::<BlockProof>(json.clone()).unwrap(),
            p
        );
        assert!(json.get("chainId").is_some() && json.get("header").is_some());
    }
}

fn opstack(name: &str, chain: u64) -> Real {
    let read = |ext: &str| -> Value {
        serde_json::from_slice(
            &std::fs::read(format!("{FIXTURES}/opstack/{name}.{ext}.json")).unwrap(),
        )
        .unwrap()
    };
    let full = read("full");
    let receipts_json = read("receipts");
    let h = logset_blockcheck::verify_header(&full).unwrap();
    let body = logset_blockcheck::verify_body(chain, &h, &full).unwrap();
    let (receipts, _) = values_and_logs_from_rpc(&receipts_json, chain).unwrap();
    assert_eq!(
        logset_blockcheck::receipts::receipts_root(&receipts).unwrap(),
        h.info.receipts_root,
        "{name}"
    );
    Real {
        chain,
        header: h.rlp,
        txs: body.tx_values,
        receipts,
        full,
        receipts_json,
    }
}

/// OP-stack blocks of each L1 fee era and each deposit receipt form: before
/// Canyon (the Regolith nonce passed through, Bedrock prices; Base 1 holds
/// only its L1 info deposit), the last block before Ecotone, Ecotone's first
/// block (Bedrock's L1 info layout, upgrade deposits creating contracts) and
/// an Ecotone block. The whole block renders as its node reported it; a proof
/// of every item proves every field but a pre-Canyon deposit's nonce.
#[test]
fn op_stack_blocks_of_every_fee_era_render_as_their_nodes_report_them() {
    use crate::render::{difference, Context, View};
    use crate::{txs, ReceiptExtras, Tx};
    for (name, chain, pre_canyon) in [
        ("base-1", 8453u64, true),
        ("base-9101526", 8453, true),
        ("op-117387811", 10, false),
        ("op-117387812", 10, false),
        ("op-117387820", 10, false),
    ] {
        let b = opstack(name, chain);
        let node_txs = b.full["transactions"].as_array().unwrap();
        let node_receipts = b.receipts_json.as_array().unwrap();
        // What the log store keeps: senders, and the nonce a pre-Canyon
        // deposit receipt's bytes lack.
        let stored: Vec<Tx> = b
            .txs
            .iter()
            .map(|v| Tx {
                value: v.clone(),
                sender: txs::sender(&txs::decode(v, chain).unwrap()).unwrap(),
            })
            .collect();
        let extras: Vec<ReceiptExtras> = node_receipts
            .iter()
            .map(|r| ReceiptExtras {
                deposit_nonce: (r["type"] == "0x7e" && r.get("depositReceiptVersion").is_none())
                    .then(|| crate::hex_u64(&r["depositNonce"]).unwrap()),
                ..Default::default()
            })
            .collect();
        assert_eq!(extras.iter().any(|x| x.deposit_nonce.is_some()), pre_canyon);
        let header: Value = b.full.clone();
        let v = View::new(
            Context::from_header(chain, &header).unwrap(),
            b.hash(),
            &stored,
            &b.receipts,
            &extras,
        )
        .unwrap();
        for i in 0..v.len() {
            let at = format!("{name} {i}");
            assert_eq!(difference(&v.tx_json(i), &node_txs[i], &at), None);
            assert_eq!(difference(&v.receipt_json(i), &node_receipts[i], &at), None);
        }
        // From the proof alone.
        let n = decode_header(&b.header).unwrap().number;
        let all = verify(&b.block(), chain, &Subject::BlockReceipts(Block::Number(n))).unwrap();
        let r = all.answer().unwrap();
        let t = all.transactions_json().unwrap();
        if pre_canyon {
            assert_eq!(r.unproven, vec!["[].depositNonce"], "{name}");
            assert_eq!(t.unproven, vec!["[].nonce"], "{name}");
        } else {
            assert!(r.unproven.is_empty(), "{name}: {:?}", r.unproven);
            assert!(t.unproven.is_empty(), "{name}: {:?}", t.unproven);
        }
        let c = compare(&r, &b.receipts_json);
        assert!(c.is_exact(), "{name}: {:?}", c.differ);
        let c = compare(&t, &b.full["transactions"]);
        assert!(c.is_exact(), "{name}: {:?}", c.differ);
    }
}
