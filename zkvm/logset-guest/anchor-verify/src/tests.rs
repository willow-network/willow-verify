//! Real mainnet data saved by `web/verify/test/save_anchor_data.py` (the checkpoint, its bootstrap,
//! two periods of updates, a finality update and EIP-2935 proofs against its head), verified as
//! the page does, then tampered every way that matters.

use super::*;
use serde_json::Value;

const CHECKPOINT: &str = include_str!("../../../../web/verify/test/anchor/checkpoint.json");
const BOOTSTRAP: &str = include_str!("../../../../web/verify/test/anchor/bootstrap.json");
const UPDATES: &str = include_str!("../../../../web/verify/test/anchor/updates.json");
const FINAL: &str = include_str!("../../../../web/verify/test/anchor/history-26226600.json");
const FINAL_NEXT: &str = include_str!("../../../../web/verify/test/anchor/history-26226601.json");
const SIGNED: &str = include_str!("../../../../web/verify/test/anchor/history-26227852.json");
const HEAD: &str = include_str!("../../../../web/verify/test/anchor/history-26227872.json");
/// Block 26,226,600's hash, as the log proofs in `eth-26226600` end at it.
const HASH_26226600: &str = "0xd9d667f111fb89c038ad7dc4374d339a1b42a1ad5325e99a563fb24b7ad3c423";

fn checkpoint() -> (String, f64) {
    let c: Value = serde_json::from_str(CHECKPOINT).unwrap();
    (
        c["root"].as_str().unwrap().into(),
        c["slot"].as_f64().unwrap(),
    )
}

/// A JSON-RPC response body around `result`.
fn body(result: &Value) -> String {
    json!({"jsonrpc": "2.0", "id": 1, "result": result}).to_string()
}

fn json_of(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

/// Seconds since 1970 when the data was saved: within the checkpoint's 30 days.
fn now() -> f64 {
    let a = json_of(FINAL);
    let slot: u64 = a["update"]["data"]["signature_slot"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    lc::slot_time(slot) as f64 + 60.0
}

fn synced() -> Anchor {
    let (root, slot) = checkpoint();
    let mut a = Anchor::new(&root, slot);
    let r = json_of(&a.bootstrap(&body(&json_of(BOOTSTRAP)), now()));
    assert_eq!(r["ok"], true, "{r}");
    let r = json_of(&a.updates(&body(&json_of(UPDATES))));
    assert_eq!(r["ok"], true, "{r}");
    a
}

fn verify(a: &mut Anchor, number: u64, hash: &str, answer: &Value) -> Value {
    json_of(&a.verify(1, number as f64, hash, &body(answer), now()))
}

fn kind(v: &Value) -> &str {
    v["kind"].as_str().unwrap_or("")
}

#[test]
fn the_pinned_history_code_is_eip_2935s() {
    let p = json_of(FINAL);
    assert_eq!(
        format!("{}", history::ethereum_code_hash()),
        p["proof"]["proof"]["codeHash"].as_str().unwrap()
    );
}

#[test]
fn a_fresh_client_asks_for_the_bootstrap_then_the_updates_then_confirms() {
    let (root, slot) = checkpoint();
    let mut a = Anchor::new(&root, slot);
    let answer = json_of(FINAL);
    assert_eq!(
        kind(&verify(&mut a, 26226600, HASH_26226600, &answer)),
        "need-bootstrap"
    );
    assert_eq!(
        json_of(&a.bootstrap(&body(&json_of(BOOTSTRAP)), now()))["period"],
        1886
    );
    let need = verify(&mut a, 26226600, HASH_26226600, &answer);
    assert_eq!(kind(&need), "need-updates");
    assert_eq!(
        (need["from"].as_u64(), need["count"].as_u64()),
        (Some(1886), Some(2))
    );
    assert_eq!(
        json_of(&a.updates(&body(&json_of(UPDATES))))["period"],
        1888
    );
    let r = verify(&mut a, 26226600, HASH_26226600, &answer);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(
        (
            r["level"].as_str(),
            r["path"].as_str(),
            r["matches"].as_bool()
        ),
        (Some("final"), Some("history"), Some(true))
    );
    assert!(r["signers"].as_u64().unwrap() >= 342);
}

#[test]
fn levels_follow_the_finalized_block() {
    let mut a = synced();
    let next = json_of(FINAL_NEXT);
    let r = verify(
        &mut a,
        26226601,
        "0x83222a42c7ee2cbb512db881bc268b57004437a988ff70d2f1fa9db59cf42cbd",
        &next,
    );
    assert_eq!(
        (r["level"].as_str(), r["matches"].as_bool()),
        (Some("final"), Some(true)),
        "{r}"
    );
    let signed = json_of(SIGNED);
    let n = 26227852;
    let r = verify(&mut a, n, &format!("0x{}", "00".repeat(32)), &signed);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(r["level"], "signed");
    assert!(r["finalized"].as_u64().unwrap() < n && n < r["head"].as_u64().unwrap());
    let head = json_of(HEAD);
    let h = head["update"]["data"]["attested_header"]["execution"]["block_hash"]
        .as_str()
        .unwrap()
        .to_owned();
    let r = verify(&mut a, 26227872, &h, &head);
    assert_eq!(
        (
            r["path"].as_str(),
            r["level"].as_str(),
            r["matches"].as_bool()
        ),
        (Some("head"), Some("signed"), Some(true)),
        "{r}"
    );
}

#[test]
fn another_hash_is_a_proven_mismatch_not_a_failure() {
    let mut a = synced();
    let other = format!("0x{}", "11".repeat(32));
    let r = verify(&mut a, 26226600, &other, &json_of(FINAL));
    assert_eq!(
        (r["ok"].as_bool(), r["matches"].as_bool()),
        (Some(true), Some(false))
    );
    assert_eq!(r["hash"], HASH_26226600);
}

/// `answer` with the string at `path` (keys, or indices into lists) replaced by `f` of it.
fn edit(answer: &Value, path: &[&str], f: impl Fn(&str) -> String) -> Value {
    let mut v = answer.clone();
    let mut cur = &mut v;
    for k in path {
        cur = match k.parse::<usize>() {
            Ok(i) => &mut cur[i],
            Err(_) => &mut cur[*k],
        };
    }
    let s = cur.as_str().unwrap().to_owned();
    *cur = Value::String(f(&s));
    v
}

/// Flips the last hex digit.
fn flip(s: &str) -> String {
    let mut c: Vec<char> = s.chars().collect();
    let last = c.len() - 1;
    c[last] = if c[last] == '0' { '1' } else { '0' };
    c.into_iter().collect()
}

#[test]
fn every_tampered_part_of_a_history_anchor_is_rejected() {
    let a0 = json_of(FINAL);
    let cases: Vec<(&str, Value)> = vec![
        (
            "signature",
            edit(
                &a0,
                &[
                    "update",
                    "data",
                    "sync_aggregate",
                    "sync_committee_signature",
                ],
                flip,
            ),
        ),
        (
            "attested state root",
            edit(
                &a0,
                &["update", "data", "attested_header", "beacon", "state_root"],
                flip,
            ),
        ),
        (
            "attested execution",
            edit(
                &a0,
                &[
                    "update",
                    "data",
                    "attested_header",
                    "execution",
                    "state_root",
                ],
                flip,
            ),
        ),
        (
            "execution branch",
            edit(
                &a0,
                &["update", "data", "attested_header", "execution_branch", "0"],
                flip,
            ),
        ),
        (
            "finalized header",
            edit(
                &a0,
                &["update", "data", "finalized_header", "beacon", "body_root"],
                flip,
            ),
        ),
        (
            "finalized execution",
            edit(
                &a0,
                &[
                    "update",
                    "data",
                    "finalized_header",
                    "execution",
                    "block_number",
                ],
                |_| "26227871".into(),
            ),
        ),
        (
            "finality branch",
            edit(&a0, &["update", "data", "finality_branch", "3"], flip),
        ),
        (
            "account proof",
            edit(&a0, &["proof", "proof", "accountProof", "2"], flip),
        ),
        (
            "storage proof",
            edit(
                &a0,
                &["proof", "proof", "storageProof", "0", "proof", "1"],
                flip,
            ),
        ),
        (
            "storage value",
            edit(&a0, &["proof", "proof", "storageProof", "0", "value"], flip),
        ),
        (
            "storage key",
            edit(&a0, &["proof", "proof", "storageProof", "0", "key"], |_| {
                "0x1c2a".into()
            }),
        ),
        (
            "storage root",
            edit(&a0, &["proof", "proof", "storageHash"], flip),
        ),
        (
            "code hash",
            edit(&a0, &["proof", "proof", "codeHash"], flip),
        ),
        (
            "address",
            edit(&a0, &["proof", "proof", "address"], |_| {
                "0x000f3df6d732807ef1319fb7b8bb8522d0beac02".into()
            }),
        ),
        (
            "block asked",
            edit(&a0, &["blockNumber"], |_| "0x1902fa9".into()),
        ),
        ("kind", edit(&a0, &["proof", "kind"], |_| "head".into())),
    ];
    for (name, answer) in cases {
        let mut a = synced();
        let r = verify(&mut a, 26226600, HASH_26226600, &answer);
        assert_eq!(r["ok"], false, "{name}: {r}");
        assert!(
            matches!(kind(&r), "rejected" | "malformed-input"),
            "{name}: {r}"
        );
    }
}

#[test]
fn too_few_signers_is_rejected() {
    let a0 = json_of(FINAL);
    // 341 of 512 bits set: one short of two thirds.
    let mut bits = [0u8; 64];
    for i in 0..341 {
        bits[i / 8] |= 1 << (i % 8);
    }
    let answer = edit(
        &a0,
        &["update", "data", "sync_aggregate", "sync_committee_bits"],
        |_| format!("0x{}", hex::encode(bits)),
    );
    let r = verify(&mut synced(), 26226600, HASH_26226600, &answer);
    assert_eq!(kind(&r), "rejected");
    assert!(r["message"].as_str().unwrap().contains("341 of 512"), "{r}");
}

#[test]
fn a_block_outside_the_history_ring_is_rejected() {
    let a0 = json_of(FINAL);
    let answer = edit(&a0, &["blockNumber"], |_| {
        format!("0x{:x}", 26227872 - 9000)
    });
    let r = verify(&mut synced(), 26227872 - 9000, HASH_26226600, &answer);
    assert_eq!(kind(&r), "rejected", "{r}");
}

#[test]
fn a_forged_bootstrap_or_update_is_rejected() {
    let (root, slot) = checkpoint();
    // Another checkpoint: the bootstrap is not for it.
    let mut a = Anchor::new(&flip(&root), slot);
    assert_eq!(
        kind(&json_of(&a.bootstrap(&body(&json_of(BOOTSTRAP)), now()))),
        "rejected"
    );
    // A committee key changed: no longer under the state root.
    let mut a = Anchor::new(&root, slot);
    let b = edit(
        &json_of(BOOTSTRAP),
        &["data", "current_sync_committee", "pubkeys", "7"],
        flip,
    );
    assert_eq!(kind(&json_of(&a.bootstrap(&body(&b), now()))), "rejected");

    let updates = json_of(UPDATES);
    for (name, u) in [
        (
            "signature",
            edit(
                &updates,
                &["0", "data", "sync_aggregate", "sync_committee_signature"],
                flip,
            ),
        ),
        (
            "next committee",
            edit(
                &updates,
                &["0", "data", "next_sync_committee", "pubkeys", "3"],
                flip,
            ),
        ),
        (
            "next committee branch",
            edit(
                &updates,
                &["0", "data", "next_sync_committee_branch", "2"],
                flip,
            ),
        ),
        (
            "attested header",
            edit(
                &updates,
                &["0", "data", "attested_header", "beacon", "proposer_index"],
                |_| "1".into(),
            ),
        ),
    ] {
        let mut a = Anchor::new(&root, slot);
        a.bootstrap(&body(&json_of(BOOTSTRAP)), now());
        let r = json_of(&a.updates(&body(&u)));
        assert_eq!(kind(&r), "rejected", "{name}: {r}");
    }
    // The second period's update before the first: not reached yet, not a forgery.
    let mut a = Anchor::new(&root, slot);
    a.bootstrap(&body(&json_of(BOOTSTRAP)), now());
    let r = json_of(&a.updates(&body(&Value::Array(vec![updates[1].clone()]))));
    assert_eq!(kind(&r), "unreachable", "{r}");
}

#[test]
fn an_unknown_fork_is_refused_by_name() {
    let a0 = json_of(FINAL);
    let answer = edit(&a0, &["update", "version"], |_| "gloas".into());
    assert_eq!(
        kind(&verify(&mut synced(), 26226600, HASH_26226600, &answer)),
        "unknown-fork"
    );
    // Calling a Fulu update Electra does not change what it is.
    let answer = edit(&a0, &["update", "version"], |_| "electra".into());
    assert_eq!(
        kind(&verify(&mut synced(), 26226600, HASH_26226600, &answer)),
        "rejected"
    );
}

#[test]
fn a_checkpoint_older_than_30_days_is_refused() {
    let (root, slot) = checkpoint();
    let mut a = Anchor::new(&root, slot);
    let late = lc::slot_time(slot as u64) as f64 + 31.0 * 86_400.0;
    assert_eq!(
        kind(&json_of(&a.bootstrap(&body(&json_of(BOOTSTRAP)), late))),
        "stale-checkpoint"
    );
}

#[test]
fn a_kept_light_client_is_verified_again_on_the_next_visit() {
    let a = synced();
    let saved = a.saved();
    let (root, slot) = checkpoint();
    let mut b = Anchor::new(&root, slot);
    assert_eq!(json_of(&b.restore(&saved, now()))["period"], 1888);
    let r = verify(&mut b, 26226600, HASH_26226600, &json_of(FINAL));
    assert_eq!(
        (r["ok"].as_bool(), r["matches"].as_bool()),
        (Some(true), Some(true)),
        "{r}"
    );
    // Kept data is never trusted: a committee changed in it fails like a forged update, and the
    // client starts over from the checkpoint.
    let mut kept = json_of(&saved);
    kept["updates"][0]["data"]["next_sync_committee"]["pubkeys"][3] = Value::String(flip(
        kept["updates"][0]["data"]["next_sync_committee"]["pubkeys"][3]
            .as_str()
            .unwrap(),
    ));
    let mut c = Anchor::new(&root, slot);
    assert_eq!(
        kind(&json_of(&c.restore(&kept.to_string(), now()))),
        "rejected"
    );
    assert_eq!(
        kind(&verify(&mut c, 26226600, HASH_26226600, &json_of(FINAL))),
        "need-bootstrap"
    );
    // Kept for another checkpoint: ignored.
    let mut other = Anchor::new(&flip(&root), slot);
    assert_eq!(
        json_of(&other.restore(&saved, now()))["period"],
        Value::Null
    );
}

#[test]
fn signatures_from_unreached_future_or_earlier_periods() {
    let a0 = json_of(FINAL);
    let mut a = synced();
    let sig = |d: u64| {
        edit(&a0, &["update", "data", "signature_slot"], move |s| {
            (s.parse::<u64>().unwrap() + d).to_string()
        })
    };
    // The next period: fetch its update first.
    assert_eq!(
        kind(&verify(&mut a, 26226600, HASH_26226600, &sig(8192))),
        "need-updates"
    );
    // Periods that have not begun: refused, never fetched.
    assert_eq!(
        kind(&verify(&mut a, 26226600, HASH_26226600, &sig(3 * 8192))),
        "rejected"
    );
    let far = edit(&a0, &["update", "data", "signature_slot"], |_| {
        u64::MAX.to_string()
    });
    assert_eq!(
        kind(&verify(&mut a, 26226600, HASH_26226600, &far)),
        "rejected"
    );
    // Before the checkpoint: not reachable from this page, not a forgery.
    let older = edit(&a0, &["update", "data", "signature_slot"], |_| {
        (1885 * 8192 + 100).to_string()
    });
    assert_eq!(
        kind(&verify(&mut a, 26226600, HASH_26226600, &older)),
        "unreachable"
    );
}

#[test]
fn a_checkpoint_with_another_slot_is_refused() {
    let (root, slot) = checkpoint();
    let mut a = Anchor::new(&root, slot + 1.0);
    assert_eq!(
        kind(&json_of(&a.bootstrap(&body(&json_of(BOOTSTRAP)), now()))),
        "rejected"
    );
}

#[test]
fn a_stale_signed_head_says_nothing_about_blocks_not_final() {
    let mut a = synced();
    let signed = json_of(SIGNED);
    let late = now() + 700.0;
    let r = json_of(&a.verify(
        1,
        26227852.0,
        &format!("0x{}", "00".repeat(32)),
        &body(&signed),
        late,
    ));
    assert_eq!(kind(&r), "stale-head", "{r}");
    // Finality does not age: the same update still proves the final block.
    let r = json_of(&a.verify(1, 26226600.0, HASH_26226600, &body(&json_of(FINAL)), late));
    assert_eq!(
        (r["ok"].as_bool(), r["level"].as_str()),
        (Some(true), Some("final")),
        "{r}"
    );
}

#[test]
fn malformed_answers_are_malformed_input() {
    let mut a = synced();
    for body in [
        "",
        "{}",
        "[1]",
        "{\"result\": 7}",
        &format!("{{\"result\": {}}}", "[".repeat(300) + &"]".repeat(300)),
    ] {
        let r = json_of(&a.verify(1, 26226600.0, HASH_26226600, body, now()));
        assert_eq!(r["ok"], false, "{body}");
        assert!(matches!(kind(&r), "malformed-input" | "rejected"), "{r}");
    }
}

const BEACON: &str = include_str!("../../../../web/verify/test/anchor/beacon-26124863.json");
/// Block 26,124,863 (two weeks before the anchor), as the receipt proofs in `test/items` end at it.
const HASH_26124863: &str = "0x28f942fc7925d908e5aee5d59a38ed9138c664a8ea8cf0a98fb270855bf6b1d8";

#[test]
fn an_older_block_is_final_through_the_beacon_state() {
    let b = json_of(BEACON);
    let r = verify(&mut synced(), 26124863, HASH_26124863, &b);
    assert_eq!(r["ok"], true, "{r}");
    assert_eq!(
        (
            r["level"].as_str(),
            r["path"].as_str(),
            r["matches"].as_bool()
        ),
        (Some("final"), Some("beacon"), Some(true))
    );
    // A witness node changed, another block asked, another anchor header, another update.
    let mut flipped = b.clone();
    let w = flipped["proof"]["witness"].as_str().unwrap().to_owned();
    let mut c: Vec<char> = w.chars().collect();
    c[600] = if c[600] == '0' { '1' } else { '0' };
    flipped["proof"]["witness"] = Value::String(c.into_iter().collect());
    let other = edit(&b, &["blockNumber"], |_| "0x18ea240".into());
    let header = edit(
        &b,
        &[
            "update",
            "data",
            "finalized_header",
            "beacon",
            "proposer_index",
        ],
        |_| "1".into(),
    );
    let mut swapped = b.clone();
    swapped["update"] = json_of(FINAL)["update"].clone();
    for (name, answer, n) in [
        ("witness", flipped, 26124863),
        ("block", other, 26124864),
        ("header", header, 26124863),
        ("update", swapped, 26124863),
    ] {
        let r = verify(&mut synced(), n, HASH_26124863, &answer);
        assert_eq!(kind(&r), "rejected", "{name}: {r}");
    }
}

const CALLS: &str = include_str!("../../../../web/verify/test/anchor/calls-op-26227790.json");

/// The finalized block of the saved finality update, verified: its state is what calls run on.
fn finalized_block() -> lc::Execution {
    let (root, slot) = checkpoint();
    let mut lc = lc::LightClient::new(
        hex::decode(&root[2..]).unwrap().try_into().unwrap(),
        slot as u64,
    );
    lc.bootstrap(&json_of(BOOTSTRAP)).unwrap();
    for u in json_of(UPDATES).as_array().unwrap() {
        lc.apply_update(u).unwrap();
    }
    let f = lc
        .finality(&json_of(FINAL)["update"], now() as u64)
        .unwrap();
    assert_eq!(f.finalized.execution.number, 26227790);
    f.finalized.execution
}

fn run_calls(accounts: &Value) -> Result<Vec<alloy_primitives::Bytes>, Failure> {
    let block = finalized_block();
    let state = call::prove_state(block.state_root.into(), accounts)?;
    let w = json_of(CALLS);
    w["calls"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            let to = c["to"].as_str().unwrap().parse().unwrap();
            let data = hex::decode(&c["data"].as_str().unwrap()[2..]).unwrap();
            call::call(&state, &block, to, data.into())
        })
        .collect()
}

#[test]
fn contract_calls_run_over_proven_state() {
    let w = json_of(CALLS);
    let out = run_calls(&w["accounts"]).unwrap();
    // OP Mainnet's registry says the old super-root game is valid; its claim is the hash of its
    // super root; SystemConfig names the sequencer's key.
    assert_eq!(call::word(&out[0], 0).unwrap()[31], 1, "isGameClaimValid");
    let extra = &out[2];
    let len = u64::from_be_bytes(call::word(extra, 1).unwrap()[24..].try_into().unwrap()) as usize;
    let preimage = &extra[64..64 + len];
    assert_eq!(preimage[0], 1, "a super root, version 1");
    assert_eq!(
        call::word(&out[1], 0).unwrap(),
        alloy_primitives::keccak256(preimage).0
    );
    let signer = call::word(&out[3], 0).unwrap();
    assert!(signer[..12].iter().all(|&b| b == 0) && signer[12..].iter().any(|&b| b != 0));
}

#[test]
fn a_call_over_altered_or_missing_state_fails() {
    let w = json_of(CALLS);
    // A storage value changed: its proof no longer verifies.
    let mut changed = w["accounts"].clone();
    let slot = changed
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .flat_map(|a| a["storageProof"].as_array_mut().unwrap().iter_mut())
        .find(|s| s["value"] != "0x0")
        .unwrap();
    slot["value"] = Value::String("0x1234".into());
    assert_eq!(run_calls(&changed).unwrap_err().kind, Kind::Rejected);
    // An account left out: the call needs it and fails, never reads it as empty.
    let mut fewer = w["accounts"].clone();
    let list = fewer.as_array_mut().unwrap();
    let i = list
        .iter()
        .position(|a| a["storageProof"].as_array().unwrap().len() > 0)
        .unwrap();
    list.remove(i);
    let e = run_calls(&fewer).unwrap_err();
    assert_eq!(e.kind, Kind::Rejected);
    // A slot left out.
    let mut no_slot = w["accounts"].clone();
    for a in no_slot.as_array_mut().unwrap() {
        a["storageProof"].as_array_mut().unwrap().clear();
    }
    assert_eq!(run_calls(&no_slot).unwrap_err().kind, Kind::Rejected);
    // Code swapped between accounts: not its code hash.
    let mut code = w["accounts"].clone();
    let list = code.as_array_mut().unwrap();
    let with_code: Vec<usize> = (0..list.len())
        .filter(|&i| list[i]["code"].as_str().unwrap().len() > 2)
        .collect();
    let other = list[with_code[1]]["code"].clone();
    list[with_code[0]]["code"] = other;
    assert_eq!(run_calls(&code).unwrap_err().kind, Kind::Rejected);
}

const BASE_FINAL: &str = include_str!("../../../../web/verify/test/anchor/l2-8453-52491359.json");
const BASE_COMMITTED: &str =
    include_str!("../../../../web/verify/test/anchor/l2-8453-52820000.json");
const UNI_COMMITTED: &str = include_str!("../../../../web/verify/test/anchor/l2-130-61680000.json");
const HASH_BASE_52491359: &str =
    "0x660647478057e516eb6d892d788c8cb29d8d1267fc3159805f5ca589feb38057";
const HASH_BASE_52820000: &str =
    "0xa110242001cbe9bca8d8d4f255adfaace5340d30d2a0efd07d6225e20e14e5d7";
const HASH_UNI_61680000: &str =
    "0xb7cffcd5634973c0c73fb8208545b0fe07208d76cc14a717e4f7b8a33bba96aa";

/// A minute after the L1 block an L2 anchor reads.
fn l2_now(answer: &Value) -> f64 {
    let t: u64 = answer["update"]["data"]["finalized_header"]["execution"]["timestamp"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    t as f64 + 60.0
}

fn verify_l2(chain: u32, number: u64, hash: &str, answer: &Value) -> Value {
    json_of(&synced().verify(chain, number as f64, hash, &body(answer), l2_now(answer)))
}

#[test]
fn op_stack_blocks_are_final_or_committed_through_their_games() {
    let r = verify_l2(8453, 52491359, HASH_BASE_52491359, &json_of(BASE_FINAL));
    assert_eq!(
        (
            r["ok"].as_bool(),
            r["level"].as_str(),
            r["matches"].as_bool()
        ),
        (Some(true), Some("final"), Some(true)),
        "{r}"
    );
    let r = verify_l2(8453, 52820000, HASH_BASE_52820000, &json_of(BASE_COMMITTED));
    assert_eq!(
        (
            r["ok"].as_bool(),
            r["level"].as_str(),
            r["matches"].as_bool()
        ),
        (Some(true), Some("committed"), Some(true)),
        "{r}"
    );
    // A super-root chain: the output root comes out of the game's super root. The game is by
    // Unichain's proposer before the 2026-10-29 rotation: committed under that pin, declined now.
    let uni = json_of(UNI_COMMITTED);
    let c = op_on(&UNICHAIN_BEFORE, 61680000, &uni, l2_now(&uni)).unwrap();
    assert_eq!(
        (c.level, format!("{}", alloy_primitives::B256::from(c.hash))),
        ("committed", HASH_UNI_61680000.to_string())
    );
    assert!(c.anchor_block >= 61680000);
    let r = verify_l2(130, 61680000, HASH_UNI_61680000, &uni);
    assert_eq!(kind(&r), "declined", "{r}");
}

/// Unichain as `OP_CHAINS` pinned it before its proposer changed on 2026-10-29.
const UNICHAIN_BEFORE: crate::l2::OpChain = crate::l2::OpChain {
    chain_id: 130,
    name: "Unichain",
    registry: alloy_primitives::address!("27Cf508E4E3Aa8d30b3226aC3b5Ea0e8bcaCAFF9"),
    proposers: &[alloy_primitives::address!(
        "D5F0E2912C70771C589CD8bB087EDE0Dab4AFA9A"
    )],
    super_root: true,
};

/// `confirm_op` with `chain`'s pins, on the light client the `l2-*` fixtures were saved against.
fn op_on(
    chain: &crate::l2::OpChain,
    number: u64,
    answer: &Value,
    now: f64,
) -> Result<crate::l2::L2Confirmed, Failure> {
    let mut a = synced();
    crate::l2::confirm_op_on(a.client().unwrap(), chain, number, answer, now as u64)
}

#[test]
fn every_tampered_part_of_an_l2_anchor_is_refused() {
    let a0 = json_of(BASE_FINAL);
    let mut swapped = a0.clone();
    swapped["game"] = json_of(BASE_COMMITTED)["game"].clone();
    let mut no_account = a0.clone();
    no_account["l1"].as_array_mut().unwrap().remove(3);
    let cases: Vec<(&str, Value)> = vec![
        ("another game", swapped),
        (
            "header",
            edit(&a0, &["headers", "0"], |s| {
                let mut c: Vec<char> = s.chars().collect();
                c[100] = if c[100] == '0' { '1' } else { '0' };
                c.into_iter().collect()
            }),
        ),
        (
            "history proof",
            edit(&a0, &["proofs", "0", "storageProof", "0", "value"], flip),
        ),
        (
            "Ethereum state",
            edit(&a0, &["l1", "2", "storageHash"], flip),
        ),
        ("a missing account", no_account),
        (
            "block asked",
            edit(&a0, &["blockNumber"], |_| format!("0x{:x}", 52491358)),
        ),
    ];
    for (name, answer) in cases {
        let r = verify_l2(8453, 52491359, HASH_BASE_52491359, &answer);
        assert_eq!(r["ok"], false, "{name}: {r}");
        assert!(
            matches!(kind(&r), "rejected" | "malformed-input"),
            "{name}: {r}"
        );
    }
    // Base's anchor read as Unichain's: Unichain's registry is not in the state.
    assert_eq!(
        kind(&verify_l2(130, 52491359, HASH_BASE_52491359, &a0)),
        "rejected"
    );
    // A block after the claimed one is out of this game's reach.
    let later = edit(&a0, &["blockNumber"], |_| format!("0x{:x}", 52491361u64));
    assert_eq!(
        kind(&verify_l2(8453, 52491361, HASH_BASE_52491359, &later)),
        "not-covered"
    );
    // Ethereum state two hours old says nothing now.
    let late = l2_now(&a0) + 7200.0;
    let r = json_of(&synced().verify(8453, 52491359.0, HASH_BASE_52491359, &body(&a0), late));
    assert_eq!(kind(&r), "stale-head", "{r}");
}

const ARB_FINAL: &str = include_str!("../../../../web/verify/test/anchor/l2-42161-511868572.json");
const ARB_COMMITTED: &str =
    include_str!("../../../../web/verify/test/anchor/l2-42161-516340000.json");
const RH_COMMITTED: &str = include_str!("../../../../web/verify/test/anchor/l2-4663-92797219.json");
const HASH_ARB_511868572: &str =
    "0xf620d973d1e5601c0db1403537e382d1bafc08109c01732dfd2317180156a2e1";
const HASH_ARB_516340000: &str =
    "0x1386e7487e3aca9be3839d84f2fb4eaf5531903f3d134afdd1b4941c4226d6ae";
const HASH_RH_92797219: &str = "0xfdad7ee4bb3e1240d06d78c2e5ac3b6ea8b79fad49fea1bad97f0dc54fcf4ebe";

#[test]
fn the_pinned_nitro_history_code_is_arbitrums() {
    // eth_getCode of 0x0000F908…2935 on Arbitrum One and Robinhood Chain, 2026-10-19.
    assert_eq!(
        format!("{}", crate::l2::nitro_code_hash()),
        "0xceef1f6ad6c0cb7eb8fb15678abf11da3ce0d3a7c7106aa900c5930e556a43ff"
    );
}

#[test]
fn nitro_blocks_are_final_or_committed_through_their_assertions() {
    for (chain, number, hash, answer, level) in [
        (42161, 511868572, HASH_ARB_511868572, ARB_FINAL, "final"),
        (
            42161,
            516340000,
            HASH_ARB_516340000,
            ARB_COMMITTED,
            "committed",
        ),
        // The asserted block itself: no history proof.
        (4663, 92797219, HASH_RH_92797219, RH_COMMITTED, "committed"),
    ] {
        let r = verify_l2(chain, number, hash, &json_of(answer));
        assert_eq!(
            (
                r["ok"].as_bool(),
                r["level"].as_str(),
                r["matches"].as_bool(),
                r["path"].as_str()
            ),
            (Some(true), Some(level), Some(true), Some("assertion")),
            "{chain} {number}: {r}"
        );
    }
}

#[test]
fn every_tampered_part_of_a_nitro_anchor_is_refused() {
    let a0 = json_of(ARB_FINAL);
    let mut no_account = a0.clone();
    no_account["l1"].as_array_mut().unwrap().remove(2);
    let mut swapped = a0.clone();
    swapped["assertion"] = json_of(ARB_COMMITTED)["assertion"].clone();
    let cases: Vec<(&str, Value)> = vec![
        ("another assertion", swapped),
        (
            "asserted block hash",
            edit(&a0, &["assertion", "state", "0"], flip),
        ),
        ("send root", edit(&a0, &["assertion", "state", "1"], flip)),
        (
            "machine status",
            edit(&a0, &["assertion", "state", "4"], |_| {
                format!("0x{:064x}", 2)
            }),
        ),
        ("parent", edit(&a0, &["assertion", "parent"], flip)),
        (
            "inbox accumulator",
            edit(&a0, &["assertion", "inboxAcc"], flip),
        ),
        (
            "header",
            edit(&a0, &["headers", "0"], |s| {
                let mut c: Vec<char> = s.chars().collect();
                c[100] = if c[100] == '0' { '1' } else { '0' };
                c.into_iter().collect()
            }),
        ),
        (
            "history proof",
            edit(&a0, &["proofs", "0", "storageProof", "0", "value"], flip),
        ),
        (
            "Ethereum state",
            edit(&a0, &["l1", "2", "storageHash"], flip),
        ),
        ("a missing account", no_account),
        (
            "block asked",
            edit(&a0, &["blockNumber"], |_| format!("0x{:x}", 511868571)),
        ),
    ];
    for (name, answer) in cases {
        let r = verify_l2(42161, 511868572, HASH_ARB_511868572, &answer);
        assert_eq!(r["ok"], false, "{name}: {r}");
        assert!(
            matches!(kind(&r), "rejected" | "malformed-input"),
            "{name}: {r}"
        );
    }
    // Arbitrum One's anchor read as Robinhood Chain's, and back: the other Rollup has no such
    // assertion.
    assert_eq!(
        kind(&verify_l2(4663, 511868572, HASH_ARB_511868572, &a0)),
        "rejected"
    );
    let rh = json_of(RH_COMMITTED);
    assert_eq!(
        kind(&verify_l2(42161, 92797219, HASH_RH_92797219, &rh)),
        "rejected"
    );
    // A block after the asserted one is out of this assertion's reach.
    let later = edit(&rh, &["blockNumber"], |_| format!("0x{:x}", 92797220u64));
    assert_eq!(
        kind(&verify_l2(4663, 92797220, HASH_RH_92797219, &later)),
        "not-covered"
    );
    // Ethereum state two hours old says nothing now.
    let late = l2_now(&a0) + 7200.0;
    let r = json_of(&synced().verify(42161, 511868572.0, HASH_ARB_511868572, &body(&a0), late));
    assert_eq!(kind(&r), "stale-head", "{r}");
}

const BASE_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-8453-52840741.json");
const ARB_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-42161-516491624.json");
const RH_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-4663-93176007.json");
const OP_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-10-158436685.json");
const UNI_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-130-61723907.json");
const INK_SIGNED: &str = include_str!("../../../../web/verify/test/anchor/l2-57073-58973872.json");
const INK_SIGNED_HOPS: &str =
    include_str!("../../../../web/verify/test/anchor/l2-57073-58965015.json");
const HASH_INK_58965015: &str =
    "0x8d97f319b314db9cb71302335d64e563ed1dedcb7dabd57e86126b52d75dc356";
const HASH_BASE_52840741: &str =
    "0xd9e2792d91ca8372dc9c6c844f6d86b84e4aecb4d121a30039098f56b86ec27d";
const HASH_ARB_516491624: &str =
    "0x0135a0c01df67944b6cbbf5d0279da707d78acb08a2335bf55e8fbf37bd02ebd";
const HASH_RH_93176007: &str = "0x54a7407ec687e9dae494cdf108b1c8efd2addf77bccfb07d7b18d88f7e80454d";

#[test]
fn fresh_blocks_are_signed_by_the_sequencer_ethereum_names() {
    for (chain, number, hash, answer) in [
        (8453, 52840741, HASH_BASE_52840741, BASE_SIGNED),
        (42161, 516491624, HASH_ARB_516491624, ARB_SIGNED),
        (4663, 93176007, HASH_RH_93176007, RH_SIGNED),
        (
            10,
            158436685,
            "0x44ca9cec8aa2c1a9652a7af01b2a10fd5ce1351e21636b1f7c87c6f42a304272",
            OP_SIGNED,
        ),
        (
            130,
            61723907,
            "0x4a53000a73db1e628038f557dbe10d8854722bf6134a63e16959d63c529aac4e",
            UNI_SIGNED,
        ),
        (
            57073,
            58973872,
            "0xf99e1a06552a06e253802cffacf3f2b793a11064aea65cc4bf38df9ead2d09f1",
            INK_SIGNED,
        ),
    ] {
        let r = verify_l2(chain, number, hash, &json_of(answer));
        assert_eq!(
            (
                r["ok"].as_bool(),
                r["level"].as_str(),
                r["matches"].as_bool(),
                r["path"].as_str()
            ),
            (Some(true), Some("signed"), Some(true), Some("sequencer")),
            "{chain} {number}: {r}"
        );
        assert!(r["anchorBlock"].as_u64().unwrap() >= number, "{r}");
    }
}

/// A character of a hex string changed at `i` from its end.
fn flip_at(s: &str, i: usize) -> String {
    let mut c: Vec<char> = s.chars().collect();
    let j = c.len() - i;
    c[j] = if c[j] == '0' { '1' } else { '0' };
    c.into_iter().collect()
}

#[test]
fn every_tampered_part_of_a_signed_anchor_is_refused() {
    let base = json_of(BASE_SIGNED);
    let arb = json_of(ARB_SIGNED);
    let mut other_version = base.clone();
    other_version["signed"]["version"] = json!(2);
    let mut resigned = arb.clone();
    resigned["signed"]["message"]["blockHash"] = json!(format!("0x{}", "07".repeat(32)));
    let mut no_header = base.clone();
    no_header["headers"] = json!([]);
    let cases: Vec<(&str, u32, u64, &str, Value)> = vec![
        // The payload's block number (8 bytes at 404 after the signature and beacon root).
        (
            "signed data",
            8453,
            52840741,
            HASH_BASE_52840741,
            edit(&base, &["signed", "message"], |s| {
                let mut c: Vec<char> = s.chars().collect();
                let at = 2 + 2 * (65 + 32 + 404);
                c[at] = if c[at] == '0' { '1' } else { '0' };
                c.into_iter().collect()
            }),
        ),
        (
            "signature",
            8453,
            52840741,
            HASH_BASE_52840741,
            edit(&base, &["signed", "message"], |s| {
                let mut c: Vec<char> = s.chars().collect();
                c[10] = if c[10] == '0' { '1' } else { '0' };
                c.into_iter().collect()
            }),
        ),
        (
            "gossip version",
            8453,
            52840741,
            HASH_BASE_52840741,
            other_version,
        ),
        (
            "feed block hash",
            42161,
            516491624,
            HASH_ARB_516491624,
            resigned,
        ),
        (
            "header",
            42161,
            516491624,
            HASH_ARB_516491624,
            edit(&arb, &["headers", "0"], |s| flip_at(s, 200)),
        ),
        ("no header", 8453, 52840741, HASH_BASE_52840741, no_header),
        (
            "history proof",
            42161,
            516491624,
            HASH_ARB_516491624,
            edit(&arb, &["proofs", "0", "storageProof", "0", "value"], flip),
        ),
        (
            "Ethereum state",
            8453,
            52840741,
            HASH_BASE_52840741,
            edit(&base, &["l1", "accounts", "2", "storageHash"], flip),
        ),
    ];
    for (name, chain, number, hash, answer) in cases {
        let r = verify_l2(chain, number, hash, &answer);
        assert_eq!(r["ok"], false, "{name}: {r}");
        assert!(
            matches!(kind(&r), "rejected" | "malformed-input"),
            "{name}: {r}"
        );
    }
    // Another chain: Base's block read as OP Mainnet's, Arbitrum One's as Robinhood Chain's.
    assert_eq!(
        kind(&verify_l2(10, 52840741, HASH_BASE_52840741, &base)),
        "rejected"
    );
    assert_eq!(
        kind(&verify_l2(4663, 516491624, HASH_ARB_516491624, &arb)),
        "rejected"
    );
    // A block after the signed one is out of its reach.
    let rh = json_of(RH_SIGNED);
    let top = verify_l2(4663, 93176007, HASH_RH_93176007, &rh)["anchorBlock"]
        .as_u64()
        .unwrap();
    let later = edit(&rh, &["blockNumber"], |_| format!("0x{:x}", top + 1));
    assert_eq!(
        kind(&verify_l2(4663, top + 1, HASH_RH_93176007, &later)),
        "not-covered"
    );
    // Ethereum state two hours old names no key now.
    let late = l2_now(&base) + 7200.0;
    let r = json_of(&synced().verify(8453, 52840741.0, HASH_BASE_52840741, &body(&base), late));
    assert_eq!(kind(&r), "stale-head", "{r}");
}

#[test]
fn a_pending_claim_counts_only_with_every_claim_it_builds_on() {
    let rh = json_of(RH_COMMITTED);
    let n = rh["assertion"]["ancestors"].as_array().unwrap().len();
    assert!(n > 100, "{n} pending claims");
    let mut short = rh.clone();
    short["assertion"]["ancestors"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(
        kind(&verify_l2(4663, 92797219, HASH_RH_92797219, &short)),
        "not-covered"
    );
    let mut none = rh.clone();
    none["assertion"]
        .as_object_mut()
        .unwrap()
        .remove("ancestors");
    assert_eq!(
        kind(&verify_l2(4663, 92797219, HASH_RH_92797219, &none)),
        "malformed-input"
    );
    let altered = edit(&rh, &["assertion", "ancestors", "40", "stateHash"], flip);
    assert_eq!(
        kind(&verify_l2(4663, 92797219, HASH_RH_92797219, &altered)),
        "rejected"
    );
    // The chain's slots left out of the state: the walk cannot read them.
    let mut unproven = rh.clone();
    let accounts = unproven["l1"]["accounts"].as_array_mut().unwrap();
    let rollup = accounts
        .iter_mut()
        .max_by_key(|a| a["storageProof"].as_array().unwrap().len())
        .unwrap();
    rollup["storageProof"].as_array_mut().unwrap().truncate(8);
    assert_eq!(
        kind(&verify_l2(4663, 92797219, HASH_RH_92797219, &unproven)),
        "rejected"
    );
}

#[test]
fn shared_trie_nodes_are_the_same_witness() {
    let a0 = json_of(ARB_FINAL);
    let shared = crate::call::share_nodes(a0["l1"].as_array().unwrap());
    assert!(!shared["nodes"].as_array().unwrap().is_empty());
    let mut a1 = a0.clone();
    a1["l1"] = shared.clone();
    assert_eq!(
        verify_l2(42161, 511868572, HASH_ARB_511868572, &a1)["level"],
        "final"
    );
    // A reference to no shared node, and a shared node changed.
    let mut dangling = a1.clone();
    dangling["l1"]["accounts"][2]["accountProof"][0] = json!("#999999");
    assert_eq!(
        kind(&verify_l2(42161, 511868572, HASH_ARB_511868572, &dangling)),
        "malformed-input"
    );
    let changed = edit(&a1, &["l1", "nodes", "0"], |s| flip_at(s, 3));
    assert_eq!(
        kind(&verify_l2(42161, 511868572, HASH_ARB_511868572, &changed)),
        "rejected"
    );
}

#[test]
fn a_signed_block_reaches_back_through_history_hops() {
    // Ink's games are 12 hours apart and its history 2.3 hours: 10,000 blocks back is one hop.
    let ink = json_of(INK_SIGNED_HOPS);
    assert_eq!(ink["headers"].as_array().unwrap().len(), 2);
    let r = verify_l2(57073, 58965015, HASH_INK_58965015, &ink);
    assert_eq!(
        (r["level"].as_str(), r["matches"].as_bool()),
        (Some("signed"), Some(true)),
        "{r}"
    );
    let mut no_hop = ink.clone();
    no_hop["headers"].as_array_mut().unwrap().pop();
    assert_eq!(
        kind(&verify_l2(57073, 58965015, HASH_INK_58965015, &no_hop)),
        "malformed-input"
    );
    let other = edit(&ink, &["headers", "1"], |s| flip_at(s, 200));
    assert_eq!(
        kind(&verify_l2(57073, 58965015, HASH_INK_58965015, &other)),
        "rejected"
    );
}

#[test]
fn shared_nodes_are_bounded_before_they_are_copied() {
    let a0 = json_of(ARB_FINAL);
    let mut big = a0.clone();
    big["l1"] = crate::call::share_nodes(a0["l1"].as_array().unwrap());
    big["l1"]["nodes"][0] = json!(format!("0x{}", "00".repeat(1_200)));
    assert_eq!(
        kind(&verify_l2(42161, 511868572, HASH_ARB_511868572, &big)),
        "malformed-input"
    );
    let mut many = a0.clone();
    many["l1"] = crate::call::share_nodes(a0["l1"].as_array().unwrap());
    many["l1"]["accounts"][2]["accountProof"] = json!(vec!["#0"; 65]);
    assert_eq!(
        kind(&verify_l2(42161, 511868572, HASH_ARB_511868572, &many)),
        "malformed-input"
    );
}

/// A state root and its witness (`eth_getProof` results with code) for `accounts`: each an
/// address, its code and its storage slots. Built with the trie code the proofs are checked with.
fn synthetic_state(
    accounts: &[(
        alloy_primitives::Address,
        Vec<u8>,
        Vec<([u8; 32], alloy_primitives::U256)>,
    )],
) -> (alloy_primitives::B256, Value) {
    use alloy_primitives::{keccak256, B256, U256};
    use alloy_trie::{proof::ProofRetainer, HashBuilder, Nibbles};
    let hexs = |b: &[u8]| format!("0x{}", hex::encode(b));
    let trie = |leaves: Vec<(B256, Vec<u8>)>| {
        let mut leaves = leaves;
        leaves.sort_by_key(|(k, _)| *k);
        let targets: Vec<Nibbles> = leaves.iter().map(|(k, _)| Nibbles::unpack(k)).collect();
        let mut hb = HashBuilder::default().with_proof_retainer(ProofRetainer::new(targets));
        for (k, v) in &leaves {
            hb.add_leaf(Nibbles::unpack(k), v);
        }
        let root = hb.root();
        let nodes = hb.take_proof_nodes();
        let proof = move |k: &B256| -> Vec<String> {
            nodes
                .matching_nodes_sorted(&Nibbles::unpack(k))
                .into_iter()
                .map(|(_, n)| hexs(&n))
                .collect()
        };
        (root, proof)
    };
    let mut leaves = Vec::new();
    let mut rest = Vec::new();
    for (address, code, slots) in accounts {
        let (storage_root, proof) = if slots.is_empty() {
            (alloy_trie::EMPTY_ROOT_HASH, None)
        } else {
            let (root, proof) = trie(
                slots
                    .iter()
                    .map(|(s, v)| (keccak256(s), alloy_rlp::encode(*v)))
                    .collect(),
            );
            (root, Some(proof))
        };
        let code_hash = keccak256(code);
        leaves.push((
            keccak256(address),
            crate::call::account_rlp(1, U256::ZERO, storage_root, code_hash),
        ));
        let storage: Vec<Value> = slots
            .iter()
            .map(|(s, v)| {
                json!({"key": hexs(s), "value": format!("{v:#x}"),
                       "proof": proof.as_ref().unwrap()(&keccak256(s))})
            })
            .collect();
        rest.push((*address, code.clone(), storage_root, code_hash, storage));
    }
    let (root, proof) = trie(leaves);
    let witness = rest
        .into_iter()
        .map(|(address, code, storage_root, code_hash, storage)| {
            json!({"address": format!("{address}"), "balance": "0x0", "nonce": "0x1",
                   "codeHash": format!("{code_hash}"), "storageHash": format!("{storage_root}"),
                   "accountProof": proof(&keccak256(address)), "storageProof": storage,
                   "code": hexs(&code)})
        })
        .collect();
    (root, Value::Array(witness))
}

/// The Rollup's node slot of an assertion (mapping slot 117) and its packed word.
fn node(
    hash: [u8; 32],
    status: u8,
    first_child: bool,
    second_child_block: u64,
) -> ([u8; 32], alloy_primitives::U256) {
    use alloy_primitives::{keccak256, U256};
    let mut pre = [0u8; 64];
    pre[..32].copy_from_slice(&hash);
    pre[63] = 117;
    let mut w = [0u8; 32];
    w[6] = status;
    w[7] = u8::from(first_child);
    w[8..16].copy_from_slice(&26_000_000u64.to_be_bytes());
    w[16..24].copy_from_slice(&second_child_block.to_be_bytes());
    w[24..32].copy_from_slice(&26_000_100u64.to_be_bytes());
    (keccak256(pre).0, U256::from_be_bytes(w))
}

/// A pending claim's chain, back to a confirmed one, run with the Rollup's own code (Arbitrum
/// One's proxy and logic, from the saved anchor) over made-up node slots: one rival anywhere in
/// the chain, or the claim itself a second child, makes it disputed.
#[test]
fn a_rival_anywhere_back_to_a_confirmed_claim_is_a_dispute() {
    use alloy_primitives::{address, keccak256, Address, U256};
    let saved = json_of(ARB_COMMITTED);
    let code_of = |a: &str| {
        let acc = saved["l1"]["accounts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["address"].as_str().unwrap().eq_ignore_ascii_case(a))
            .unwrap();
        hex::decode(&acc["code"].as_str().unwrap()[2..]).unwrap()
    };
    let rollup = address!("4DCeB440657f21083db8aDd07665f8ddBe1DCfc0");
    let logic = address!("6490ba0a60cc7d3a59c9eee135d9eed24553a60d");
    let slot = |h: &str| -> [u8; 32] { hex::decode(&h[2..]).unwrap().try_into().unwrap() };
    // The proxy's own slots, as read on Ethereum: the user logic and the admin.
    let proxy_slots = vec![
        (
            slot("0x2b1dbce74324248c222f0ec2d5ed7bd323cfc425b336f0253c5ccfda7265546d"),
            U256::from_be_slice(logic.as_slice()),
        ),
        (
            slot("0xb53127684a568b3173ae13b9f8a6016e243e63b6e8ee1178d6a717850b5d6103"),
            U256::from_be_slice(address!("3fffbadaf827559da092217e474760e2b2c3cedd").as_slice()),
        ),
    ];
    // X builds on P, which builds on Q: P's preimage is sent.
    let q = [0x11u8; 32];
    let (p_state, p_acc) = ([0x22u8; 32], [0x33u8; 32]);
    let p = keccak256([q, p_state, p_acc].concat()).0;
    let x = [0x44u8; 32];
    let ancestors = vec![json!({"parent": format!("0x{}", hex::encode(q)),
        "stateHash": format!("0x{}", hex::encode(p_state)),
        "inboxAcc": format!("0x{}", hex::encode(p_acc))})];
    let chain = &crate::l2::NITRO_CHAINS[0];
    let run = |nodes: Vec<([u8; 32], U256)>| {
        let mut slots = proxy_slots.clone();
        // Each assertion's config hash, the slot after its node, which getAssertion reads too.
        for (s, _) in &nodes {
            let next = U256::from_be_bytes(*s) + U256::from(1);
            slots.push((next.to_be_bytes(), U256::from(1)));
        }
        slots.extend(nodes);
        let block = finalized_block();
        let accounts = vec![
            (
                rollup,
                code_of("0x4dceb440657f21083db8add07665f8ddbe1dcfc0"),
                slots,
            ),
            (
                logic,
                code_of("0x6490ba0a60cc7d3a59c9eee135d9eed24553a60d"),
                vec![],
            ),
            (Address::ZERO, vec![], vec![]),
            (Address::from(block.fee_recipient), vec![], vec![]),
        ];
        let (root, witness) = synthetic_state(&accounts);
        let state = crate::call::prove_state(root, &witness).unwrap();
        let l1 = lc::Execution {
            state_root: root.0,
            ..block
        };
        crate::l2::pending_chain_stands(&state, &l1, chain, 7, x, p, &ancestors).map_err(|f| f.kind)
    };
    // Every link a first child, every parent without a second: the chain stands.
    assert_eq!(
        run(vec![
            node(x, 1, true, 0),
            node(p, 1, true, 0),
            node(q, 2, true, 0)
        ]),
        Ok(())
    );
    // A rival to X (P has a second child), to P (Q has one), or X itself the rival.
    for nodes in [
        vec![
            node(x, 1, true, 0),
            node(p, 1, true, 26_000_050),
            node(q, 2, true, 0),
        ],
        vec![
            node(x, 1, true, 0),
            node(p, 1, true, 0),
            node(q, 2, true, 26_000_050),
        ],
        vec![
            node(x, 1, false, 0),
            node(p, 1, true, 0),
            node(q, 2, true, 0),
        ],
    ] {
        assert_eq!(run(nodes), Err(Kind::Disputed));
    }
    // A chain whose links Willow did not all send (Q pending too, its preimage missing), and an
    // end Ethereum does not have.
    assert_eq!(
        run(vec![
            node(x, 1, true, 0),
            node(p, 1, true, 0),
            node(q, 1, true, 0)
        ]),
        Err(Kind::NotCovered)
    );
    assert_eq!(
        run(vec![
            node(x, 1, true, 0),
            node(p, 1, true, 0),
            node(q, 0, false, 0)
        ]),
        Err(Kind::Rejected)
    );
}

/// Ink's game 2574, read on Ethereum on 2026-10-20: created by someone other than Ink's proposer,
/// in progress, claiming a time 5.4 days ahead.
const INK_GAME_2574: &str =
    include_str!("../../../../web/verify/test/anchor/l2-57073-ink-game-2574.json");

#[test]
fn a_game_by_someone_else_is_declined_not_called_committed_or_forged() {
    let a = json_of(INK_GAME_2574);
    let r = verify_l2(
        57073,
        58_980_000,
        "0x0000000000000000000000000000000000000000000000000000000000000001",
        &a,
    );
    assert_eq!(kind(&r), "declined", "{r}");
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("nor a standing claim by Ink's proposer"),
        "{r}"
    );
}

// `willow_getBlockAnchor ["latest"]` and `["finalized"]` (interface I5), saved by
// `web/verify/test/save_tag_anchors.py`: each verified as the client does, by the response's own
// `blockNumber` and any hash, then tampered.

const TAGS_CHECKPOINT: &str =
    include_str!("../../../../web/verify/test/anchor/tags/checkpoint.json");
const TAGS_BOOTSTRAP: &str = include_str!("../../../../web/verify/test/anchor/tags/bootstrap.json");
const TAGS_UPDATES: &str = include_str!("../../../../web/verify/test/anchor/tags/updates.json");
const LATEST_1: &str = include_str!("../../../../web/verify/test/anchor/tags/latest-1.json");
const FINALIZED_1: &str = include_str!("../../../../web/verify/test/anchor/tags/finalized-1.json");
const LATEST_L2: [(u32, &str); 6] = [
    (
        8453,
        include_str!("../../../../web/verify/test/anchor/tags/latest-8453.json"),
    ),
    (
        10,
        include_str!("../../../../web/verify/test/anchor/tags/latest-10.json"),
    ),
    (
        130,
        include_str!("../../../../web/verify/test/anchor/tags/latest-130.json"),
    ),
    (
        57073,
        include_str!("../../../../web/verify/test/anchor/tags/latest-57073.json"),
    ),
    (
        42161,
        include_str!("../../../../web/verify/test/anchor/tags/latest-42161.json"),
    ),
    (
        4663,
        include_str!("../../../../web/verify/test/anchor/tags/latest-4663.json"),
    ),
];
const ANY_HASH: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

/// A light client synced from the tags' checkpoint at `now`.
fn tags_synced(now: f64) -> Anchor {
    let c = json_of(TAGS_CHECKPOINT);
    let mut a = Anchor::new(c["root"].as_str().unwrap(), c["slot"].as_f64().unwrap());
    let r = json_of(&a.bootstrap(&body(&json_of(TAGS_BOOTSTRAP)), now));
    assert_eq!(r["ok"], true, "{r}");
    let r = json_of(&a.updates(&body(&json_of(TAGS_UPDATES))));
    assert_eq!(r["ok"], true, "{r}");
    a
}

fn saved_at(fixture: &Value) -> f64 {
    fixture["savedAt"].as_f64().unwrap()
}

fn number_of(result: &Value) -> u64 {
    u64::from_str_radix(&result["blockNumber"].as_str().unwrap()[2..], 16).unwrap()
}

/// `Anchor.verify` of `result` on `chain` for block `number`, at `now`.
fn verify_at(chain: u32, number: u64, result: &Value, now: f64) -> Value {
    json_of(&tags_synced(now).verify(chain, number as f64, ANY_HASH, &body(result), now))
}

/// A fixture's response verified as the client verifies it: the block its `blockNumber` names.
fn verify_tag(chain: u32, fixture: &Value) -> Value {
    let result = &fixture["response"]["result"];
    verify_at(chain, number_of(result), result, saved_at(fixture))
}

#[test]
fn latest_on_ethereum_is_the_light_clients_head() {
    let fx = json_of(LATEST_1);
    assert_eq!(fx["request"]["params"], json!(["latest"]));
    let n = number_of(&fx["response"]["result"]);
    let r = verify_tag(1, &fx);
    assert_eq!(
        (r["ok"].as_bool(), r["path"].as_str(), r["level"].as_str()),
        (Some(true), Some("head"), Some("signed")),
        "{r}"
    );
    // The proven hash is the block a public node has; the client reads it from `hash`.
    assert_eq!(r["hash"], fx["expect"]["hash"]);
    assert_eq!(r["head"].as_u64(), Some(n));
    assert!(
        saved_at(&fx) - r["headTime"].as_f64().unwrap() < 60.0,
        "{r}"
    );
    let hash = fx["expect"]["hash"].as_str().unwrap();
    let body = fx["response"].to_string();
    let r = json_of(&tags_synced(saved_at(&fx)).verify(1, n as f64, hash, &body, saved_at(&fx)));
    assert_eq!(r["matches"], true, "{r}");
}

#[test]
fn finalized_on_ethereum_is_the_heads_finalized_block_by_history() {
    let fx = json_of(FINALIZED_1);
    assert_eq!(fx["request"]["params"], json!(["finalized"]));
    let n = number_of(&fx["response"]["result"]);
    let r = verify_tag(1, &fx);
    assert_eq!(
        (r["ok"].as_bool(), r["path"].as_str(), r["level"].as_str()),
        (Some(true), Some("history"), Some("final")),
        "{r}"
    );
    assert_eq!(r["hash"], fx["expect"]["hash"]);
    // The update's own finalized block, not an older final one: the client compares them.
    assert_eq!(r["finalized"].as_u64(), Some(n));
    assert!(r["head"].as_u64().unwrap() > n);
}

#[test]
fn latest_on_each_l2_is_the_newest_block_its_sequencer_signed() {
    for (chain, s) in LATEST_L2 {
        let fx = json_of(s);
        assert_eq!(fx["request"]["params"], json!(["latest"]), "{chain}");
        let result = &fx["response"]["result"];
        let n = number_of(result);
        // No history hops: the signed block is the block.
        assert_eq!(result["headers"].as_array().unwrap().len(), 1, "{chain}");
        assert!(result["proofs"].as_array().unwrap().is_empty(), "{chain}");
        let r = verify_tag(chain, &fx);
        assert_eq!(
            (r["ok"].as_bool(), r["path"].as_str(), r["level"].as_str()),
            (Some(true), Some("sequencer"), Some("signed")),
            "{chain}: {r}"
        );
        assert_eq!(r["hash"], fx["expect"]["hash"], "{chain}");
        assert_eq!(r["anchorBlock"].as_u64(), Some(n), "{chain}");
        assert_eq!(r["anchorTime"], fx["expect"]["anchorTime"], "{chain}");
    }
}

#[test]
fn another_blocks_tag_anchor_is_rejected() {
    let latest = json_of(LATEST_1);
    let (result, now) = (&latest["response"]["result"], saved_at(&latest));
    let n = number_of(result);
    // Read as another block, or renamed: the head is block n alone.
    assert_eq!(kind(&verify_at(1, n - 1, result, now)), "rejected");
    let renamed = edit(result, &["blockNumber"], |_| format!("0x{:x}", n - 1));
    assert_eq!(kind(&verify_at(1, n - 1, &renamed, now)), "rejected");
    // The finalized block's history proof, renamed to the block before it.
    let fin = json_of(FINALIZED_1);
    let (fr, fnow) = (&fin["response"]["result"], saved_at(&fin));
    let f = number_of(fr);
    let renamed = edit(fr, &["blockNumber"], |_| format!("0x{:x}", f - 1));
    assert_eq!(kind(&verify_at(1, f - 1, &renamed, fnow)), "rejected");
    // An L2's signed block, read as the one before it (no hop to it) or after it (out of reach).
    for (chain, s) in LATEST_L2 {
        let fx = json_of(s);
        let (r, now) = (&fx["response"]["result"], saved_at(&fx));
        let m = number_of(r);
        assert_eq!(
            kind(&verify_at(chain, m - 1, r, now)),
            "rejected",
            "{chain}"
        );
        let before = edit(r, &["blockNumber"], |_| format!("0x{:x}", m - 1));
        assert_eq!(
            kind(&verify_at(chain, m - 1, &before, now)),
            "malformed-input",
            "{chain}"
        );
        let after = edit(r, &["blockNumber"], |_| format!("0x{:x}", m + 1));
        assert_eq!(
            kind(&verify_at(chain, m + 1, &after, now)),
            "not-covered",
            "{chain}"
        );
    }
}

#[test]
fn a_stale_tag_anchor_is_refused_or_shows_its_age() {
    // Ethereum's head over 10 minutes old says nothing.
    let latest = json_of(LATEST_1);
    let result = &latest["response"]["result"];
    let t = json_of(&tags_synced(saved_at(&latest)).verify(
        1,
        number_of(result) as f64,
        ANY_HASH,
        &body(result),
        saved_at(&latest),
    ))["headTime"]
        .as_f64()
        .unwrap();
    let r = verify_at(1, number_of(result), result, t + 601.0);
    assert_eq!(kind(&r), "stale-head", "{r}");
    // A finalized block stays final: an old `finalized` verifies, and `headTime` shows its age,
    // which the client bounds.
    let fin = json_of(FINALIZED_1);
    let fr = &fin["response"]["result"];
    let day_later = saved_at(&fin) + 86_400.0;
    let r = verify_at(1, number_of(fr), fr, day_later);
    assert_eq!(
        (r["ok"].as_bool(), r["level"].as_str()),
        (Some(true), Some("final"))
    );
    assert!(
        day_later - r["headTime"].as_f64().unwrap() > 86_400.0,
        "{r}"
    );
    for (chain, s) in LATEST_L2 {
        let fx = json_of(s);
        let (r, now) = (&fx["response"]["result"], saved_at(&fx));
        let m = number_of(r);
        // Twenty minutes on, it verifies, and `anchorTime` shows its age.
        let v = verify_at(chain, m, r, now + 1200.0);
        assert_eq!(v["ok"], true, "{chain}: {v}");
        assert!(
            now + 1200.0 - v["anchorTime"].as_f64().unwrap() >= 1200.0,
            "{chain}"
        );
        // An hour on, the Ethereum state naming the key is too old.
        assert_eq!(
            kind(&verify_at(chain, m, r, now + 3600.0)),
            "stale-head",
            "{chain}"
        );
    }
}

#[test]
fn a_tag_anchor_for_another_chain_is_refused() {
    let l2 = |c: u32| {
        let fx = json_of(LATEST_L2.iter().find(|(id, _)| *id == c).unwrap().1);
        (fx["response"]["result"].clone(), saved_at(&fx))
    };
    // Another chain's sequencer key or feed signature.
    for (from, read_as) in [
        (8453, 10),
        (10, 130),
        (130, 57073),
        (57073, 8453),
        (42161, 4663),
        (4663, 42161),
    ] {
        let (r, now) = l2(from);
        let v = verify_at(read_as, number_of(&r), &r, now);
        assert_eq!(kind(&v), "rejected", "{from} as {read_as}: {v}");
    }
    // Ethereum's head on an L2's route, and an L2's block on Ethereum's: neither shape is the other.
    let latest = json_of(LATEST_1);
    let (r, now) = (&latest["response"]["result"], saved_at(&latest));
    for chain in [8453, 42161] {
        let v = verify_at(chain, number_of(r), r, now);
        assert_eq!(v["ok"], false, "{chain}: {v}");
    }
    let (r, now) = l2(8453);
    assert_eq!(
        kind(&verify_at(1, number_of(&r), &r, now)),
        "malformed-input"
    );
}

// Unichain after its proposer changed on 2026-10-29, saved by eth-anchor's
// `record_unichain_anchors` on 2026-11-11 at one finalized Ethereum block and verified with the
// `tags/` light client: `{chainId, block, hash (the public node's), savedAt, result}`.

/// The block the public route refused on 2026-11-11 (about 2 days old), through the first game
/// after it by the proposer the factory names now, in progress.
const UNI_REPRO: &str = include_str!("../../../../web/verify/test/anchor/l2-130-repro.json");
/// A week-old block through the newest game valid on Ethereum, as the any-creator search found it
/// with Unichain's old pin.
const UNI_FINAL: &str = include_str!("../../../../web/verify/test/anchor/l2-130-final.json");
/// The same block through the next game: resolved for its defender, inside the finality delay.
const UNI_AIRGAPPED: &str =
    include_str!("../../../../web/verify/test/anchor/l2-130-airgapped.json");
/// The same block through a game by someone who posted one twice a day for a week, which lost.
const UNI_LOST: &str = include_str!("../../../../web/verify/test/anchor/l2-130-lost.json");
/// The same block's Ethereum state for a game of type 8, respected until 2026-09-24.
const UNI_TYPE_8: &str = include_str!("../../../../web/verify/test/anchor/l2-130-type-8.json");

/// Unichain with no proposer pinned.
const UNICHAIN_NO_PINS: crate::l2::OpChain = crate::l2::OpChain {
    proposers: &[],
    ..UNICHAIN_BEFORE
};

struct Uni {
    result: Value,
    block: u64,
    hash: String,
    now: f64,
}

fn uni(fixture: &str) -> Uni {
    let f = json_of(fixture);
    Uni {
        result: f["result"].clone(),
        block: f["block"].as_u64().unwrap(),
        hash: f["hash"].as_str().unwrap().into(),
        now: f["savedAt"].as_f64().unwrap(),
    }
}

/// `Anchor.verify` of `result` for a fixture's block, as the client runs it.
fn verify_uni(u: &Uni, result: &Value) -> Value {
    json_of(&tags_synced(u.now).verify(130, u.block as f64, &u.hash, &body(result), u.now))
}

/// `confirm_op` of a fixture with `chain`'s pins.
fn uni_on(chain: &crate::l2::OpChain, u: &Uni) -> Result<crate::l2::L2Confirmed, Failure> {
    let mut a = tags_synced(u.now);
    crate::l2::confirm_op_on(a.client().unwrap(), chain, u.block, &u.result, u.now as u64)
}

fn level(r: &Value) -> (Option<bool>, Option<&str>, Option<bool>) {
    (
        r["ok"].as_bool(),
        r["level"].as_str(),
        r["matches"].as_bool(),
    )
}

#[test]
fn the_unichain_block_refused_on_11_11_is_committed_through_its_proposers_game() {
    let u = uni(UNI_REPRO);
    let r = verify_uni(&u, &u.result);
    assert_eq!(
        level(&r),
        (Some(true), Some("committed"), Some(true)),
        "{r}"
    );
    // With the pin before the rotation the game is no one's, and still in progress: declined,
    // the refusal the route gave.
    assert_eq!(
        uni_on(&UNICHAIN_BEFORE, &u).unwrap_err().kind,
        Kind::Declined
    );
}

#[test]
fn a_claim_valid_on_ethereum_is_final_whoever_made_it() {
    let u = uni(UNI_FINAL);
    let r = verify_uni(&u, &u.result);
    assert_eq!(level(&r), (Some(true), Some("final"), Some(true)), "{r}");
    for chain in [&UNICHAIN_BEFORE, &UNICHAIN_NO_PINS] {
        let c = uni_on(chain, &u).unwrap();
        assert_eq!(
            (c.level, format!("{}", B256::from(c.hash))),
            ("final", u.hash.clone())
        );
    }
}

#[test]
fn committed_needs_a_pinned_proposer() {
    let u = uni(UNI_AIRGAPPED);
    let fin = uni(UNI_FINAL);
    assert_eq!(u.block, fin.block);
    assert_ne!(u.result["game"], fin.result["game"]);
    let r = verify_uni(&u, &u.result);
    assert_eq!(
        level(&r),
        (Some(true), Some("committed"), Some(true)),
        "{r}"
    );
    for chain in [&UNICHAIN_BEFORE, &UNICHAIN_NO_PINS] {
        let e = uni_on(chain, &u).unwrap_err();
        assert_eq!(e.kind, Kind::Declined, "{}", e.message);
    }
    // An unresolved game too: the refused block's, with no pin for its creator.
    let e = uni_on(&UNICHAIN_NO_PINS, &uni(UNI_REPRO)).unwrap_err();
    assert_eq!(e.kind, Kind::Declined, "{}", e.message);
}

#[test]
fn a_lost_game_or_one_of_another_type_is_refused() {
    let u = uni(UNI_LOST);
    assert_eq!(u.block, uni(UNI_FINAL).block);
    let r = verify_uni(&u, &u.result);
    assert_eq!(kind(&r), "declined", "{r}");
    // The registry still counts a type-8 game's claim valid (respected when it was made, after
    // the retirement), but its claim is an output root, not the super root Unichain's games claim
    // now: refused.
    let u = uni(UNI_TYPE_8);
    let r = verify_uni(&u, &u.result);
    assert_eq!(kind(&r), "rejected", "{r}");
    assert!(r["message"].as_str().unwrap().contains("super root"), "{r}");
}

#[test]
fn every_tampered_part_of_a_final_unichain_anchor_is_refused() {
    let u = uni(UNI_FINAL);
    let a0 = &u.result;
    let good = verify_uni(&u, a0);
    let m = good["anchorBlock"].as_u64().unwrap();
    assert!(m > u.block, "{good}");
    let mut swapped = a0.clone();
    swapped["game"] = json_of(UNI_AIRGAPPED)["result"]["game"].clone();
    let mut no_account = a0.clone();
    no_account["l1"]["accounts"]
        .as_array_mut()
        .unwrap()
        .remove(3);
    let cases: Vec<(&str, Value)> = vec![
        ("another game over this one's state", swapped),
        ("header", edit(a0, &["headers", "0"], |s| flip_at(s, 200))),
        (
            "history proof",
            edit(a0, &["proofs", "0", "storageProof", "0", "value"], flip),
        ),
        (
            "Ethereum state",
            edit(a0, &["l1", "accounts", "2", "storageHash"], flip),
        ),
        (
            "a shared proof node",
            edit(a0, &["l1", "nodes", "0"], |s| flip_at(s, 3)),
        ),
        ("a missing account", no_account),
        (
            "block asked",
            edit(a0, &["blockNumber"], |_| format!("0x{:x}", u.block - 1)),
        ),
    ];
    for (name, response) in cases {
        let r = verify_uni(&u, &response);
        assert_eq!(r["ok"], false, "{name}: {r}");
        assert!(
            matches!(kind(&r), "rejected" | "malformed-input"),
            "{name}: {r}"
        );
    }
    // Unichain's anchor read as OP Mainnet's: OP Mainnet's registry is not in the state.
    let r = json_of(&tags_synced(u.now).verify(10, u.block as f64, &u.hash, &body(a0), u.now));
    assert_eq!(kind(&r), "rejected", "{r}");
    // A block after the claimed one is out of this game's reach.
    let later = edit(a0, &["blockNumber"], |_| format!("0x{:x}", m + 1));
    let r = json_of(&tags_synced(u.now).verify(130, (m + 1) as f64, &u.hash, &body(&later), u.now));
    assert_eq!(kind(&r), "not-covered", "{r}");
    // Ethereum state two hours old says nothing now.
    let late = u.now + 7200.0;
    let r = json_of(&tags_synced(u.now).verify(130, u.block as f64, &u.hash, &body(a0), late));
    assert_eq!(kind(&r), "stale-head", "{r}");
}
