//! Real OpenVM proofs from the 10/28 GPU run under the minted programs (RTX PRO 6000; see
//! `web/verify/test/logs/{eth-26069632,base-52969856}/README.txt`): an Ethereum leaf, the
//! Ethereum L1 over its 64-block window, and a Base L1 over two 64-block leaves. The L1s carry
//! their children's proofs as deferrals. Each verifies under its minted key and nothing else, a
//! changed byte anywhere is refused, and each `willow_getLogsProof` answer built from the same
//! store verifies end to end with `logset_query::verify`.
#![cfg(feature = "verifier")]

use logset_query::{
    verify, LogFilter, LogsProof, ProofCheck, ProofPart, Trust, OPENVM_PROOF_MAGIC,
};
use serde_json::Value;
use std::path::PathBuf;
use std::time::Instant;
use willow_logs_verify_openvm::pinned::{self, Kind};

fn data(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web/verify/test/logs")
        .join(rel)
}

struct Answer {
    filter: LogFilter,
    logs: usize,
    proof: LogsProof,
    dir: PathBuf,
}

fn answer(rel: &str) -> Answer {
    let v: Value = serde_json::from_slice(&std::fs::read(data(rel)).unwrap()).unwrap();
    let proof: LogsProof = serde_json::from_value(v["result"].clone()).unwrap();
    let filter =
        LogFilter::from_eth_filter(&v["filter"], proof.from_block, proof.to_block).unwrap();
    Answer {
        filter,
        logs: v["logs"].as_u64().unwrap() as usize,
        proof,
        dir: data(rel).parent().unwrap().join("store"),
    }
}

/// The part's proof bytes and public values.
fn part(a: &Answer) -> (Vec<u8>, Vec<u8>) {
    let (id, pv) = match &a.proof.parts[0] {
        ProofPart::Segment(s) => (&s.proof, &s.public_values),
        ProofPart::Block(b) => (&b.proof, &b.public_values),
        ProofPart::Leaf(l) => (&l.proof, &l.public_values),
    };
    (
        std::fs::read(a.dir.join(format!("{id}.proof"))).unwrap(),
        pv.clone(),
    )
}

fn rezip(raw: &[u8]) -> Vec<u8> {
    [&OPENVM_PROOF_MAGIC[..], &zstd::encode_all(raw, 0).unwrap()].concat()
}

const CASES: [(&str, Kind); 3] = [
    ("eth-26069632/block.json", Kind::Leaf),
    ("eth-26069632/segment.json", Kind::L1),
    ("base-52969856/segment.json", Kind::L1),
];

#[test]
fn each_real_proof_verifies_under_its_minted_key_only() {
    let mut c = pinned::check().unwrap();
    for (rel, kind) in CASES {
        let (proof, pv) = part(&answer(rel));
        let t = Instant::now();
        c.check(&pinned::key(kind).unwrap(), &proof, &pv)
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
        eprintln!(
            "{rel}: {} B proof, verified in {:.1} ms",
            proof.len(),
            t.elapsed().as_secs_f64() * 1e3
        );
        for other in Kind::ALL.into_iter().filter(|k| *k != kind) {
            let e = c
                .check(&pinned::key(other).unwrap(), &proof, &pv)
                .unwrap_err();
            eprintln!("  under the {} key: {e}", other.guest());
        }
        // Without its magic it is not an OpenVM proof, and a key that is not minted is refused.
        assert_eq!(
            c.check(&pinned::key(kind).unwrap(), &proof[4..], &pv)
                .unwrap_err(),
            "not an OpenVM proof"
        );
        assert!(c
            .check(&[7; 32], &proof, &pv)
            .unwrap_err()
            .starts_with("no OpenVM program"));
    }
}

#[test]
fn a_changed_byte_anywhere_is_refused() {
    let mut c = pinned::check().unwrap();
    for (rel, kind) in CASES {
        let (proof, pv) = part(&answer(rel));
        let key = pinned::key(kind).unwrap();
        let raw = zstd::decode_all(&proof[4..]).unwrap();
        assert!(
            c.check(&key, &rezip(&raw), &pv).is_ok(),
            "{rel}: re-encoding changes nothing"
        );
        // Bytes through the proof: the start, the trace commitments, the middle, the openings,
        // and the user public values near the end.
        for at in [
            8,
            raw.len() / 8,
            raw.len() / 2,
            raw.len() * 7 / 8,
            raw.len() - 40,
            raw.len() - 1,
        ] {
            let mut bad = raw.clone();
            bad[at] ^= 1;
            let e = c.check(&key, &rezip(&bad), &pv).unwrap_err();
            eprintln!("{rel}: byte {at} of {} changed: {e}", raw.len());
        }
        // Every public value byte is bound: the statement, the index commitment, the leaf key.
        for at in [0, pv.len() / 2, pv.len() - 1] {
            let mut other = pv.clone();
            other[at] ^= 1;
            assert_eq!(
                c.check(&key, &proof, &other).unwrap_err(),
                "proof is for other public values",
                "{rel} pv byte {at}"
            );
        }
        // A cut proof is refused, not a panic.
        assert!(c.check(&key, &proof[..proof.len() / 2], &pv).is_err());
    }
}

#[test]
fn each_real_answer_verifies_end_to_end_to_its_logs() {
    let trust = |chain_id: u64| Trust {
        chain_id,
        sp1: Default::default(),
        openvm: pinned::client_keys().unwrap(),
        openvm_from: None,
    };
    for (rel, _) in CASES {
        let a = answer(rel);
        let mut fetch =
            |id: &str| std::fs::read(a.dir.join(format!("{id}.proof"))).map_err(|e| e.to_string());
        let mut c = pinned::check().unwrap();
        let got = verify(
            &a.proof,
            &a.filter,
            &trust(a.proof.chain_id),
            &mut fetch,
            &mut c,
        )
        .unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(got.logs.len(), a.logs, "{rel}");
        assert!(a.logs > 0, "{rel}");
        assert_eq!(got.openvm_from, Some(a.proof.from_block));
        // The same answer with an SP1-only trust, or a trusted cutover after it, is refused.
        let mut t = trust(a.proof.chain_id);
        t.openvm = Default::default();
        assert!(verify(&a.proof, &a.filter, &t, &mut fetch, &mut c).is_err());
        let mut t = trust(a.proof.chain_id);
        t.openvm_from = Some(a.proof.to_block + 1);
        assert!(verify(&a.proof, &a.filter, &t, &mut fetch, &mut c).is_err());
    }
}
