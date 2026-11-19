











mod rules;

use crate::bls::{aggregate_verify_points, public_key};
use crate::lc::bytes_of;
use crate::{Failure, Kind};
use alloy_primitives::{Address, B256, U256};
use bls12_381::G1Affine;
use parlia_verify::header::Header;
use parlia_verify::spec::{MAINNET as SPEC, MAXWELL_EPOCH};
use parlia_verify::vote::{Attestation, MAX_EXTRA};
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub const CHAIN_ID: u64 = parlia_verify::spec::BSC_MAINNET;

pub const EPOCH: u64 = MAXWELL_EPOCH;


pub const START_OFFSET: u64 = 200;

pub const MAX_SIGNED_AGE_SECS: u64 = 600;


pub const MAX_START_AGE_SECS: u64 = 7 * 86_400;

pub const MAX_UPDATES: usize = 64;

pub const MAX_LINK: usize = 64;

const HALF_ORDER: U256 = U256::from_limbs([
    0xDFE92F46681B20A0,
    0x5D576E7357A4501D,
    0xFFFFFFFFFFFFFFFF,
    0x7FFFFFFFFFFFFFFF,
]);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start {
    pub number: u64,
    pub hash: [u8; 32],
}



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


#[derive(Clone, Debug)]
struct Era {

    epoch: u64,
    from: u64,

    validators: Vec<Address>,
    keys: Vec<G1Affine>,
    raw: Vec<[u8; 48]>,
    turn_length: u64,
}

impl Era {

    fn check_len(&self) -> u64 {
        rules::check_len(self.validators.len(), self.turn_length)
    }


    fn until(&self) -> u64 {
        self.epoch + EPOCH + self.check_len()
    }



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


fn header(v: &Value, what: &str) -> Result<(Header, B256), Failure> {
    let s = v
        .as_str()
        .ok_or_else(|| bad(format!("{what} is not hex")))?;
    Header::from_rlp(&bytes_of(s, what)?).map_err(|e| bad(format!("{what}: {e}")))
}


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







#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BnbConfirmed {

    pub level: &'static str,
    pub hash: [u8; 32],

    pub anchor_block: u64,

    pub time: u64,

    pub signers: usize,
    pub of: usize,
}


#[derive(Clone)]
pub struct Bnb {
    pub start: Start,

    start_time: Option<u64>,


    ages: bool,
    eras: Vec<Era>,


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


    pub fn confirm(&self, number: u64, answer: &Value, now: u64) -> Result<BnbConfirmed, Failure> {
        self.confirm_header(number, answer, now).map(|(c, _)| c)
    }



    pub fn confirm_header(
        &self,
        number: u64,
        answer: &Value,
        now: u64,
    ) -> Result<(BnbConfirmed, Header), Failure> {
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
                Ok((
                    BnbConfirmed {
                        level: "final",
                        hash: hash.0,
                        anchor_block: headers.last().unwrap().0.number,
                        time: h.time,
                        signers,
                        of,
                    },
                    h.clone(),
                ))
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
                Ok((
                    BnbConfirmed {
                        level: "signed",
                        hash: hash.0,
                        anchor_block: number,
                        time: h.time,
                        signers: 1,
                        of: era.validators.len(),
                    },
                    h.clone(),
                ))
            }
        }
    }


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


pub fn execution(h: &Header) -> crate::lc::Execution {
    crate::lc::Execution {
        number: h.number,
        hash: h.hash().0,
        state_root: h.state_root.0,
        timestamp: h.time,
        gas_limit: h.gas_limit,
        base_fee: h.base_fee.unwrap_or_default(),
        prev_randao: h.mix_digest.0,
        fee_recipient: h.coinbase.0 .0,
    }
}


fn check_age(start_time: u64, now: u64) -> Result<(), Failure> {
    if now.saturating_sub(start_time) > MAX_START_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleCheckpoint,
            "this page's starting point on BNB is more than 7 days old",
        ));
    }
    Ok(())
}


fn lc() -> Value {
    json!({"lightClient": "bnb"})
}

pub fn start_json(s: &Start) -> Value {
    json!({"number": s.number, "hash": format!("{}", B256::from(s.hash))})
}


pub fn start_of(v: &Value) -> Option<Start> {
    let number = v.get("number")?.as_u64().filter(|n| n % EPOCH == 0)?;
    let hash: B256 = v.get("hash")?.as_str()?.parse().ok()?;
    Some(Start {
        number,
        hash: hash.0,
    })
}
