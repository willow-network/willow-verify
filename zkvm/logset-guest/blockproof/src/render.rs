



use super::derive::{
    blob_count, create_address, effective_gas_price, nitro_collects_tips, tx_gas_price,
    BlobSchedule, GAS_PER_BLOB,
};
use super::opfee::{FeeFork, L1Info};
use super::txs::{self, Decoded, Field};
use crate::ReceiptExtras;
use alloy::consensus::{Transaction as _, TxEnvelope};
use alloy::primitives::{Signature, U256};
use anyhow::{bail, ensure, Context as _, Result};
use logset_blockcheck::receipts::{
    deposits_allowed, logs_from_value, nitro_allowed, rlp_item, rlp_list_items, Log,
    DEPOSIT_TX_TYPE,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub ty: u8,

    pub status: bool,

    pub root: Option<[u8; 32]>,
    pub cumulative_gas_used: u64,
    pub bloom: Vec<u8>,
    pub logs: Vec<Log>,
    pub deposit_nonce: Option<u64>,
    pub deposit_receipt_version: Option<u64>,
}

pub fn decode_receipt(value: &[u8], chain_id: u64) -> Result<Receipt> {
    let first = *value.first().context("empty receipt value")?;
    let (ty, body) = if first >= 0xc0 {
        (0, value)
    } else {
        (first, &value[1..])
    };
    let (payload, _, is_list) = rlp_item(body)?;
    ensure!(is_list, "receipt body is not a list");
    let f = rlp_list_items(payload)?;
    ensure!(f.len() >= 4, "receipt has {} fields", f.len());
    let scalar = |b: &[u8]| -> Result<u64> {
        ensure!(b.len() <= 8, "scalar of {} bytes", b.len());
        Ok(b.iter().fold(0u64, |a, x| a << 8 | u64::from(*x)))
    };
    let (status, root) = match f[0] {
        ([], false) => (false, None),
        ([1], false) => (true, None),

        (r, false) if r.len() == 32 && ty == 0 => (false, Some(r.try_into().expect("32 bytes"))),
        (s, _) => bail!("receipt status of {} bytes", s.len()),
    };
    let deposit = ty == DEPOSIT_TX_TYPE && deposits_allowed(chain_id);
    Ok(Receipt {
        ty,
        status,
        root,
        cumulative_gas_used: scalar(f[1].0)?,
        bloom: f[2].0.to_vec(),
        logs: logs_from_value(value, chain_id)?,
        deposit_nonce: if deposit && f.len() > 4 {
            Some(scalar(f[4].0)?)
        } else {
            None
        },
        deposit_receipt_version: if deposit && f.len() > 5 {
            Some(scalar(f[5].0)?)
        } else {
            None
        },
    })
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Context {
    pub chain_id: u64,
    pub number: u64,
    pub hash: [u8; 32],
    pub timestamp: u64,
    pub base_fee: Option<U256>,
    pub excess_blob_gas: Option<u64>,

    pub l1_block_number: Option<u64>,

    pub collects_tips: bool,

    pub blobs: BlobSchedule,
}

impl Context {


























    pub fn from_rlp_header(chain_id: u64, h: &alloy::consensus::Header, hash: [u8; 32]) -> Self {
        Self {
            chain_id,
            number: h.number,
            hash,
            timestamp: h.timestamp,
            base_fee: h.base_fee_per_gas.map(U256::from),
            excess_blob_gas: h.excess_blob_gas,
            l1_block_number: nitro_allowed(chain_id)
                .then(|| u64::from_be_bytes(h.mix_hash.0[8..16].try_into().expect("8 bytes"))),
            collects_tips: nitro_allowed(chain_id) && nitro_collects_tips(&h.mix_hash.0),
            blobs: BlobSchedule::builtin(chain_id),
        }
    }






}


struct Slot<'a> {
    value: &'a [u8],
    sender: [u8; 20],
    tx: Decoded,
    hash: [u8; 32],
}




const BASE_TIME: [u8; 20] = [
    0x42, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0x30,
];
const SET_MILLIS: [u8; 4] = [0x86, 0xbd, 0xf3, 0x94];



fn denim_at(chain_id: u64) -> Option<u64> {
    match chain_id {
        8_453 => Some(1_793_901_600),
        84_532 => Some(1_792_605_600),
        _ => None,
    }
}


pub fn base_time_millis(tx: &Decoded) -> Option<u64> {
    let input = tx.input();
    let ms = u16::from_be_bytes(input.get(34..36)?.try_into().ok()?);
    (tx.ty() == DEPOSIT_TX_TYPE
        && tx.to() == Some(BASE_TIME)
        && input.len() == 36
        && input[..4] == SET_MILLIS
        && input[4..34].iter().all(|b| *b == 0)
        && ms < 1_000)
        .then_some(u64::from(ms))
}



pub fn block_millis(chain_id: u64, txs: &[&[u8]]) -> Option<u64> {
    let tx1 = txs.get(1).filter(|t| t.first() == Some(&DEPOSIT_TX_TYPE))?;
    deposits_allowed(chain_id)
        .then(|| txs::decode(tx1, chain_id).ok())
        .flatten()
        .as_ref()
        .and_then(base_time_millis)
}


#[derive(Clone, Copy)]
enum Millis {

    None,
    Known(u64),

    Unknown,
}


enum L1 {

    None,
    Known(L1Info),

    Unknown,
}




#[derive(Clone, Debug, PartialEq)]
pub struct Rendered {
    pub json: Value,
    pub unproven: Vec<String>,
}


const L1_FIELDS: [&str; 11] = [
    "l1GasPrice",
    "l1GasUsed",
    "l1Fee",
    "l1FeeScalar",
    "l1BaseFeeScalar",
    "l1BlobBaseFee",
    "l1BlobBaseFeeScalar",
    "operatorFeeScalar",
    "operatorFeeConstant",
    "daFootprintGasScalar",
    "blobGasUsed",
];




pub struct View<'a> {
    pub ctx: Context,
    txs: Vec<Option<Slot<'a>>>,
    receipts: Vec<Option<Receipt>>,

    extras: Option<&'a [ReceiptExtras]>,


    first_log: Vec<Option<u64>>,
    l1: L1,
    millis: Millis,
}

impl<'a> View<'a> {







































    pub fn partial(
        ctx: Context,
        txs: &'a BTreeMap<usize, Vec<u8>>,
        receipts: &BTreeMap<usize, Vec<u8>>,
    ) -> Result<Self> {
        let len = txs.keys().chain(receipts.keys()).max().map_or(0, |i| i + 1);
        let mut slots: Vec<Option<Slot>> = (0..len).map(|_| None).collect();
        for (&i, value) in txs {
            let tx =
                txs::decode(value, ctx.chain_id).with_context(|| format!("transaction {i}"))?;
            let sender = txs::sender(&tx).with_context(|| format!("transaction {i}: sender"))?;
            slots[i] = Some(Slot {
                tx,
                value,
                sender,
                hash: alloy::primitives::keccak256(value).0,
            });
        }
        let mut decoded: Vec<Option<Receipt>> = vec![None; len];
        for (&i, value) in receipts {
            decoded[i] =
                Some(decode_receipt(value, ctx.chain_id).with_context(|| format!("receipt {i}"))?);
        }
        Ok(Self::assemble(ctx, slots, decoded, None))
    }

    fn assemble(
        ctx: Context,
        txs: Vec<Option<Slot<'a>>>,
        receipts: Vec<Option<Receipt>>,
        extras: Option<&'a [ReceiptExtras]>,
    ) -> Self {
        let mut first_log = Vec::with_capacity(receipts.len());
        let mut n = Some(0u64);
        for r in &receipts {
            first_log.push(n);
            n = n.zip(r.as_ref()).map(|(n, r)| n + r.logs.len() as u64);
        }

        let l1 = if !deposits_allowed(ctx.chain_id) {
            L1::None
        } else {
            match txs.first() {
                Some(Some(s)) if s.tx.ty() == DEPOSIT_TX_TYPE => {
                    L1Info::parse(&s.tx.input()).map_or(L1::None, L1::Known)
                }
                Some(Some(_)) => L1::None,
                _ => L1::Unknown,
            }
        };
        let millis = match txs.get(1) {
            Some(Some(s)) => base_time_millis(&s.tx).map_or(Millis::None, Millis::Known),
            _ if denim_at(ctx.chain_id).is_some_and(|t| ctx.timestamp >= t) => Millis::Unknown,
            _ => Millis::None,
        };
        Self {
            ctx,
            txs,
            receipts,
            extras,
            first_log,
            l1,
            millis,
        }
    }


    pub fn len(&self) -> usize {
        self.txs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }



























    pub fn tx(&self, i: usize) -> Option<Rendered> {
        let Slot {
            tx, hash, sender, ..
        } = self.txs.get(i)?.as_ref()?;
        let ctx = &self.ctx;
        let mut unproven = Vec::new();
        let mut o = Map::new();
        put(&mut o, "blockHash", hx(&ctx.hash));
        put(&mut o, "blockNumber", q(ctx.number));
        put(&mut o, "blockTimestamp", q(ctx.timestamp));
        match self.millis {
            Millis::Known(ms) => put(
                &mut o,
                "blockTimestampMs",
                q(ctx.timestamp.saturating_mul(1_000).saturating_add(ms)),
            ),
            Millis::Unknown => unproven.push("blockTimestampMs".to_string()),
            Millis::None => {}
        }
        put(&mut o, "transactionIndex", q(i as u64));
        put(&mut o, "hash", hx(hash));
        put(&mut o, "from", hx(sender));
        put(&mut o, "type", q(u64::from(tx.ty())));
        match self.nonce(i) {
            Some(n) => put(&mut o, "nonce", q(n)),
            None => unproven.push("nonce".to_string()),
        }
        put(&mut o, "gas", q(tx.gas()));
        put(&mut o, "gasPrice", qu(tx_gas_price(tx, ctx.base_fee)));


        if tx.to().is_some() || tx.ty() != DEPOSIT_TX_TYPE {
            o.insert("to".into(), opt_addr(tx.to()));
        }
        put(&mut o, "value", qu(tx.value()));
        put(&mut o, "input", hx(&tx.input()));
        match tx {
            Decoded::Eth(env) => eth_fields(env, &mut o),
            Decoded::Table { ty, .. } if *ty == DEPOSIT_TX_TYPE => {
                put(&mut o, "sourceHash", field_json(tx.field("sourceHash")));
                put(&mut o, "mint", qu(tx.uint("mint").unwrap_or_default()));
                match self.receipts.get(i).and_then(Option::as_ref) {
                    Some(r) => {
                        if let Some(v) = r.deposit_receipt_version {
                            put(&mut o, "depositReceiptVersion", q(v));
                        }
                    }
                    None => unproven.push("depositReceiptVersion".to_string()),
                }
                if tx.field("isSystemTx") == Some(&Field::Bool(true)) {
                    o.insert("isSystemTx".into(), Value::Bool(true));
                }
                for k in ["v", "r", "s", "yParity"] {
                    put(&mut o, k, "0x0".into());
                }
            }
            Decoded::Table { ty, .. } => {
                put(
                    &mut o,
                    "chainId",
                    qu(tx.uint("chainId").unwrap_or_default()),
                );
                for k in ["v", "r", "s"] {
                    put(&mut o, k, "0x0".into());
                }
                for (name, _) in txs::layout(*ty).unwrap_or_default() {
                    if NITRO_COMMON.contains(name) {
                        continue;
                    }
                    match tx.field(name) {
                        Some(Field::Address(None)) => {}
                        f => put(&mut o, name, field_json(f)),
                    }
                }
            }
        }
        Some(Rendered {
            json: Value::Object(o),
            unproven,
        })
    }



    fn nonce(&self, i: usize) -> Option<u64> {
        let slot = self.txs.get(i)?.as_ref()?;
        match &slot.tx {
            Decoded::Table { ty, .. } if *ty == DEPOSIT_TX_TYPE => self.deposit_nonce(i),
            tx => Some(tx.nonce()),
        }
    }



    fn deposit_nonce(&self, i: usize) -> Option<u64> {
        let r = self.receipts.get(i)?.as_ref()?;
        r.deposit_nonce
            .or_else(|| self.extras.and_then(|x| x[i].deposit_nonce))
    }


    pub fn receipt(&self, i: usize) -> Option<Rendered> {
        let r = self.receipts.get(i)?.as_ref()?;
        let ctx = &self.ctx;
        let slot = self.txs.get(i).and_then(Option::as_ref);
        let mut unproven: Vec<String> = Vec::new();
        let mut missing = |fields: &[&str]| unproven.extend(fields.iter().map(|f| f.to_string()));
        let mut o = Map::new();
        put(&mut o, "blockHash", hx(&ctx.hash));
        put(&mut o, "blockNumber", q(ctx.number));
        match slot {
            Some(s) => put(&mut o, "transactionHash", hx(&s.hash)),
            None => missing(&["transactionHash"]),
        }
        put(&mut o, "transactionIndex", q(i as u64));
        put(&mut o, "type", q(u64::from(r.ty)));

        match r.root {
            Some(root) => put(&mut o, "root", hx(&root)),
            None => put(&mut o, "status", q(r.status as u64)),
        }
        put(&mut o, "cumulativeGasUsed", q(r.cumulative_gas_used));
        let prev = match i.checked_sub(1) {
            None => Some(0),
            Some(p) => self.receipts[p].as_ref().map(|p| p.cumulative_gas_used),
        };
        match prev {
            Some(prev) => put(
                &mut o,
                "gasUsed",
                q(r.cumulative_gas_used.saturating_sub(prev)),
            ),
            None => missing(&["gasUsed"]),
        }
        put(&mut o, "logsBloom", hx(&r.bloom));
        o.insert(
            "logs".into(),
            Value::Array(self.logs_of(i, slot.map(|s| s.hash), &mut missing)),
        );
        match slot {
            Some(s) => {
                put(&mut o, "from", hx(&s.sender));
                let to = s.tx.to();
                o.insert("to".into(), opt_addr(to));
                match (to, self.nonce(i)) {
                    (None, Some(n)) => {
                        put(&mut o, "contractAddress", hx(&create_address(&s.sender, n)))
                    }
                    (None, None) => missing(&["contractAddress"]),
                    (Some(_), _) => {
                        o.insert("contractAddress".into(), Value::Null);
                    }
                }
                put(
                    &mut o,
                    "effectiveGasPrice",
                    qu(effective_gas_price(
                        ctx.chain_id,
                        &s.tx,
                        ctx.base_fee,
                        ctx.collects_tips,
                    )),
                );
                if let Decoded::Eth(env) = &s.tx {
                    if let (Some(n), Some(price)) = (
                        blob_count(env),
                        ctx.excess_blob_gas
                            .and_then(|x| ctx.blobs.price(ctx.timestamp, x)),
                    ) {
                        put(&mut o, "blobGasUsed", q(n as u64 * GAS_PER_BLOB));
                        put(&mut o, "blobGasPrice", qu(price));
                    }
                }
            }
            None => missing(&[
                "from",
                "to",
                "contractAddress",
                "effectiveGasPrice",
                "blobGasUsed",
                "blobGasPrice",
            ]),
        }
        match (&self.l1, slot) {
            (L1::None, _) => {}
            (L1::Known(l1), Some(s)) => {
                let deposit = r.ty == DEPOSIT_TX_TYPE;
                let fork = FeeFork::at(ctx.chain_id, ctx.timestamp);
                let (gas, fee, da) = l1.cost(fork, s.value, deposit);
                put(&mut o, "l1GasPrice", qu(l1.l1_base_fee));
                put(&mut o, "l1GasUsed", q(gas));
                put(&mut o, "l1Fee", qu(fee));
                if fork == FeeFork::Bedrock {
                    put(&mut o, "l1FeeScalar", l1.fee_scalar_decimal());
                }
                put(&mut o, "l1BaseFeeScalar", qu(l1.base_fee_scalar));
                if let Some(fee) = l1.l1_blob_base_fee {
                    put(&mut o, "l1BlobBaseFee", qu(fee));
                }
                if let Some(scalar) = l1.blob_base_fee_scalar {
                    put(&mut o, "l1BlobBaseFeeScalar", q(u64::from(scalar)));
                }
                if let Some((scalar, constant)) = l1.operator_fee.filter(|f| *f != (0, 0)) {
                    put(&mut o, "operatorFeeScalar", q(u64::from(scalar)));
                    put(&mut o, "operatorFeeConstant", q(constant));
                }
                if let (Some(s), Some(da)) = (l1.da_footprint_gas_scalar, da) {
                    put(&mut o, "daFootprintGasScalar", q(u64::from(s)));
                    put(&mut o, "blobGasUsed", q(da));
                }
            }
            (L1::Known(_) | L1::Unknown, _) => missing(&L1_FIELDS),
        }
        if r.ty == DEPOSIT_TX_TYPE && deposits_allowed(ctx.chain_id) {
            match self.deposit_nonce(i) {
                Some(n) => put(&mut o, "depositNonce", q(n)),
                None => missing(&["depositNonce"]),
            }
        }
        if let Some(v) = r.deposit_receipt_version {
            put(&mut o, "depositReceiptVersion", q(v));
        }
        if let Some(l1) = ctx.l1_block_number {
            match self.extras {
                Some(extras) => {
                    let x = extras[i];
                    if let Some(g) = x.gas_used_for_l1 {
                        put(&mut o, "gasUsedForL1", q(g));
                    }
                    put(&mut o, "l1BlockNumber", q(l1));
                    if let Some(t) = x.timeboosted {
                        o.insert("timeboosted".into(), Value::Bool(t));
                    }
                }
                None => {
                    put(&mut o, "l1BlockNumber", q(l1));
                    missing(&["gasUsedForL1", "timeboosted"]);
                }
            }
        }
        unproven.sort();
        unproven.dedup();
        Some(Rendered {
            json: Value::Object(o),
            unproven,
        })
    }



    fn logs_of(
        &self,
        i: usize,
        tx_hash: Option<[u8; 32]>,
        missing: &mut impl FnMut(&[&str]),
    ) -> Vec<Value> {
        let Some(r) = self.receipts[i].as_ref() else {
            return vec![];
        };
        if !r.logs.is_empty() {
            if self.first_log[i].is_none() {
                missing(&["logs[].logIndex"]);
            }
            if tx_hash.is_none() {
                missing(&["logs[].transactionHash"]);
            }
            if matches!(self.millis, Millis::Unknown) {
                missing(&["logs[].blockTimestampMs"]);
            }
        }
        r.logs
            .iter()
            .enumerate()
            .map(|(k, l)| self.log_json(i, tx_hash, self.first_log[i].map(|f| f + k as u64), l))
            .collect()
    }






















    fn log_json(&self, tx: usize, tx_hash: Option<[u8; 32]>, index: Option<u64>, l: &Log) -> Value {
        let millis = match self.millis {
            Millis::Known(ms) => Some(ms),
            _ => None,
        };
        log_json(&self.ctx, millis, tx, tx_hash, index, l)
    }








}




















































fn log_json(
    ctx: &Context,
    millis: Option<u64>,
    tx: usize,
    tx_hash: Option<[u8; 32]>,
    index: Option<u64>,
    l: &Log,
) -> Value {
    let mut o = Map::new();
    put(&mut o, "address", hx(&l.address));
    o.insert(
        "topics".into(),
        Value::Array(l.topics.iter().map(|t| Value::String(hx(t))).collect()),
    );
    put(&mut o, "data", hx(&l.data));
    put(&mut o, "blockNumber", q(ctx.number));
    put(&mut o, "blockHash", hx(&ctx.hash));
    put(&mut o, "blockTimestamp", q(ctx.timestamp));
    if let Some(ms) = millis {
        put(
            &mut o,
            "blockTimestampMs",
            q(ctx.timestamp.saturating_mul(1_000).saturating_add(ms)),
        );
    }
    if let Some(h) = tx_hash {
        put(&mut o, "transactionHash", hx(&h));
    }
    put(&mut o, "transactionIndex", q(tx as u64));
    if let Some(index) = index {
        put(&mut o, "logIndex", q(index));
    }
    o.insert("removed".into(), Value::Bool(false));
    Value::Object(o)
}


const NITRO_COMMON: [&str; 7] = ["chainId", "from", "to", "value", "gas", "nonce", "input"];

fn eth_fields(env: &TxEnvelope, o: &mut Map<String, Value>) {
    let (sig, legacy_chain): (&Signature, Option<u64>) = match env {
        TxEnvelope::Legacy(s) => (s.signature(), s.tx().chain_id),
        TxEnvelope::Eip2930(s) => (s.signature(), None),
        TxEnvelope::Eip1559(s) => (s.signature(), None),
        TxEnvelope::Eip4844(s) => (s.signature(), None),
        TxEnvelope::Eip7702(s) => (s.signature(), None),
    };
    let parity = sig.v() as u64;
    put(o, "r", qu(sig.r()));
    put(o, "s", qu(sig.s()));
    if env.is_legacy() {
        let v = match legacy_chain {
            Some(id) => U256::from(id) * U256::from(2u8) + U256::from(35 + parity),
            None => U256::from(27 + parity),
        };
        put(o, "v", qu(v));
        if let Some(id) = legacy_chain {
            put(o, "chainId", q(id));
        }
        return;
    }
    put(o, "v", q(parity));
    put(o, "yParity", q(parity));
    put(o, "chainId", q(env.chain_id().unwrap_or_default()));
    let access: Vec<Value> = env
        .access_list()
        .map(|l| {
            l.iter()
                .map(|item| {
                    json!({
                        "address": hx(item.address.as_slice()),
                        "storageKeys": item.storage_keys.iter().map(|k| hx(k.as_slice())).collect::<Vec<_>>(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    o.insert("accessList".into(), Value::Array(access));
    if env.is_eip2930() {
        return;
    }
    put(o, "maxFeePerGas", qu(U256::from(env.max_fee_per_gas())));
    put(
        o,
        "maxPriorityFeePerGas",
        qu(U256::from(
            env.max_priority_fee_per_gas().unwrap_or_default(),
        )),
    );
    if let Some(hashes) = env.blob_versioned_hashes() {
        put(
            o,
            "maxFeePerBlobGas",
            qu(U256::from(env.max_fee_per_blob_gas().unwrap_or_default())),
        );
        o.insert(
            "blobVersionedHashes".into(),
            Value::Array(hashes.iter().map(|h| hx(h.as_slice()).into()).collect()),
        );
    }
    if let Some(auths) = env.authorization_list() {
        let list = auths
            .iter()
            .map(|a| {
                json!({
                    "chainId": qu(a.chain_id),
                    "address": hx(a.address.as_slice()),
                    "nonce": q(a.nonce),
                    "yParity": q(u64::from(a.y_parity())),
                    "r": qu(a.r()),
                    "s": qu(a.s()),
                })
            })
            .collect();
        o.insert("authorizationList".into(), Value::Array(list));
    }
}

fn field_json(f: Option<&Field>) -> String {
    match f {
        Some(Field::Uint(v)) => qu(*v),
        Some(Field::Bool(b)) => q(*b as u64),
        Some(Field::Address(Some(a))) => hx(a),
        Some(Field::Address(None)) | None => "0x".into(),
        Some(Field::Hash(h)) => hx(h),
        Some(Field::Bytes(b)) => hx(b),
    }
}

fn put(o: &mut Map<String, Value>, k: &str, v: String) {
    o.insert(k.to_string(), Value::String(v));
}

fn opt_addr(a: Option<[u8; 20]>) -> Value {
    a.map_or(Value::Null, |a| Value::String(hx(&a)))
}

pub fn hx(b: &[u8]) -> String {
    format!("0x{}", hex::encode(b))
}

pub fn q(n: u64) -> String {
    format!("0x{n:x}")
}

pub fn qu(n: U256) -> String {
    format!("{n:#x}")
}





























































































