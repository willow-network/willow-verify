














use crate::bls::{aggregate_verify_with, public_key};
use crate::lc::bytes_of;
use crate::{Failure, Kind};
use alloy_primitives::B256;
use bls12_381::G1Affine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

pub const CHAIN_ID: u64 = 80094;

pub const COMET_CHAIN: &str = "mainnet-beacon-80094";

pub const BLOCK_PROTOCOL: u64 = 11;


pub const ELECTRA_TIME: u64 = 1_749_056_400;


const DST: &[u8] = b"BLS_SIG_BLS12381G2_XMD:SHA-256_SSWU_RO_NUL_";


pub const MAX_START_AGE_SECS: u64 = 24 * 3600;

pub const MAX_UPDATES: usize = 16;

const MAX_VALIDATORS: usize = 128;

const MAX_TOTAL_POWER: u128 = (i64::MAX / 8) as u128;

const MAX_BLOCK_BYTES: usize = 8 << 20;

const MAX_OTHER_TXS: usize = 1;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start {
    pub height: u64,
    pub hash: [u8; 32],
}




pub const PINNED: Start = Start {
    height: 28_955_000,
    hash: alloy_primitives::b256!(
        "4096d1707b98861a946fbb4b85068b94cc846654d14424ddeaaf0b7e1ff27337"
    )
    .0,
};

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

fn sha256(parts: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parts {
        h.update(p);
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&h.finalize());
    out
}


pub fn merkle<T: AsRef<[u8]>>(items: &[T]) -> [u8; 32] {
    match items.len() {
        0 => sha256(&[]),
        1 => sha256(&[&[0], items[0].as_ref()]),
        n => {
            let k = 1usize << (usize::BITS - 1 - (n - 1).leading_zeros());
            sha256(&[&[1], &merkle(&items[..k]), &merkle(&items[k..])])
        }
    }
}

fn varint(out: &mut Vec<u8>, mut n: u64) {
    loop {
        let b = (n & 0x7f) as u8;
        n >>= 7;
        if n == 0 {
            out.push(b);
            return;
        }
        out.push(b | 0x80);
    }
}


fn uint_field(out: &mut Vec<u8>, field: u8, n: u64) {
    if n != 0 {
        out.push(field << 3);
        varint(out, n);
    }
}


fn len_field(out: &mut Vec<u8>, field: u8, b: &[u8]) {
    out.push(field << 3 | 2);
    varint(out, b.len() as u64);
    out.extend_from_slice(b);
}


fn bytes_field(out: &mut Vec<u8>, field: u8, b: &[u8]) {
    if !b.is_empty() {
        len_field(out, field, b);
    }
}

fn get<'a>(v: &'a Value, name: &str) -> Result<&'a Value, Failure> {
    v.get(name).ok_or_else(|| bad(format!("no {name}")))
}

fn uint(v: &Value, name: &str) -> Result<u64, Failure> {
    get(v, name)?
        .as_u64()
        .ok_or_else(|| bad(format!("{name} is not a number")))
}

fn fixed<const N: usize>(v: &Value, name: &str) -> Result<[u8; N], Failure> {
    let s = get(v, name)?
        .as_str()
        .ok_or_else(|| bad(format!("{name} is not hex")))?;
    bytes_of(s, name)?
        .try_into()
        .map_err(|_| bad(format!("{name} is not {N} bytes")))
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BlockId {
    hash: [u8; 32],
    parts_total: u32,
    parts_hash: [u8; 32],
}

impl BlockId {
    fn of(v: &Value, name: &str) -> Result<BlockId, Failure> {
        let v = get(v, name)?;
        let parts_total = uint(v, "partsTotal")?;
        Ok(BlockId {
            hash: fixed(v, "hash")?,
            parts_total: u32::try_from(parts_total)
                .ok()
                .filter(|t| *t > 0)
                .ok_or_else(|| bad(format!("{name} has {parts_total} parts")))?,
            parts_hash: fixed(v, "partsHash")?,
        })
    }


    fn parts(&self) -> Vec<u8> {
        let mut p = Vec::new();
        uint_field(&mut p, 1, self.parts_total.into());
        bytes_field(&mut p, 2, &self.parts_hash);
        p
    }


    fn encode(&self) -> Vec<u8> {
        let mut b = Vec::new();
        bytes_field(&mut b, 1, &self.hash);
        len_field(&mut b, 2, &self.parts());
        b
    }
}


#[derive(Clone, Debug)]
pub struct Header {
    pub height: u64,

    pub time: (u64, u32),
    pub data_hash: [u8; 32],
    pub validators_hash: [u8; 32],
    pub next_validators_hash: [u8; 32],
}


pub fn header(v: &Value) -> Result<(Header, [u8; 32]), Failure> {
    let version = get(v, "version")?;
    let block = uint(version, "block")?;
    if block != BLOCK_PROTOCOL {
        return Err(Failure::new(
            Kind::UnknownFork,
            format!("CometBFT block protocol {block} is not one this page knows"),
        ));
    }
    let app = uint(version, "app")?;
    if get(v, "chainId")?.as_str() != Some(COMET_CHAIN) {
        return Err(rejected("the header is not Berachain mainnet's"));
    }
    let height = uint(v, "height")?;
    let time = get(v, "time")?
        .as_array()
        .filter(|t| t.len() == 2)
        .and_then(|t| Some((t[0].as_u64()?, t[1].as_u64()?)))
        .filter(|(s, n)| *s > 0 && *s < i64::MAX as u64 && *n < 1_000_000_000)
        .ok_or_else(|| bad("the header's time is not [seconds, nanoseconds]"))?;
    if height == 0 || height > i64::MAX as u64 {
        return Err(bad("the header's height is out of range"));
    }
    let last = BlockId::of(v, "lastBlockId")?;
    let hashes = [
        "lastCommitHash",
        "dataHash",
        "validatorsHash",
        "nextValidatorsHash",
        "consensusHash",
        "appHash",
        "lastResultsHash",
        "evidenceHash",
    ]
    .map(|f| fixed::<32>(v, f));
    let hashes = {
        let mut out = [[0u8; 32]; 8];
        for (o, h) in out.iter_mut().zip(hashes) {
            *o = h?;
        }
        out
    };
    let proposer: [u8; 20] = fixed(v, "proposerAddress")?;
    let mut leaves: Vec<Vec<u8>> = Vec::with_capacity(14);
    let mut ver = Vec::new();
    uint_field(&mut ver, 1, block);
    uint_field(&mut ver, 2, app);
    leaves.push(ver);
    let mut chain = Vec::new();
    bytes_field(&mut chain, 1, COMET_CHAIN.as_bytes());
    leaves.push(chain);
    let mut h = Vec::new();
    uint_field(&mut h, 1, height);
    leaves.push(h);
    let mut t = Vec::new();
    uint_field(&mut t, 1, time.0);
    uint_field(&mut t, 2, time.1);
    leaves.push(t);
    leaves.push(last.encode());
    for x in &hashes {
        let mut b = Vec::new();
        bytes_field(&mut b, 1, x);
        leaves.push(b);
    }
    let mut p = Vec::new();
    bytes_field(&mut p, 1, &proposer);
    leaves.push(p);
    Ok((
        Header {
            height,
            time: (time.0, time.1 as u32),
            data_hash: hashes[1],
            validators_hash: hashes[2],
            next_validators_hash: hashes[3],
        },
        merkle(&leaves),
    ))
}






















fn sign_bytes(height: u64, round: u32, id: &BlockId) -> Vec<u8> {
    let mut m = vec![0x08, 0x02, 0x11];
    m.extend_from_slice(&height.to_le_bytes());
    if round != 0 {
        m.push(0x19);
        m.extend_from_slice(&i64::from(round).to_le_bytes());
    }
    len_field(&mut m, 4, &id.encode());
    bytes_field(&mut m, 6, COMET_CHAIN.as_bytes());
    let mut out = Vec::with_capacity(m.len() + 2);
    varint(&mut out, m.len() as u64);
    out.extend(m);
    out
}


#[derive(Clone, Debug)]
struct Set {
    raw: Vec<[u8; 48]>,
    keys: Vec<G1Affine>,
    addresses: Vec<[u8; 20]>,
    powers: Vec<u64>,
    total: u128,
    hash: [u8; 32],
}

impl Set {


    fn of(v: &Value, before: Option<&Set>) -> Result<Set, Failure> {
        let list = v
            .as_array()
            .filter(|l| !l.is_empty() && l.len() <= MAX_VALIDATORS)
            .ok_or_else(|| {
                bad(format!(
                    "validators are not a list of 1 to {MAX_VALIDATORS}"
                ))
            })?;
        let mut s = Set {
            raw: Vec::with_capacity(list.len()),
            keys: Vec::with_capacity(list.len()),
            addresses: Vec::with_capacity(list.len()),
            powers: Vec::with_capacity(list.len()),
            total: 0,
            hash: [0; 32],
        };
        let mut leaves = Vec::with_capacity(list.len());
        for item in list {
            let raw: [u8; 48] = fixed(item, "key")?;
            let power = get(item, "power")?
                .as_str()
                .filter(|p| !p.is_empty() && p.len() <= 20 && p.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|p| p.parse::<u64>().ok())
                .filter(|p| *p > 0)
                .ok_or_else(|| bad("a validator's power is not a positive decimal string"))?;
            let key = before
                .and_then(|b| b.raw.iter().position(|r| *r == raw).map(|i| b.keys[i]))
                .or_else(|| public_key(&raw))
                .ok_or_else(|| rejected("a validator's key is not a key"))?;
            let full = key.to_uncompressed();
            let mut address = [0u8; 20];
            address.copy_from_slice(&sha256(&[&full])[..20]);
            if s.addresses.contains(&address) {
                return Err(rejected("a validator is in the set twice"));
            }
            let mut pk = Vec::with_capacity(100);
            len_field(&mut pk, 3, &full);
            let mut leaf = Vec::with_capacity(112);
            len_field(&mut leaf, 1, &pk);
            uint_field(&mut leaf, 2, power);
            leaves.push(leaf);
            s.total += u128::from(power);
            s.raw.push(raw);
            s.keys.push(key);
            s.addresses.push(address);
            s.powers.push(power);
        }
        if s.total > MAX_TOTAL_POWER {
            return Err(rejected("the validators' power is past CometBFT's limit"));
        }
        s.hash = merkle(&leaves);
        Ok(s)
    }
}

























#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Payload {
    pub slot: u64,
    pub number: u64,
    pub block_hash: [u8; 32],
}

fn u32_at(b: &[u8], at: usize) -> Option<usize> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?) as usize)
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    Some(u64::from_le_bytes(b.get(at..at + 8)?.try_into().ok()?))
}



fn offsets(b: &[u8], fixed: usize, at: &[usize]) -> Option<Vec<usize>> {
    let o: Vec<usize> = at.iter().map(|a| u32_at(b, *a)).collect::<Option<_>>()?;
    (b.len() >= fixed
        && o.first() == Some(&fixed)
        && o.windows(2).all(|w| w[0] <= w[1])
        && *o.last()? <= b.len())
    .then_some(o)
}



pub fn payload(block: &[u8], time: u64) -> Result<Payload, Failure> {
    let malformed = || bad("the beacon block is not a BeaconKit block's SSZ");

    offsets(block, 100, &[0]).ok_or_else(malformed)?;
    let msg = &block[100..];

    offsets(msg, 84, &[80]).ok_or_else(malformed)?;
    let slot = u64_at(msg, 0).ok_or_else(malformed)?;
    let body = &msg[84..];


    let electra = time >= ELECTRA_TIME;
    let fixed = if electra { 396 } else { 392 };
    let mut at = vec![200, 204, 208, 212, 216, 380, 384, 388];
    if electra {
        at.push(392);
    }
    let o = offsets(body, fixed, &at).ok_or_else(malformed)?;
    let p = &body[o[5]..o[6]];


    let po = offsets(p, 528, &[436, 504, 508]).ok_or_else(malformed)?;
    if po[1] - po[0] > 32 {
        return Err(malformed());
    }
    Ok(Payload {
        slot,
        number: u64_at(p, 404).ok_or_else(malformed)?,
        block_hash: p[472..504].try_into().map_err(|_| malformed())?,
    })
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BeraConfirmed {

    pub level: &'static str,
    pub hash: [u8; 32],

    pub height: u64,

    pub time: u64,

    pub signers: usize,
    pub of: usize,
}


#[derive(Clone, Debug)]
struct Trusted {
    height: u64,
    time: (u64, u32),
    next: [u8; 32],
    set: Set,
}


struct Signed {
    header: Header,
    set: Set,
    signers: usize,
}


#[derive(Clone)]
pub struct Berachain {
    pub start: Start,

    start_time: Option<u64>,


    ages: bool,
    trusted: Option<Trusted>,

    chain: Option<(Value, Vec<Value>)>,
}

impl Default for Berachain {
    fn default() -> Self {
        Berachain::new(PINNED)
    }
}

impl Berachain {
    pub fn new(start: Start) -> Berachain {
        Berachain {
            start,
            start_time: None,
            ages: true,
            trusted: None,
            chain: None,
        }
    }













    fn trusted(&self) -> Result<&Trusted, Failure> {
        self.trusted
            .as_ref()
            .ok_or_else(|| Failure::new(Kind::NeedBootstrap, "no validator set yet").with(lc()))
    }



    pub fn bootstrap(&mut self, v: &Value, now: u64) -> Result<u64, Failure> {
        let (h, hash) = header(get(v, "header")?)?;
        if hash != self.start.hash || h.height != self.start.height {
            return Err(rejected(
                "the bootstrap is not this page's starting header on Berachain",
            ));
        }
        if self.ages {
            check_age(h.time.0, now)?;
        }
        let set = Set::of(get(v, "validators")?, None)?;
        if set.hash != h.validators_hash {
            return Err(rejected("the bootstrap's validators are not its header's"));
        }
        self.start_time = Some(h.time.0);
        self.trusted = Some(Trusted {
            height: h.height,
            time: h.time,
            next: h.next_validators_hash,
            set,
        });
        self.chain = Some((v.clone(), Vec::new()));
        Ok(h.height)
    }



    fn signed(&self, v: &Value, header_and_hash: (Header, [u8; 32])) -> Result<Signed, Failure> {
        let t = self.trusted()?;
        let (h, hash) = header_and_hash;
        let set = Set::of(get(v, "validators")?, Some(&t.set))?;
        if set.hash != h.validators_hash {
            return Err(rejected(format!(
                "the validators are not height {}'s",
                h.height
            )));
        }
        let c = get(v, "commit")?;
        let id = BlockId::of(c, "blockId")?;
        if uint(c, "height")? != h.height || id.hash != hash {
            return Err(rejected(format!(
                "the commit is not for height {}'s header",
                h.height
            )));
        }
        let round = u32::try_from(uint(c, "round")?)
            .ok()
            .filter(|r| *r <= i32::MAX as u32)
            .ok_or_else(|| bad("the commit's round is out of range"))?;
        let flags = get(c, "flags")?
            .as_array()
            .filter(|f| f.len() == set.keys.len())
            .ok_or_else(|| bad("the commit's flags are not one per validator"))?;


        let mut signers = Vec::new();
        for (i, f) in flags.iter().enumerate() {
            match f.as_u64() {
                Some(4) if signers.is_empty() => signers.push(i),
                Some(4) => return Err(rejected("the commit has two aggregate signatures")),
                Some(5) if !signers.is_empty() => signers.push(i),
                Some(5) => return Err(rejected("a vote names an aggregate signature before it")),
                Some(1..=3 | 6 | 7) => {}
                _ => return Err(bad("a commit flag is of no kind this page knows")),
            }
        }
        let power: u128 = signers.iter().map(|i| u128::from(set.powers[*i])).sum();
        if power * 3 <= set.total * 2 {
            return Err(rejected(format!(
                "validators with {power} of {} power signed height {}, not two thirds",
                set.total, h.height
            )));
        }
        let adjacent = h.height == t.height + 1 && h.validators_hash == t.next;



        let followed: Option<u128> = signers
            .iter()
            .map(|i| {
                let j = t.set.raw.iter().position(|r| *r == set.raw[*i])?;
                Some(u128::from(t.set.powers[j]))
            })
            .sum();
        if !adjacent && followed.is_none_or(|p| p * 3 <= t.set.total * 2) {
            let m = format!(
                "height {} is not signed by two thirds of the validators this page follows alone",
                h.height
            );
            return Err(if h.height > t.height {
                Failure::new(Kind::NeedUpdates, m).with(json!({"from": t.height,
                    "count": h.height - t.height, "lightClient": "berachain"}))
            } else {
                Failure::new(Kind::Unreachable, m)
            });
        }
        let signature: [u8; 96] = fixed(c, "signature")?;
        let keys: Vec<G1Affine> = signers.iter().map(|i| set.keys[*i]).collect();
        aggregate_verify_with(&keys, &signature, &sign_bytes(h.height, round, &id), DST).map_err(
            |_| {
                rejected(format!(
                    "the validators' signature on height {} does not verify",
                    h.height
                ))
            },
        )?;
        Ok(Signed {
            header: h,
            set,
            signers: signers.len(),
        })
    }



    pub fn apply_update(&mut self, u: &Value) -> Result<Option<u64>, Failure> {
        let t = self.trusted()?;
        let hh = header(get(u, "header")?)?;
        if hh.0.height <= t.height {
            return Ok(None);
        }
        if hh.0.time <= t.time {
            return Err(rejected(format!(
                "height {} is not later than the header followed",
                hh.0.height
            )));
        }
        let s = self.signed(u, hh).map_err(|f| match f.kind {
            Kind::NeedUpdates => Failure::new(Kind::Unreachable, f.message),
            _ => f,
        })?;
        self.trusted = Some(Trusted {
            height: s.header.height,
            time: s.header.time,
            next: s.header.next_validators_hash,
            set: s.set,
        });
        if let Some((_, updates)) = self.chain.as_mut() {
            updates.push(u.clone());
        }
        Ok(Some(s.header.height))
    }



    pub fn confirm(&self, number: u64, answer: &Value, now: u64) -> Result<BeraConfirmed, Failure> {
        self.trusted()?;
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
        let c = get(answer, "cometbft")?;
        let (h, hash) = header(get(c, "header")?)?;

        let block = bytes_of(
            get(c, "block")?
                .as_str()
                .ok_or_else(|| bad("the beacon block is not hex"))?,
            "the beacon block",
        )?;
        if block.len() > MAX_BLOCK_BYTES {
            return Err(bad("the beacon block is too large"));
        }
        let txs = get(c, "txs")?
            .as_array()
            .filter(|t| t.len() <= MAX_OTHER_TXS)
            .ok_or_else(|| bad(format!("txs are not a list of at most {MAX_OTHER_TXS}")))?;
        let mut leaves = vec![sha256(&[&block])];
        for t in txs {
            let h = t.as_str().ok_or_else(|| bad("a tx hash is not hex"))?;
            leaves.push(
                bytes_of(h, "a tx hash")?
                    .try_into()
                    .map_err(|_| bad("a tx hash is not 32 bytes"))?,
            );
        }
        if merkle(&leaves) != h.data_hash {
            return Err(rejected(format!(
                "the beacon block is not height {}'s",
                h.height
            )));
        }
        let p = payload(&block, h.time.0)?;
        if p.slot != h.height {
            return Err(rejected(format!(
                "the beacon block is for slot {}, not height {}",
                p.slot, h.height
            )));
        }
        if p.number != number {
            return Err(rejected(format!(
                "height {} carries EVM block {}, not {number}",
                h.height, p.number
            )));
        }
        let s = self.signed(c, (h, hash))?;
        Ok(BeraConfirmed {
            level: "final",
            hash: p.block_hash,
            height: s.header.height,
            time: s.header.time.0,
            signers: s.signers,
            of: s.set.keys.len(),
        })
    }


    pub fn saved(&self) -> Value {
        match &self.chain {
            Some((bootstrap, updates)) => json!({
                "start": start_json(&self.start),
                "bootstrap": bootstrap,
                "updates": updates,
            }),
            None => Value::Null,
        }
    }



    pub fn restore(&mut self, v: &Value, now: u64) -> Result<Option<u64>, Failure> {
        if v.get("start") != Some(&start_json(&self.start)) {
            return Ok(None);
        }
        let mut newest = Some(self.bootstrap(get(v, "bootstrap")?, now)?);
        let updates = v
            .get("updates")
            .and_then(Value::as_array)
            .ok_or_else(|| bad("kept updates are not a list"))?;
        for u in updates {
            newest = self.apply_update(u)?.or(newest);
        }
        Ok(newest)
    }

    pub fn reset(&mut self) {
        self.start_time = None;
        self.trusted = None;
        self.chain = None;
    }
}


fn check_age(start_time: u64, now: u64) -> Result<(), Failure> {
    if now.saturating_sub(start_time) > MAX_START_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleCheckpoint,
            "this page's starting point on Berachain is more than a day old",
        ));
    }
    Ok(())
}


fn lc() -> Value {
    json!({"lightClient": "berachain"})
}

pub fn start_json(s: &Start) -> Value {
    json!({"height": s.height, "hash": format!("{}", B256::from(s.hash))})
}


pub fn start_of(v: &Value) -> Option<Start> {
    let height = v.get("height")?.as_u64().filter(|h| *h > 0)?;
    let hash: B256 = v.get("hash")?.as_str()?.parse().ok()?;
    Some(Start {
        height,
        hash: hash.0,
    })
}
