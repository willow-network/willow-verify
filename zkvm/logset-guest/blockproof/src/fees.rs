//! `willow-fee-v1`: `eth_gasPrice`, `eth_maxPriorityFeePerGas`,
//! `eth_blobBaseFee` and `eth_feeHistory` as a published formula over verified
//! blocks, and the proofs a client recomputes them from
//! (`willow_getFeeProof`, `willow_getFeeHistoryProof`).
//!
//! Ported line for line from geth v1.17.10 (`consensus/misc/eip1559`,
//! `consensus/misc/eip4844`, `eth/gasprice/gasprice.go`, `feehistory.go`) and
//! op-geth v1.101702.3 (`consensus/misc/eip1559`, `optimism-gasprice.go`),
//! with deviations on purpose, so the formula is stateless and its proof
//! small: Ethereum samples 4 blocks, not 20, and never reads further back
//! (geth reads up to twice as many when a block yields at most one tip); a
//! block without a tip to sample adds nothing (geth adds its last price), and
//! an empty sample gives a constant; `eth_gasPrice` adds the *next* block's
//! base fee. Nitro (OffchainLabs go-ethereum `arbitrum/apibackend.go`): no
//! tips, the next base fee is the head's, fee-history rewards are zero and
//! blob fields absent, as Nitro reports them; its `gasUsedRatio` is geth's
//! `gasUsed / gasLimit`, as Nitro's own figure comes from ArbOS state no
//! header commits to.

use crate::derive::{BlobSchedule, GAS_PER_BLOB};
use crate::fault::{check, reject, Fault, Tag};
use crate::proof::{
    decode_header, parse_bytes, parse_qty, verify, Block, BlockProof, Subject, Verified,
};
use crate::render::{decode_receipt, qu};
use crate::txs::{self, Decoded};
use alloy::consensus::{Header, Transaction as _};
use alloy::primitives::U256;
use anyhow::{Context as _, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// The formula's name, which every fee and fee-history proof carries.
pub const FORMULA: &str = "willow-fee-v1";
/// Ethereum: blocks sampled for the tip (geth's default is 20).
pub const TIP_BLOCKS: u64 = 4;
/// geth `sampleNumber`: tips kept per block.
pub const SAMPLE_NUMBER: usize = 3;
/// geth `DefaultIgnorePrice`: tips under it are not sampled.
pub const IGNORE_PRICE: u64 = 2;
/// The tip when nothing was sampled: geth's start price, `Miner.GasPrice` (0.001 gwei).
pub const START_PRICE: u64 = 1_000_000;
/// geth `FullNodeGPO.Percentile`.
pub const PERCENTILE: usize = 60;
/// geth `DefaultMaxPrice`: 500 gwei.
pub const MAX_PRICE: u64 = 500_000_000_000;
/// op-geth `DefaultMinSuggestedPriorityFee`: 0.001 gwei.
pub const MIN_SUGGESTED_PRIORITY_FEE: u64 = 1_000_000;
/// geth `FullNodeGPO.MaxHeaderHistory` and `MaxBlockHistory`: fee history's longest range.
pub const MAX_HISTORY: u64 = 1024;
/// Blocks one fee-history proof with rewards covers; a client splits a longer range.
pub const MAX_HISTORY_WITH_REWARDS: u64 = 16;
/// geth `maxQueryLimit`: reward percentiles per request.
pub const MAX_PERCENTILES: usize = 100;
/// EIP-1559's first base fee (geth `params.InitialBaseFee`).
pub const INITIAL_BASE_FEE: u64 = 1_000_000_000;
/// EIP-7918's `BLOB_BASE_COST` (geth `params.BlobBaseCost`).
pub const BLOB_BASE_COST: u64 = 1 << 13;
/// Ethereum mainnet's London block and Osaka time (geth v1.17.10 `chainspecs/mainnet.json`).
pub const MAINNET_LONDON: u64 = 12_965_000;
pub const MAINNET_OSAKA: u64 = 1_764_798_551;

/// An OP-stack chain's EIP-1559 configuration, from op-geth v1.101702.3's
/// superchain registry (Base's from the registry before its removal there,
/// superchain-registry 5b055be4).
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

/// How a chain prices its next block and its tips.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rules {
    Ethereum,
    OpStack(OpStack),
    Nitro,
}

const SUPERCHAIN_2025: OpStack = OpStack {
    london_block: 0,
    canyon_time: 1_704_992_401,
    holocene_time: 1_736_445_601,
    jovian_time: 1_764_691_201,
    elasticity: 6,
    denominator: 50,
    denominator_canyon: 250,
};

/// The fee rules of `chain_id`, `None` for a chain Willow does not serve.
pub fn rules(chain_id: u64) -> Option<Rules> {
    Some(match chain_id {
        1 => Rules::Ethereum,
        10 => Rules::OpStack(OpStack {
            london_block: 105_235_063,
            ..SUPERCHAIN_2025
        }),
        8453 => Rules::OpStack(SUPERCHAIN_2025),
        130 => Rules::OpStack(OpStack {
            canyon_time: 0,
            ..SUPERCHAIN_2025
        }),
        57073 => Rules::OpStack(OpStack {
            canyon_time: 0,
            holocene_time: 1_742_396_400,
            denominator: 250,
            ..SUPERCHAIN_2025
        }),
        42161 | 4663 => Rules::Nitro,
        _ => return None,
    })
}

fn rules_of(chain_id: u64) -> Result<Rules> {
    rules(chain_id).ok_or_else(|| {
        reject(
            Fault::Malformed,
            format!("chain {chain_id} has no fee rules"),
        )
    })
}

/// op-geth `DecodeOptimismExtraData` at the parent's time: the denominator,
/// elasticity and (from Jovian) minimum base fee the parent's extraData sets.
fn op_extra_data(c: &OpStack, time: u64, extra: &[u8]) -> (u64, u64, Option<u64>) {
    let params = |p: &[u8]| {
        (
            u64::from(u32::from_be_bytes(p[..4].try_into().expect("4 bytes"))),
            u64::from(u32::from_be_bytes(p[4..8].try_into().expect("4 bytes"))),
        )
    };
    if time >= c.jovian_time {
        match extra.len() {
            9 => {
                let (d, e) = params(&extra[1..9]);
                (d, e, None)
            }
            17 => {
                let (d, e) = params(&extra[1..9]);
                (
                    d,
                    e,
                    Some(u64::from_be_bytes(
                        extra[9..17].try_into().expect("8 bytes"),
                    )),
                )
            }
            _ => (0, 0, None),
        }
    } else if time >= c.holocene_time {
        match extra.len() {
            9 => {
                let (d, e) = params(&extra[1..9]);
                (d, e, None)
            }
            _ => (0, 0, None),
        }
    } else {
        (0, 0, None)
    }
}

/// geth `calcBaseFeeInner` / `CalcBaseFee` past London: the parent's base fee
/// moved by how far its (metered) gas is from the target.
fn base_fee_after(
    parent: &Header,
    metered: u64,
    elasticity: u64,
    denominator: u64,
) -> Result<U256> {
    let base = U256::from(
        parent
            .base_fee_per_gas
            .context("a parent after London without a base fee")?,
    );
    anyhow::ensure!(
        elasticity > 0 && denominator > 0,
        "EIP-1559 elasticity {elasticity}, denominator {denominator}"
    );
    let target = parent.gas_limit / elasticity;
    anyhow::ensure!(target > 0, "a gas target of 0");
    if metered == target {
        return Ok(base);
    }
    if metered > target {
        let num =
            U256::from(metered - target) * base / U256::from(target) / U256::from(denominator);
        if num < U256::from(1u8) {
            return Ok(base + U256::from(1u8));
        }
        Ok(base + num)
    } else {
        let num =
            U256::from(target - metered) * base / U256::from(target) / U256::from(denominator);
        Ok(base.saturating_sub(num))
    }
}

/// The base fee of the block after `h`: geth `CalcBaseFee(config, h)`;
/// op-geth's `CalcBaseFee(config, h, h.Time + 1)` (its fee history's time;
/// from Holocene the parameters come from `h`'s extraData, from Jovian with a
/// minimum and the DA footprint in `blobGasUsed` metered); Nitro: `h`'s own,
/// which ArbOS sets from its state. `None` when the next block is before London.
pub fn next_base_fee(chain_id: u64, h: &Header) -> Result<Option<U256>> {
    match rules_of(chain_id)? {
        Rules::Ethereum => {
            if h.number.saturating_add(1) < MAINNET_LONDON {
                return Ok(None);
            }
            if h.number < MAINNET_LONDON {
                return Ok(Some(U256::from(INITIAL_BASE_FEE)));
            }
            base_fee_after(h, h.gas_used, 2, 8).map(Some)
        }
        Rules::OpStack(c) => {
            if h.number.saturating_add(1) < c.london_block {
                return Ok(None);
            }
            if h.number < c.london_block {
                return Ok(Some(U256::from(INITIAL_BASE_FEE)));
            }
            let time = h.timestamp.saturating_add(1);
            let mut elasticity = c.elasticity;
            let mut denominator = if time >= c.canyon_time {
                c.denominator_canyon
            } else {
                c.denominator
            };
            let mut min = None;
            if h.timestamp >= c.holocene_time {
                (denominator, elasticity, min) = op_extra_data(&c, h.timestamp, &h.extra_data);
            }
            let mut metered = h.gas_used;
            if h.timestamp >= c.jovian_time {
                let da = h
                    .blob_gas_used
                    .context("a Jovian parent block without blobGasUsed")?;
                if da > h.gas_used {
                    metered = da;
                }
            }
            let mut fee = base_fee_after(h, metered, elasticity, denominator)?;
            if let Some(m) = min {
                fee = fee.max(U256::from(m));
            }
            Ok(Some(fee))
        }
        Rules::Nitro => Ok(h.base_fee_per_gas.map(U256::from)),
    }
}

/// `eth_blobBaseFee` at `h`: geth `CalcBlobFee(config, head)`; op-geth's 1
/// for an OP-stack block (whose excessBlobGas is always 0); `None` (a `null`
/// response) for a header without excessBlobGas, Nitro's included.
pub fn blob_base_fee(chain_id: u64, h: &Header, blobs: &BlobSchedule) -> Result<Option<U256>> {
    let Some(excess) = h.excess_blob_gas else {
        return Ok(None);
    };
    match rules_of(chain_id)? {
        Rules::Ethereum => blobs
            .price(h.timestamp, excess)
            .map(Some)
            .with_context(|| format!("no blob schedule at time {}", h.timestamp)),
        Rules::OpStack(_) => {
            anyhow::ensure!(excess == 0, "an OP-stack block with excessBlobGas {excess}");
            Ok(Some(U256::from(1u8)))
        }
        Rules::Nitro => Ok(None),
    }
}

/// geth `CalcExcessBlobGas(config, parent, head_timestamp)`, EIP-7918 from
/// Osaka; op-geth's 0 on the OP stack.
pub fn excess_blob_gas(
    chain_id: u64,
    parent: &Header,
    head_timestamp: u64,
    blobs: &BlobSchedule,
) -> Result<u64> {
    match rules_of(chain_id)? {
        Rules::OpStack(_) => return Ok(0),
        Rules::Nitro => anyhow::bail!("Nitro has no blob gas"),
        Rules::Ethereum => {}
    }
    let is_osaka = head_timestamp >= MAINNET_OSAKA;
    let b = blobs
        .at(head_timestamp)
        .with_context(|| format!("no blob schedule at time {head_timestamp}"))?;
    let (target, max) = b
        .target
        .zip(b.max)
        .with_context(|| format!("no blob target and max for the fork at {}", b.at))?;
    let (parent_excess, parent_used) = match parent.excess_blob_gas {
        Some(x) => (
            x,
            parent
                .blob_gas_used
                .context("excessBlobGas without blobGasUsed")?,
        ),
        None => (0, 0),
    };
    let excess = parent_excess
        .checked_add(parent_used)
        .context("blob gas past 64 bits")?;
    let target_gas = target * GAS_PER_BLOB;
    if excess < target_gas {
        return Ok(0);
    }
    if is_osaka {
        let base = U256::from(
            parent
                .base_fee_per_gas
                .context("a parent without a base fee")?,
        );
        let reserve = U256::from(BLOB_BASE_COST) * base;
        let price = blobs
            .price(head_timestamp, parent_excess)
            .context("excess blob gas out of range")?
            * U256::from(GAS_PER_BLOB);
        if reserve > price {
            let scaled = u128::from(parent_used) * u128::from(max - target) / u128::from(max);
            return u64::try_from(scaled)
                .ok()
                .and_then(|s| parent_excess.checked_add(s))
                .context("blob gas past 64 bits");
        }
    }
    Ok(excess - target_gas)
}

/// The most blob gas a block at `timestamp` may use (geth `MaxBlobGasPerBlock`); 0 without blobs.
pub fn max_blob_gas(chain_id: u64, timestamp: u64, blobs: &BlobSchedule) -> u64 {
    match rules(chain_id) {
        Some(Rules::Ethereum) => blobs
            .at(timestamp)
            .and_then(|b| b.max)
            .map_or(0, |m| m * GAS_PER_BLOB),
        _ => 0,
    }
}

/// geth `EffectiveGasTip(baseFee)`: `min(tipCap, feeCap - baseFee)`; zero for
/// an OP-stack deposit (op-geth's) and for Nitro's own types.
pub fn effective_tip(tx: &Decoded, base_fee: Option<U256>) -> Result<U256> {
    match tx {
        Decoded::Eth(env) => {
            let fee_cap = U256::from(env.max_fee_per_gas());
            let tip_cap = U256::from(
                env.max_priority_fee_per_gas()
                    .unwrap_or_else(|| env.max_fee_per_gas()),
            );
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
        Decoded::Table { .. } => Ok(U256::ZERO),
    }
}

/// A verified body's transactions, decoded, each with its tip at the block's base fee.
fn tips(v: &Verified) -> Result<Vec<(U256, Decoded)>> {
    let base = v.header.base_fee_per_gas.map(U256::from);
    v.transactions
        .iter()
        .map(|(i, t)| {
            let d = txs::decode(t, v.chain_id).with_context(|| format!("transaction {i}"))?;
            Ok((effective_tip(&d, base)?, d))
        })
        .collect()
}

/// Each receipt's gas used (its cumulative gas less the previous one's).
fn gas_used(v: &Verified) -> Result<Vec<u64>> {
    let mut prev = 0u64;
    v.receipts
        .values()
        .map(|r| {
            let c = decode_receipt(r, v.chain_id)
                .tag(Fault::Receipts)?
                .cumulative_gas_used;
            let g = c.saturating_sub(prev);
            prev = c;
            Ok(g)
        })
        .collect()
}

/// Ethereum's tip: geth `SuggestTipCap` over `bodies` (oldest first, no
/// genesis), without state: per block the 3 lowest tips of at least 2 wei
/// from senders other than its miner (senders recovered only for those
/// looked at), then the 60th percentile, at most 500 gwei.
pub(crate) fn ethereum_tip(bodies: &[Verified]) -> Result<U256> {
    let mut sample = Vec::new();
    for v in bodies.iter().filter(|v| v.block_number > 0) {
        let miner = v.header.beneficiary.0 .0;
        let mut txs = tips(v)?;
        txs.sort_by_key(|a| a.0);
        let mut kept = 0;
        for (tip, tx) in &txs {
            if *tip < U256::from(IGNORE_PRICE) {
                continue;
            }
            if matches!(txs::sender(tx), Ok(from) if from != miner) {
                sample.push(*tip);
                kept += 1;
                if kept >= SAMPLE_NUMBER {
                    break;
                }
            }
        }
    }
    let price = if sample.is_empty() {
        U256::from(START_PRICE)
    } else {
        sample.sort();
        sample[(sample.len() - 1) * PERCENTILE / 100]
    };
    Ok(price.min(U256::from(MAX_PRICE)))
}

/// The OP stack's tip: op-geth `SuggestOptimismPriorityFee` at `v` (its
/// transactions and receipts): the minimum, or when the block was at
/// capacity 10% over its median tip if that is more, at most 500 gwei.
pub(crate) fn op_tip(v: &Verified) -> Result<U256> {
    let h = &v.header;
    let mut suggestion = U256::from(MIN_SUGGESTED_PRIORITY_FEE);
    let max_used = gas_used(v)?.into_iter().max().unwrap_or(0);
    if max_used > h.gas_limit {
        return Ok(suggestion);
    }
    if h.gas_used.saturating_add(max_used) > h.gas_limit {
        let mut t: Vec<U256> = tips(v)?.into_iter().map(|(t, _)| t).collect();
        if t.is_empty() {
            return Ok(suggestion);
        }
        t.sort();
        let median = t[t.len() / 2];
        let new = median + median / U256::from(10u8);
        if new > suggestion {
            suggestion = new;
        }
    }
    Ok(suggestion.min(U256::from(MAX_PRICE)))
}

/// `willow_getFeeProof` and `willow_getFeeHistoryProof`'s result: the header
/// RLPs of a range, oldest first, ending with the block named, and the bodies
/// the formula reads.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeeProof {
    pub chain_id: String,
    pub formula: String,
    /// The newest block: the last header's number and hash.
    pub block_number: String,
    pub block_hash: String,
    pub headers: Vec<String>,
    /// Fee proofs: Ethereum, a block proof of each header's block; the OP
    /// stack, a block-receipts proof of the newest; Nitro, none. Fee-history
    /// proofs with percentiles: a block-receipts proof of each (Nitro: none).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bodies: Vec<BlockProof>,
}

/// The same wire shape, for fee history.
pub type FeeHistoryProof = FeeProof;

/// What bodies a proof carries for each header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bodies {
    None,
    /// `willow_getBlockProof` results.
    Blocks,
    /// `willow_getBlockReceiptsProof` results.
    BlockReceipts,
}

/// The blocks a fee proof at block `h` covers (oldest first) and its bodies.
pub fn fee_proof_shape(chain_id: u64, h: u64) -> Result<(std::ops::RangeInclusive<u64>, Bodies)> {
    Ok(match rules_of(chain_id)? {
        Rules::Ethereum => (h.saturating_sub(TIP_BLOCKS - 1)..=h, Bodies::Blocks),
        Rules::OpStack(_) => (h..=h, Bodies::BlockReceipts),
        Rules::Nitro => (h..=h, Bodies::None),
    })
}

/// A fee-history request, as the client sends it to the proof method.
#[derive(Clone, Debug, PartialEq)]
pub struct FeeHistoryRequest {
    pub block_count: u64,
    pub newest: Block,
    pub reward_percentiles: Vec<f64>,
}

impl FeeHistoryRequest {
    /// geth's rules for the percentiles: at most 100, each in [0, 100], increasing.
    pub fn check(&self) -> Result<()> {
        let p = &self.reward_percentiles;
        check!(
            p.len() <= MAX_PERCENTILES,
            Fault::Malformed,
            "invalid reward percentile: over the query limit {MAX_PERCENTILES}"
        );
        for (i, x) in p.iter().enumerate() {
            check!(
                (0.0..=100.0).contains(x),
                Fault::Malformed,
                "invalid reward percentile: {x}"
            );
            if i > 0 {
                check!(
                    x.partial_cmp(&p[i - 1]) == Some(std::cmp::Ordering::Greater),
                    Fault::Malformed,
                    "invalid reward percentile: #{}:{} >= #{i}:{x}",
                    i - 1,
                    p[i - 1]
                );
            }
        }
        check!(
            self.block_count >= 1,
            Fault::Malformed,
            "blockCount 0 needs no proof"
        );
        Ok(())
    }

    /// Blocks asked for: geth's 1,024 at most.
    pub fn count(&self) -> u64 {
        self.block_count.min(MAX_HISTORY)
    }

    /// Whether the proof carries bodies (rewards on a chain with tips).
    pub fn with_bodies(&self, chain_id: u64) -> bool {
        !self.reward_percentiles.is_empty() && rules(chain_id) != Some(Rules::Nitro)
    }
}

/// The blocks a fee-history proof with newest block `newest` covers, and its bodies.
pub fn fee_history_shape(
    chain_id: u64,
    newest: u64,
    req: &FeeHistoryRequest,
) -> Result<(std::ops::RangeInclusive<u64>, Bodies)> {
    rules_of(chain_id)?;
    req.check()?;
    let count = req.count().min(newest.saturating_add(1));
    if req.with_bodies(chain_id) {
        check!(
            count <= MAX_HISTORY_WITH_REWARDS,
            Fault::Malformed,
            "a fee-history proof with rewards covers at most {MAX_HISTORY_WITH_REWARDS} blocks, not {count}"
        );
    }
    let bodies = if req.with_bodies(chain_id) {
        Bodies::BlockReceipts
    } else {
        Bodies::None
    };
    Ok((newest - (count - 1)..=newest, bodies))
}

/// A fee or fee-history proof from header RLPs (oldest first) and bodies.
pub fn build_fee_proof(
    chain_id: u64,
    headers: &[&[u8]],
    bodies: Vec<BlockProof>,
) -> Result<FeeProof> {
    let last = headers.last().context("no headers")?;
    let h = decode_header(last)?;
    Ok(FeeProof {
        chain_id: format!("0x{chain_id:x}"),
        formula: FORMULA.into(),
        block_number: format!("0x{:x}", h.number),
        block_hash: format!("0x{}", hex::encode(alloy_primitives::keccak256(last))),
        headers: headers
            .iter()
            .map(|h| format!("0x{}", hex::encode(h)))
            .collect(),
        bodies,
    })
}

/// A proof's headers, verified: each canonical, each its predecessor's child,
/// the last the stated block and the one asked for.
struct Headers {
    headers: Vec<Header>,
    hashes: Vec<[u8; 32]>,
}

fn verify_headers(p: &FeeProof, chain_id: u64, want: Block) -> Result<Headers> {
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
    let mut headers = Vec::with_capacity(p.headers.len());
    let mut hashes = Vec::with_capacity(p.headers.len());
    for (i, x) in p.headers.iter().enumerate() {
        let rlp = parse_bytes(x, &format!("header {i}"))?;
        let h =
            decode_header(&rlp).map_err(|e| reject(Fault::Header, format!("header {i}: {e:#}")))?;
        if let (Some(prev), Some(prev_hash)) = (headers.last(), hashes.last()) {
            let prev: &Header = prev;
            check!(
                h.parent_hash.0 == *prev_hash && prev.number.checked_add(1) == Some(h.number),
                Fault::Link,
                "header {i} (block {}) is not the child of header {} (block {})",
                h.number,
                i - 1,
                prev.number
            );
        }
        hashes.push(alloy_primitives::keccak256(&rlp).0);
        headers.push(h);
    }
    let (last, hash) = (headers.last().expect("one"), *hashes.last().expect("one"));
    check!(
        parse_bytes(&p.block_hash, "blockHash")? == hash,
        Fault::Header,
        "blockHash is not the keccak of the last header"
    );
    check!(
        parse_qty(&p.block_number, "blockNumber")? == last.number,
        Fault::Header,
        "blockNumber is not the last header's"
    );
    match want {
        Block::Number(n) => check!(
            last.number == n,
            Fault::Block,
            "the proof is for block {}, not {n}",
            last.number
        ),
        Block::Hash(w) => check!(
            hash == w,
            Fault::Block,
            "the proof is for block 0x{}, not 0x{}",
            hex::encode(hash),
            hex::encode(w)
        ),
        Block::Tag => {}
    }
    Ok(Headers { headers, hashes })
}

fn verify_bodies(p: &FeeProof, chain_id: u64, hs: &Headers, kind: Bodies) -> Result<Vec<Verified>> {
    if kind == Bodies::None {
        check!(
            p.bodies.is_empty(),
            Fault::Malformed,
            "bodies in a proof that needs none"
        );
        return Ok(vec![]);
    }
    check!(
        p.bodies.len() <= hs.hashes.len(),
        Fault::Malformed,
        "{} bodies for {} headers",
        p.bodies.len(),
        hs.hashes.len()
    );
    check!(
        p.bodies.len() == hs.hashes.len(),
        Fault::Missing,
        "{} bodies for {} headers",
        p.bodies.len(),
        hs.hashes.len()
    );
    p.bodies
        .iter()
        .zip(&hs.hashes)
        .enumerate()
        .map(|(i, (b, h))| {
            let got = parse_bytes(&b.block_hash, &format!("body {i} blockHash"))?;
            check!(
                got == *h,
                Fault::Body,
                "body {i} is for block 0x{}, not header {i}'s 0x{}",
                hex::encode(&got),
                hex::encode(h)
            );
            let s = match kind {
                Bodies::Blocks => Subject::Block(Block::Hash(*h)),
                _ => Subject::BlockReceipts(Block::Hash(*h)),
            };
            verify(b, chain_id, &s).map_err(|e| e.context(format!("body {i}")))
        })
        .collect()
}

/// What a fee proof proves at its block H.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fees {
    pub chain_id: u64,
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub base_fee: Option<U256>,
    pub next_base_fee: Option<U256>,
    /// `eth_maxPriorityFeePerGas`.
    pub tip: U256,
    /// `eth_gasPrice`: the next base fee plus the tip.
    pub gas_price: U256,
    /// `eth_blobBaseFee`; `None` is a `null` response.
    pub blob_base_fee: Option<U256>,
}

impl Fees {
    /// The three standard responses, and what they were computed from.
    pub fn json(&self) -> Value {
        json!({
            "formula": FORMULA,
            "blockNumber": format!("0x{:x}", self.block_number),
            "blockHash": format!("0x{}", hex::encode(self.block_hash)),
            "eth_gasPrice": qu(self.gas_price),
            "eth_maxPriorityFeePerGas": qu(self.tip),
            "eth_blobBaseFee": self.blob_base_fee.map(qu),
            "baseFeePerGas": self.base_fee.map(qu),
            "nextBaseFeePerGas": self.next_base_fee.map(qu),
        })
    }
}

/// Verify a fee proof as the response for block `block` on chain `chain_id` and
/// compute `willow-fee-v1` from it, with the chain's built-in blob schedule.
pub fn verify_fees(p: &FeeProof, chain_id: u64, block: Block) -> Result<Fees> {
    verify_fees_with(p, chain_id, block, &BlobSchedule::builtin(chain_id))
}

/// [`verify_fees`] with blob prices from `blobs` (a schedule configured past the built-in one).
pub fn verify_fees_with(
    p: &FeeProof,
    chain_id: u64,
    block: Block,
    blobs: &BlobSchedule,
) -> Result<Fees> {
    let rules = rules_of(chain_id)?;
    let hs = verify_headers(p, chain_id, block)?;
    let h = hs.headers.last().expect("one").clone();
    let (range, kind) = fee_proof_shape(chain_id, h.number)?;
    let want = range.end() - range.start() + 1;
    check!(
        hs.headers.len() as u64 == want && hs.headers[0].number == *range.start(),
        Fault::Range,
        "{} headers from block {}: a fee proof at block {} covers blocks {range:?}",
        hs.headers.len(),
        hs.headers[0].number,
        h.number
    );
    let bodies = verify_bodies(p, chain_id, &hs, kind)?;
    let tip = match rules {
        Rules::Ethereum => ethereum_tip(&bodies),
        Rules::OpStack(_) => op_tip(&bodies[0]),
        Rules::Nitro => Ok(U256::ZERO),
    }
    .tag(Fault::Transactions)?;
    let next = next_base_fee(chain_id, &h).tag(Fault::Header)?;
    let blob = blob_base_fee(chain_id, &h, blobs).tag(Fault::Header)?;
    Ok(Fees {
        chain_id,
        block_number: h.number,
        block_hash: *hs.hashes.last().expect("one"),
        base_fee: h.base_fee_per_gas.map(U256::from),
        next_base_fee: next,
        tip,
        gas_price: next.unwrap_or_default() + tip,
        blob_base_fee: blob,
    })
}

/// `eth_feeHistory` as a fee-history proof proves it (geth `FeeHistory`).
#[derive(Clone, Debug, PartialEq)]
pub struct FeeHistory {
    /// The newest block, which the client anchors.
    pub block_number: u64,
    pub block_hash: [u8; 32],
    pub oldest_block: u64,
    pub reward: Option<Vec<Vec<U256>>>,
    pub base_fee_per_gas: Vec<U256>,
    pub gas_used_ratio: Vec<f64>,
    /// Absent on Nitro, as Nitro reports it.
    pub base_fee_per_blob_gas: Option<Vec<U256>>,
    pub blob_gas_used_ratio: Option<Vec<f64>>,
}

impl FeeHistory {
    /// The standard response.
    pub fn json(&self) -> Value {
        let mut o = serde_json::Map::new();
        o.insert(
            "oldestBlock".into(),
            json!(format!("0x{:x}", self.oldest_block)),
        );
        if let Some(r) = &self.reward {
            let rows: Vec<Value> = r
                .iter()
                .map(|row| Value::Array(row.iter().map(|v| json!(qu(*v))).collect()))
                .collect();
            o.insert("reward".into(), Value::Array(rows));
        }
        let fees = |v: &[U256]| Value::Array(v.iter().map(|x| json!(qu(*x))).collect());
        o.insert("baseFeePerGas".into(), fees(&self.base_fee_per_gas));
        o.insert("gasUsedRatio".into(), json!(self.gas_used_ratio));
        if let Some(b) = &self.base_fee_per_blob_gas {
            o.insert("baseFeePerBlobGas".into(), fees(b));
        }
        if let Some(b) = &self.blob_gas_used_ratio {
            o.insert("blobGasUsedRatio".into(), json!(b));
        }
        Value::Object(o)
    }
}

/// geth `processBlock`'s rewards: the block's transactions by tip, each
/// weighted by its gas used, read at each percentile of the block's gas.
fn rewards(v: &Verified, percentiles: &[f64]) -> Result<Vec<U256>> {
    let txs = tips(v)?;
    if txs.is_empty() {
        return Ok(vec![U256::ZERO; percentiles.len()]);
    }
    let gas = gas_used(v)?;
    check!(
        gas.len() == txs.len(),
        Fault::Missing,
        "{} receipts for {} transactions",
        gas.len(),
        txs.len()
    );
    let mut sorter: Vec<(u64, U256)> = gas
        .into_iter()
        .zip(txs.into_iter().map(|(t, _)| t))
        .collect();
    sorter.sort_by_key(|a| a.1);
    let (mut i, mut sum) = (0usize, sorter[0].0);
    Ok(percentiles
        .iter()
        .map(|p| {
            let threshold = (v.header.gas_used as f64 * p / 100.0) as u64;
            while sum < threshold && i < sorter.len() - 1 {
                i += 1;
                sum = sum.saturating_add(sorter[i].0);
            }
            sorter[i].1
        })
        .collect())
}

/// Verify a fee-history proof as the response to `req` on chain `chain_id` and
/// compute `eth_feeHistory` from it, with the chain's built-in blob schedule.
pub fn verify_fee_history(
    p: &FeeHistoryProof,
    chain_id: u64,
    req: &FeeHistoryRequest,
) -> Result<FeeHistory> {
    verify_fee_history_with(p, chain_id, req, &BlobSchedule::builtin(chain_id))
}

/// [`verify_fee_history`] with blob prices and limits from `blobs`.
pub fn verify_fee_history_with(
    p: &FeeHistoryProof,
    chain_id: u64,
    req: &FeeHistoryRequest,
    blobs: &BlobSchedule,
) -> Result<FeeHistory> {
    let rules = rules_of(chain_id)?;
    req.check()?;
    let hs = verify_headers(p, chain_id, req.newest)?;
    let newest = hs.headers.last().expect("one").number;
    let (range, kind) = fee_history_shape(chain_id, newest, req)?;
    check!(
        hs.headers.len() as u64 == range.end() - range.start() + 1
            && hs.headers[0].number == *range.start(),
        Fault::Range,
        "{} headers from block {}: the request covers blocks {range:?}",
        hs.headers.len(),
        hs.headers[0].number
    );
    let bodies = verify_bodies(p, chain_id, &hs, kind)?;
    let nitro = rules == Rules::Nitro;
    let n = hs.headers.len();
    let mut base = Vec::with_capacity(n + 1);
    let mut blob_fee = Vec::with_capacity(n + 1);
    let mut ratio = Vec::with_capacity(n);
    let mut blob_ratio = Vec::with_capacity(n);
    for h in &hs.headers {
        base.push(U256::from(h.base_fee_per_gas.unwrap_or_default()));
        ratio.push(h.gas_used as f64 / h.gas_limit as f64);
        blob_fee.push(match h.excess_blob_gas {
            Some(_) => blob_base_fee(chain_id, h, blobs)
                .tag(Fault::Header)?
                .unwrap_or_default(),
            None => U256::ZERO,
        });
        let max = max_blob_gas(chain_id, h.timestamp, blobs);
        blob_ratio.push(match h.blob_gas_used {
            Some(used) if max != 0 => used as f64 / max as f64,
            _ => 0.0,
        });
    }
    let last = hs.headers.last().expect("one");
    base.push(
        next_base_fee(chain_id, last)
            .tag(Fault::Header)?
            .unwrap_or_default(),
    );
    blob_fee.push(match last.excess_blob_gas {
        Some(_) => {
            let excess =
                excess_blob_gas(chain_id, last, last.timestamp, blobs).tag(Fault::Header)?;
            let mut next = last.clone();
            next.excess_blob_gas = Some(excess);
            blob_base_fee(chain_id, &next, blobs)
                .tag(Fault::Header)?
                .unwrap_or_default()
        }
        None => U256::ZERO,
    });
    let reward = if req.reward_percentiles.is_empty() {
        None
    } else if nitro {
        Some(vec![vec![U256::ZERO; req.reward_percentiles.len()]; n])
    } else {
        Some(
            bodies
                .iter()
                .map(|v| rewards(v, &req.reward_percentiles))
                .collect::<Result<Vec<_>>>()
                .tag(Fault::Transactions)?,
        )
    };
    Ok(FeeHistory {
        block_number: newest,
        block_hash: *hs.hashes.last().expect("one"),
        oldest_block: hs.headers[0].number,
        reward,
        base_fee_per_gas: base,
        gas_used_ratio: ratio,
        base_fee_per_blob_gas: (!nitro).then_some(blob_fee),
        blob_gas_used_ratio: (!nitro).then_some(blob_ratio),
    })
}

/// The JSON-RPC params of a fee-history request, as the proof method takes them.
pub fn fee_history_params(req: &FeeHistoryRequest) -> Value {
    let newest = match req.newest {
        Block::Number(n) => json!(format!("0x{n:x}")),
        Block::Hash(h) => json!({"blockHash": format!("0x{}", hex::encode(h))}),
        Block::Tag => json!("latest"),
    };
    json!([
        format!("0x{:x}", req.block_count),
        newest,
        req.reward_percentiles
    ])
}
