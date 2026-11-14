//! The anchoring flow on anchors saved from Willow and public endpoints (web/verify/test/anchor):
//! every path (Ethereum's head, history, beacon and pre-merge accumulator; OP games, Nitro
//! assertions, sequencer signatures and header descents; Sepolia's light client for the testnets;
//! BNB's validators) on every chain saved, each light client's data asked for on its route, kept
//! and restored, and refused when it should be.

use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../web/verify/test/anchor")
}

fn read(rel: &str) -> Value {
    serde_json::from_slice(&std::fs::read(dir().join(rel)).unwrap()).unwrap()
}

fn files(rel: &str, prefix: &str) -> Vec<(String, Value)> {
    let mut v: Vec<(String, Value)> = std::fs::read_dir(dir().join(rel))
        .unwrap()
        .filter_map(|e| {
            let name = e.unwrap().file_name().into_string().unwrap();
            name.starts_with(prefix)
                .then(|| (name.clone(), read(&format!("{rel}/{name}"))))
        })
        .collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn period(u: &Value) -> u64 {
    let slot: u64 = u["data"]["attested_header"]["beacon"]["slot"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    slot / 8192
}

/// Willow as the recorders saw it: each light client's data on its route, the saved anchors.
#[derive(Default)]
struct World {
    ethereum: Option<(Value, Vec<Value>)>,
    sepolia: Option<(Value, Vec<Value>)>,
    bnb: Option<(Value, Vec<Value>)>,
    anchors: HashMap<(u64, String), Value>,
    asked: RefCell<Vec<(u64, String, Value)>>,
}

impl World {
    fn light_client_asks(&self) -> Vec<(u64, String, Value)> {
        self.asked
            .borrow()
            .iter()
            .filter(|(_, m, _)| m.starts_with("willow_lightClient"))
            .cloned()
            .collect()
    }
}

impl Transport for World {
    fn body(&self, chain: u64, method: &str, params: &Value) -> Result<String, String> {
        self.asked
            .borrow_mut()
            .push((chain, method.to_string(), params.clone()));
        let lc = match chain {
            1 => &self.ethereum,
            56 => &self.bnb,
            _ => &self.sepolia,
        };
        let result = match method {
            "willow_lightClientBootstrap" => lc.as_ref().map(|(b, _)| b.clone()),
            "willow_lightClientUpdates" => lc.as_ref().map(|(_, u)| {
                let (from, count) = (params[0].as_u64().unwrap(), params[1].as_u64().unwrap());
                let within = |k: u64| k >= from && k < from + count;
                Value::Array(
                    u.iter()
                        .filter(|u| within(u["epoch"].as_u64().unwrap_or_else(|| period(u))))
                        .cloned()
                        .collect(),
                )
            }),
            "willow_getBlockAnchor" => {
                let p = params[0].as_str().unwrap().to_string();
                self.anchors.get(&(chain, p)).cloned()
            }
            _ => None,
        };
        Ok(match result {
            Some(r) => json!({"jsonrpc": "2.0", "id": 1, "result": r}),
            None => json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "not saved"}}),
        }
        .to_string())
    }
}

fn temp_dir() -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!(
        "wlv-anchor-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn opts(checkpoint: &Value, now: u64, cache: Option<PathBuf>) -> Options {
    Options {
        checkpoint: Some((
            checkpoint["root"].as_str().unwrap().to_string(),
            checkpoint["slot"].as_u64().unwrap(),
        )),
        cache_dir: cache,
        now: Some(now),
        waiting_ms: 0,
        ..Options::default()
    }
}

fn quantity(n: u64) -> String {
    format!("0x{n:x}")
}

const HASH_26226600: &str = "0xd9d667f111fb89c038ad7dc4374d339a1b42a1ad5325e99a563fb24b7ad3c423";
const HASH_26226601: &str = "0x83222a42c7ee2cbb512db881bc268b57004437a988ff70d2f1fa9db59cf42cbd";
const HASH_26124863: &str = "0x28f942fc7925d908e5aee5d59a38ed9138c664a8ea8cf0a98fb270855bf6b1d8";
const OTHER: &str = "0xabababababababababababababababababababababababababababababababab";

/// The original saved light-client data (test/anchor/) and when it was current.
fn original() -> (World, Value, u64) {
    let w = World {
        ethereum: Some((
            read("bootstrap.json"),
            read("updates.json").as_array().unwrap().clone(),
        )),
        ..World::default()
    };
    let slot: u64 = read("history-26226600.json")["update"]["data"]["signature_slot"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    (
        w,
        read("checkpoint.json"),
        ETHEREUM_GENESIS_TIME + 12 * slot + 60,
    )
}

#[test]
fn ethereum_blocks_by_history_and_beacon_are_final_and_the_light_client_is_kept() {
    let (mut w, cp, now) = original();
    for (n, f) in [
        (26226600, "history-26226600.json"),
        (26226601, "history-26226601.json"),
        (26124863, "beacon-26124863.json"),
    ] {
        w.anchors.insert((1, quantity(n)), read(f));
    }
    let cache = temp_dir();
    let mut a = Anchoring::new(&w, 1, opts(&cp, now, Some(cache.clone())), None).unwrap();
    for (n, hash, path) in [
        (26226600, HASH_26226600, "history"),
        (26226601, HASH_26226601, "history"),
        (26124863, HASH_26124863, "beacon"),
    ] {
        let r = a.block(n, hash).unwrap();
        assert_eq!(
            (&r["level"], &r["path"], &r["hash"], &r["anchored"]),
            (&json!("final"), &json!(path), &json!(hash), &json!(true)),
            "{n}: {r}"
        );
        assert_eq!(r["start"]["from"], "--checkpoint");
        assert_eq!(r["start"]["lightClient"], "ethereum");
    }
    // Ethereum's light-client data came from chain 1's route, and was kept.
    let lc = w.light_client_asks();
    assert!(
        !lc.is_empty() && lc.iter().all(|(c, _, _)| *c == 1),
        "{lc:?}"
    );
    assert!(cache.join("light-client.json").exists());
    // Another hash for the block: shown wrong, not merely unchecked.
    let f = a.block(26226600, OTHER).unwrap_err();
    assert_eq!(f.kind, "mismatch");
    assert!(f.is_verdict());
    // A new run restores what was kept, verifying it again, and asks for no light-client data.
    w.asked.borrow_mut().clear();
    let mut b = Anchoring::new(&w, 1, opts(&cp, now, Some(cache.clone())), None).unwrap();
    assert_eq!(b.block(26226600, HASH_26226600).unwrap()["level"], "final");
    assert!(w.light_client_asks().is_empty());
    // A kept file that was altered is not trusted: the client starts over from the checkpoint.
    let kept = std::fs::read_to_string(cache.join("light-client.json")).unwrap();
    let i = kept.find("aggregate_pubkey").unwrap() + 30;
    let mut altered = kept.into_bytes();
    altered[i] = if altered[i] == b'1' { b'2' } else { b'1' };
    std::fs::write(cache.join("light-client.json"), altered).unwrap();
    let mut c = Anchoring::new(&w, 1, opts(&cp, now, Some(cache)), None).unwrap();
    assert_eq!(c.block(26226600, HASH_26226600).unwrap()["level"], "final");
    assert!(!w.light_client_asks().is_empty());
    // The anchor of another block, served for this one: rejected.
    let mut w2 = original().0;
    w2.anchors
        .insert((1, quantity(26226600)), read("history-26226601.json"));
    let mut d = Anchoring::new(&w2, 1, opts(&cp, now, None), None).unwrap();
    let f = d.block(26226600, HASH_26226600).unwrap_err();
    assert_eq!(f.kind, "rejected", "{f:?}");
    assert!(f.is_verdict());
    // A checkpoint past its 30 days.
    let mut e = Anchoring::new(&w, 1, opts(&cp, now + 40 * DAY, None), None).unwrap();
    assert_eq!(
        e.block(26226600, HASH_26226600).unwrap_err().kind,
        "stale-checkpoint"
    );
    // Willow refusing the anchor.
    let mut g = Anchoring::new(&w, 1, opts(&cp, now, None), None).unwrap();
    assert_eq!(
        g.block(26226602, HASH_26226600).unwrap_err().kind,
        "refused"
    );
}

#[test]
fn ethereums_newest_block_is_the_signed_head_and_finalized_is_final() {
    let mut w = World {
        ethereum: Some((
            read("tags/bootstrap.json"),
            read("tags/updates.json").as_array().unwrap().clone(),
        )),
        ..World::default()
    };
    let cp = read("tags/checkpoint.json");
    let latest = read("tags/latest-1.json");
    let fin = read("tags/finalized-1.json");
    w.anchors
        .insert((1, "latest".into()), latest["response"]["result"].clone());
    w.anchors
        .insert((1, "finalized".into()), fin["response"]["result"].clone());
    let now = latest["savedAt"].as_u64().unwrap();
    let mut a = Anchoring::new(&w, 1, opts(&cp, now, None), None).unwrap();
    let h = a.head("latest").unwrap();
    assert_eq!(
        (&h.report["path"], &h.report["level"], json!(h.hash.clone())),
        (
            &json!("head"),
            &json!("signed"),
            latest["expect"]["hash"].clone()
        )
    );
    // The head, once anchored, anchors its own block with no other request.
    let asked = w.asked.borrow().len();
    assert_eq!(a.block(h.number, &h.hash).unwrap()["path"], "head");
    assert_eq!(w.asked.borrow().len(), asked);
    // At --min-level final, the newest block is the finalized one.
    let mut o = opts(&cp, now, None);
    o.min_level = Level::Final;
    let mut f = Anchoring::new(&w, 1, o, None).unwrap();
    let h = f.head("latest").unwrap();
    assert_eq!(
        (&h.report["level"], json!(h.hash), &h.report["finalized"]),
        (
            &json!("final"),
            fin["expect"]["hash"].clone(),
            &json!(h.number)
        )
    );
    // Too old a head: no newest block.
    let mut o = opts(&cp, now, None);
    o.max_head_age = 0;
    let mut s = Anchoring::new(&w, 1, o, None).unwrap();
    assert_eq!(s.head("latest").unwrap_err().kind, "stale-head");
    // The finalized block's anchor served for "latest": not the head.
    let mut w2 = World {
        ethereum: w.ethereum.clone(),
        ..World::default()
    };
    w2.anchors
        .insert((1, "latest".into()), fin["response"]["result"].clone());
    let mut t = Anchoring::new(&w2, 1, opts(&cp, now, None), None).unwrap();
    let e = t.head("latest").unwrap_err();
    assert_eq!(e.kind, "not-the-head");
    assert!(e.is_verdict());
}

#[test]
fn each_l2s_newest_block_is_the_one_its_sequencer_signed() {
    let cp = read("tags/checkpoint.json");
    for chain in [8453u64, 10, 130, 57073, 42161, 4663] {
        let fx = read(&format!("tags/latest-{chain}.json"));
        let mut w = World {
            ethereum: Some((
                read("tags/bootstrap.json"),
                read("tags/updates.json").as_array().unwrap().clone(),
            )),
            ..World::default()
        };
        w.anchors
            .insert((chain, "latest".into()), fx["response"]["result"].clone());
        let now = fx["savedAt"].as_u64().unwrap();
        let mut a = Anchoring::new(&w, chain, opts(&cp, now, None), None).unwrap();
        let h = a.head("latest").unwrap();
        assert_eq!(
            (&h.report["path"], &h.report["level"], json!(h.hash)),
            (
                &json!("sequencer"),
                &json!("signed"),
                fx["expect"]["hash"].clone()
            ),
            "{chain}"
        );
        // Ethereum's light-client data on chain 1's route.
        assert!(w.light_client_asks().iter().all(|(c, _, _)| *c == 1));
        // An L2's newest block is only ever signed.
        let mut o = opts(&cp, now, None);
        o.min_level = Level::Committed;
        let mut b = Anchoring::new(&w, chain, o, None).unwrap();
        assert_eq!(b.head("latest").unwrap_err().kind, "below-min-level");
        assert_eq!(a.head("finalized").unwrap_err().kind, "unverifiable-tag");
    }
}

/// An L2 anchor's time: a minute after the Ethereum block it reads.
fn l2_now(anchor: &Value) -> u64 {
    anchor["update"]["data"]["finalized_header"]["execution"]["timestamp"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        + 60
}

/// The proven hash of `n` (any hash asked), then that hash anchored at `level` by `path`.
fn l2_block(chain: u64, n: u64, anchor: Value, level: &str, path: &str) -> String {
    let (mut w, cp, _) = original();
    let now = l2_now(&anchor);
    w.anchors.insert((chain, quantity(n)), anchor);
    let mut o = opts(&cp, now, None);
    o.min_level = Level::Signed;
    let mut a = Anchoring::new(&w, chain, o.clone(), None).unwrap();
    let f = a.block(n, OTHER).unwrap_err();
    assert_eq!(f.kind, "mismatch", "{chain} {n}: {f:?}");
    let hash = f.message.split_whitespace().nth(6).unwrap().to_string();
    let mut b = Anchoring::new(&w, chain, o, None).unwrap();
    let r = b.block(n, &hash).unwrap();
    assert_eq!(
        (&r["level"], &r["path"]),
        (&json!(level), &json!(path)),
        "{chain} {n}: {r}"
    );
    assert!(w.light_client_asks().iter().all(|(c, _, _)| *c == 1));
    hash
}

#[test]
fn l2_blocks_by_games_assertions_and_sequencer_signatures() {
    l2_block(
        8453,
        52491359,
        read("l2-8453-52491359.json"),
        "final",
        "game",
    );
    assert_eq!(
        l2_block(
            8453,
            52820000,
            read("l2-8453-52820000.json"),
            "committed",
            "game"
        ),
        "0xa110242001cbe9bca8d8d4f255adfaace5340d30d2a0efd07d6225e20e14e5d7"
    );
    l2_block(
        42161,
        511868572,
        read("l2-42161-511868572.json"),
        "final",
        "assertion",
    );
    l2_block(
        4663,
        92797219,
        read("l2-4663-92797219.json"),
        "committed",
        "assertion",
    );
    for (chain, n, f) in [
        (8453, 52840741, "l2-8453-52840741.json"),
        (42161, 516491624, "l2-42161-516491624.json"),
        (4663, 93176007, "l2-4663-93176007.json"),
        (10, 158436685, "l2-10-158436685.json"),
        (130, 61723907, "l2-130-61723907.json"),
        (57073, 58973872, "l2-57073-58973872.json"),
    ] {
        l2_block(chain, n, read(f), "signed", "sequencer");
    }
    // A committed anchor below --min-level final.
    let (mut w, cp, _) = original();
    let anchor = read("l2-8453-52820000.json");
    let now = l2_now(&anchor);
    w.anchors.insert((8453, quantity(52820000)), anchor);
    let mut o = opts(&cp, now, None);
    o.min_level = Level::Final;
    let mut a = Anchoring::new(&w, 8453, o, None).unwrap();
    let f = a
        .block(
            52820000,
            "0xa110242001cbe9bca8d8d4f255adfaace5340d30d2a0efd07d6225e20e14e5d7",
        )
        .unwrap_err();
    assert_eq!(f.kind, "below-min-level");
    assert!(!f.is_verdict());
}

#[test]
fn blocks_below_an_assertion_by_their_header_descent() {
    let d = read("descent-4663-92797219.json");
    let headers = d["descent"].as_array().unwrap();
    for k in [1usize, 10, 48] {
        let mut anchor = read("l2-4663-92797219.json");
        let n = 92797219 - k as u64;
        anchor["blockNumber"] = json!(quantity(n));
        anchor["descent"] = json!(headers[..k]);
        l2_block(4663, n, anchor, "committed", "assertion");
    }
    // A descent with a header left out reaches no block.
    let mut anchor = read("l2-4663-92797219.json");
    anchor["blockNumber"] = json!(quantity(92797219 - 10));
    let mut gap: Vec<Value> = headers[..10].to_vec();
    gap.remove(4);
    anchor["descent"] = json!(gap);
    let (mut w, cp, _) = original();
    let now = l2_now(&anchor);
    w.anchors.insert((4663, quantity(92797219 - 10)), anchor);
    let mut a = Anchoring::new(&w, 4663, opts(&cp, now, None), None).unwrap();
    assert!(a.block(92797219 - 10, OTHER).unwrap_err().is_verdict());
}

/// The chains added after the first seven (chains/): Ethereum's or Sepolia's light client.
fn new_chain(chain: u64) {
    let all = files("chains", &format!("l2-{chain}-"));
    assert!(!all.is_empty(), "{chain}");
    let sepolia = light_client_of(chain) == Some("sepolia");
    for (name, f) in all {
        let mut w = World {
            ethereum: Some((
                read("chains/bootstrap.json"),
                read("chains/updates.json").as_array().unwrap().clone(),
            )),
            sepolia: Some((
                read("chains/sepolia-bootstrap.json"),
                read("chains/sepolia-updates.json")
                    .as_array()
                    .unwrap()
                    .clone(),
            )),
            ..World::default()
        };
        let n = f["block"].as_u64().unwrap();
        w.anchors.insert((chain, quantity(n)), f["result"].clone());
        let mut o = opts(
            &read("chains/checkpoint.json"),
            f["savedAt"].as_u64().unwrap(),
            None,
        );
        let sc = read("chains/sepolia-checkpoint.json");
        o.sepolia = Some((
            sc["root"].as_str().unwrap().into(),
            sc["slot"].as_u64().unwrap(),
        ));
        let mut a = Anchoring::new(&w, chain, o.clone(), None).unwrap();
        let hash = f["hash"].as_str().unwrap();
        let r = a.block(n, hash).unwrap();
        assert_eq!(r["level"], f["expect"]["level"], "{name}: {r}");
        assert_eq!(
            r["start"]["lightClient"],
            if sepolia { "sepolia" } else { "ethereum" }
        );
        // Sepolia's data on the testnet's own route; Ethereum's on chain 1's.
        let lc = w.light_client_asks();
        assert!(!lc.is_empty(), "{name}");
        let route = if sepolia { chain } else { 1 };
        assert!(lc.iter().all(|(c, _, _)| *c == route), "{name}: {lc:?}");
        let mut b = Anchoring::new(&w, chain, o, None).unwrap();
        assert_eq!(b.block(n, OTHER).unwrap_err().kind, "mismatch", "{name}");
    }
}

#[test]
fn soneium_zora_and_world_chain_anchor_on_ethereum() {
    for chain in [1868, 7777777, 480] {
        new_chain(chain);
    }
}

#[test]
fn the_testnets_anchor_on_sepolia_with_its_data_on_their_own_route() {
    for chain in [11155420, 84532, 1301, 763373, 421614, 46630] {
        new_chain(chain);
    }
}

#[test]
fn bnb_blocks_are_final_by_its_validators_votes_kept_and_restored() {
    let bnb = read("chains/bnb.json");
    let mut w = World {
        bnb: Some((
            bnb["bootstrap"].clone(),
            bnb["updates"].as_array().unwrap().clone(),
        )),
        ..World::default()
    };
    let start = (
        bnb["start"]["number"].as_u64().unwrap(),
        bnb["start"]["hash"].as_str().unwrap().to_string(),
    );
    for x in bnb["anchors"].as_array().unwrap() {
        w.anchors.insert(
            (56, quantity(x["block"].as_u64().unwrap())),
            x["result"].clone(),
        );
    }
    let cache = temp_dir();
    for x in bnb["anchors"].as_array().unwrap() {
        let o = Options {
            bnb: Some(start.clone()),
            cache_dir: Some(cache.clone()),
            now: x["savedAt"].as_u64(),
            ..Options::default()
        };
        let mut a = Anchoring::new(&w, 56, o, None).unwrap();
        let r = a
            .block(x["block"].as_u64().unwrap(), x["hash"].as_str().unwrap())
            .unwrap();
        assert_eq!(
            (&r["level"], &r["path"]),
            (&json!("final"), &json!("parlia"))
        );
        assert_eq!(r["start"]["from"], "--bnb-start");
    }
    // BNB's data on chain 56's route, at most 64 epochs an ask, kept under its own name.
    let lc = w.light_client_asks();
    assert!(lc
        .iter()
        .all(|(c, m, p)| *c == 56
            && (m != "willow_lightClientUpdates" || p[1].as_u64().unwrap() <= 64)));
    assert!(lc
        .iter()
        .any(|(_, m, p)| m == "willow_lightClientBootstrap" && p[0] == json!(start.1)));
    assert!(cache.join("light-client-bnb.json").exists());
    assert!(!cache.join("light-client.json").exists());
    // Restored on the next run: no light-client data asked for.
    w.asked.borrow_mut().clear();
    let x = &bnb["anchors"][0];
    let o = Options {
        bnb: Some(start.clone()),
        cache_dir: Some(cache),
        now: x["savedAt"].as_u64(),
        ..Options::default()
    };
    let mut a = Anchoring::new(&w, 56, o, None).unwrap();
    a.block(x["block"].as_u64().unwrap(), x["hash"].as_str().unwrap())
        .unwrap();
    assert!(w.light_client_asks().is_empty());
    // BNB's newest block is final: "latest" at --min-level final.
    let newest = bnb["anchors"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|x| x["block"].as_u64().unwrap())
        .unwrap();
    w.anchors
        .insert((56, "latest".into()), newest["result"].clone());
    let o = Options {
        bnb: Some(start),
        cache_dir: None,
        now: newest["savedAt"].as_u64(),
        min_level: Level::Final,
        max_head_age: u64::MAX,
        ..Options::default()
    };
    let mut a = Anchoring::new(&w, 56, o, None).unwrap();
    let h = a.head("latest").unwrap();
    assert_eq!(
        (h.number, h.hash.as_str(), &h.report["level"]),
        (
            newest["block"].as_u64().unwrap(),
            newest["hash"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()
                .as_str(),
            &json!("final")
        )
    );
}

/// SSZ's sha256 of two nodes.
fn pair(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(a);
    h.update(b);
    h.finalize().into()
}

/// The siblings of `leaves[index]` in a zero-padded tree of `depth`.
fn branch(leaves: &[[u8; 32]], depth: usize, index: usize) -> Vec<[u8; 32]> {
    let mut z = vec![[0u8; 32]];
    for i in 0..depth {
        let n = pair(&z[i], &z[i]);
        z.push(n);
    }
    let (mut layer, mut i, mut out) = (leaves.to_vec(), index, Vec::new());
    for zl in z.iter().take(depth) {
        out.push(*layer.get(i ^ 1).unwrap_or(zl));
        layer = layer
            .chunks(2)
            .map(|p| pair(&p[0], p.get(1).unwrap_or(zl)))
            .collect();
        i /= 2;
    }
    out
}

#[test]
fn pre_merge_blocks_by_the_accumulator_need_no_light_client() {
    use anchor_verify::premerge::{EPOCH_DEPTH, EPOCH_ROOTS, EPOCH_SIZE};
    let roots: Vec<[u8; 32]> = EPOCH_ROOTS
        .chunks_exact(32)
        .map(|c| c.try_into().unwrap())
        .collect();
    let v: Value = serde_json::from_slice(
        &std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../anchor-verify/testdata/premerge-vectors.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut w = World::default();
    let mut blocks = Vec::new();
    for x in v["vectors"].as_array().unwrap() {
        let n = x["number"].as_u64().unwrap();
        let eb: Vec<String> = branch(&roots, EPOCH_DEPTH, (n / EPOCH_SIZE) as usize)
            .iter()
            .map(|b| format!("0x{}", hex::encode(b)))
            .collect();
        let td = x["totalDifficulty"].as_str().unwrap();
        let td = format!("0x{}", td[2..].trim_start_matches('0'));
        w.anchors.insert(
            (1, quantity(n)),
            json!({"blockNumber": quantity(n), "proof": {"kind": "accumulator", "hash": x["hash"],
                "totalDifficulty": td, "branch": x["branch"], "epochBranch": eb}}),
        );
        blocks.push((n, x["hash"].as_str().unwrap().to_string()));
    }
    assert_eq!(blocks.len(), 4);
    // A start years old: no matter before the merge.
    let (_, cp, _) = original();
    let mut a = Anchoring::new(&w, 1, opts(&cp, 2_000_000_000, None), None).unwrap();
    for (n, hash) in &blocks {
        let r = a.block(*n, hash).unwrap();
        assert_eq!(
            (&r["level"], &r["path"]),
            (&json!("final"), &json!("accumulator")),
            "{n}: {r}"
        );
        assert_eq!(a.block(*n, OTHER).unwrap_err().kind, "mismatch");
    }
    assert!(w.light_client_asks().is_empty());
}

#[test]
fn starts_come_from_the_flags_else_the_binary() {
    let shipped: Value = serde_json::from_str(SHIPPED).unwrap();
    let now = ETHEREUM_GENESIS_TIME + 12 * shipped["slot"].as_u64().unwrap() + DAY;
    let o = Options {
        now: Some(now),
        ..Options::default()
    };
    let s = choose_start(None, &o, now, None);
    assert_eq!((s.from, &s.json["root"]), ("shipped", &shipped["root"]));
    // Too old, and nothing newer can be fetched: the shipped one, with why.
    let s = choose_start(None, &o, now + 40 * DAY, None);
    assert_eq!(s.from, "shipped");
    assert!(s.note.unwrap().contains("days old"));
    let s = choose_start(Some("sepolia"), &o, now, None);
    assert_eq!(s.json["root"], shipped["sepolia"]["root"]);
    let s = choose_start(
        Some("bnb"),
        &o,
        shipped["bnb"]["time"].as_u64().unwrap() + 60,
        None,
    );
    assert_eq!(
        (s.from, &s.json["number"]),
        ("shipped", &shipped["bnb"]["number"])
    );
    // The flags win.
    let o = Options {
        checkpoint: Some((OTHER.into(), 7)),
        bnb: Some((1000, OTHER.into())),
        ..Options::default()
    };
    assert_eq!(choose_start(None, &o, now, None).from, "--checkpoint");
    assert_eq!(choose_start(Some("bnb"), &o, now, None).from, "--bnb-start");
    assert!(parse_checkpoint("0x12:5").is_err());
    assert_eq!(
        parse_checkpoint(&format!("{OTHER}:5")),
        Ok((OTHER.into(), 5))
    );
    assert!(parse_bnb_start(&format!("1001:{OTHER}")).is_err());
    assert_eq!(
        parse_bnb_start(&format!("2000:{OTHER}")),
        Ok((2000, OTHER.into()))
    );
}
