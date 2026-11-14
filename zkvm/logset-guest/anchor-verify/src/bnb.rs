//! BNB Smart Chain blocks against BNB's own validators: a light client of the validator set from a
//! pinned epoch block, each later set taken from an epoch block that 2/3+ of the set before it
//! voted final, read with `parlia-verify`'s header, seal and vote code.
//!
//! - Bootstrap: the pinned epoch block's header; its set seals from [`START_OFFSET`] blocks on.
//! - Update (one per epoch): the epoch block's header, linked by parent hashes to a block `F`, and
//!   the vote attestation whose source is `F` and target `F + 1` (BNB's rule for "F is
//!   finalized"), signed by 2/3+ of the set sealing `F + 1`. The set it names seals from
//!   `checkLen` blocks on, as `Snapshot.apply` switches it.
//! - Anchor: block `n`'s header linked to a finalized `F` the same way (`final`), or `n` alone,
//!   sealed by a validator of its set (`signed`, at most [`MAX_SIGNED_AGE_SECS`] old).

use crate::bls::{aggregate_verify_points, public_key};
use crate::lc::bytes_of;
use crate::{Failure, Kind};
use alloy_primitives::{Address, B256, U256};
use bls12_381::G1Affine;
use parlia_verify::header::{Header, EXTRA_SEAL, EXTRA_VANITY, VALIDATOR_BYTES};
use parlia_verify::spec::{MAINNET as SPEC, MAXWELL_EPOCH};
use parlia_verify::vote::{Attestation, MAX_EXTRA};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CHAIN_ID: u64 = parlia_verify::spec::BSC_MAINNET;
/// Blocks per epoch since Maxwell; every pinned start is later.
pub const EPOCH: u64 = MAXWELL_EPOCH;
/// A pinned epoch block's set seals from this many blocks on: past its switch, which comes
/// `checkLen` blocks in (87 with 21 validators taking turns of 8), as parlia-verify's checkpoint.
pub const START_OFFSET: u64 = 200;
/// A block one validator sealed and 2/3+ have not voted final in this long is not counted on.
pub const MAX_SIGNED_AGE_SECS: u64 = 600;
/// A start older than this is refused: past BNB's 7-day unbonding, the validators of its set can
/// have left with their stake, and could sign another history from it unpunished.
pub const MAX_START_AGE_SECS: u64 = 7 * 86_400;
/// Epoch updates applied from one response at most.
pub const MAX_UPDATES: usize = 64;
/// Headers linking a block to the finalized one that covers it, at most.
pub const MAX_LINK: usize = 64;
/// secp256k1's group order over 2: a seal's `s` above it is a malleated copy of another.
const HALF_ORDER: U256 = U256::from_limbs([
    0xDFE92F46681B20A0,
    0x5D576E7357A4501D,
    0xFFFFFFFFFFFFFFFF,
    0x7FFFFFFFFFFFFFFF,
]);

/// Where the light client starts: an epoch block and its hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start {
    pub number: u64,
    pub hash: [u8; 32],
}

/// Block 133,195,000 (2026-11-12), the epoch block under parlia-verify's pinned checkpoint, the
/// same hash at publicnode, bnbchain and defibit (2026-11-14).
pub const PINNED: Start = Start {
    number: 133_195_000,
    hash: alloy_primitives::b256!(
        "676b297413a5404e846b7261eaed415587b8dfdd43536f608bd912eaf53d37ca"
    )
    .0,
};

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

/// One validator set and the blocks it seals.
#[derive(Clone, Debug)]
struct Era {
    /// The epoch block that named it, and the first block it seals.
    epoch: u64,
    from: u64,
    /// Sorted by address; `keys[i]` is `validators[i]`'s vote key, checked, from bytes `raw[i]`.
    validators: Vec<Address>,
    keys: Vec<G1Affine>,
    raw: Vec<[u8; 48]>,
    turn_length: u64,
}

impl Era {
    /// `minerHistoryCheckLen`: how far into the next epoch this set still seals.
    fn check_len(&self) -> u64 {
        (self.validators.len() as u64 / 2 + 1) * self.turn_length - 1
    }

    /// The last block this set seals, unless an update says otherwise first.
    fn until(&self) -> u64 {
        self.epoch + EPOCH + self.check_len()
    }

    /// The set an epoch header names, sealing from `from`. A key `before` (the set it follows)
    /// already checked from the same bytes is not decompressed again: most epochs name the same set.
    fn of(h: &Header, from: u64, before: Option<&Era>) -> Result<Era, Failure> {
        if h.number % EPOCH != 0 {
            return Err(rejected(format!(
                "block {} is not an epoch block",
                h.number
            )));
        }
        let set = h
            .epoch_set(&SPEC, EPOCH)
            .map_err(|e| rejected(format!("epoch block {}: {e}", h.number)))?;
        let turn_length = set
            .turn_length
            .filter(|t| (1..=64).contains(t))
            .ok_or_else(|| rejected(format!("epoch block {} names no turn length", h.number)))?;
        let mut v = set.validators;
        v.sort_by(|a, b| a.0.cmp(&b.0));
        if v.is_empty() || v.len() > 64 || v.windows(2).any(|w| w[0].0 == w[1].0) {
            return Err(rejected(format!(
                "epoch block {} names {} validators, or one twice",
                h.number,
                v.len()
            )));
        }
        let checked = |k: &[u8; 48]| {
            let b = before?;
            b.raw.iter().position(|r| r == k).map(|i| b.keys[i])
        };
        let keys = v
            .iter()
            .map(|(a, k)| {
                checked(k)
                    .or_else(|| public_key(k))
                    .ok_or_else(|| rejected(format!("validator {a}'s vote key is not a key")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Era {
            epoch: h.number,
            from,
            raw: v.iter().map(|(_, k)| *k).collect(),
            validators: v.into_iter().map(|(a, _)| a).collect(),
            keys,
            turn_length: u64::from(turn_length),
        })
    }
}

/// A header from its RLP hex, and its hash.
fn header(v: &Value, what: &str) -> Result<(Header, B256), Failure> {
    let s = v
        .as_str()
        .ok_or_else(|| bad(format!("{what} is not hex")))?;
    Header::from_rlp(&bytes_of(s, what)?).map_err(|e| bad(format!("{what}: {e}")))
}

/// Headers `[a, a + 1, …]`, each the parent of the next.
fn linked(list: &Value) -> Result<Vec<(Header, B256)>, Failure> {
    let list = list
        .as_array()
        .filter(|l| !l.is_empty() && l.len() <= MAX_LINK)
        .ok_or_else(|| bad(format!("headers are not a list of 1 to {MAX_LINK}")))?;
    let headers = list
        .iter()
        .map(|h| header(h, "a header"))
        .collect::<Result<Vec<_>, _>>()?;
    for w in headers.windows(2) {
        if w[1].0.number != w[0].0.number + 1 || w[1].0.parent_hash != w[0].1 {
            return Err(rejected(format!(
                "block {} is not the parent of the next header",
                w[0].0.number
            )));
        }
    }
    Ok(headers)
}

/// The vote attestation a header carries, as bytes (`getVoteAttestationFromHeader`'s cut).
pub fn attestation_bytes(h: &Header) -> Option<&[u8]> {
    if h.extra.len() <= EXTRA_VANITY + EXTRA_SEAL {
        return None;
    }
    let end = h.extra.len() - EXTRA_SEAL;
    let start = if h.number % EPOCH != 0 {
        EXTRA_VANITY
    } else {
        let n = h.extra[EXTRA_VANITY] as usize;
        EXTRA_VANITY + 1 + n * VALIDATOR_BYTES + 1
    };
    (end > start).then(|| &h.extra[start..end])
}

/// What an anchor showed about block `number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BnbConfirmed {
    /// `final` (2/3+ of the validators voted a block at or after it final) or `signed`.
    pub level: &'static str,
    pub hash: [u8; 32],
    /// The finalized block the vote names, or the block itself when signed.
    pub anchor_block: u64,
    /// Block `number`'s own time.
    pub time: u64,
    /// Voters, or 1 when signed, of the set's size.
    pub signers: usize,
    pub of: usize,
}

/// BNB's validator sets from a pinned start, as verified.
#[derive(Clone)]
pub struct Bnb {
    pub start: Start,
    /// The start's own time, once bootstrapped.
    start_time: Option<u64>,
    /// Whether a start older than [`MAX_START_AGE_SECS`] is refused (not by a service that has
    /// followed the chain from it all along).
    ages: bool,
    eras: Vec<Era>,
    /// What the sets were verified from: the bootstrap and each epoch's update, as served, so a
    /// later visit verifies the same chain again instead of trusting its result.
    chain: Option<(Value, BTreeMap<u64, Value>)>,
}

impl Default for Bnb {
    fn default() -> Self {
        Bnb::new(PINNED)
    }
}

impl Bnb {
    pub fn new(start: Start) -> Bnb {
        Bnb {
            start,
            start_time: None,
            ages: true,
            eras: Vec::new(),
            chain: None,
        }
    }

    /// The same light client without the start's age limit: for a service that has checked the
    /// chain forward from its start since it began.
    pub fn without_age_limit(mut self) -> Bnb {
        self.ages = false;
        self
    }

    /// The newest epoch whose set is known, once bootstrapped.
    pub fn last_epoch(&self) -> Option<u64> {
        self.eras.last().map(|e| e.epoch / EPOCH)
    }

    /// The bootstrap's result `{header}`: the pinned epoch block, at most [`MAX_START_AGE_SECS`]
    /// old at `now`. Returns its epoch.
    pub fn bootstrap(&mut self, v: &Value, now: u64) -> Result<u64, Failure> {
        let (h, hash) = header(
            v.get("header")
                .ok_or_else(|| bad("the bootstrap has no header"))?,
            "the bootstrap",
        )?;
        if hash.0 != self.start.hash || h.number != self.start.number {
            return Err(rejected(
                "the bootstrap is not this page's starting block on BNB",
            ));
        }
        if self.ages {
            check_age(h.time, now)?;
        }
        let era = Era::of(&h, h.number + START_OFFSET, None)?;
        self.start_time = Some(h.time);
        self.eras = vec![era];
        self.chain = Some((v.clone(), BTreeMap::new()));
        Ok(h.number / EPOCH)
    }

    fn last(&self) -> Result<&Era, Failure> {
        self.eras
            .last()
            .ok_or_else(|| Failure::new(Kind::NeedBootstrap, "no validator set yet").with(lc()))
    }

    /// The set sealing block `b`, or what to fetch first.
    fn era_at(&self, b: u64) -> Result<&Era, Failure> {
        let last = self.last()?;
        if b < self.eras[0].from {
            return Err(Failure::new(
                Kind::Unreachable,
                format!("BNB block {b} is before this page's starting point"),
            ));
        }
        if b > last.until() {
            let have = last.epoch / EPOCH;
            let want = (b - last.check_len() - 1) / EPOCH;
            return Err(
                Failure::new(Kind::NeedUpdates, "BNB validator sets needed").with(
                    json!({"from": have + 1, "count": want.saturating_sub(have).max(1),
                       "lightClient": "bnb"}),
                ),
            );
        }
        Ok(self.eras.iter().rev().find(|e| e.from <= b).unwrap_or(last))
    }

    /// That `attestation` finalizes the last of `headers` (source it, target the next block) with
    /// the votes of 2/3+ of the set sealing that next block: voters and set size.
    fn finalizes(
        &self,
        headers: &[(Header, B256)],
        attestation: &Value,
    ) -> Result<(usize, usize), Failure> {
        let (f, fh) = headers.last().expect("a non-empty list");
        let bytes = bytes_of(
            attestation
                .as_str()
                .ok_or_else(|| bad("the attestation is not hex"))?,
            "the attestation",
        )?;
        let a = Attestation::decode(&bytes).map_err(|e| bad(format!("the attestation: {e}")))?;
        let d = a.data;
        if a.extra.len() > MAX_EXTRA {
            return Err(bad("the attestation's extra is too long"));
        }
        if d.source_number != f.number || d.source_hash != *fh || d.target_number != f.number + 1 {
            return Err(rejected(format!(
                "the vote ({} → {}) does not finalize block {}",
                d.source_number, d.target_number, f.number
            )));
        }
        let era = self.era_at(d.target_number)?;
        let of = era.validators.len();
        if of < 64 && a.voters >> of != 0 {
            return Err(rejected(format!(
                "voters {:#x} past {of} validators",
                a.voters
            )));
        }
        let keys: Vec<G1Affine> = (0..of)
            .filter(|i| a.voters & (1u64 << i) != 0)
            .map(|i| era.keys[i])
            .collect();
        if keys.len() * 3 < of * 2 {
            return Err(rejected(format!(
                "{} of {of} BNB validators voted, fewer than two thirds",
                keys.len()
            )));
        }
        aggregate_verify_points(&keys, &a.signature, &d.hash().0).map_err(|_| {
            rejected(format!(
                "the validators' vote finalizing BNB block {} does not verify",
                f.number
            ))
        })?;
        Ok((keys.len(), of))
    }

    /// One epoch's update `{epoch, headers, attestation}`. Returns the epoch learned, or `None`
    /// when it was known already.
    pub fn apply_update(&mut self, u: &Value) -> Result<Option<u64>, Failure> {
        let last = self.last()?.clone();
        let headers = linked(
            u.get("headers")
                .ok_or_else(|| bad("an update has no headers"))?,
        )?;
        let e = &headers[0].0;
        if e.number <= last.epoch {
            return Ok(None);
        }
        if e.number != last.epoch + EPOCH {
            return Err(Failure::new(
                Kind::Unreachable,
                format!(
                    "an update for epoch block {}, but the next is {}",
                    e.number,
                    last.epoch + EPOCH
                ),
            ));
        }
        // The vote's target is sealed by the set before the new one takes over.
        if headers.last().unwrap().0.number + 1 > last.until() {
            return Err(rejected(format!(
                "epoch block {}'s vote is past the old set's last block",
                e.number
            )));
        }
        self.finalizes(
            &headers,
            u.get("attestation")
                .ok_or_else(|| bad("an update has no attestation"))?,
        )?;
        let era = Era::of(e, e.number + last.check_len() + 1, Some(&last))?;
        let k = e.number / EPOCH;
        self.eras.push(era);
        if let Some((_, updates)) = self.chain.as_mut() {
            updates.insert(k, u.clone());
        }
        Ok(Some(k))
    }

    /// Block `number` of BNB from a `willow_getBlockAnchor` result `{blockNumber, parlia}`.
    pub fn confirm(&self, number: u64, answer: &Value, now: u64) -> Result<BnbConfirmed, Failure> {
        if let Some(t) = self.start_time.filter(|_| self.ages) {
            check_age(t, now)?;
        }
        let asked = answer
            .get("blockNumber")
            .and_then(Value::as_str)
            .and_then(|s| s.strip_prefix("0x"))
            .and_then(|h| u64::from_str_radix(h, 16).ok())
            .ok_or_else(|| bad("the anchor has no blockNumber"))?;
        if asked != number {
            return Err(rejected(format!(
                "the anchor is for block {asked}, not {number}"
            )));
        }
        let p = answer
            .get("parlia")
            .ok_or_else(|| bad("the anchor has no BNB headers"))?;
        let headers = linked(
            p.get("headers")
                .ok_or_else(|| bad("the anchor has no headers"))?,
        )?;
        let (h, hash) = &headers[0];
        if h.number != number {
            return Err(rejected(format!(
                "the anchor's first header is block {}, not {number}",
                h.number
            )));
        }
        match p.get("attestation") {
            Some(a) if !a.is_null() => {
                let (signers, of) = self.finalizes(&headers, a)?;
                Ok(BnbConfirmed {
                    level: "final",
                    hash: hash.0,
                    anchor_block: headers.last().unwrap().0.number,
                    time: h.time,
                    signers,
                    of,
                })
            }
            _ => {
                if headers.len() != 1 {
                    return Err(bad("a sealed block's anchor has links but no vote"));
                }
                let era = self.era_at(number)?;
                let seal = h.seal().ok_or_else(|| bad("the header holds no seal"))?;
                if U256::from_be_slice(&seal[32..64]) > HALF_ORDER {
                    return Err(rejected(format!(
                        "block {number}'s seal is a malleated copy"
                    )));
                }
                let signer = h
                    .signer(CHAIN_ID)
                    .map_err(|e| rejected(format!("block {number}: {e}")))?;
                if signer != h.coinbase || era.validators.binary_search(&signer).is_err() {
                    return Err(rejected(format!(
                        "block {number} is sealed by {signer}, not a validator of its set"
                    )));
                }
                if now.saturating_sub(h.time) > MAX_SIGNED_AGE_SECS {
                    return Err(Failure::new(
                        Kind::StaleHead,
                        format!(
                            "BNB block {number}, sealed by one validator and not voted final, is \
                             {} minutes old",
                            now.saturating_sub(h.time) / 60
                        ),
                    ));
                }
                Ok(BnbConfirmed {
                    level: "signed",
                    hash: hash.0,
                    anchor_block: number,
                    time: h.time,
                    signers: 1,
                    of: era.validators.len(),
                })
            }
        }
    }

    /// `{start, bootstrap, updates}` to keep for the next visit, or `null` before a bootstrap.
    pub fn saved(&self) -> Value {
        match &self.chain {
            Some((bootstrap, updates)) => json!({
                "start": start_json(&self.start),
                "bootstrap": bootstrap,
                "updates": updates.values().collect::<Vec<_>>(),
            }),
            None => Value::Null,
        }
    }

    /// The chain kept from an earlier visit, verified again from this start; kept for another
    /// start, it is ignored. Returns the newest epoch reached.
    pub fn restore(&mut self, v: &Value, now: u64) -> Result<Option<u64>, Failure> {
        if v.get("start") != Some(&start_json(&self.start)) {
            return Ok(None);
        }
        self.bootstrap(
            v.get("bootstrap").ok_or_else(|| bad("no kept bootstrap"))?,
            now,
        )?;
        let updates = v
            .get("updates")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("kept updates are not a list"))?;
        let mut newest = None;
        for u in updates {
            newest = self.apply_update(u)?.or(newest);
        }
        Ok(newest)
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.eras.clear();
        self.chain = None;
    }
}

/// Refuses a start made more than [`MAX_START_AGE_SECS`] before `now`.
fn check_age(start_time: u64, now: u64) -> Result<(), Failure> {
    if now.saturating_sub(start_time) > MAX_START_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleCheckpoint,
            "this page's starting point on BNB is more than 7 days old",
        ));
    }
    Ok(())
}

/// The `lightClient` field of a need-* failure for BNB.
fn lc() -> Value {
    json!({"lightClient": "bnb"})
}

pub fn start_json(s: &Start) -> Value {
    json!({"number": s.number, "hash": format!("{}", B256::from(s.hash))})
}

/// `{number, hash}`: an epoch block.
pub fn start_of(v: &Value) -> Option<Start> {
    let number = v.get("number")?.as_u64().filter(|n| n % EPOCH == 0)?;
    let hash: B256 = v.get("hash")?.as_str()?.parse().ok()?;
    Some(Start {
        number,
        hash: hash.0,
    })
}
