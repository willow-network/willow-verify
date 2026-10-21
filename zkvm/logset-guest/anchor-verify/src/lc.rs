//! Ethereum's light client, as the beacon API serves it: a bootstrap at the checkpoint the page
//! pins, one update per sync-committee period after it, and finality updates. Header and committee
//! roots, signing domains, gindices and the fork schedule are the SP1 anchor guest's (`anchor-core`).

use crate::bls::{fast_aggregate_verify, BlsFailure};
use crate::{Failure, Kind};
use alloy_primitives::U256;
use anchor_core::gindex::{finalized_root_gindex, next_sync_committee_gindex};
use anchor_core::header::{
    beacon_block_header_root, bit_set, compute_domain, compute_signing_root,
    sync_committee_hash_tree_root, DOMAIN_SYNC_COMMITTEE, SYNC_COMMITTEE_SIZE,
};
use anchor_core::schedule::{fork_at_slot, fork_version_bytes, SLOTS_PER_SYNC_COMMITTEE_PERIOD};
use beacon_cl_core::fork::{verify_branch, ForkVersion};
use beacon_cl_core::ssz::{merkleize, mix_in_length, uint64_leaf, Bytes32};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

/// Ethereum mainnet's `genesis_validators_root`: what makes a signature mainnet's.
pub const GENESIS_VALIDATORS_ROOT: Bytes32 =
    hex32("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95");
pub const GENESIS_TIME: u64 = 1_606_824_023;
pub const SECONDS_PER_SLOT: u64 = 12;
/// A starting point older than this is refused: past it, the keys of that period's committee
/// are the only thing standing between the page and a forged chain.
pub const MAX_START_AGE_SECS: u64 = 30 * 86_400;
/// `BeaconBlockBody.execution_payload`: field 9 of at most 16, Capella through Fulu.
const EXECUTION_PAYLOAD_GINDEX: u64 = 25;
/// Two thirds of the committee: the most signers a forged update could lack.
pub const MIN_PARTICIPANTS: usize = SYNC_COMMITTEE_SIZE * 2 / 3 + 1;
/// Updates applied from one answer at most.
pub const MAX_UPDATES: usize = 16;

const fn hex32(s: &str) -> Bytes32 {
    let b = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nib(b[2 * i]) << 4) | nib(b[2 * i + 1]);
        i += 1;
    }
    out
}

const fn nib(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("not lowercase hex"),
    }
}

pub fn period(slot: u64) -> u64 {
    slot / SLOTS_PER_SYNC_COMMITTEE_PERIOD
}

pub fn slot_time(slot: u64) -> u64 {
    GENESIS_TIME + slot * SECONDS_PER_SLOT
}

/// `current_sync_committee` (field 22): 54 through Deneb, 86 from Electra.
const fn current_sync_committee_gindex(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella | ForkVersion::Deneb => 54,
        ForkVersion::Electra | ForkVersion::Fulu => 86,
    }
}

const fn fork_name(fork: ForkVersion) -> &'static str {
    match fork {
        ForkVersion::Capella => "capella",
        ForkVersion::Deneb => "deneb",
        ForkVersion::Electra => "electra",
        ForkVersion::Fulu => "fulu",
    }
}

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

fn field<'a>(v: &'a Value, name: &str) -> Result<&'a Value, Failure> {
    v.get(name).ok_or_else(|| bad(format!("no {name}")))
}

fn text<'a>(v: &'a Value, name: &str) -> Result<&'a str, Failure> {
    field(v, name)?
        .as_str()
        .ok_or_else(|| bad(format!("{name} is not a string")))
}

/// A decimal `uint64`, as the beacon API writes them.
fn dec(v: &Value, name: &str) -> Result<u64, Failure> {
    let s = text(v, name)?;
    if s.is_empty() || s.len() > 20 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return Err(bad(format!("{name} is not a decimal number")));
    }
    s.parse()
        .map_err(|_| bad(format!("{name} is not a uint64")))
}

pub fn bytes_of(s: &str, name: &str) -> Result<Vec<u8>, Failure> {
    let h = s
        .strip_prefix("0x")
        .ok_or_else(|| bad(format!("{name} is not 0x hex")))?;
    hex::decode(h).map_err(|_| bad(format!("{name} is not hex")))
}

fn fixed<const N: usize>(v: &Value, name: &str) -> Result<[u8; N], Failure> {
    bytes_of(text(v, name)?, name)?
        .try_into()
        .map_err(|_| bad(format!("{name} is not {N} bytes")))
}

fn branch(v: &Value, name: &str, depth: usize) -> Result<Vec<Bytes32>, Failure> {
    let list = field(v, name)?
        .as_array()
        .ok_or_else(|| bad(format!("{name} is not a list")))?;
    if list.len() != depth {
        return Err(rejected(format!(
            "{name} has {} nodes, not {depth}",
            list.len()
        )));
    }
    list.iter()
        .map(|n| {
            n.as_str()
                .ok_or_else(|| bad(format!("{name} holds a non-string")))
                .and_then(|s| bytes_of(s, name))
                .and_then(|b| {
                    b.try_into()
                        .map_err(|_| bad(format!("{name} node is not 32 bytes")))
                })
        })
        .collect()
}

/// A `BeaconBlockHeader`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Header {
    pub slot: u64,
    pub proposer_index: u64,
    pub parent_root: Bytes32,
    pub state_root: Bytes32,
    pub body_root: Bytes32,
}

impl Header {
    fn parse(v: &Value) -> Result<Self, Failure> {
        Ok(Header {
            slot: dec(v, "slot")?,
            proposer_index: dec(v, "proposer_index")?,
            parent_root: fixed(v, "parent_root")?,
            state_root: fixed(v, "state_root")?,
            body_root: fixed(v, "body_root")?,
        })
    }

    pub fn root(&self) -> Bytes32 {
        beacon_block_header_root(
            self.slot,
            self.proposer_index,
            &self.parent_root,
            &self.state_root,
            &self.body_root,
        )
    }

    fn is_zero(&self) -> bool {
        *self
            == Header {
                slot: 0,
                proposer_index: 0,
                parent_root: [0; 32],
                state_root: [0; 32],
                body_root: [0; 32],
            }
    }
}

/// The execution block a beacon header carries, checked against its body root.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Execution {
    pub number: u64,
    pub hash: Bytes32,
    pub state_root: Bytes32,
    pub timestamp: u64,
    /// The rest of the block's environment, for calls run against its state.
    pub gas_limit: u64,
    pub base_fee: U256,
    pub prev_randao: Bytes32,
    pub fee_recipient: [u8; 20],
}

/// `ExecutionPayloadHeader` fields in declaration order; Deneb added the last two.
const PAYLOAD_FIELDS: [&str; 17] = [
    "parent_hash",
    "fee_recipient",
    "state_root",
    "receipts_root",
    "logs_bloom",
    "prev_randao",
    "block_number",
    "gas_limit",
    "gas_used",
    "timestamp",
    "extra_data",
    "base_fee_per_gas",
    "block_hash",
    "transactions_root",
    "withdrawals_root",
    "blob_gas_used",
    "excess_blob_gas",
];

/// `hash_tree_root(ExecutionPayloadHeader)` from the beacon API's JSON, which must hold exactly
/// the fork's fields, and the four fields read from it.
fn execution_root(fork: ForkVersion, v: &Value) -> Result<(Bytes32, Execution), Failure> {
    let m: &Map<String, Value> = v
        .as_object()
        .ok_or_else(|| bad("execution is not an object"))?;
    let count = match fork {
        ForkVersion::Capella => 15,
        ForkVersion::Deneb | ForkVersion::Electra | ForkVersion::Fulu => 17,
    };
    let names = &PAYLOAD_FIELDS[..count];
    if m.len() != count || names.iter().any(|n| !m.contains_key(*n)) {
        return Err(bad(format!(
            "execution header does not have the {} fields of a {} header",
            count,
            fork_name(fork)
        )));
    }
    let mut leaves = Vec::with_capacity(count);
    for &n in names {
        let leaf = match n {
            "fee_recipient" => {
                let a: [u8; 20] = fixed(v, n)?;
                let mut l = [0u8; 32];
                l[..20].copy_from_slice(&a);
                l
            }
            "logs_bloom" => {
                let b: [u8; 256] = fixed(v, n)?;
                let chunks: Vec<Bytes32> =
                    b.chunks_exact(32).map(|c| c.try_into().unwrap()).collect();
                merkleize(&chunks, 3)
            }
            "block_number" | "gas_limit" | "gas_used" | "timestamp" | "blob_gas_used"
            | "excess_blob_gas" => uint64_leaf(dec(v, n)?),
            "extra_data" => {
                let b = bytes_of(text(v, n)?, n)?;
                if b.len() > 32 {
                    return Err(bad("extra_data is over 32 bytes"));
                }
                let mut l = [0u8; 32];
                l[..b.len()].copy_from_slice(&b);
                mix_in_length(&l, b.len() as u64)
            }
            "base_fee_per_gas" => {
                let s = text(v, n)?;
                if s.is_empty() || s.len() > 78 || !s.bytes().all(|b| b.is_ascii_digit()) {
                    return Err(bad("base_fee_per_gas is not a decimal number"));
                }
                U256::from_str_radix(s, 10)
                    .map_err(|_| bad("base_fee_per_gas is not a uint256"))?
                    .to_le_bytes::<32>()
            }
            _ => fixed(v, n)?,
        };
        leaves.push(leaf);
    }
    let depth = count.next_power_of_two().trailing_zeros() as usize;
    let exec = Execution {
        number: dec(v, "block_number")?,
        hash: fixed(v, "block_hash")?,
        state_root: fixed(v, "state_root")?,
        timestamp: dec(v, "timestamp")?,
        gas_limit: dec(v, "gas_limit")?,
        base_fee: U256::from_le_bytes(leaves[11]),
        prev_randao: fixed(v, "prev_randao")?,
        fee_recipient: fixed(v, "fee_recipient")?,
    };
    Ok((merkleize(&leaves, depth), exec))
}

/// A `LightClientHeader`: the beacon header and its execution block, the latter checked.
#[derive(Clone, Copy, Debug)]
pub struct LcHeader {
    pub beacon: Header,
    pub execution: Execution,
}

fn fork_of(slot: u64) -> Result<ForkVersion, Failure> {
    fork_at_slot(slot).ok_or_else(|| rejected(format!("slot {slot} is before Capella")))
}

impl LcHeader {
    fn parse(v: &Value) -> Result<Self, Failure> {
        let beacon = Header::parse(field(v, "beacon")?)?;
        let fork = fork_of(beacon.slot)?;
        let (root, execution) = execution_root(fork, field(v, "execution")?)?;
        let b = branch(v, "execution_branch", 4)?;
        if !verify_branch(&root, &b, EXECUTION_PAYLOAD_GINDEX, &beacon.body_root) {
            return Err(rejected(format!(
                "the execution block of slot {} is not under its body root",
                beacon.slot
            )));
        }
        Ok(LcHeader { beacon, execution })
    }

    /// Only the beacon header, for a part whose execution block is not used.
    fn beacon_only(v: &Value) -> Result<Header, Failure> {
        Header::parse(field(v, "beacon")?)
    }
}

/// A sync committee: 512 keys and their aggregate.
#[derive(Clone)]
pub struct Committee {
    pub pubkeys: Box<[[u8; 48]; SYNC_COMMITTEE_SIZE]>,
    pub aggregate: [u8; 48],
}

impl Committee {
    fn parse(v: &Value) -> Result<Self, Failure> {
        let keys = field(v, "pubkeys")?
            .as_array()
            .ok_or_else(|| bad("pubkeys is not a list"))?;
        if keys.len() != SYNC_COMMITTEE_SIZE {
            return Err(rejected(format!("a committee of {} keys", keys.len())));
        }
        let mut pubkeys = Box::new([[0u8; 48]; SYNC_COMMITTEE_SIZE]);
        for (pk, k) in pubkeys.iter_mut().zip(keys) {
            let s = k.as_str().ok_or_else(|| bad("a pubkey is not a string"))?;
            *pk = bytes_of(s, "pubkey")?
                .try_into()
                .map_err(|_| bad("a pubkey is not 48 bytes"))?;
        }
        Ok(Committee {
            pubkeys,
            aggregate: fixed(v, "aggregate_pubkey")?,
        })
    }

    pub fn root(&self) -> Bytes32 {
        sync_committee_hash_tree_root(&self.pubkeys, &self.aggregate)
    }
}

/// A verified finality update: the signed head and the finalized header under it.
#[derive(Clone, Copy, Debug)]
pub struct Finality {
    pub attested: LcHeader,
    pub finalized: LcHeader,
    pub signers: usize,
    pub period: u64,
}

/// The light client's state: the committees it verified, by period.
#[derive(Clone)]
pub struct LightClient {
    pub checkpoint_root: Bytes32,
    pub checkpoint_slot: u64,
    pub committees: BTreeMap<u64, Committee>,
    /// What the committees were verified from: the bootstrap and each period's update, as served,
    /// so a later visit re-verifies the same chain instead of trusting its result.
    chain: Option<(Value, BTreeMap<u64, Value>)>,
}

/// The `{version, data}` a beacon API sends, with a fork this page knows.
fn versioned(v: &Value) -> Result<(&'static str, &Value), Failure> {
    let version = text(v, "version")?;
    let known = ["capella", "deneb", "electra", "fulu"];
    let Some(name) = known.iter().find(|k| **k == version) else {
        return Err(Failure::new(
            Kind::UnknownFork,
            "Ethereum's light client data is from an upgrade newer than this page",
        ));
    };
    Ok((name, field(v, "data")?))
}

fn check_version(name: &str, slot: u64) -> Result<ForkVersion, Failure> {
    let fork = fork_of(slot)?;
    if fork_name(fork) != name {
        return Err(rejected(format!(
            "a {name} answer for slot {slot}, which is {}",
            fork_name(fork)
        )));
    }
    Ok(fork)
}

/// The sync aggregate: participation bits and signature.
fn aggregate(v: &Value) -> Result<([u8; 64], [u8; 96]), Failure> {
    let a = field(v, "sync_aggregate")?;
    Ok((
        fixed(a, "sync_committee_bits")?,
        fixed(a, "sync_committee_signature")?,
    ))
}

/// `committee`'s signature over `attested` at `signature_slot`: at least 342 of 512 signers.
fn verify_signature(
    committee: &Committee,
    attested: &Header,
    bits: &[u8; 64],
    signature: &[u8; 96],
    signature_slot: u64,
) -> Result<usize, Failure> {
    let signers = (0..SYNC_COMMITTEE_SIZE)
        .filter(|&i| bit_set(bits, i))
        .count();
    if signers < MIN_PARTICIPANTS {
        return Err(rejected(format!(
            "{signers} of 512 validators signed, fewer than the {MIN_PARTICIPANTS} needed"
        )));
    }
    let fork = fork_of(signature_slot.saturating_sub(1))?;
    let domain = compute_domain(
        DOMAIN_SYNC_COMMITTEE,
        fork_version_bytes(fork),
        &GENESIS_VALIDATORS_ROOT,
    );
    let signing_root = compute_signing_root(&attested.root(), &domain);
    fast_aggregate_verify(&committee.pubkeys[..], bits, signature, &signing_root).map_err(|e| {
        rejected(match e {
            BlsFailure::InvalidPublicKey(i) => format!("committee key {i} is not a valid key"),
            BlsFailure::InvalidSignature => "the signature is not a valid BLS signature".into(),
            BlsFailure::AggregateIsIdentity => "no signers".into(),
            BlsFailure::VerificationFailed => format!(
                "the validators' signature over slot {} does not verify",
                attested.slot
            ),
        })
    })
}

impl LightClient {
    pub fn new(checkpoint_root: Bytes32, checkpoint_slot: u64) -> Self {
        LightClient {
            checkpoint_root,
            checkpoint_slot,
            committees: BTreeMap::new(),
            chain: None,
        }
    }

    /// Refuses a checkpoint older than [`MAX_START_AGE_SECS`] at `now`.
    pub fn check_age(&self, now: u64) -> Result<(), Failure> {
        if now.saturating_sub(slot_time(self.checkpoint_slot)) > MAX_START_AGE_SECS {
            return Err(Failure::new(
                Kind::StaleCheckpoint,
                "this page's starting point on Ethereum is more than 30 days old",
            ));
        }
        Ok(())
    }

    /// The bootstrap's result (`{version, data}`): its header must be the checkpoint (root and
    /// slot), its committee under the header's state root. Returns the committee's period.
    pub fn bootstrap(&mut self, v: &Value) -> Result<u64, Failure> {
        let (name, d) = versioned(v)?;
        let header = Header::parse(field(field(d, "header")?, "beacon")?)?;
        if header.root() != self.checkpoint_root || header.slot != self.checkpoint_slot {
            return Err(rejected("the bootstrap is not for this page's checkpoint"));
        }
        let fork = check_version(name, header.slot)?;
        let committee = Committee::parse(field(d, "current_sync_committee")?)?;
        let g = current_sync_committee_gindex(fork);
        let b = branch(
            d,
            "current_sync_committee_branch",
            beacon_cl_core::fork::depth_for_gindex(g),
        )?;
        if !verify_branch(&committee.root(), &b, g, &header.state_root) {
            return Err(rejected(
                "the checkpoint's sync committee is not under its state root",
            ));
        }
        let p = period(header.slot);
        self.committees.clear();
        self.committees.insert(p, committee);
        self.chain = Some((v.clone(), BTreeMap::new()));
        Ok(p)
    }

    /// One period's update: signed by the committee of its period, it gives the next one.
    /// Returns the period learned, or `None` when it was known already.
    pub fn apply_update(&mut self, v: &Value) -> Result<Option<u64>, Failure> {
        let (_, d) = versioned(v)?;
        let attested = Header::parse(field(field(d, "attested_header")?, "beacon")?)?;
        if self.committees.contains_key(&(period(attested.slot) + 1)) {
            return Ok(None);
        }
        let (p, next) = self.check_update(v)?;
        self.committees.insert(p + 1, next);
        if let Some((_, updates)) = self.chain.as_mut() {
            updates.insert(p, v.clone());
        }
        Ok(Some(p + 1))
    }

    /// An update verified against the committee of its period, which must be known: its period
    /// and the next committee it hands over.
    pub fn check_update(&self, v: &Value) -> Result<(u64, Committee), Failure> {
        let (name, d) = versioned(v)?;
        let attested = Header::parse(field(field(d, "attested_header")?, "beacon")?)?;
        let fork = check_version(name, attested.slot)?;
        let signature_slot = dec(d, "signature_slot")?;
        let p = period(attested.slot);
        if signature_slot <= attested.slot {
            return Err(rejected(format!(
                "the update for period {p} is signed before its slot"
            )));
        }
        if period(signature_slot) != p {
            return Err(Failure::new(
                Kind::Unreachable,
                format!("Willow's update for period {p} is signed in the next period, so it cannot hand over the committee"),
            ));
        }
        let committee = self.committees.get(&p).ok_or_else(|| {
            Failure::new(
                Kind::Unreachable,
                format!("an update for period {p}, which this light client has not reached"),
            )
        })?;
        let (bits, signature) = aggregate(d)?;
        verify_signature(committee, &attested, &bits, &signature, signature_slot)?;
        let next = Committee::parse(field(d, "next_sync_committee")?)?;
        let g = next_sync_committee_gindex(fork);
        let b = branch(
            d,
            "next_sync_committee_branch",
            beacon_cl_core::fork::depth_for_gindex(g),
        )?;
        if !verify_branch(&next.root(), &b, g, &attested.state_root) {
            return Err(rejected(format!(
                "period {}'s committee is not under the signed state root",
                p + 1
            )));
        }
        let finalized = LcHeader::beacon_only(field(d, "finalized_header")?)?;
        if !finalized.is_zero() {
            let g = finalized_root_gindex(fork);
            let fb = branch(
                d,
                "finality_branch",
                beacon_cl_core::fork::depth_for_gindex(g),
            )?;
            if finalized.slot > attested.slot
                || !verify_branch(&finalized.root(), &fb, g, &attested.state_root)
            {
                return Err(rejected(format!(
                    "period {p}'s finalized header is not under the signed state root"
                )));
            }
        }
        Ok((p, next))
    }

    /// What is missing to check a signature from `period` (at `now`, in seconds), if anything.
    pub fn need(&self, sig_period: u64, now: u64) -> Result<(), Failure> {
        // Nothing is signed in a period that has not begun.
        let now_period = period(now.saturating_sub(GENESIS_TIME) / SECONDS_PER_SLOT);
        if sig_period > now_period + 1 {
            return Err(rejected(format!(
                "signed in period {sig_period}, which has not begun"
            )));
        }
        let (Some(&lo), Some(&hi)) = (
            self.committees.keys().next(),
            self.committees.keys().next_back(),
        ) else {
            return Err(Failure::new(Kind::NeedBootstrap, "no committee yet"));
        };
        if sig_period > hi {
            return Err(Failure::new(Kind::NeedUpdates, "updates needed")
                .with(serde_json::json!({"from": hi, "count": sig_period - hi})));
        }
        if sig_period < lo {
            return Err(Failure::new(
                Kind::Unreachable,
                format!("signed in period {sig_period}, before this page's starting point"),
            ));
        }
        Ok(())
    }

    /// A finality update (`{version, data}`), verified: signature, finality branch, and both
    /// headers' execution blocks.
    pub fn finality(&self, v: &Value, now: u64) -> Result<Finality, Failure> {
        let (name, d) = versioned(v)?;
        let signature_slot = dec(d, "signature_slot")?;
        let sig_period = period(signature_slot);
        self.need(sig_period, now)?;
        let attested = LcHeader::parse(field(d, "attested_header")?)?;
        let fork = check_version(name, attested.beacon.slot)?;
        let finalized = LcHeader::parse(field(d, "finalized_header")?)?;
        if signature_slot <= attested.beacon.slot
            || finalized.beacon.slot > attested.beacon.slot
            || finalized.beacon.is_zero()
        {
            return Err(rejected("the finality update's slots are out of order"));
        }
        let (bits, signature) = aggregate(d)?;
        let committee = &self.committees[&sig_period];
        let signers = verify_signature(
            committee,
            &attested.beacon,
            &bits,
            &signature,
            signature_slot,
        )?;
        let g = finalized_root_gindex(fork);
        let fb = branch(
            d,
            "finality_branch",
            beacon_cl_core::fork::depth_for_gindex(g),
        )?;
        if !verify_branch(
            &finalized.beacon.root(),
            &fb,
            g,
            &attested.beacon.state_root,
        ) {
            return Err(rejected(
                "the finalized header is not under the signed state root",
            ));
        }
        if finalized.execution.number > attested.execution.number {
            return Err(rejected("the finalized block is after the signed head"));
        }
        Ok(Finality {
            attested,
            finalized,
            signers,
            period: sig_period,
        })
    }

    /// What the committees were verified from, to keep for the next visit:
    /// `{root, bootstrap, updates}`, or `null` before a bootstrap.
    pub fn saved(&self) -> Value {
        match &self.chain {
            Some((bootstrap, updates)) => serde_json::json!({
                "root": format!("0x{}", hex::encode(self.checkpoint_root)),
                "bootstrap": bootstrap,
                "updates": updates.values().collect::<Vec<_>>(),
            }),
            None => Value::Null,
        }
    }

    /// The chain kept from an earlier visit ([`Self::saved`]), verified again from this page's
    /// checkpoint exactly as when it was fetched: nothing kept is trusted. Kept for another
    /// checkpoint, it is ignored. Returns the newest period reached.
    pub fn restore(&mut self, v: &Value, now: u64) -> Result<Option<u64>, Failure> {
        let root = format!("0x{}", hex::encode(self.checkpoint_root));
        if v.get("root").and_then(Value::as_str) != Some(root.as_str()) {
            return Ok(None);
        }
        self.check_age(now)?;
        self.bootstrap(field(v, "bootstrap")?)?;
        let updates = field(v, "updates")?
            .as_array()
            .ok_or_else(|| bad("kept updates are not a list"))?;
        let mut newest = None;
        for u in updates {
            newest = self.apply_update(u)?.or(newest);
        }
        Ok(newest)
    }

    /// Forgets every committee: the next check starts from the checkpoint again.
    pub fn reset(&mut self) {
        self.committees.clear();
        self.chain = None;
    }
}
