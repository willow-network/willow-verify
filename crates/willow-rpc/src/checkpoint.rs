//! Where the light clients start. Ethereum's checkpoint (and Sepolia's and BNB's starts) ship in
//! `checkpoint.json`, refreshed with each release. Once a shipped start is too old for its
//! verifier, a newer one is read from Willow's published `checkpoint.json`; an Ethereum one is
//! taken only when independent beacon APIs confirm it. A start passed in is used as it is.

use crate::error::Error;
use crate::hex::is_hash_str;
use serde_json::Value;
use std::time::Duration;

/// The starts this release ships (Ethereum's at the top level, as Willow's own checkpoint.json).
pub const SHIPPED_JSON: &str = include_str!("../checkpoint.json");
/// Where Willow publishes newer starts, beside the verify page.
pub const CHECKPOINT_URL: &str = "https://api.willow.tech/verify/checkpoint.json";
/// Beacon APIs that must confirm a fetched Ethereum checkpoint (the verify page build's two).
pub const BEACON_APIS: [&str; 2] = [
    "https://ethereum-beacon-api.publicnode.com",
    "https://lodestar-mainnet.chainsafe.io",
];
/// The verifier refuses an Ethereum or Sepolia start older than this (anchor-verify `MAX_START_AGE_SECS`).
pub const MAX_START_AGE_SECS: u64 = 30 * 24 * 3600;
/// The verifier refuses a BNB start older than BNB's 7-day unbonding (anchor-verify bnb.rs).
pub const MAX_BNB_START_AGE_SECS: u64 = 7 * 24 * 3600;
/// A shipped BNB start older than this gives way to a newer published one: a cold client walks
/// every epoch since its start.
pub const BNB_REFRESH_SECS: u64 = 24 * 3600;
const DAY: u64 = 24 * 3600;
const GENESIS_TIME: u64 = 1_606_824_023;
const SEPOLIA_GENESIS_TIME: u64 = 1_655_733_600;
const FETCH_TIMEOUT: Duration = Duration::from_secs(30);

/// A finalized beacon block root and its slot: where a sync-committee light client starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    pub root: String,
    pub slot: u64,
}

/// BNB Smart Chain's light client starts at an epoch block; `time` is its timestamp (seconds).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BnbStart {
    pub number: u64,
    pub hash: String,
    pub time: u64,
}

/// Every light client's start.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Starts {
    pub ethereum: Checkpoint,
    pub sepolia: Checkpoint,
    pub bnb: BnbStart,
}

/// The Unix time of an Ethereum slot.
pub fn slot_time(slot: u64) -> u64 {
    GENESIS_TIME + 12 * slot
}

fn sepolia_slot_time(slot: u64) -> u64 {
    SEPOLIA_GENESIS_TIME + 12 * slot
}

fn checkpoint_of(v: &Value) -> Option<Checkpoint> {
    let root = v.get("root")?.as_str().filter(|r| is_hash_str(r))?;
    let slot = v
        .get("slot")?
        .as_u64()
        .filter(|s| *s > 0 && *s < (1 << 53))?;
    Some(Checkpoint {
        root: root.to_ascii_lowercase(),
        slot,
    })
}

fn bnb_of(v: &Value) -> Option<BnbStart> {
    let hash = v.get("hash")?.as_str().filter(|h| is_hash_str(h))?;
    let number = v
        .get("number")?
        .as_u64()
        .filter(|n| n % 1000 == 0 && *n < (1 << 53))?;
    let time = v.get("time")?.as_u64()?;
    Some(BnbStart {
        number,
        hash: hash.to_ascii_lowercase(),
        time,
    })
}

/// The starts this release ships.
pub fn shipped() -> Starts {
    let v: Value = serde_json::from_str(SHIPPED_JSON).expect("checkpoint.json is JSON");
    Starts {
        ethereum: checkpoint_of(&v).expect("checkpoint.json has root and slot"),
        sepolia: checkpoint_of(&v["sepolia"]).expect("checkpoint.json has sepolia"),
        bnb: bnb_of(&v["bnb"]).expect("checkpoint.json has bnb"),
    }
}

/// When the shipped Ethereum checkpoint stops being used by itself (a day before the verifier
/// refuses it; past this, a release needs Willow's published checkpoint.json or a newer release).
pub fn shipped_checkpoint_expires() -> u64 {
    slot_time(shipped().ethereum.slot) + MAX_START_AGE_SECS - DAY
}

pub(crate) struct Choice<'a> {
    pub http: &'a reqwest::Client,
    pub checkpoint: Option<&'a Checkpoint>,
    pub sepolia: Option<&'a Checkpoint>,
    pub bnb: Option<&'a BnbStart>,
    pub checkpoint_url: &'a str,
    pub beacon_apis: &'a [String],
    pub now_sec: u64,
}

async fn get_json(http: &reqwest::Client, url: &str) -> Result<Value, String> {
    let r = http
        .get(url)
        .header("accept", "application/json")
        .timeout(FETCH_TIMEOUT)
        .send()
        .await
        .map_err(|e| e.without_url().to_string())?;
    if !r.status().is_success() {
        return Err(format!("HTTP {}", r.status().as_u16()));
    }
    r.json().await.map_err(|e| e.without_url().to_string())
}

/// Willow's published starts, fetched at most once.
async fn fetch(
    o: &Choice<'_>,
    published: &mut Option<Result<Value, String>>,
) -> Result<Value, String> {
    if published.is_none() {
        *published = Some(get_json(o.http, o.checkpoint_url).await);
    }
    published.clone().expect("set")
}

fn stale(why: String) -> Error {
    Error::unverifiable("anchor-stale-checkpoint", why, None)
}

/// Every light client's start, by the rules in the module docs.
pub(crate) async fn choose(o: &Choice<'_>) -> Result<Starts, Error> {
    let shipped = shipped();
    let mut published: Option<Result<Value, String>> = None;

    // Ethereum's: the one passed in; else the shipped one with a day's margin; else
    // checkpoint.json's, confirmed by the beacon APIs (one confirms, none disagrees).
    let ethereum = match o.checkpoint {
        Some(c) => c.clone(),
        None if o.now_sec.saturating_sub(slot_time(shipped.ethereum.slot)) + DAY
            < MAX_START_AGE_SECS =>
        {
            shipped.ethereum.clone()
        }
        None => {
            let c = fetch(o, &mut published).await.map_err(|e| {
                stale(format!(
                    "the shipped checkpoint is old and {} could not be read: {e}",
                    o.checkpoint_url
                ))
            })?;
            let c = checkpoint_of(&c)
                .filter(|c| c.slot > shipped.ethereum.slot)
                .ok_or_else(|| stale("checkpoint.json holds no newer checkpoint".into()))?;
            confirm(o, &c).await?;
            c
        }
    };

    // The others: passed in; else shipped; else, once the shipped one is too old (a day's
    // margin; BNB's after a day), checkpoint.json's when it holds a newer one. A start that stays
    // too old only fails the chains it anchors.
    let sepolia_old = |c: &Checkpoint| {
        o.now_sec.saturating_sub(sepolia_slot_time(c.slot)) + DAY >= MAX_START_AGE_SECS
    };
    let bnb_old = |b: &BnbStart| o.now_sec.saturating_sub(b.time) >= BNB_REFRESH_SECS;
    let mut sepolia = o.sepolia.cloned().unwrap_or(shipped.sepolia.clone());
    let mut bnb = o.bnb.cloned().unwrap_or(shipped.bnb.clone());
    if (o.sepolia.is_none() && sepolia_old(&sepolia)) || (o.bnb.is_none() && bnb_old(&bnb)) {
        if let Ok(c) = fetch(o, &mut published).await {
            if o.sepolia.is_none() {
                if let Some(s) = checkpoint_of(&c["sepolia"]).filter(|s| s.slot > sepolia.slot) {
                    sepolia = s;
                }
            }
            if o.bnb.is_none() {
                if let Some(b) = bnb_of(&c["bnb"]).filter(|b| b.number > bnb.number) {
                    bnb = b;
                }
            }
        }
    }
    Ok(Starts {
        ethereum,
        sepolia,
        bnb,
    })
}

/// A fetched checkpoint confirmed by the beacon APIs: at least one serves the same root at its
/// slot, and none another. An API that can't be asked (unreachable, an HTTP error) is skipped.
async fn confirm(o: &Choice<'_>, c: &Checkpoint) -> Result<(), Error> {
    if o.beacon_apis.is_empty() {
        return Ok(());
    }
    let mut confirmed = 0;
    for api in o.beacon_apis {
        let url = format!(
            "{}/eth/v1/beacon/headers/{}",
            api.trim_end_matches('/'),
            c.slot
        );
        let Ok(j) = get_json(o.http, &url).await else {
            continue;
        };
        let root = if j["data"]["canonical"] == Value::Bool(false) {
            None
        } else {
            j["data"]["root"].as_str()
        };
        if !root.is_some_and(|r| r.eq_ignore_ascii_case(&c.root)) {
            return Err(stale(format!(
                "{api} does not confirm checkpoint {} at slot {}",
                c.root, c.slot
            )));
        }
        confirmed += 1;
    }
    if confirmed == 0 {
        return Err(stale(
            "no beacon API could confirm the fetched checkpoint".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_starts_read() {
        let s = shipped();
        assert!(s.ethereum.slot > 15_000_000);
        assert!(s.sepolia.slot > 11_000_000);
        assert_eq!(s.bnb.number % 1000, 0);
        assert_eq!(
            shipped_checkpoint_expires(),
            slot_time(s.ethereum.slot) + 29 * DAY
        );
    }
}
