








use crate::derive::{BlobSchedule, GAS_PER_BLOB};
use crate::fault::{check, reject, Fault, Tag};
use crate::proof::{
    decode_header, parse_bytes, parse_qty, verify, Block, BlockProof, Subject, Verified,
};
use crate::render::{decode_receipt, qu};
use crate::txs::{self, Decoded};
use alloy::consensus::{Header, Transaction as _};
use alloy::primitives::{Address, U256};
use anyhow::{anyhow, bail, Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::ops::RangeInclusive;


pub const FORMULA: &str = "willow-fee-v1";

pub const TIP_BLOCKS: u64 = 4;

pub const SAMPLE_NUMBER: usize = 3;

pub const IGNORE_PRICE: u64 = 2;

pub const START_PRICE: u64 = 1_000_000;

pub const PERCENTILE: usize = 60;

pub const MAX_PRICE: u64 = 500_000_000_000;

pub const MIN_SUGGESTED_PRIORITY_FEE: u64 = 1_000_000;

pub const MAX_HISTORY: u64 = 1024;

pub const MAX_HISTORY_WITH_REWARDS: u64 = 16;

pub const MAX_PERCENTILES: usize = 100;

pub const INITIAL_BASE_FEE: u64 = 1_000_000_000;

pub const BLOB_BASE_COST: u64 = 1 << 13;

pub const MAINNET_LONDON: u64 = 12_965_000;

pub const MAINNET_OSAKA: u64 = 1_764_798_551;

const OP_CANYON: u64 = 1_704_992_401;
const OP_HOLOCENE: u64 = 1_736_445_601;
const INK_HOLOCENE: u64 = 1_742_396_400;
const OP_JOVIAN: u64 = 1_764_691_201;
const OP_ELASTICITY: u64 = 6;
const ETH_ELASTICITY: u64 = 2;
const ETH_DENOMINATOR: u64 = 8;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpStack {
    pub london_block: u64,
    pub canyon_time: u64,
    pub holocene_time: u64,
    pub jovian_time: u64,
    pub elasticity: u64,
    pub denominator: u64,
    pub denominator_canyon: u64,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rules {
    Ethereum,
    OpStack(OpStack),
    Nitro,
}


pub fn rules(chain_id: u64) -> Option<Rules> {
    let op = |london_block, canyon_time, holocene_time, denominator| {
        Rules::OpStack(OpStack {
            london_block,
            canyon_time,
            holocene_time,
            jovian_time: OP_JOVIAN,
            elasticity: OP_ELASTICITY,
            denominator,
            denominator_canyon: 250,
        })
    };
    match chain_id {
        1 => Some(Rules::Ethereum),
        10 => Some(op(105_235_063, OP_CANYON, OP_HOLOCENE, 50)),
        8453 => Some(op(0, OP_CANYON, OP_HOLOCENE, 50)),
        130 => Some(op(0, 0, OP_HOLOCENE, 50)),
        57073 => Some(op(0, 0, INK_HOLOCENE, 250)),
        42161 | 4663 => Some(Rules::Nitro),
        _ => None,
    }
}


fn covered(chain_id: u64) -> Result<Rules> {
    rules(chain_id).ok_or_else(|| {
        reject(
            Fault::Malformed,
            format!("chain {chain_id} has no fee rules"),
        )
    })
}

fn quantity(n: u64) -> String {
    format!("0x{n:x}")
}

fn hex_text(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

fn quantity_or_null(v: Option<U256>) -> Value {
    v.map_or(Value::Null, |x| Value::String(qu(x)))
}





pub fn next_base_fee(chain_id: u64, h: &Header) -> Result<Option<U256>> {
    match covered(chain_id)? {
        Rules::Ethereum => {
            let Some(fee) = london_gate(h.number, MAINNET_LONDON) else {
                return Ok(None);
            };
            if let Some(first) = fee {
                return Ok(Some(first));
            }
            base_fee_step(
                h.base_fee_per_gas,
                h.gas_limit,
                h.gas_used,
                ETH_ELASTICITY,
                ETH_DENOMINATOR,
            )
            .map(Some)
        }
        Rules::OpStack(op) => op_next_base_fee(&op, h),
        Rules::Nitro => Ok(h.base_fee_per_gas.map(U256::from)),
    }
}




fn london_gate(number: u64, london: u64) -> Option<Option<U256>> {
    if number.saturating_add(1) < london {
        None
    } else if number < london {
        Some(Some(U256::from(INITIAL_BASE_FEE)))
    } else {
        Some(None)
    }
}



fn base_fee_step(
    base: Option<u64>,
    limit: u64,
    metered: u64,
    elasticity: u64,
    denominator: u64,
) -> Result<U256> {
    let base = base.ok_or_else(|| anyhow!("a parent after London without a base fee"))?;
    if elasticity == 0 || denominator == 0 {
        bail!("EIP-1559 elasticity {elasticity}, denominator {denominator}");
    }
    let target = limit / elasticity;
    if target == 0 {
        bail!("a gas target of 0");
    }
    let parent = U256::from(base);
    let change = |gap: u64| U256::from(gap) * parent / U256::from(target) / U256::from(denominator);
    Ok(match metered.cmp(&target) {
        Ordering::Equal => parent,
        Ordering::Greater => parent + change(metered - target).max(U256::from(1u8)),
        Ordering::Less => parent.saturating_sub(change(target - metered)),
    })
}

fn be_u32(b: &[u8]) -> u64 {
    u64::from(u32::from_be_bytes(b.try_into().expect("4 bytes")))
}

fn be_u64(b: &[u8]) -> u64 {
    u64::from_be_bytes(b.try_into().expect("8 bytes"))
}

fn op_next_base_fee(op: &OpStack, h: &Header) -> Result<Option<U256>> {
    let Some(fee) = london_gate(h.number, op.london_block) else {
        return Ok(None);
    };
    if let Some(first) = fee {
        return Ok(Some(first));
    }

    let mut denominator = if h.timestamp.saturating_add(1) >= op.canyon_time {
        op.denominator_canyon
    } else {
        op.denominator
    };
    let mut elasticity = op.elasticity;
    let mut minimum = None;
    let jovian = h.timestamp >= op.jovian_time;
    if h.timestamp >= op.holocene_time {

        let x: &[u8] = h.extra_data.as_ref();
        (denominator, elasticity) = match (x.len(), jovian) {
            (9, _) | (17, true) => (be_u32(&x[1..5]), be_u32(&x[5..9])),
            _ => (0, 0),
        };
        if jovian && x.len() == 17 {
            minimum = Some(U256::from(be_u64(&x[9..17])));
        }
    }
    let metered = if jovian {
        let blob = h
            .blob_gas_used
            .ok_or_else(|| anyhow!("a Jovian parent block without blobGasUsed"))?;
        h.gas_used.max(blob)
    } else {
        h.gas_used
    };
    let next = base_fee_step(
        h.base_fee_per_gas,
        h.gas_limit,
        metered,
        elasticity,
        denominator,
    )?;
    Ok(Some(match minimum {
        Some(m) if next < m => m,
        _ => next,
    }))
}





pub fn blob_base_fee(chain_id: u64, h: &Header, blobs: &BlobSchedule) -> Result<Option<U256>> {
    let Some(excess) = h.excess_blob_gas else {
        return Ok(None);
    };
    match covered(chain_id)? {
        Rules::Ethereum => blobs
            .price(h.timestamp, excess)
            .map(Some)
            .ok_or_else(|| anyhow!("no blob schedule at time {}", h.timestamp)),
        Rules::OpStack(_) => {
            if excess != 0 {
                bail!("an OP-stack block with excessBlobGas {excess}");
            }
            Ok(Some(U256::from(1u8)))
        }
        Rules::Nitro => Ok(None),
    }
}


pub fn excess_blob_gas(
    chain_id: u64,
    parent: &Header,
    head_timestamp: u64,
    blobs: &BlobSchedule,
) -> Result<u64> {
    match covered(chain_id)? {
        Rules::Ethereum => {}
        Rules::OpStack(_) => return Ok(0),
        Rules::Nitro => bail!("Nitro has no blob gas"),
    }
    let t = head_timestamp;
    let fork = blobs
        .at(t)
        .ok_or_else(|| anyhow!("no blob schedule at time {t}"))?;
    let (Some(target), Some(max)) = (fork.target, fork.max) else {
        bail!("no blob target and max for the fork at {}", fork.at);
    };
    let (excess, used) = match parent.excess_blob_gas {
        Some(x) => (
            x,
            parent
                .blob_gas_used
                .ok_or_else(|| anyhow!("excessBlobGas without blobGasUsed"))?,
        ),
        None => (0, 0),
    };
    let past_64_bits = || anyhow!("blob gas past 64 bits");
    let total = excess.checked_add(used).ok_or_else(past_64_bits)?;
    let target_gas = target.wrapping_mul(GAS_PER_BLOB);
    if total < target_gas {
        return Ok(0);
    }
    if t >= MAINNET_OSAKA {

        let base = parent
            .base_fee_per_gas
            .ok_or_else(|| anyhow!("a parent without a base fee"))?;
        let reserve = U256::from(BLOB_BASE_COST) * U256::from(base);
        let price = blobs
            .price(t, excess)
            .ok_or_else(|| anyhow!("excess blob gas out of range"))?;
        if reserve > price * U256::from(GAS_PER_BLOB) {
            let share = u128::from(used) * u128::from(max.wrapping_sub(target)) / u128::from(max);
            return u64::try_from(share)
                .ok()
                .and_then(|s| excess.checked_add(s))
                .ok_or_else(past_64_bits);
        }
    }
    Ok(total - target_gas)
}


pub fn max_blob_gas(chain_id: u64, timestamp: u64, blobs: &BlobSchedule) -> u64 {
    if chain_id != 1 {
        return 0;
    }
    blobs
        .at(timestamp)
        .and_then(|f| f.max)
        .map_or(0, |m| m.wrapping_mul(GAS_PER_BLOB))
}






pub fn effective_tip(tx: &Decoded, base_fee: Option<U256>) -> Result<U256> {
    let Decoded::Eth(env) = tx else {
        return Ok(U256::ZERO);
    };

    let fee_cap = U256::from(env.max_fee_per_gas());
    let tip_cap = env.max_priority_fee_per_gas().map_or(fee_cap, U256::from);
    let Some(base) = base_fee else {
        return Ok(tip_cap);
    };
    check!(
        fee_cap >= base,
        Fault::Transactions,
        "a transaction priced under its block's base fee"
    );
    Ok(tip_cap.min(fee_cap - base))
}



fn block_tips(v: &Verified) -> Result<Vec<(U256, Decoded)>> {
    let base = v.header.base_fee_per_gas.map(U256::from);
    v.transactions
        .iter()
        .map(|(i, raw)| {
            let tx = txs::decode(raw, v.chain_id).with_context(|| format!("transaction {i}"))?;
            let tip = effective_tip(&tx, base)?;
            Ok((tip, tx))
        })
        .collect()
}


fn receipt_gas(v: &Verified) -> Result<Vec<u64>> {
    let mut before = 0u64;
    v.receipts
        .values()
        .map(|raw| {
            let cumulative = decode_receipt(raw, v.chain_id)
                .tag(Fault::Receipts)?
                .cumulative_gas_used;
            let used = cumulative.saturating_sub(before);
            before = cumulative;
            Ok(used)
        })
        .collect()
}



pub(crate) fn ethereum_tip(bodies: &[Verified]) -> Result<U256> {
    let mut sample: Vec<U256> = Vec::new();
    for v in bodies.iter().filter(|v| v.block_number != 0) {
        let mut tips = block_tips(v)?;
        tips.sort_by_key(|(tip, _)| *tip);
        let producer = v.header.beneficiary;
        let lowest = tips
            .iter()
            .filter(|(tip, _)| *tip >= U256::from(IGNORE_PRICE))
            .filter(|(_, tx)| matches!(txs::sender(tx), Ok(s) if Address::from(s) != producer))
            .map(|(tip, _)| *tip)
            .take(SAMPLE_NUMBER);
        sample.extend(lowest);
    }
    let tip = if sample.is_empty() {
        U256::from(START_PRICE)
    } else {
        sample.sort();
        sample[(sample.len() - 1) * PERCENTILE / 100]
    };
    Ok(tip.min(U256::from(MAX_PRICE)))
}



pub(crate) fn op_tip(v: &Verified) -> Result<U256> {
    let minimum = U256::from(MIN_SUGGESTED_PRIORITY_FEE);
    let largest = receipt_gas(v)?.into_iter().max().unwrap_or(0);
    let h = &v.header;
    if largest > h.gas_limit {
        return Ok(minimum);
    }
    let mut tip = minimum;
    if h.gas_used.saturating_add(largest) > h.gas_limit {
        let mut tips: Vec<U256> = block_tips(v)?.into_iter().map(|(t, _)| t).collect();
        if tips.is_empty() {
            return Ok(minimum);
        }
        tips.sort();
        let median = tips[tips.len() / 2];
        tip = tip.max(median + median / U256::from(10u8));
    }
    Ok(tip.min(U256::from(MAX_PRICE)))
}






#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeProof {
    pub chain_id: String,
    pub formula: String,
    pub block_number: String,
    pub block_hash: String,
    pub headers: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<BlockProof>,
}


pub type FeeHistoryProof = FeeProof;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bodies {
    None,
    Blocks,
    BlockReceipts,
}


pub fn fee_proof_shape(chain_id: u64, h: u64) -> Result<(RangeInclusive<u64>, Bodies)> {
    Ok(match covered(chain_id)? {
        Rules::Ethereum => (h.saturating_sub(TIP_BLOCKS - 1)..=h, Bodies::Blocks),
        Rules::OpStack(_) => (h..=h, Bodies::BlockReceipts),
        Rules::Nitro => (h..=h, Bodies::None),
    })
}


#[derive(Clone, Debug, PartialEq)]
pub struct FeeHistoryRequest {
    pub block_count: u64,
    pub newest: Block,
    pub reward_percentiles: Vec<f64>,
}

impl FeeHistoryRequest {

    pub fn check(&self) -> Result<()> {
        let p = &self.reward_percentiles;
        check!(
            p.len() <= MAX_PERCENTILES,
            Fault::Malformed,
            "invalid reward percentile: over the query limit {MAX_PERCENTILES}"
        );
        for (i, &x) in p.iter().enumerate() {
            check!(
                (0.0..=100.0).contains(&x),
                Fault::Malformed,
                "invalid reward percentile: {x}"
            );
            if i > 0 {
                let before = p[i - 1];
                check!(
                    x.partial_cmp(&before) == Some(Ordering::Greater),
                    Fault::Malformed,
                    "invalid reward percentile: #{}:{before} >= #{i}:{x}",
                    i - 1
                );
            }
        }
        check!(
            self.block_count != 0,
            Fault::Malformed,
            "blockCount 0 needs no proof"
        );
        Ok(())
    }


    pub fn count(&self) -> u64 {
        self.block_count.min(MAX_HISTORY)
    }


    pub fn with_bodies(&self, chain_id: u64) -> bool {
        !self.reward_percentiles.is_empty() && rules(chain_id) != Some(Rules::Nitro)
    }
}


pub fn fee_history_shape(
    chain_id: u64,
    newest: u64,
    req: &FeeHistoryRequest,
) -> Result<(RangeInclusive<u64>, Bodies)> {
    covered(chain_id)?;
    req.check()?;
    let k = req.count().min(newest.saturating_add(1));
    let bodies = req.with_bodies(chain_id);
    check!(
        !bodies || k <= MAX_HISTORY_WITH_REWARDS,
        Fault::Malformed,
        "a fee-history proof with rewards covers at most {MAX_HISTORY_WITH_REWARDS} blocks, not {k}"
    );
    let kind = if bodies {
        Bodies::BlockReceipts
    } else {
        Bodies::None
    };
    Ok((newest - (k - 1)..=newest, kind))
}




















struct Chain {
    headers: Vec<Header>,
    hashes: Vec<[u8; 32]>,
}

impl Chain {
    fn head(&self) -> (&Header, [u8; 32]) {
        let last = self.headers.len() - 1;
        (&self.headers[last], self.hashes[last])
    }


    fn covers(&self, range: &RangeInclusive<u64>, what: impl FnOnce() -> String) -> Result<()> {
        let count = self.headers.len() as u64;
        let first = self.headers[0].number;
        check!(
            first == *range.start() && count - 1 == range.end() - range.start(),
            Fault::Range,
            "{count} headers from block {first}: {} covers blocks {}..={}",
            what(),
            range.start(),
            range.end()
        );
        Ok(())
    }
}


fn verify_headers(p: &FeeProof, chain_id: u64, asked: Block) -> Result<Chain> {
    let got = parse_qty(&p.chain_id, "chainId")?;
    check!(
        got == chain_id,
        Fault::Chain,
        "the proof is for chain {got}, not {chain_id}"
    );
    check!(
        p.formula == FORMULA,
        Fault::Formula,
        "formula {:?} is not {FORMULA}",
        p.formula
    );
    check!(!p.headers.is_empty(), Fault::Range, "no headers");
    check!(
        p.headers.len() as u64 <= MAX_HISTORY,
        Fault::Range,
        "{} headers, more than any request asks for",
        p.headers.len()
    );
    let mut chain = Chain {
        headers: Vec::with_capacity(p.headers.len()),
        hashes: Vec::with_capacity(p.headers.len()),
    };
    for (i, text) in p.headers.iter().enumerate() {
        let bytes = parse_bytes(text, &format!("header {i}"))?;
        let h = decode_header(&bytes)
            .map_err(|e| reject(Fault::Header, format!("header {i}: {e:#}")))?;
        if i > 0 {
            let (parent, parent_hash) = (&chain.headers[i - 1], chain.hashes[i - 1]);
            check!(
                h.parent_hash.0 == parent_hash && parent.number.checked_add(1) == Some(h.number),
                Fault::Link,
                "header {i} (block {}) is not the child of header {} (block {})",
                h.number,
                i - 1,
                parent.number
            );
        }
        chain.hashes.push(crate::keccak(&bytes));
        chain.headers.push(h);
    }
    let (head, hash) = chain.head();
    check!(
        parse_bytes(&p.block_hash, "blockHash")?[..] == hash[..],
        Fault::Header,
        "blockHash is not the keccak of the last header"
    );
    check!(
        parse_qty(&p.block_number, "blockNumber")? == head.number,
        Fault::Header,
        "blockNumber is not the last header's"
    );
    match asked {
        Block::Number(n) => check!(
            head.number == n,
            Fault::Block,
            "the proof is for block {}, not {n}",
            head.number
        ),
        Block::Hash(want) => check!(
            hash == want,
            Fault::Block,
            "the proof is for block 0x{}, not 0x{}",
            hex::encode(hash),
            hex::encode(want)
        ),
        Block::Tag => {}
    }
    Ok(chain)
}


fn verify_bodies(
    p: &FeeProof,
    chain_id: u64,
    kind: Bodies,
    hashes: &[[u8; 32]],
) -> Result<Vec<Verified>> {
    let subject: fn(Block) -> Subject = match kind {
        Bodies::None => {
            check!(
                p.bodies.is_empty(),
                Fault::Malformed,
                "bodies in a proof that needs none"
            );
            return Ok(Vec::new());
        }
        Bodies::Blocks => Subject::Block,
        Bodies::BlockReceipts => Subject::BlockReceipts,
    };
    let (b, h) = (p.bodies.len(), hashes.len());
    check!(b <= h, Fault::Malformed, "{b} bodies for {h} headers");
    check!(b >= h, Fault::Missing, "{b} bodies for {h} headers");
    p.bodies
        .iter()
        .zip(hashes)
        .enumerate()
        .map(|(i, (body, hash))| {
            let claimed = parse_bytes(&body.block_hash, &format!("body {i} blockHash"))?;
            check!(
                claimed[..] == hash[..],
                Fault::Body,
                "body {i} is for block 0x{}, not header {i}'s 0x{}",
                hex::encode(&claimed),
                hex::encode(hash)
            );
            verify(body, chain_id, &subject(Block::Hash(*hash)))
                .with_context(|| format!("body {i}"))
        })
        .collect()
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fees {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub base_fee: Option<U256>,
    pub next_base_fee: Option<U256>,
    pub tip: U256,
    pub gas_price: U256,
    pub blob_base_fee: Option<U256>,
}

impl Fees {

    pub fn json(&self) -> Value {
        json!({
            "formula": FORMULA,
            "blockNumber": quantity(self.block_number),
            "blockHash": hex_text(&self.block_hash),
            "eth_gasPrice": qu(self.gas_price),
            "eth_maxPriorityFeePerGas": qu(self.tip),
            "eth_blobBaseFee": quantity_or_null(self.blob_base_fee),
            "baseFeePerGas": quantity_or_null(self.base_fee),
            "nextBaseFeePerGas": quantity_or_null(self.next_base_fee),
        })
    }
}


pub fn verify_fees(p: &FeeProof, chain_id: u64, block: Block) -> Result<Fees> {
    covered(chain_id)?;
    verify_fees_with(p, chain_id, block, &BlobSchedule::builtin(chain_id))
}


pub fn verify_fees_with(
    p: &FeeProof,
    chain_id: u64,
    block: Block,
    blobs: &BlobSchedule,
) -> Result<Fees> {
    let rules = covered(chain_id)?;
    let chain = verify_headers(p, chain_id, block)?;
    let (head, hash) = chain.head();
    let (range, kind) = fee_proof_shape(chain_id, head.number)?;
    chain.covers(&range, || format!("a fee proof at block {}", head.number))?;
    let bodies = verify_bodies(p, chain_id, kind, &chain.hashes)?;
    let tip = match rules {
        Rules::Ethereum => ethereum_tip(&bodies),
        Rules::OpStack(_) => op_tip(&bodies[0]),
        Rules::Nitro => Ok(U256::ZERO),
    }
    .tag(Fault::Transactions)?;
    let next = next_base_fee(chain_id, head).tag(Fault::Header)?;
    let blob = blob_base_fee(chain_id, head, blobs).tag(Fault::Header)?;
    Ok(Fees {
        chain_id,
        block_number: head.number,
        block_hash: hash,
        base_fee: head.base_fee_per_gas.map(U256::from),
        next_base_fee: next,
        tip,
        gas_price: next.unwrap_or_default() + tip,
        blob_base_fee: blob,
    })
}


#[derive(Clone, Debug, PartialEq)]
pub struct FeeHistory {
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub oldest_block: u64,
    pub reward: Option<Vec<Vec<U256>>>,
    pub base_fee_per_gas: Vec<U256>,
    pub gas_used_ratio: Vec<f64>,
    pub base_fee_per_blob_gas: Option<Vec<U256>>,
    pub blob_gas_used_ratio: Option<Vec<f64>>,
}

impl FeeHistory {

    pub fn json(&self) -> Value {
        let quantities =
            |v: &[U256]| Value::Array(v.iter().map(|x| Value::String(qu(*x))).collect());
        let numbers = |v: &[f64]| Value::Array(v.iter().map(|&x| Value::from(x)).collect());
        let mut o = Map::new();
        o.insert(
            "oldestBlock".into(),
            Value::String(quantity(self.oldest_block)),
        );
        if let Some(rows) = &self.reward {
            o.insert(
                "reward".into(),
                Value::Array(rows.iter().map(|r| quantities(r)).collect()),
            );
        }
        o.insert("baseFeePerGas".into(), quantities(&self.base_fee_per_gas));
        o.insert("gasUsedRatio".into(), numbers(&self.gas_used_ratio));
        if let Some(v) = &self.base_fee_per_blob_gas {
            o.insert("baseFeePerBlobGas".into(), quantities(v));
        }
        if let Some(v) = &self.blob_gas_used_ratio {
            o.insert("blobGasUsedRatio".into(), numbers(v));
        }
        Value::Object(o)
    }
}



pub fn verify_fee_history(
    p: &FeeHistoryProof,
    chain_id: u64,
    req: &FeeHistoryRequest,
) -> Result<FeeHistory> {
    covered(chain_id)?;
    verify_fee_history_with(p, chain_id, req, &BlobSchedule::builtin(chain_id))
}


pub fn verify_fee_history_with(
    p: &FeeHistoryProof,
    chain_id: u64,
    req: &FeeHistoryRequest,
    blobs: &BlobSchedule,
) -> Result<FeeHistory> {
    let rules = covered(chain_id)?;
    req.check()?;
    let chain = verify_headers(p, chain_id, req.newest)?;
    let (last, hash) = chain.head();
    let (range, kind) = fee_history_shape(chain_id, last.number, req)?;
    chain.covers(&range, || "the request".to_string())?;
    let bodies = verify_bodies(p, chain_id, kind, &chain.hashes)?;

    let n = chain.headers.len();
    let mut base_fee_per_gas = Vec::with_capacity(n + 1);
    let mut gas_used_ratio = Vec::with_capacity(n);
    let mut base_fee_per_blob_gas = Vec::with_capacity(n + 1);
    let mut blob_gas_used_ratio = Vec::with_capacity(n);
    for h in &chain.headers {
        base_fee_per_gas.push(U256::from(h.base_fee_per_gas.unwrap_or(0)));
        gas_used_ratio.push(h.gas_used as f64 / h.gas_limit as f64);
        base_fee_per_blob_gas.push(
            blob_base_fee(chain_id, h, blobs)
                .tag(Fault::Header)?
                .unwrap_or_default(),
        );
        let max = max_blob_gas(chain_id, h.timestamp, blobs);
        blob_gas_used_ratio.push(match h.blob_gas_used {
            Some(used) if max != 0 => used as f64 / max as f64,
            _ => 0.0,
        });
    }


    base_fee_per_gas.push(
        next_base_fee(chain_id, last)
            .tag(Fault::Header)?
            .unwrap_or_default(),
    );
    let next_blob_fee = if last.excess_blob_gas.is_some() {
        let excess = excess_blob_gas(chain_id, last, last.timestamp, blobs).tag(Fault::Header)?;
        let mut child = last.clone();
        child.excess_blob_gas = Some(excess);
        blob_base_fee(chain_id, &child, blobs)
            .tag(Fault::Header)?
            .unwrap_or_default()
    } else {
        U256::ZERO
    };
    base_fee_per_blob_gas.push(next_blob_fee);

    let percentiles = &req.reward_percentiles;
    let nitro = rules == Rules::Nitro;
    let reward = if percentiles.is_empty() {
        None
    } else if nitro {
        Some(vec![vec![U256::ZERO; percentiles.len()]; n])
    } else {
        let rows = bodies
            .iter()
            .zip(&chain.headers)
            .map(|(v, h)| block_rewards(v, h.gas_used, percentiles).tag(Fault::Transactions))
            .collect::<Result<Vec<_>>>()?;
        Some(rows)
    };

    Ok(FeeHistory {
        block_number: last.number,
        block_hash: hash,
        oldest_block: chain.headers[0].number,
        reward,
        base_fee_per_gas,
        gas_used_ratio,
        base_fee_per_blob_gas: (!nitro).then_some(base_fee_per_blob_gas),
        blob_gas_used_ratio: (!nitro).then_some(blob_gas_used_ratio),
    })
}



fn block_rewards(v: &Verified, gas_used: u64, percentiles: &[f64]) -> Result<Vec<U256>> {
    let tips = block_tips(v)?;
    if tips.is_empty() {
        return Ok(vec![U256::ZERO; percentiles.len()]);
    }
    let gas = receipt_gas(v)?;
    check!(
        gas.len() == tips.len(),
        Fault::Missing,
        "{} receipts for {} transactions",
        gas.len(),
        tips.len()
    );
    let mut pairs: Vec<(U256, u64)> = tips.into_iter().map(|(t, _)| t).zip(gas).collect();
    pairs.sort_by_key(|(tip, _)| *tip);
    let last = pairs.len() - 1;
    let (mut j, mut sum) = (0usize, pairs[0].1);
    let mut row = Vec::with_capacity(percentiles.len());
    for &p in percentiles {
        let threshold = (gas_used as f64 * p / 100.0) as u64;
        while sum < threshold && j < last {
            j += 1;
            sum = sum.saturating_add(pairs[j].1);
        }
        row.push(pairs[j].0);
    }
    Ok(row)
}















