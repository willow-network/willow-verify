//! BNB's light client on real headers: blocks 133,194,000..=133,196,400 (2026-11-12), saved by
//! parlia-verify's tests, which cross two epoch blocks and two validator-set switches; then
//! tampered every way that matters.

use crate::bnb::{self, attestation_bytes, Start, EPOCH};
use crate::Anchor;
use alloy_primitives::{B256, U256};
use alloy_rlp::Encodable;
use parlia_verify::header::{Header, EXTRA_SEAL};
use parlia_verify::spec::MAINNET as SPEC;
use parlia_verify::vote::Attestation;
use serde_json::{json, Value};
use std::io::Read;

const FIRST: u64 = 133_194_000;

fn headers() -> Vec<(Header, B256)> {
    let gz: &[u8] = include_bytes!("../../parlia/testdata/headers-133194000-133196400.json.gz");
    let mut s = String::new();
    flate2::read::GzDecoder::new(gz)
        .read_to_string(&mut s)
        .unwrap();
    let list: Vec<Value> = serde_json::from_str(&s).unwrap();
    list.iter().map(|v| Header::from_json(v).unwrap()).collect()
}

fn at(hs: &[(Header, B256)], n: u64) -> &(Header, B256) {
    &hs[(n - FIRST) as usize]
}

fn hex(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

fn rlp(h: &Header) -> String {
    hex(&h.rlp())
}

/// A minute after the last header.
fn now(hs: &[(Header, B256)]) -> f64 {
    (hs.last().unwrap().0.time + 60) as f64
}

fn body(result: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string()
}

fn parse(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// The first block at or after `n` finalized by a vote in the headers (source it, target the
/// next), and that vote's bytes.
fn finalized_from(hs: &[(Header, B256)], n: u64) -> (u64, Vec<u8>) {
    let last = hs.last().unwrap().0.number;
    for f in n..n + 8 {
        for h in (f + 2..f + 5).filter(|h| *h <= last) {
            let (hh, _) = at(hs, h);
            let Some(b) = attestation_bytes(hh) else {
                continue;
            };
            let a = Attestation::decode(b).unwrap();
            if a.data.source_number == f
                && a.data.source_hash == at(hs, f).1
                && a.data.target_number == f + 1
            {
                return (f, b.to_vec());
            }
        }
    }
    panic!("no vote finalizes a block from {n}");
}

fn update(hs: &[(Header, B256)], epoch: u64) -> Value {
    let (f, att) = finalized_from(hs, epoch);
    json!({"epoch": epoch / EPOCH,
           "headers": (epoch..=f).map(|n| rlp(&at(hs, n).0)).collect::<Vec<_>>(),
           "attestation": hex(&att)})
}

fn anchor(hs: &[(Header, B256)], n: u64) -> Value {
    let (f, att) = finalized_from(hs, n);
    json!({"blockNumber": format!("0x{n:x}"), "parlia": {
        "headers": (n..=f).map(|m| rlp(&at(hs, m).0)).collect::<Vec<_>>(),
        "attestation": hex(&att)}})
}

fn start_json(hs: &[(Header, B256)]) -> String {
    json!({"number": FIRST, "hash": format!("{}", at(hs, FIRST).1)}).to_string()
}

/// An anchor whose BNB light client starts at block 133,194,000.
fn started(hs: &[(Header, B256)]) -> Anchor {
    let mut a = Anchor::new(&format!("0x{}", "11".repeat(32)), 1.0);
    let r = parse(&a.start("bnb", &start_json(hs)));
    assert_eq!(r["ok"], true, "{r}");
    a
}

fn synced(hs: &[(Header, B256)]) -> Anchor {
    let mut a = started(hs);
    let r = parse(&a.bootstrap_for(
        "bnb",
        &body(&json!({"header": rlp(&at(hs, FIRST).0)})),
        now(hs),
    ));
    assert_eq!(r["ok"], true, "{r}");
    let r = parse(&a.updates_for(
        "bnb",
        &body(&json!([update(hs, 133_195_000), update(hs, 133_196_000)])),
    ));
    assert_eq!(
        (r["ok"].as_bool(), r["period"].as_u64()),
        (Some(true), Some(133_196)),
        "{r}"
    );
    a
}

fn verify(a: &mut Anchor, n: u64, hash: &B256, answer: &Value, now: f64) -> Value {
    parse(&a.verify(56, n as f64, &format!("{hash}"), &body(answer), now))
}

fn kind(v: &Value) -> &str {
    v["kind"].as_str().unwrap_or("")
}

#[test]
fn a_fresh_client_bootstraps_walks_the_epochs_then_finalizes() {
    let hs = headers();
    let t = now(&hs);
    let mut a = started(&hs);
    let n = 133_195_500;
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(
        (kind(&r), r["lightClient"].as_str()),
        ("need-bootstrap", Some("bnb")),
        "{r}"
    );
    let r = parse(&a.bootstrap_for("bnb", &body(&json!({"header": rlp(&at(&hs, FIRST).0)})), t));
    assert_eq!(r["period"], 133_194, "{r}");
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(
        (
            kind(&r),
            r["from"].as_u64(),
            r["count"].as_u64(),
            r["lightClient"].as_str()
        ),
        ("need-updates", Some(133_195), Some(1), Some("bnb")),
        "{r}"
    );
    let r = parse(&a.updates_for("bnb", &body(&json!([update(&hs, 133_195_000)]))));
    assert_eq!(r["period"], 133_195, "{r}");
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(
        (
            r["ok"].as_bool(),
            r["level"].as_str(),
            r["path"].as_str(),
            r["matches"].as_bool()
        ),
        (Some(true), Some("final"), Some("parlia"), Some(true)),
        "{r}"
    );
    assert!(r["signers"].as_u64().unwrap() * 3 >= r["of"].as_u64().unwrap() * 2);
    assert_eq!(r["anchorTime"].as_u64(), Some(at(&hs, n).0.time));
    // Past the next switch: the next epoch's set first.
    let n = 133_196_300;
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(
        (kind(&r), r["from"].as_u64()),
        ("need-updates", Some(133_196)),
        "{r}"
    );
    parse(&a.updates_for("bnb", &body(&json!([update(&hs, 133_196_000)]))));
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(
        (r["level"].as_str(), r["matches"].as_bool()),
        (Some("final"), Some(true)),
        "{r}"
    );
    // An update already applied changes nothing.
    let r = parse(&a.updates_for("bnb", &body(&json!([update(&hs, 133_195_000)]))));
    assert_eq!(
        (r["ok"].as_bool(), r["period"].is_null()),
        (Some(true), true),
        "{r}"
    );
}

#[test]
fn every_block_of_the_run_after_the_start_is_final_with_its_own_hash() {
    let hs = headers();
    let t = now(&hs);
    let mut a = synced(&hs);
    for n in (FIRST + 200..=133_196_380).step_by(97) {
        let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
        assert_eq!(
            (r["level"].as_str(), r["matches"].as_bool()),
            (Some("final"), Some(true)),
            "block {n}: {r}"
        );
        // Another hash is not this block's.
        let r = verify(&mut a, n, &at(&hs, n + 1).1, &anchor(&hs, n), t);
        assert_eq!(r["matches"], false, "block {n}: {r}");
    }
    // Either side of the second switch, the set sealing the vote's target counts.
    for n in [
        133_196_080,
        133_196_085,
        133_196_086,
        133_196_087,
        133_196_088,
        133_196_090,
    ] {
        let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
        assert_eq!(r["level"], "final", "block {n}: {r}");
    }
}

#[test]
fn a_block_before_the_start_or_from_another_start_is_not_reached() {
    let hs = headers();
    let t = now(&hs);
    let mut a = synced(&hs);
    let n = FIRST + 100;
    let r = verify(&mut a, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(kind(&r), "unreachable", "{r}");
    // A bootstrap that is not the start's.
    let mut b = started(&hs);
    let r = parse(&b.bootstrap_for(
        "bnb",
        &body(&json!({"header": rlp(&at(&hs, 133_195_000).0)})),
        t,
    ));
    assert_eq!(kind(&r), "rejected", "{r}");
    // A start more than 7 days old.
    let r = parse(&b.bootstrap_for(
        "bnb",
        &body(&json!({"header": rlp(&at(&hs, FIRST).0)})),
        t + 8.0 * 86_400.0,
    ));
    assert_eq!(kind(&r), "stale-checkpoint", "{r}");
    let n = 133_195_500;
    let r = verify(
        &mut a,
        n,
        &at(&hs, n).1,
        &anchor(&hs, n),
        t + 8.0 * 86_400.0,
    );
    assert_eq!(kind(&r), "stale-checkpoint", "{r}");
    // An update that skips an epoch.
    let mut c = started(&hs);
    parse(&c.bootstrap_for("bnb", &body(&json!({"header": rlp(&at(&hs, FIRST).0)})), t));
    let r = parse(&c.updates_for("bnb", &body(&json!([update(&hs, 133_196_000)]))));
    assert_eq!(kind(&r), "unreachable", "{r}");
}

/// `VoteAttestation`'s RLP from its parts.
fn encode(a: &Attestation, voters: u64, signature: &[u8; 96]) -> String {
    let mut data = Vec::new();
    a.data.source_number.encode(&mut data);
    a.data.source_hash.encode(&mut data);
    a.data.target_number.encode(&mut data);
    a.data.target_hash.encode(&mut data);
    let mut payload = Vec::new();
    voters.encode(&mut payload);
    signature[..].encode(&mut payload);
    alloy_rlp::Header {
        list: true,
        payload_length: data.len(),
    }
    .encode(&mut payload);
    payload.extend_from_slice(&data);
    a.extra[..].encode(&mut payload);
    let mut out = Vec::new();
    alloy_rlp::Header {
        list: true,
        payload_length: payload.len(),
    }
    .encode(&mut out);
    out.extend_from_slice(&payload);
    hex(&out)
}

#[test]
fn tampered_updates_are_refused() {
    let hs = headers();
    let t = now(&hs);
    let fresh = || {
        let mut a = started(&hs);
        parse(&a.bootstrap_for("bnb", &body(&json!({"header": rlp(&at(&hs, FIRST).0)})), t));
        a
    };
    let good = update(&hs, 133_195_000);
    assert_eq!(
        parse(&fresh().updates_for("bnb", &body(&json!([good.clone()]))))["ok"],
        true
    );
    let refused =
        |u: Value| kind(&parse(&fresh().updates_for("bnb", &body(&json!([u]))))).to_string();

    // Another validator key in the epoch block: it no longer hashes to what was voted final.
    let mut e = at(&hs, 133_195_000).0.clone();
    e.extra[33 + 20] ^= 1;
    let mut u = good.clone();
    u["headers"][0] = json!(rlp(&e));
    assert_eq!(refused(u), "rejected");

    let att =
        Attestation::decode(&hex::decode(&good["attestation"].as_str().unwrap()[2..]).unwrap())
            .unwrap();
    // A signature byte flipped.
    let mut sig = att.signature;
    sig[40] ^= 1;
    let mut u = good.clone();
    u["attestation"] = json!(encode(&att, att.voters, &sig));
    assert_eq!(refused(u), "rejected");
    // Fewer than two thirds of the voters claimed.
    let mut fewer = att.voters;
    for _ in 0..8 {
        fewer &= fewer - 1;
    }
    let mut u = good.clone();
    u["attestation"] = json!(encode(&att, fewer, &att.signature));
    assert_eq!(refused(u), "rejected");
    // Another set of voters than signed.
    let mut u = good.clone();
    u["attestation"] = json!(encode(&att, att.voters ^ 1, &att.signature));
    assert_eq!(refused(u), "rejected");
    // A voter past the set.
    let mut u = good.clone();
    u["attestation"] = json!(encode(&att, att.voters | (1 << 40), &att.signature));
    assert_eq!(refused(u), "rejected");
    // Another block's vote.
    let (_, other) = finalized_from(&hs, 133_195_010);
    let mut u = good.clone();
    u["attestation"] = json!(hex(&other));
    assert_eq!(refused(u), "rejected");
    // A link left out between the epoch block and the vote's source.
    let mut u = update(&hs, 133_195_000);
    u["headers"] = json!([rlp(&at(&hs, 133_195_000).0), rlp(&at(&hs, 133_195_002).0)]);
    assert_eq!(refused(u), "rejected");
    // Not RLP.
    let mut u = good.clone();
    u["headers"][0] = json!("0x1234");
    assert_eq!(refused(u), "malformed-input");
}

#[test]
fn tampered_anchors_are_refused() {
    let hs = headers();
    let t = now(&hs);
    let mut a = synced(&hs);
    let n = 133_195_700;
    let good = anchor(&hs, n);
    let mut refused =
        |v: Value, number: u64| kind(&verify(&mut a, number, &at(&hs, n).1, &v, t)).to_string();
    // The anchor for another block, asked for this one.
    assert_eq!(refused(anchor(&hs, n + 1), n), "rejected");
    // A header byte changed: its hash is not the voted one.
    let mut h = at(&hs, n).0.clone();
    h.gas_used += 1;
    let mut v = good.clone();
    v["parlia"]["headers"][0] = json!(rlp(&h));
    assert_eq!(refused(v, n), "rejected");
    // A vote for another block.
    let (_, other) = finalized_from(&hs, n + 5);
    let mut v = good.clone();
    v["parlia"]["attestation"] = json!(hex(&other));
    assert_eq!(refused(v, n), "rejected");
    // A link left out.
    let mut v = anchor(&hs, n);
    let list = v["parlia"]["headers"].as_array().unwrap().clone();
    if list.len() == 1 {
        v["parlia"]["headers"] = json!([list[0], rlp(&at(&hs, n + 2).0)]);
        assert_eq!(refused(v, n), "rejected");
    }
}

#[test]
fn a_block_one_validator_sealed_is_signed_until_ten_minutes() {
    let hs = headers();
    let n = 133_196_400;
    let (h, hash) = at(&hs, n).clone();
    let t = (h.time + 5) as f64;
    let mut a = synced(&hs);
    let sealed = json!({"blockNumber": format!("0x{n:x}"), "parlia": {"headers": [rlp(&h)], "attestation": null}});
    let r = verify(&mut a, n, &hash, &sealed, t);
    assert_eq!(
        (
            r["ok"].as_bool(),
            r["level"].as_str(),
            r["matches"].as_bool(),
            r["signers"].as_u64()
        ),
        (Some(true), Some("signed"), Some(true), Some(1)),
        "{r}"
    );
    let r = verify(&mut a, n, &hash, &sealed, (h.time + 601) as f64);
    assert_eq!(kind(&r), "stale-head", "{r}");
    // A changed field: the seal recovers someone outside the set.
    let mut x = h.clone();
    x.gas_used += 1;
    let v = json!({"blockNumber": format!("0x{n:x}"), "parlia": {"headers": [rlp(&x)], "attestation": null}});
    assert_eq!(kind(&verify(&mut a, n, &hash, &v, t)), "rejected");
    // The same seal malleated (s → n − s, parity flipped): the same signer, another hash.
    let mut m = h.clone();
    let i = m.extra.len() - EXTRA_SEAL;
    let order = U256::from_str_radix(
        "fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141",
        16,
    )
    .unwrap();
    let s = U256::from_be_slice(&m.extra[i + 32..i + 64]);
    m.extra[i + 32..i + 64].copy_from_slice(&(order - s).to_be_bytes::<32>());
    m.extra[i + 64] ^= 1;
    if let Ok(signer) = m.signer(56) {
        assert_eq!(signer, h.signer(56).unwrap());
    }
    let v = json!({"blockNumber": format!("0x{n:x}"), "parlia": {"headers": [rlp(&m)], "attestation": null}});
    assert_eq!(kind(&verify(&mut a, n, &m.hash(), &v, t)), "rejected");
    // Two headers and no vote.
    let v = json!({"blockNumber": format!("0x{:x}", n - 1), "parlia": {"headers": [rlp(&at(&hs, n - 1).0), rlp(&h)], "attestation": null}});
    assert_eq!(
        kind(&verify(&mut a, n - 1, &at(&hs, n - 1).1, &v, t)),
        "malformed-input"
    );
}

#[test]
fn the_kept_sets_are_verified_again_on_restore() {
    let hs = headers();
    let t = now(&hs);
    let a = synced(&hs);
    let saved = a.saved_for("bnb");
    let mut b = started(&hs);
    let r = parse(&b.restore_for("bnb", &saved, t));
    assert_eq!(r["period"], 133_196, "{r}");
    let n = 133_196_300;
    let r = verify(&mut b, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(r["level"], "final", "{r}");
    // Kept with a tampered update: refused, and the client starts over.
    let mut v = parse(&saved);
    v["updates"][1]["headers"][0] = json!(rlp(&at(&hs, 133_196_001).0));
    let mut c = started(&hs);
    assert_ne!(parse(&c.restore_for("bnb", &v.to_string(), t))["ok"], true);
    let r = verify(&mut c, n, &at(&hs, n).1, &anchor(&hs, n), t);
    assert_eq!(kind(&r), "need-bootstrap", "{r}");
    // Kept for another start: ignored.
    let mut d = Anchor::new(&format!("0x{}", "11".repeat(32)), 1.0);
    let r = parse(&d.restore_for("bnb", &saved, t));
    assert_eq!(
        (r["ok"].as_bool(), r["period"].is_null()),
        (Some(true), true),
        "{r}"
    );
    // The pin built in is BNB's block 133,195,000.
    assert_eq!(bnb::PINNED.number, 133_195_000);
    assert_eq!(B256::from(bnb::PINNED.hash), at(&hs, 133_195_000).1);
}

#[test]
fn the_sets_read_from_the_epoch_blocks_are_parlia_verifys() {
    let hs = headers();
    for e in [FIRST, 133_195_000, 133_196_000] {
        let set = at(&hs, e).0.epoch_set(&SPEC, EPOCH).unwrap();
        assert!(
            set.validators.len() >= 3 && set.turn_length.is_some(),
            "{e}"
        );
    }
    assert!(
        bnb::start_of(&json!({"number": 133_195_001u64, "hash": format!("{}", B256::ZERO)}))
            .is_none()
    );
    let s: Start = bnb::start_of(&parse(&start_json(&hs))).unwrap();
    assert_eq!(s.number, FIRST);
}

/// BNB data eth-anchor's `record_bnb_anchors` saved from public RPCs: the start, its bootstrap,
/// every epoch's update to then, and anchors, verified as the client verifies them.
#[test]
fn anchors_saved_from_public_rpcs_verify_and_tampered_ones_do_not() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../web/verify/test/anchor/chains/bnb.json");
    let saved: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let anchors = saved["anchors"].as_array().unwrap();
    assert!(anchors.len() >= 2, "{} anchors saved", anchors.len());
    let mut levels = Vec::new();
    for x in anchors {
        let now = x["savedAt"].as_f64().unwrap();
        let mut a = Anchor::new(&format!("0x{}", "11".repeat(32)), 1.0);
        assert_eq!(
            parse(&a.start("bnb", &saved["start"].to_string()))["ok"],
            true
        );
        let r = parse(&a.bootstrap_for("bnb", &body(&saved["bootstrap"]), now));
        assert_eq!(r["ok"], true, "{r}");
        for chunk in saved["updates"]
            .as_array()
            .unwrap()
            .chunks(bnb::MAX_UPDATES)
        {
            let r = parse(&a.updates_for("bnb", &body(&json!(chunk))));
            assert_eq!(r["ok"], true, "{r}");
        }
        let n = x["block"].as_u64().unwrap();
        let hash: B256 = x["hash"].as_str().unwrap().parse().unwrap();
        let r = verify(&mut a, n, &hash, &x["result"], now);
        assert_eq!(
            (r["ok"].as_bool(), r["matches"].as_bool(), &r["level"]),
            (Some(true), Some(true), &x["expect"]["level"]),
            "block {n}: {r}"
        );
        levels.push(r["level"].as_str().unwrap().to_string());
        let mut t = x["result"].clone();
        let h = t["parlia"]["headers"][0].as_str().unwrap().to_string();
        let i = h.len() - 200;
        t["parlia"]["headers"][0] = json!(format!(
            "{}{}{}",
            &h[..i],
            if &h[i..i + 1] == "0" { "1" } else { "0" },
            &h[i + 1..]
        ));
        assert_ne!(verify(&mut a, n, &hash, &t, now)["ok"], true, "block {n}");
    }
    assert!(levels.iter().any(|l| l == "final"), "{levels:?}");
}
