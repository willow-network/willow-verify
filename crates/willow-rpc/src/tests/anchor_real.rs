//! The anchor flow with the REAL light clients (anchor_verify::Anchor), on the
//! anchors saved from Willow and public endpoints (web/verify/test/anchor): the npm suite's
//! anchor.real, anchor.chains.real and the CLI's anchoring tests (query-verify src/anchor/tests.rs):
//! every path (Ethereum's head, history, beacon and pre-merge accumulator; OP games, Nitro
//! assertions, sequencer signatures and header descents; Sepolia's light client for the testnets;
//! BNB's validators), each light client's data asked for on its route, kept and restored, and
//! refused when it should be.

use super::support::*;
use crate::anchor::{self, Anchors};
use crate::checkpoint::{BnbStart, Checkpoint, Starts};
use crate::config::{Level, Status};
use crate::error::Error;
use crate::http::Reply;
use crate::storage::Storage;
use crate::verifiers::{real_anchor, Backend, OpenVm};
use futures::FutureExt;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

fn dir() -> PathBuf {
    repo().join("web/verify/test/anchor")
}

fn read(rel: &str) -> Value {
    json_file(&dir().join(rel))
}

const DAY: u64 = 86_400;
const GENESIS: u64 = 1_606_824_023;
const HASH_26226600: &str = "0xd9d667f111fb89c038ad7dc4374d339a1b42a1ad5325e99a563fb24b7ad3c423";
const HASH_26226601: &str = "0x83222a42c7ee2cbb512db881bc268b57004437a988ff70d2f1fa9db59cf42cbd";
const HASH_26124863: &str = "0x28f942fc7925d908e5aee5d59a38ed9138c664a8ea8cf0a98fb270855bf6b1d8";
const OTHER: &str = "0xabababababababababababababababababababababababababababababababab";

fn period(u: &Value) -> u64 {
    u["data"]["attested_header"]["beacon"]["slot"]
        .as_str()
        .unwrap()
        .parse::<u64>()
        .unwrap()
        / 8192
}

/// Willow as the recorders saw it: each light client's data on its route, the saved anchors.
#[derive(Default)]
struct Fake {
    ethereum: Option<(Value, Vec<Value>)>,
    sepolia: Option<(Value, Vec<Value>)>,
    bnb: Option<(Value, Vec<Value>)>,
    anchors: Mutex<HashMap<(u64, String), Value>>,
    asked: Mutex<Vec<(u64, String, Value)>>,
}

impl Fake {
    fn reply(&self, chain: u64, method: &str, params: &Value) -> Value {
        self.asked
            .lock()
            .unwrap()
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
                Value::Array(
                    u.iter()
                        .filter(|u| {
                            let k = u["epoch"].as_u64().unwrap_or_else(|| period(u));
                            k >= from && k < from + count
                        })
                        .cloned()
                        .collect(),
                )
            }),
            "willow_getBlockAnchor" => {
                let p = params[0].as_str().unwrap().to_string();
                self.anchors.lock().unwrap().get(&(chain, p)).cloned()
            }
            _ => None,
        };
        match result {
            Some(r) => json!({"jsonrpc": "2.0", "id": 1, "result": r}),
            None => {
                json!({"jsonrpc": "2.0", "id": 1, "error": {"code": -32000, "message": "not saved"}})
            }
        }
    }

    fn light_client_asks(&self) -> Vec<(u64, String, Value)> {
        self.asked
            .lock()
            .unwrap()
            .iter()
            .filter(|(_, m, _)| m.starts_with("willow_lightClient"))
            .cloned()
            .collect()
    }

    fn anchor(&self, chain: u64, param: &str, v: Value) {
        self.anchors
            .lock()
            .unwrap()
            .insert((chain, param.to_string()), v);
    }
}

#[derive(Default)]
struct Memory(Mutex<HashMap<String, String>>, Mutex<usize>);

impl Storage for Memory {
    fn get(&self, key: &str) -> Option<String> {
        self.0.lock().unwrap().get(key).cloned()
    }
    fn set(&self, key: &str, value: &str) {
        *self.1.lock().unwrap() += 1;
        self.0.lock().unwrap().insert(key.into(), value.into());
    }
}

fn starts(ethereum: &Value) -> Starts {
    let s = crate::checkpoint::shipped();
    Starts {
        ethereum: Checkpoint {
            root: ethereum["root"].as_str().unwrap().into(),
            slot: ethereum["slot"].as_u64().unwrap(),
        },
        ..s
    }
}

fn anchors(
    fake: Arc<Fake>,
    st: Starts,
    now: u64,
    storage: Option<Arc<dyn Storage>>,
    min: Level,
    max_head_age: u64,
) -> Arc<Anchors> {
    let rpc: anchor::Rpc = Arc::new(move |c: u64, m: &str, p: Value| {
        let v = fake.reply(c, m, &p);
        async move {
            Ok(Reply {
                text: v.to_string().into(),
                value: Arc::new(v),
            })
        }
        .boxed()
    });
    Anchors::new(anchor::Options {
        backend: Arc::new(Backend::new(real_anchor(), OpenVm::None)),
        rpc,
        storage,
        starts: Arc::new(move || {
            let s = st.clone();
            async move { Ok::<_, Error>(s) }.boxed()
        }),
        min_level: min,
        head_max_age_ms: Some(2000),
        max_head_age_sec: max_head_age,
        now_ms: Arc::new(move || now * 1000),
        waiting_ms: Some(0),
        anchor_wait_ms: 50,
    })
}

/// The original saved light-client data (test/anchor/) and when it was current.
fn original() -> (Fake, Value, u64) {
    let f = Fake {
        ethereum: Some((
            read("bootstrap.json"),
            read("updates.json").as_array().unwrap().clone(),
        )),
        ..Fake::default()
    };
    let slot: u64 = read("history-26226600.json")["update"]["data"]["signature_slot"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    (f, read("checkpoint.json"), GENESIS + 12 * slot + 60)
}

fn kind(e: &Error) -> &str {
    e.kind().unwrap_or("")
}

#[tokio::test(flavor = "multi_thread")]
async fn ethereum_blocks_by_history_and_beacon_are_final_and_the_light_client_is_kept() {
    let (f, cp, now) = original();
    for (n, file) in [
        (26226600, "history-26226600.json"),
        (26226601, "history-26226601.json"),
        (26124863, "beacon-26124863.json"),
    ] {
        f.anchor(1, &q(n), read(file));
    }
    let f = Arc::new(f);
    let mem = Arc::new(Memory::default());
    let a = anchors(
        f.clone(),
        starts(&cp),
        now,
        Some(mem.clone()),
        Level::Signed,
        600,
    );
    for (n, hash, path) in [
        (26226600, HASH_26226600, "history"),
        (26226601, HASH_26226601, "history"),
        (26124863, HASH_26124863, "beacon"),
    ] {
        let r = a.block(1, n, hash, None, None).await.unwrap();
        assert_eq!(
            (r.level, r.path.as_str(), r.hash.as_str()),
            (Some(Level::Final), path, hash),
            "{n}"
        );
    }
    // Ethereum's light-client data came from chain 1's route, and was kept.
    let lc = f.light_client_asks();
    assert!(!lc.is_empty() && lc.iter().all(|(c, _, _)| *c == 1));
    assert!(mem.get(crate::storage::LIGHT_CLIENT_KEY).is_some());
    // Another hash for the block: shown wrong.
    let e = a.block(1, 26226600, OTHER, None, None).await.unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32098, "anchor-mismatch"));
    // A new client restores what was kept, verifying it again, and asks for no light-client data.
    f.asked.lock().unwrap().clear();
    let b = anchors(
        f.clone(),
        starts(&cp),
        now,
        Some(mem.clone()),
        Level::Signed,
        600,
    );
    assert_eq!(
        b.block(1, 26226600, HASH_26226600, None, None)
            .await
            .unwrap()
            .level,
        Some(Level::Final)
    );
    assert!(f.light_client_asks().is_empty());
    // A kept value that was altered is not trusted: the client starts over from the checkpoint.
    let kept = mem.get(crate::storage::LIGHT_CLIENT_KEY).unwrap();
    let i = kept.find("aggregate_pubkey").unwrap() + 30;
    let mut altered = kept.into_bytes();
    altered[i] = if altered[i] == b'1' { b'2' } else { b'1' };
    mem.0.lock().unwrap().insert(
        crate::storage::LIGHT_CLIENT_KEY.into(),
        String::from_utf8(altered).unwrap(),
    );
    let c = anchors(
        f.clone(),
        starts(&cp),
        now,
        Some(mem.clone()),
        Level::Signed,
        600,
    );
    assert_eq!(
        c.block(1, 26226600, HASH_26226600, None, None)
            .await
            .unwrap()
            .level,
        Some(Level::Final)
    );
    assert!(!f.light_client_asks().is_empty());
    // The anchor of another block, served for this one: rejected.
    let (f2, _, _) = original();
    f2.anchor(1, &q(26226600), read("history-26226601.json"));
    let d = anchors(Arc::new(f2), starts(&cp), now, None, Level::Signed, 600);
    let e = d
        .block(1, 26226600, HASH_26226600, None, None)
        .await
        .unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32098, "anchor-rejected"));
    // A checkpoint past its 30 days.
    let g = anchors(
        f.clone(),
        starts(&cp),
        now + 40 * DAY,
        None,
        Level::Signed,
        600,
    );
    let e = g
        .block(1, 26226600, HASH_26226600, None, None)
        .await
        .unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32099, "anchor-stale-checkpoint"));
    // Willow refusing the anchor.
    let e = g
        .block(1, 26226602, HASH_26226600, None, None)
        .await
        .unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32099, "anchor-refused"));
}

fn tags_fake() -> Fake {
    Fake {
        ethereum: Some((
            read("tags/bootstrap.json"),
            read("tags/updates.json").as_array().unwrap().clone(),
        )),
        ..Fake::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ethereums_newest_block_is_the_signed_head_and_finalized_is_final() {
    let cp = read("tags/checkpoint.json");
    let latest = read("tags/latest-1.json");
    let fin = read("tags/finalized-1.json");
    let f = tags_fake();
    f.anchor(1, "latest", latest["response"]["result"].clone());
    f.anchor(1, "finalized", fin["response"]["result"].clone());
    let f = Arc::new(f);
    let now = latest["savedAt"].as_u64().unwrap();
    let a = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 600);
    let h = a.head(1, "latest", false, None).await.unwrap();
    assert_eq!(
        (h.path.as_str(), h.level, json!(h.hash)),
        (
            "head",
            Some(Level::Signed),
            latest["expect"]["hash"].clone()
        )
    );
    // The head, once anchored, anchors its own block with no other request.
    let asked = f.asked.lock().unwrap().len();
    assert_eq!(
        a.block(1, h.number, &h.hash, None, None)
            .await
            .unwrap()
            .path,
        "head"
    );
    assert_eq!(f.asked.lock().unwrap().len(), asked);
    // At min level final, the newest block is the finalized one.
    let b = anchors(f.clone(), starts(&cp), now, None, Level::Final, 600);
    let h = b.head(1, "latest", false, None).await.unwrap();
    assert_eq!(
        (h.level, json!(h.hash)),
        (Some(Level::Final), fin["expect"]["hash"].clone())
    );
    // Too old a head: no newest block.
    let s = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 0);
    let e = s.head(1, "latest", false, None).await.unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32099, "anchor-stale-head"));
    // The finalized block's anchor served for "latest": not the head.
    let f2 = tags_fake();
    f2.anchor(1, "latest", fin["response"]["result"].clone());
    let t = anchors(Arc::new(f2), starts(&cp), now, None, Level::Signed, 600);
    let e = t.head(1, "latest", false, None).await.unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32098, "anchor-head"));
    // "finalized" must be the light client's finalized block: the latest anchor served for it is not.
    let f3 = tags_fake();
    f3.anchor(1, "finalized", latest["response"]["result"].clone());
    let u = anchors(Arc::new(f3), starts(&cp), now, None, Level::Signed, 600);
    let e = u.head(1, "finalized", false, None).await.unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32098, "anchor-head"));
}

#[tokio::test(flavor = "multi_thread")]
async fn each_l2s_newest_block_is_the_one_its_sequencer_signed() {
    let cp = read("tags/checkpoint.json");
    for chain in [8453u64, 10, 130, 57073, 42161, 4663] {
        let fx = read(&format!("tags/latest-{chain}.json"));
        let f = tags_fake();
        f.anchor(chain, "latest", fx["response"]["result"].clone());
        let f = Arc::new(f);
        let now = fx["savedAt"].as_u64().unwrap();
        let a = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 600);
        let h = a.head(chain, "latest", false, None).await.unwrap();
        assert_eq!(
            (h.path.as_str(), h.level, json!(h.hash)),
            (
                "sequencer",
                Some(Level::Signed),
                fx["expect"]["hash"].clone()
            ),
            "{chain}"
        );
        assert!(f.light_client_asks().iter().all(|(c, _, _)| *c == 1));
        // An L2's newest block is only ever signed.
        let b = anchors(f.clone(), starts(&cp), now, None, Level::Committed, 600);
        let e = b.head(chain, "latest", false, None).await.unwrap_err();
        assert_eq!((e.code(), kind(&e)), (-32099, "anchor-level"), "{chain}");
        // Older than the head age limit: unverifiable.
        let c = anchors(
            f.clone(),
            starts(&cp),
            fx["expect"]["anchorTime"].as_u64().unwrap() + 601,
            None,
            Level::Signed,
            600,
        );
        let e = c.head(chain, "latest", false, None).await.unwrap_err();
        assert_eq!(
            (e.code(), kind(&e)),
            (-32099, "anchor-stale-head"),
            "{chain}"
        );
        // An anchor proving another hash for the head's block is a mismatch.
        let n = n(&fx["response"]["result"]["blockNumber"]);
        f.anchor(chain, &q(n), fx["response"]["result"].clone());
        let d = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 600);
        let e = d
            .block(chain, n, &format!("0x{}", "ee".repeat(32)), None, None)
            .await
            .unwrap_err();
        assert_eq!((e.code(), kind(&e)), (-32098, "anchor-mismatch"), "{chain}");
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
async fn l2_block(chain: u64, n: u64, anchor: Value, level: Level, path: &str) -> String {
    let (f, cp, _) = original();
    let now = l2_now(&anchor);
    f.anchor(chain, &q(n), anchor);
    let f = Arc::new(f);
    let a = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 600);
    let e = a.block(chain, n, OTHER, None, None).await.unwrap_err();
    assert_eq!(kind(&e), "anchor-mismatch", "{chain} {n}: {e}");
    let hash = e
        .data()
        .unwrap()
        .detail
        .as_deref()
        .unwrap()
        .split_whitespace()
        .nth(6)
        .unwrap()
        .to_string();
    let b = anchors(f.clone(), starts(&cp), now, None, Level::Signed, 600);
    let r = b.block(chain, n, &hash, None, None).await.unwrap();
    assert_eq!(
        (r.level, r.path.as_str()),
        (Some(level), path),
        "{chain} {n}"
    );
    assert!(f.light_client_asks().iter().all(|(c, _, _)| *c == 1));
    hash
}

#[tokio::test(flavor = "multi_thread")]
async fn l2_blocks_by_games_assertions_and_sequencer_signatures() {
    l2_block(
        8453,
        52491359,
        read("l2-8453-52491359.json"),
        Level::Final,
        "game",
    )
    .await;
    assert_eq!(
        l2_block(
            8453,
            52820000,
            read("l2-8453-52820000.json"),
            Level::Committed,
            "game"
        )
        .await,
        "0xa110242001cbe9bca8d8d4f255adfaace5340d30d2a0efd07d6225e20e14e5d7"
    );
    l2_block(
        42161,
        511868572,
        read("l2-42161-511868572.json"),
        Level::Final,
        "assertion",
    )
    .await;
    l2_block(
        4663,
        92797219,
        read("l2-4663-92797219.json"),
        Level::Committed,
        "assertion",
    )
    .await;
    for (chain, n, f) in [
        (8453, 52840741, "l2-8453-52840741.json"),
        (42161, 516491624, "l2-42161-516491624.json"),
        (4663, 93176007, "l2-4663-93176007.json"),
        (10, 158436685, "l2-10-158436685.json"),
        (130, 61723907, "l2-130-61723907.json"),
        (57073, 58973872, "l2-57073-58973872.json"),
    ] {
        l2_block(chain, n, read(f), Level::Signed, "sequencer").await;
    }
    // A committed anchor below min level final.
    let (f, cp, _) = original();
    let anchor = read("l2-8453-52820000.json");
    let now = l2_now(&anchor);
    f.anchor(8453, &q(52820000), anchor);
    let a = anchors(Arc::new(f), starts(&cp), now, None, Level::Final, 600);
    let e = a
        .block(
            8453,
            52820000,
            "0xa110242001cbe9bca8d8d4f255adfaace5340d30d2a0efd07d6225e20e14e5d7",
            None,
            None,
        )
        .await
        .unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32099, "anchor-level"));
}

#[tokio::test(flavor = "multi_thread")]
async fn blocks_below_an_assertion_by_their_header_descent() {
    let d = read("descent-4663-92797219.json");
    let headers = d["descent"].as_array().unwrap();
    for k in [1usize, 10, 48] {
        let mut anchor = read("l2-4663-92797219.json");
        let n = 92797219 - k as u64;
        anchor["blockNumber"] = json!(q(n));
        anchor["descent"] = json!(headers[..k]);
        l2_block(4663, n, anchor, Level::Committed, "assertion").await;
    }
    // A descent with a header left out reaches no block.
    let mut anchor = read("l2-4663-92797219.json");
    anchor["blockNumber"] = json!(q(92797219 - 10));
    let mut gap: Vec<Value> = headers[..10].to_vec();
    gap.remove(4);
    anchor["descent"] = json!(gap);
    let (f, cp, _) = original();
    let now = l2_now(&anchor);
    f.anchor(4663, &q(92797219 - 10), anchor);
    let a = anchors(Arc::new(f), starts(&cp), now, None, Level::Signed, 600);
    let e = a
        .block(4663, 92797219 - 10, OTHER, None, None)
        .await
        .unwrap_err();
    assert_eq!(e.code(), -32098, "{e}");
}

/// The chains added after the first seven: Ethereum's or Sepolia's light client.
async fn new_chain(chain: u64) {
    let mut all: Vec<(String, Value)> = std::fs::read_dir(dir().join("chains"))
        .unwrap()
        .filter_map(|e| {
            let name = e.ok()?.file_name().into_string().ok()?;
            name.starts_with(&format!("l2-{chain}-"))
                .then(|| (name.clone(), read(&format!("chains/{name}"))))
        })
        .collect();
    all.sort_by(|a, b| a.0.cmp(&b.0));
    assert!(!all.is_empty(), "{chain}");
    let sepolia = crate::verifiers::light_client_of(chain) == Some("sepolia");
    let sc = read("chains/sepolia-checkpoint.json");
    for (name, x) in all {
        let f = Fake {
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
            ..Fake::default()
        };
        let num = x["block"].as_u64().unwrap();
        f.anchor(chain, &q(num), x["result"].clone());
        let f = Arc::new(f);
        let mut st = starts(&read("chains/checkpoint.json"));
        st.sepolia = Checkpoint {
            root: sc["root"].as_str().unwrap().into(),
            slot: sc["slot"].as_u64().unwrap(),
        };
        let now = x["savedAt"].as_u64().unwrap();
        let a = anchors(f.clone(), st.clone(), now, None, Level::Signed, 600);
        let r = a
            .block(chain, num, x["hash"].as_str().unwrap(), None, None)
            .await
            .unwrap();
        assert_eq!(
            r.level.map(|l| l.as_str()),
            x["expect"]["level"].as_str(),
            "{name}"
        );
        // Sepolia's data on the testnet's own route; Ethereum's on chain 1's.
        let lc = f.light_client_asks();
        assert!(!lc.is_empty(), "{name}");
        let route = if sepolia { chain } else { 1 };
        assert!(lc.iter().all(|(c, _, _)| *c == route), "{name}: {lc:?}");
        let b = anchors(f.clone(), st, now, None, Level::Signed, 600);
        let e = b.block(chain, num, OTHER, None, None).await.unwrap_err();
        assert_eq!(kind(&e), "anchor-mismatch", "{name}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn soneium_zora_and_world_chain_anchor_on_ethereum() {
    for chain in [1868, 7777777, 480] {
        new_chain(chain).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_testnets_anchor_on_sepolia_with_its_data_on_their_own_route() {
    for chain in [11155420, 84532, 1301, 763373, 421614, 46630] {
        new_chain(chain).await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn bnb_blocks_are_final_by_its_validators_votes_kept_and_restored() {
    let bnb = read("chains/bnb.json");
    let f = Fake {
        bnb: Some((
            bnb["bootstrap"].clone(),
            bnb["updates"].as_array().unwrap().clone(),
        )),
        ..Fake::default()
    };
    for x in bnb["anchors"].as_array().unwrap() {
        f.anchor(56, &q(x["block"].as_u64().unwrap()), x["result"].clone());
    }
    let f = Arc::new(f);
    let mut st = crate::checkpoint::shipped();
    st.bnb = BnbStart {
        number: bnb["start"]["number"].as_u64().unwrap(),
        hash: bnb["start"]["hash"].as_str().unwrap().to_ascii_lowercase(),
        time: 0,
    };
    let mem = Arc::new(Memory::default());
    for x in bnb["anchors"].as_array().unwrap() {
        let a = anchors(
            f.clone(),
            st.clone(),
            x["savedAt"].as_u64().unwrap(),
            Some(mem.clone()),
            Level::Signed,
            600,
        );
        let r = a
            .block(
                56,
                x["block"].as_u64().unwrap(),
                x["hash"].as_str().unwrap(),
                None,
                None,
            )
            .await
            .unwrap();
        assert_eq!((r.level, r.path.as_str()), (Some(Level::Final), "parlia"));
    }
    // BNB's data on chain 56's route, at most 64 epochs an ask, kept under its own key.
    let lc = f.light_client_asks();
    assert!(lc
        .iter()
        .all(|(c, m, p)| *c == 56
            && (m != "willow_lightClientUpdates" || p[1].as_u64().unwrap() <= 64)));
    assert!(lc
        .iter()
        .any(|(_, m, p)| m == "willow_lightClientBootstrap" && p[0] == json!(st.bnb.hash)));
    assert!(mem
        .get(&format!("{}:bnb", crate::storage::LIGHT_CLIENT_KEY))
        .is_some());
    assert!(mem.get(crate::storage::LIGHT_CLIENT_KEY).is_none());
    // Restored by a new client: no light-client data asked for.
    f.asked.lock().unwrap().clear();
    let x = &bnb["anchors"][0];
    let a = anchors(
        f.clone(),
        st.clone(),
        x["savedAt"].as_u64().unwrap(),
        Some(mem.clone()),
        Level::Signed,
        600,
    );
    a.block(
        56,
        x["block"].as_u64().unwrap(),
        x["hash"].as_str().unwrap(),
        None,
        None,
    )
    .await
    .unwrap();
    assert!(f.light_client_asks().is_empty());
    // BNB's newest block is final: "latest" at min level final.
    let newest = bnb["anchors"]
        .as_array()
        .unwrap()
        .iter()
        .max_by_key(|x| x["block"].as_u64().unwrap())
        .unwrap();
    f.anchor(56, "latest", newest["result"].clone());
    let a = anchors(
        f.clone(),
        st,
        newest["savedAt"].as_u64().unwrap(),
        None,
        Level::Final,
        u64::MAX / 2,
    );
    let h = a.head(56, "latest", false, None).await.unwrap();
    assert_eq!(
        (h.number, h.hash.as_str(), h.level),
        (
            newest["block"].as_u64().unwrap(),
            newest["hash"]
                .as_str()
                .unwrap()
                .to_ascii_lowercase()
                .as_str(),
            Some(Level::Final)
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

#[tokio::test(flavor = "multi_thread")]
async fn pre_merge_blocks_by_the_accumulator_need_no_light_client() {
    use anchor_verify::premerge::{EPOCH_DEPTH, EPOCH_ROOTS, EPOCH_SIZE};
    let roots: Vec<[u8; 32]> = EPOCH_ROOTS
        .chunks_exact(32)
        .map(|c| c.try_into().unwrap())
        .collect();
    let v =
        json_file(&repo().join("zkvm/logset-guest/anchor-verify/testdata/premerge-vectors.json"));
    let f = Fake::default();
    let mut blocks = vec![];
    for x in v["vectors"].as_array().unwrap() {
        let num = x["number"].as_u64().unwrap();
        let eb: Vec<String> = branch(&roots, EPOCH_DEPTH, (num / EPOCH_SIZE) as usize)
            .iter()
            .map(|b| format!("0x{}", hex::encode(b)))
            .collect();
        let td = x["totalDifficulty"].as_str().unwrap();
        let td = format!("0x{}", td[2..].trim_start_matches('0'));
        f.anchor(1, &q(num), json!({"blockNumber": q(num), "proof": {"kind": "accumulator", "hash": x["hash"], "totalDifficulty": td, "branch": x["branch"], "epochBranch": eb}}));
        blocks.push((num, x["hash"].as_str().unwrap().to_string()));
    }
    assert_eq!(blocks.len(), 4);
    let f = Arc::new(f);
    // A start years old: no matter before the merge.
    let (_, cp, _) = original();
    let a = anchors(
        f.clone(),
        starts(&cp),
        2_000_000_000,
        None,
        Level::Signed,
        600,
    );
    for (num, hash) in &blocks {
        let r = a.block(1, *num, hash, None, None).await.unwrap();
        assert_eq!(
            (r.level, r.path.as_str()),
            (Some(Level::Final), "accumulator"),
            "{num}"
        );
        let e = a.block(1, *num, OTHER, None, None).await.unwrap_err();
        assert_eq!(kind(&e), "anchor-mismatch");
    }
    assert!(f.light_client_asks().is_empty());
}

/// The npm suite's anchor.real: the saved heads through the whole client and the mock server.
fn i5_world() -> Arc<World> {
    let w = World::new();
    let tag = |n: &str| fixture(&format!("anchor-tags/{n}.json"));
    {
        let mut s = w.s();
        s.now_sec = tag("latest-1")["savedAt"].as_u64().unwrap();
        s.bootstrap = Some(tag("bootstrap"));
        s.updates = Some(tag("updates"));
        for c in [1, 8453, 10, 130, 57073, 42161, 4663] {
            s.real_anchors.insert(
                format!("{c}:latest"),
                tag(&format!("latest-{c}"))["response"]["result"].clone(),
            );
        }
        s.real_anchors.insert(
            "1:finalized".into(),
            tag("finalized-1")["response"]["result"].clone(),
        );
    }
    w
}

#[tokio::test(flavor = "multi_thread")]
async fn eth_block_number_on_each_chain_is_the_anchored_head() {
    let cp = fixture("anchor-tags/checkpoint.json");
    for c in [1, 8453, 10, 130, 57073, 42161, 4663] {
        let t = fixture(&format!("anchor-tags/latest-{c}.json"));
        let cpc = Checkpoint {
            root: cp["root"].as_str().unwrap().into(),
            slot: cp["slot"].as_u64().unwrap(),
        };
        let h = harness_with(c, i5_world(), "test-key", move |mut x| {
            x.anchor_factory = Some(real_anchor());
            x.checkpoint(cpc)
        })
        .await;
        let got = h.ask("eth_blockNumber", json!([])).await.unwrap();
        assert_eq!(n(&got), n(&t["response"]["result"]["blockNumber"]), "{c}");
        let e = h.last();
        assert_eq!(
            (e.status, e.level),
            (Status::Verified, Some(Level::Signed)),
            "{c}"
        );
        assert_eq!(json!(e.block.unwrap().hash), t["expect"]["hash"], "{c}");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn the_light_client_catches_up_once_and_the_next_client_restores_it_from_storage() {
    let cp = fixture("anchor-tags/checkpoint.json");
    let cpc = Checkpoint {
        root: cp["root"].as_str().unwrap().into(),
        slot: cp["slot"].as_u64().unwrap(),
    };
    let mem = Arc::new(Memory::default());
    let (m1, c1) = (mem.clone(), cpc.clone());
    let h = harness_with(1, i5_world(), "test-key", move |mut x| {
        x.anchor_factory = Some(real_anchor());
        x.checkpoint(c1).storage(crate::StorageChoice::Custom(m1))
    })
    .await;
    h.ask("eth_blockNumber", json!([])).await.unwrap();
    let sent = h.world.methods_sent();
    assert_eq!(
        sent.iter()
            .filter(|m| *m == "willow_lightClientBootstrap")
            .count(),
        1
    );
    assert_eq!(
        sent.iter()
            .filter(|m| *m == "willow_lightClientUpdates")
            .count(),
        1
    );
    assert!(mem.get(crate::storage::LIGHT_CLIENT_KEY).unwrap().len() > 1000);
    let writes = *mem.1.lock().unwrap();
    let m2 = mem.clone();
    let h = harness_with(1, i5_world(), "test-key", move |mut x| {
        x.anchor_factory = Some(real_anchor());
        x.checkpoint(cpc).storage(crate::StorageChoice::Custom(m2))
    })
    .await;
    h.ask("eth_blockNumber", json!([])).await.unwrap();
    let again = h.world.methods_sent();
    assert!(
        !again.iter().any(|m| m.starts_with("willow_lightClient")),
        "{again:?}"
    );
    // Written when it changed, not on every anchor.
    assert_eq!(*mem.1.lock().unwrap(), writes);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_forged_validators_signature_or_another_blocks_head_fails_verification() {
    let cp = fixture("anchor-tags/checkpoint.json");
    let cpc = Checkpoint {
        root: cp["root"].as_str().unwrap().into(),
        slot: cp["slot"].as_u64().unwrap(),
    };
    let mut forged = fixture("anchor-tags/latest-1.json")["response"]["result"].clone();
    let sig = forged["update"]["data"]["sync_aggregate"]["sync_committee_signature"]
        .as_str()
        .unwrap()
        .to_string();
    forged["update"]["data"]["sync_aggregate"]["sync_committee_signature"] = json!(flip(&sig, 10));
    let mut moved = fixture("anchor-tags/latest-1.json")["response"]["result"].clone();
    moved["blockNumber"] = json!(q(n(&moved["blockNumber"]) - 1));
    for body in [forged, moved] {
        let w = i5_world();
        w.s().real_anchors.insert("1:latest".into(), body);
        let c = cpc.clone();
        let h = harness_with(1, w, "test-key", move |mut x| {
            x.anchor_factory = Some(real_anchor());
            x.checkpoint(c)
        })
        .await;
        let e = h.ask("eth_blockNumber", json!([])).await.unwrap_err();
        assert_eq!((e.code(), kind(&e)), (-32098, "anchor-rejected"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn ethereums_signed_head_over_600_s_old_is_unverifiable() {
    let cp = fixture("anchor-tags/checkpoint.json");
    let cpc = Checkpoint {
        root: cp["root"].as_str().unwrap().into(),
        slot: cp["slot"].as_u64().unwrap(),
    };
    let w = i5_world();
    w.s().now_sec += 1200;
    let h = harness_with(1, w, "test-key", move |mut x| {
        x.anchor_factory = Some(real_anchor());
        x.checkpoint(cpc)
    })
    .await;
    let e = h.ask("eth_blockNumber", json!([])).await.unwrap_err();
    assert_eq!((e.code(), kind(&e)), (-32099, "anchor-stale-head"));
}
