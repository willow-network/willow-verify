//! The validator snapshot after a block, and the checks a header must pass to extend it
//! (`Snapshot.apply`, `verifyUnsealedHeader`, `verifyCascadingFields`, `verifySeal`,
//! `verifyVoteAttestation` in bnb-chain/bsc `consensus/parlia`).

use crate::header::{EpochSet, Header, EMPTY_ROOT, EMPTY_UNCLE_HASH, EXTRA_SEAL, EXTRA_VANITY};
use crate::spec::{self, Spec};
use crate::vote::{self, VoteData, MAX_EXTRA};
use crate::Error;
use alloy_primitives::{Address, B256, U256};
use std::collections::BTreeMap;

/// A header's time may run this far ahead of the clock (geth allows none; the clock may lag).
pub const MAX_FUTURE_SECS: u64 = 2;
/// Turn lengths an epoch block may name.
pub const MAX_TURN_LENGTH: u8 = 64;
/// secp256k1's group order over 2: a seal's `s` above it is a malleated copy.
const HALF_ORDER: U256 = U256::from_limbs([
    0xDFE92F46681B20A0,
    0x5D576E7357A4501D,
    0xFFFFFFFFFFFFFFFF,
    0x7FFFFFFFFFFFFFFF,
]);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validator {
    pub address: Address,
    /// BLS12-381 public key (zero before Luban).
    pub vote_key: [u8; 48],
}

/// The state after block `number`: who may seal the next block, and what was voted final.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub number: u64,
    pub hash: B256,
    /// Block `number`'s time in seconds and milliseconds, and gas limit (the next header's bounds).
    pub time: u64,
    pub milli_time: u64,
    pub gas_limit: u64,
    pub epoch_length: u64,
    pub block_interval_ms: u64,
    pub turn_length: u8,
    /// Sorted by address.
    pub validators: Vec<Validator>,
    /// Block → its sealer; `u64::MAX - n / epoch` → zero marks epoch `n / epoch`'s switch done.
    pub recents: BTreeMap<u64, Address>,
    /// The latest attestation's vote: its target is justified, its source finalized.
    pub attestation: Option<VoteData>,
    /// The set the latest epoch block named, until it takes over: (epoch block, set).
    pub next: Option<(u64, EpochSet)>,
}

/// What checking one header found besides the new snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Vote {
    /// The header carries no attestation.
    None,
    /// 2/3+ of the target's validators signed it (`voters` of `of`).
    Verified { data: VoteData, voters: usize, of: usize },
    /// Not checkable here: the target's validator set or ancestry is older than the snapshots held.
    Unchecked(String),
    /// Invalid, before Plato, when the chain ignored bad attestations.
    Ignored(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub snapshot: Snapshot,
    pub signer: Address,
    pub in_turn: bool,
    pub vote: Vote,
}

fn sorted(set: &EpochSet) -> Result<Vec<Validator>, Error> {
    let mut v: Vec<Validator> = set
        .validators
        .iter()
        .map(|(a, k)| Validator {
            address: *a,
            vote_key: *k,
        })
        .collect();
    v.sort_by(|a, b| a.address.cmp(&b.address));
    if v.windows(2).any(|w| w[0].address == w[1].address) {
        return Err(Error::BadExtra("an epoch block names a validator twice".into()));
    }
    if v.is_empty() || v.len() > 64 {
        return Err(Error::BadExtra(format!(
            "an epoch block names {} validators",
            v.len()
        )));
    }
    Ok(v)
}

fn turn(tl: u8) -> Result<u8, Error> {
    if tl == 0 || tl > MAX_TURN_LENGTH {
        return Err(Error::BadExtra(format!("turn length {tl}")));
    }
    Ok(tl)
}

fn interval_ms(spec: &Spec, time: u64, current: u64) -> u64 {
    if spec.is_fermi(time) {
        spec::FERMI_INTERVAL_MS
    } else if spec.is_maxwell(time) {
        spec::MAXWELL_INTERVAL_MS
    } else if spec.is_lorentz(time) {
        spec::LORENTZ_INTERVAL_MS
    } else {
        current
    }
}

impl Snapshot {
    /// The snapshot at a pinned `checkpoint`, as geth builds a trusted one: the set the epoch block
    /// `epoch` below it names, already in force. The caller has linked `checkpoint` (whose hash it
    /// pins) back to `epoch` by parent hashes. No recent sealers (a lenient start) and no vote yet:
    /// the first verified attestation with a source directly below its target sets the votes.
    pub fn trusted(
        spec: &Spec,
        epoch: &Header,
        checkpoint: &Header,
        checkpoint_hash: B256,
        epoch_length: u64,
    ) -> Result<Snapshot, Error> {
        if epoch_length == 0 || epoch.number % epoch_length != 0 {
            return Err(Error::Checkpoint(format!(
                "block {} is not an epoch block",
                epoch.number
            )));
        }
        let set = epoch.epoch_set(spec, epoch_length)?;
        let mut s = Snapshot {
            number: checkpoint.number,
            hash: checkpoint_hash,
            time: checkpoint.time,
            milli_time: checkpoint.milli_time(),
            gas_limit: checkpoint.gas_limit,
            epoch_length,
            block_interval_ms: interval_ms(spec, checkpoint.time, spec::DEFAULT_INTERVAL_MS),
            turn_length: turn(set.turn_length.unwrap_or(1))?,
            validators: sorted(&set)?,
            recents: BTreeMap::new(),
            attestation: None,
            next: None,
        };
        let after = checkpoint.number.checked_sub(epoch.number).unwrap_or(0);
        if after <= s.check_len() || after >= epoch_length {
            return Err(Error::Checkpoint(format!(
                "checkpoint {} is not past epoch {}'s switch ({} blocks) within its epoch",
                checkpoint.number,
                epoch.number,
                s.check_len()
            )));
        }
        let signer = checkpoint.signer(spec.chain_id)?;
        if !s.has(&signer) {
            return Err(Error::Checkpoint(format!(
                "checkpoint {} is sealed by {signer}, not in epoch {}'s set",
                checkpoint.number, epoch.number
            )));
        }
        if spec.is_bohr(checkpoint.time) {
            s.recents
                .insert(u64::MAX - checkpoint.number / epoch_length, Address::ZERO);
        }
        Ok(s)
    }

    pub fn has(&self, a: &Address) -> bool {
        self.validators
            .binary_search_by(|v| v.address.cmp(a))
            .is_ok()
    }

    /// `minerHistoryCheckLen`: blocks over which each validator seals at most a turn.
    pub fn check_len(&self) -> u64 {
        (self.validators.len() as u64 / 2 + 1) * u64::from(self.turn_length) - 1
    }

    /// The validator whose turn block `number + 1` is.
    pub fn in_turn(&self) -> Address {
        let i = (self.number + 1) / u64::from(self.turn_length) % self.validators.len() as u64;
        self.validators[i as usize].address
    }

    /// Blocks `validator` sealed in the window the next block is checked against.
    fn sealed_recently(&self, validator: &Address) -> u64 {
        let cl = self.check_len();
        let left = if self.number > cl { self.number - cl } else { 0 };
        self.recents
            .iter()
            .filter(|(n, a)| **n > left && **a != Address::ZERO && *a == validator)
            .count() as u64
    }

    /// The highest justified block (the latest vote's target).
    pub fn justified(&self) -> Option<(u64, B256)> {
        self.attestation.map(|a| (a.target_number, a.target_hash))
    }

    /// The highest finalized block (the latest vote's source).
    pub fn finalized(&self) -> Option<(u64, B256)> {
        self.attestation.map(|a| (a.source_number, a.source_hash))
    }
}

fn find(ancestors: &[Snapshot], number: u64) -> Option<&Snapshot> {
    ancestors.iter().rev().find(|s| s.number == number)
}

/// Check `header` (hashing to `hash`) as the block after the last of `ancestors` (consecutive
/// snapshots, oldest first; the four before the parent let its votes be checked), with the clock
/// at `now` seconds if given, and return the snapshot after it.
pub fn verify(
    spec: &Spec,
    ancestors: &[Snapshot],
    header: &Header,
    hash: B256,
    now: Option<u64>,
) -> Result<Checked, Error> {
    let parent = ancestors
        .last()
        .ok_or_else(|| Error::NotChild("no parent snapshot".into()))?;
    let n = header.number;
    if n != parent.number + 1 || header.parent_hash != parent.hash {
        return Err(Error::NotChild(format!(
            "block {n} (parent {}) does not extend block {} ({})",
            header.parent_hash, parent.number, parent.hash
        )));
    }
    if let Some(now) = now {
        if header.time > now + MAX_FUTURE_SECS {
            return Err(Error::Field(format!(
                "block {n} is from the future ({} > {now})",
                header.time
            )));
        }
    }
    unsealed(spec, parent, header)?;
    cascading(spec, parent, header)?;

    // The seal.
    let signer = header.signer(spec.chain_id)?;
    let seal = header.seal().expect("checked above");
    if U256::from_be_slice(&seal[32..64]) > HALF_ORDER {
        return Err(Error::BadSeal(format!("block {n}: seal s is not low")));
    }
    if signer != header.coinbase {
        return Err(Error::CoinbaseMismatch(format!(
            "block {n}: sealed by {signer}, miner {}",
            header.coinbase
        )));
    }
    if !parent.has(&signer) {
        return Err(Error::Unauthorized(format!(
            "block {n}: {signer} is not a validator"
        )));
    }
    let in_turn = parent.in_turn() == signer;
    let want = U256::from(if in_turn { 2u8 } else { 1u8 });
    if header.difficulty != want {
        return Err(Error::WrongDifficulty(format!(
            "block {n}: difficulty {} for a{} validator",
            header.difficulty,
            if in_turn { "n in-turn" } else { "n out-of-turn" }
        )));
    }

    let vote = attestation(spec, ancestors, header)?;
    let snapshot = apply(spec, parent, header, hash, signer, &vote)?;
    Ok(Checked {
        snapshot,
        signer,
        in_turn,
        vote,
    })
}

/// `VerifyUnsealedHeader`: the fields that need no other header.
fn unsealed(spec: &Spec, parent: &Snapshot, h: &Header) -> Result<(), Error> {
    let n = h.number;
    let bad = |m: String| Err(Error::Field(format!("block {n}: {m}")));
    if h.extra.len() < EXTRA_VANITY + EXTRA_SEAL {
        return Err(Error::BadExtra(format!(
            "block {n}: extraData is {} bytes",
            h.extra.len()
        )));
    }
    let epoch = parent.epoch_length;
    let names = !h.validator_bytes(spec, epoch).is_empty();
    match (n % epoch == 0, names) {
        (false, true) => {
            return Err(Error::BadExtra(format!(
                "block {n} is not an epoch block but names validators"
            )))
        }
        (true, false) => {
            return Err(Error::BadExtra(format!(
                "epoch block {n} names no validators"
            )))
        }
        _ => {}
    }
    if !spec.is_lorentz(h.time) {
        if h.mix_digest != B256::ZERO {
            return bad("mixHash is not zero before Lorentz".into());
        }
    } else if h.mix_digest[..24] != [0u8; 24] || h.millis() >= 1000 {
        return bad(format!("mixHash {} is not milliseconds", h.mix_digest));
    }
    if h.uncle_hash != EMPTY_UNCLE_HASH {
        return bad("uncles".into());
    }
    match (spec.is_bohr(h.time), h.parent_beacon_root) {
        (false, None) | (true, Some(B256::ZERO)) => {}
        (b, r) => return bad(format!("parentBeaconBlockRoot {r:?} (Bohr: {b})")),
    }
    if spec.is_prague(h.time) != h.requests_hash.is_some() {
        return bad("requestsHash present across Prague".into());
    }
    if spec.is_london(n) != h.base_fee.is_some() {
        return bad("baseFeePerGas present across London".into());
    }
    if spec.is_cancun(h.time) {
        if h.withdrawals_hash != Some(EMPTY_ROOT)
            || h.blob_gas_used.is_none()
            || h.excess_blob_gas.is_none()
        {
            return bad("Cancun fields".into());
        }
    } else if h.withdrawals_hash.is_some() || h.blob_gas_used.is_some() {
        return bad("Cancun fields before Cancun".into());
    }
    if spec.is_amsterdam(h.time) {
        return bad("Amsterdam is not supported".into());
    }
    Ok(())
}

/// `verifyCascadingFields` without the state-dependent rules (base fee, blob gas, the back-off
/// schedule beyond its floor of one block interval).
fn cascading(spec: &Spec, parent: &Snapshot, h: &Header) -> Result<(), Error> {
    let n = h.number;
    let bad = |m: String| Err(Error::Field(format!("block {n}: {m}")));
    if h.milli_time() < parent.milli_time + parent.block_interval_ms {
        return bad(format!(
            "time {} ms is under {} ms after its parent",
            h.milli_time(),
            parent.block_interval_ms
        ));
    }
    if h.gas_limit > 0x7fff_ffff_ffff_ffff || h.gas_used > h.gas_limit {
        return bad(format!("gas {} of {}", h.gas_used, h.gas_limit));
    }
    let diff = h.gas_limit.abs_diff(parent.gas_limit);
    let limit = parent.gas_limit / spec.gas_limit_divisor(h.time);
    if diff >= limit || h.gas_limit < 5000 {
        return bad(format!(
            "gas limit {} from {}",
            h.gas_limit, parent.gas_limit
        ));
    }
    Ok(())
}

/// `verifyVoteAttestation`.
fn attestation(spec: &Spec, ancestors: &[Snapshot], h: &Header) -> Result<Vote, Error> {
    let parent = ancestors.last().expect("checked by the caller");
    let a = match h.attestation(spec, parent.epoch_length) {
        Ok(None) => return Ok(Vote::None),
        Ok(Some(a)) => a,
        Err(e) if !spec.is_plato(h.number) => return Ok(Vote::Ignored(e.to_string())),
        Err(e) => return Err(e),
    };
    let checked = (|| -> Result<Vote, String> {
        let d = a.data;
        if a.extra.len() > MAX_EXTRA {
            return Err(format!("extra is {} bytes", a.extra.len()));
        }
        if d.source_number >= d.target_number {
            return Err(format!(
                "source {} not below target {}",
                d.source_number, d.target_number
            ));
        }
        if let Some(j) = parent.justified() {
            if (d.source_number, d.source_hash) != j {
                return Err(format!(
                    "source {} {} is not the justified block {} {}",
                    d.source_number, d.source_hash, j.0, j.1
                ));
            }
        }
        let depth = spec.vote_depth(h.time);
        let lowest = h.number.saturating_sub(depth);
        if d.target_number >= h.number || d.target_number < lowest {
            return Err(format!(
                "target {} is not one of the {depth} blocks before",
                d.target_number
            ));
        }
        let Some(target) = find(ancestors, d.target_number) else {
            return Ok(Vote::Unchecked(format!(
                "target {} older than the snapshots held",
                d.target_number
            )));
        };
        if target.hash != d.target_hash {
            return Err(format!(
                "target {} {} is not an ancestor",
                d.target_number, d.target_hash
            ));
        }
        let Some(set) = find(ancestors, d.target_number - 1) else {
            return Ok(Vote::Unchecked(format!(
                "the validators of target {} are older than the snapshots held",
                d.target_number
            )));
        };
        let of = set.validators.len();
        if of < 64 && a.voters >> of != 0 {
            return Err(format!("voters {:#x} past {of} validators", a.voters));
        }
        let keys: Vec<[u8; 48]> = set
            .validators
            .iter()
            .enumerate()
            .filter(|(i, _)| a.voters & (1u64 << i) != 0)
            .map(|(_, v)| v.vote_key)
            .collect();
        if keys.len() * 3 < of * 2 {
            return Err(format!("{} of {of} validators voted", keys.len()));
        }
        if !vote::fast_aggregate_verify(&keys, &a.signature, &d.hash()) {
            return Err("aggregate signature does not verify".into());
        }
        Ok(Vote::Verified {
            data: d,
            voters: keys.len(),
            of,
        })
    })();
    match checked {
        Ok(v) => Ok(v),
        Err(e) if !spec.is_plato(h.number) => Ok(Vote::Ignored(e)),
        Err(e) => Err(Error::Attestation(format!("block {}: {e}", h.number))),
    }
}

/// `Snapshot.apply` for one checked header.
fn apply(
    spec: &Spec,
    parent: &Snapshot,
    h: &Header,
    hash: B256,
    signer: Address,
    vote: &Vote,
) -> Result<Snapshot, Error> {
    let n = h.number;
    let mut s = parent.clone();
    let limit = s.check_len() + 1;
    if n >= limit {
        s.recents.remove(&(n - limit));
    }
    if spec.is_bohr(h.time) {
        if s.sealed_recently(&signer) >= u64::from(s.turn_length) {
            return Err(Error::RecentlySigned(format!(
                "block {n}: {signer} sealed its turn already"
            )));
        }
    } else if s.recents.values().any(|a| *a == signer) {
        return Err(Error::RecentlySigned(format!(
            "block {n}: {signer} sealed recently"
        )));
    }

    // `updateAttestation`; a vote that could not be checked here moves nothing.
    let data = match vote {
        Vote::Verified { data, .. } => Some(*data),
        Vote::Ignored(_) => h
            .attestation(spec, parent.epoch_length)
            .ok()
            .flatten()
            .map(|a| a.data),
        Vote::None | Vote::Unchecked(_) => None,
    };
    if let Some(d) = data {
        let to_parent = d.target_hash == h.parent_hash && d.target_number + 1 == n;
        if spec.is_fermi(h.time) || to_parent {
            let finalizes = d.source_number + 1 == d.target_number;
            match s.attestation {
                Some(ref mut cur) if !finalizes => {
                    cur.target_number = d.target_number;
                    cur.target_hash = d.target_hash;
                }
                Some(ref mut cur) => *cur = d,
                // The first vote after a checkpoint: only one that finalizes its source.
                None if finalizes => s.attestation = Some(d),
                None => {}
            }
        }
    }

    s.recents.insert(n, signer);
    if spec.is_maxwell(h.time) {
        if let Some((f, _)) = s.finalized() {
            let keep = s.recents.split_off(&(f + 1));
            s.recents = keep;
        }
    }
    s.block_interval_ms = interval_ms(spec, h.time, s.block_interval_ms);

    let epoch = s.epoch_length;
    if s.epoch_length == spec::DEFAULT_EPOCH
        && spec.is_lorentz(h.time)
        && (n + 1) % spec::LORENTZ_EPOCH == 0
    {
        s.epoch_length = spec::LORENTZ_EPOCH;
    }
    if s.epoch_length == spec::LORENTZ_EPOCH
        && spec.is_maxwell(h.time)
        && (n + 1) % spec::MAXWELL_EPOCH == 0
    {
        s.epoch_length = spec::MAXWELL_EPOCH;
    }
    if n % epoch == 0 {
        s.next = Some((n, h.epoch_set(spec, epoch)?));
    }

    let cl = s.check_len();
    if n > 0 && n % epoch == cl {
        let key = u64::MAX - n / epoch;
        let skip = spec.is_bohr(h.time) && s.recents.contains_key(&key);
        if !skip {
            let set = match s.next.take() {
                Some((at, set)) if at == n - cl => set,
                other => {
                    return Err(Error::Checkpoint(format!(
                        "block {n} switches validators to epoch block {}'s, which is not held ({:?})",
                        n - cl,
                        other.map(|o| o.0)
                    )))
                }
            };
            if let Some(tl) = set.turn_length {
                s.turn_length = turn(tl)?;
            }
            let validators = sorted(&set)?;
            if spec.is_bohr(h.time) {
                s.recents = BTreeMap::from([(key, Address::ZERO)]);
            } else {
                let old = s.validators.len() as u64 / 2 + 1;
                let new = validators.len() as u64 / 2 + 1;
                for i in 0..old.saturating_sub(new) {
                    s.recents.remove(&(n - new - i));
                }
            }
            s.validators = validators;
        }
    }

    s.number = n;
    s.hash = hash;
    s.time = h.time;
    s.milli_time = h.milli_time();
    s.gas_limit = h.gas_limit;
    Ok(s)
}

fn hex_of(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

fn json_set(v: &[(Address, [u8; 48])]) -> serde_json::Value {
    serde_json::Value::Array(
        v.iter()
            .map(|(a, k)| serde_json::json!({"address": a.to_string(), "voteKey": hex_of(k)}))
            .collect(),
    )
}

fn read_set(v: &serde_json::Value) -> Option<Vec<(Address, [u8; 48])>> {
    v.as_array()?
        .iter()
        .map(|x| {
            let a: Address = x.get("address")?.as_str()?.parse().ok()?;
            let k = hex::decode(x.get("voteKey")?.as_str()?.strip_prefix("0x")?).ok()?;
            Some((a, k.try_into().ok()?))
        })
        .collect()
}

fn vote_json(d: &VoteData) -> serde_json::Value {
    serde_json::json!({
        "sourceNumber": d.source_number, "sourceHash": d.source_hash.to_string(),
        "targetNumber": d.target_number, "targetHash": d.target_hash.to_string(),
    })
}

impl Snapshot {
    /// A self-contained record of the snapshot, read back by [`Snapshot::from_json`].
    pub fn to_json(&self) -> serde_json::Value {
        let set: Vec<(Address, [u8; 48])> = self
            .validators
            .iter()
            .map(|v| (v.address, v.vote_key))
            .collect();
        serde_json::json!({
            "number": self.number,
            "hash": self.hash.to_string(),
            "time": self.time,
            "milliTime": self.milli_time,
            "gasLimit": self.gas_limit,
            "epochLength": self.epoch_length,
            "blockIntervalMs": self.block_interval_ms,
            "turnLength": self.turn_length,
            "validators": json_set(&set),
            "recents": self.recents.iter().map(|(n, a)| serde_json::json!([n, a.to_string()])).collect::<Vec<_>>(),
            "attestation": self.attestation.as_ref().map(vote_json),
            "next": self.next.as_ref().map(|(at, s)| serde_json::json!({
                "epochBlock": at, "validators": json_set(&s.validators), "turnLength": s.turn_length,
            })),
        })
    }

    pub fn from_json(v: &serde_json::Value) -> Result<Snapshot, Error> {
        let bad = || Error::Malformed(format!("snapshot record {v}"));
        let u = |k: &str| v.get(k).and_then(serde_json::Value::as_u64).ok_or_else(bad);
        let b = |x: &serde_json::Value| -> Option<B256> { x.as_str()?.parse().ok() };
        let validators = read_set(v.get("validators").ok_or_else(bad)?)
            .ok_or_else(bad)?
            .into_iter()
            .map(|(address, vote_key)| Validator { address, vote_key })
            .collect::<Vec<_>>();
        if validators.is_empty() || validators.windows(2).any(|w| w[0].address >= w[1].address) {
            return Err(bad());
        }
        let recents = v
            .get("recents")
            .and_then(serde_json::Value::as_array)
            .ok_or_else(bad)?
            .iter()
            .map(|e| Some((e.get(0)?.as_u64()?, e.get(1)?.as_str()?.parse().ok()?)))
            .collect::<Option<BTreeMap<u64, Address>>>()
            .ok_or_else(bad)?;
        let attestation = match v.get("attestation") {
            None | Some(serde_json::Value::Null) => None,
            Some(a) => Some(
                (|| {
                    Some(VoteData {
                        source_number: a.get("sourceNumber")?.as_u64()?,
                        source_hash: b(a.get("sourceHash")?)?,
                        target_number: a.get("targetNumber")?.as_u64()?,
                        target_hash: b(a.get("targetHash")?)?,
                    })
                })()
                .ok_or_else(bad)?,
            ),
        };
        let next = match v.get("next") {
            None | Some(serde_json::Value::Null) => None,
            Some(n) => Some(
                (|| {
                    let tl = match n.get("turnLength")? {
                        serde_json::Value::Null => None,
                        t => Some(u8::try_from(t.as_u64()?).ok()?),
                    };
                    Some((
                        n.get("epochBlock")?.as_u64()?,
                        EpochSet {
                            validators: read_set(n.get("validators")?)?,
                            turn_length: tl,
                        },
                    ))
                })()
                .ok_or_else(bad)?,
            ),
        };
        let s = Snapshot {
            number: u("number")?,
            hash: v.get("hash").and_then(b).ok_or_else(bad)?,
            time: u("time")?,
            milli_time: u("milliTime")?,
            gas_limit: u("gasLimit")?,
            epoch_length: u("epochLength")?,
            block_interval_ms: u("blockIntervalMs")?,
            turn_length: turn(u8::try_from(u("turnLength")?).map_err(|_| bad())?)?,
            validators,
            recents,
            attestation,
            next,
        };
        if s.epoch_length == 0 {
            return Err(bad());
        }
        Ok(s)
    }
}
