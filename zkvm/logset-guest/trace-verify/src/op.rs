//! Unichain's blocks run again as op-reth runs them ([`willow_op_exec::block`], the code the log
//! RPC's Unichain trace methods run), every transaction checked against its receipt as
//! `state/trace_op.rs` checks it.

use crate::block::{Block, Tx};
use crate::request::Tracer;
use crate::state::Pre;
use crate::{Stop, Traces, MAX_TRACE_BYTES};
use alloy::consensus::Transaction;
use alloy::eips::Typed2718;
use alloy_primitives::{Address, B256};
use logset_blockproof::txs::{Decoded, Field};
use willow_op_exec::block as op;

/// A block's transaction as the OP stack's EVM runs it.
fn op_tx(t: &Tx) -> Option<op::Tx> {
    match &t.decoded {
        Decoded::Eth(e) => Some(op::Tx {
            ty: e.ty(),
            hash: t.hash,
            from: t.sender?,
            to: e.to(),
            nonce: e.nonce(),
            gas_limit: e.gas_limit(),
            gas_price: e.max_fee_per_gas(),
            priority_fee: e.max_priority_fee_per_gas(),
            value: e.value(),
            data: e.input().clone(),
            chain_id: e.chain_id(),
            access_list: e
                .access_list()
                .map(|l| {
                    l.iter()
                        .map(|i| (i.address, i.storage_keys.clone()))
                        .collect()
                })
                .unwrap_or_default(),
            authorizations: e
                .authorization_list()
                .map(<[_]>::to_vec)
                .unwrap_or_default(),
            encoded: t.raw.clone(),
            deposit: None,
        }),
        d @ Decoded::Table { ty, .. } if *ty == logset_blockcheck::receipts::DEPOSIT_TX_TYPE => {
            let source_hash = match d.field("sourceHash") {
                Some(Field::Hash(h)) => B256::from(*h),
                _ => return None,
            };
            let is_system = matches!(d.field("isSystemTx"), Some(Field::Bool(true)));
            let mint = d.uint("mint").unwrap_or_default();
            Some(op::Tx {
                ty: *ty,
                hash: t.hash,
                from: t.sender?,
                to: d.to().map(Address::from),
                nonce: 0,
                gas_limit: d.gas(),
                gas_price: 0,
                priority_fee: None,
                value: d.value(),
                data: d.input().into(),
                chain_id: None,
                access_list: vec![],
                authorizations: vec![],
                encoded: t.raw.clone(),
                deposit: Some(op::Deposit {
                    source_hash,
                    mint: Some(u128::try_from(mint).ok()?),
                    is_system,
                }),
            })
        }
        Decoded::Table { .. } => None,
    }
}

/// [`crate::eth::run_block`] on Unichain.
pub fn run_block(
    block: &Block,
    pre: &Pre<'_>,
    tracer: &Tracer,
    last: usize,
    wanted: &dyn Fn(usize) -> bool,
) -> Result<Traces, Stop> {
    let tracer = match tracer {
        Tracer::Flat => op::Tracer::Flat,
        Tracer::Geth(o) => {
            op::Tracer::Geth(serde_json::to_value(o).map_err(|e| Stop::Failed(e.to_string()))?)
        }
        Tracer::Replay(_) => return Err(Stop::Failed("trace replays are not run here".into())),
    };
    let txs = block
        .txs
        .iter()
        .take(last.saturating_add(1))
        .enumerate()
        .map(|(i, t)| {
            op_tx(t).ok_or_else(|| Stop::Failed(format!("transaction {i} is not the OP stack's")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let h = &block.header;
    let env = willow_op_exec::Block {
        number: h.number,
        timestamp: h.timestamp,
        gas_limit: h.gas_limit,
        base_fee: h.base_fee_per_gas.unwrap_or(0),
        mix_hash: h.mix_hash,
        beneficiary: h.beneficiary,
    };
    let ran = op::run_block(
        pre,
        &env,
        block.hash,
        h.parent_hash,
        h.parent_beacon_block_root,
        &txs,
        last,
        wanted,
        &tracer,
        MAX_TRACE_BYTES,
    );
    if pre.has_missing() {
        return Err(Stop::Failed("the block read state not held".into()));
    }
    let ran = ran.map_err(|s| match s {
        op::Stop::NoBlockHash(n) => Stop::NoBlockHash(n),
        op::Stop::TooLarge => Stop::TooLarge,
        op::Stop::Failed(m) => Stop::Failed(m),
    })?;
    let mut traces = Vec::new();
    let mut cumulative = 0u64;
    for (i, (r, receipt)) in ran.into_iter().zip(&block.receipts).enumerate() {
        cumulative += r.gas_used;
        // A deposit's receipt carries its nonce and version 1 (Canyon on, all of Unichain).
        let deposit = match (receipt.deposit, r.deposit_nonce) {
            (Some((nonce, version)), Some(ran)) => nonce == Some(ran) && version == Some(1),
            (None, None) => true,
            _ => false,
        };
        let ok =
            receipt.success.is_none_or(|s| s == r.success) && r.logs == receipt.logs && deposit;
        if cumulative != receipt.cumulative_gas || !ok {
            return Err(Stop::Disagrees(
                i,
                format!(
                    "used {cumulative} gas in all ({}, deposit nonce {:?}), the receipt says {} \
                     ({:?}, {:?})",
                    r.success,
                    r.deposit_nonce,
                    receipt.cumulative_gas,
                    receipt.success,
                    receipt.deposit
                ),
            ));
        }
        if let Some(v) = r.trace {
            traces.push((i, v));
        }
    }
    Ok(traces)
}
