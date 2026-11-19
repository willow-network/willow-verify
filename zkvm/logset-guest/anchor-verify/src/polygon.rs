






use crate::call::{self, calldata, word};
use crate::l2::{ethereum_at, rlp_items, uint};
use crate::lc::{bytes_of, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address};
use serde_json::Value;

pub const CHAIN_ID: u64 = 137;





pub const ROOT_CHAIN: Address = address!("86E4Dc95c7FBdBf52e33D563BbDB00823894C287");


pub const MAX_DEPTH: usize = 16;

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkpointed {

    pub level: &'static str,

    pub header_hash: [u8; 32],
    pub transactions_root: [u8; 32],
    pub receipts_root: [u8; 32],
    pub timestamp: u64,

    pub checkpoint: u64,
    pub start: u64,
    pub end: u64,

    pub l1_block: u64,
}



pub fn leaf(number: u64, timestamp: u64, tx_root: &[u8; 32], receipts_root: &[u8; 32]) -> [u8; 32] {
    let mut pre = [0u8; 128];
    pre[24..32].copy_from_slice(&number.to_be_bytes());
    pre[56..64].copy_from_slice(&timestamp.to_be_bytes());
    pre[64..96].copy_from_slice(tx_root);
    pre[96..].copy_from_slice(receipts_root);
    keccak256(pre).0
}


pub fn depth(len: u64) -> usize {
    (64 - len.saturating_sub(1).leading_zeros()) as usize
}


pub fn root_from(leaf: [u8; 32], mut index: u64, siblings: &[[u8; 32]]) -> [u8; 32] {
    let mut h = leaf;
    for s in siblings {
        let mut pre = [0u8; 64];
        if index.is_multiple_of(2) {
            pre[..32].copy_from_slice(&h);
            pre[32..].copy_from_slice(s);
        } else {
            pre[..32].copy_from_slice(s);
            pre[32..].copy_from_slice(&h);
        }
        h = keccak256(pre).0;
        index /= 2;
    }
    h
}

fn quantity(v: &Value, what: &str) -> Result<u64, Failure> {
    v.as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| bad(format!("{what} is not a 0x quantity")))
}

fn word_u64(w: [u8; 32], what: &str) -> Result<u64, Failure> {
    if w[..24].iter().any(|&b| b != 0) {
        return Err(rejected(format!("the checkpoint's {what} is over 64 bits")));
    }
    Ok(u64::from_be_bytes(w[24..].try_into().unwrap()))
}




pub fn confirm_checkpoint(
    lc: &LightClient,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<Checkpointed, Failure> {
    let (l1, state) = ethereum_at(lc, number, answer, now)?;
    let id = quantity(
        answer
            .get("checkpoint")
            .ok_or_else(|| bad("the anchor names no checkpoint"))?,
        "the checkpoint",
    )?;
    let mut arg = [0u8; 32];
    arg[24..].copy_from_slice(&id.to_be_bytes());
    let out = call::call(
        &state,
        &l1,
        ROOT_CHAIN,
        calldata("headerBlocks(uint256)", &[arg]),
    )?;
    let root = word(&out, 0)?;
    let start = word_u64(word(&out, 1)?, "start")?;
    let end = word_u64(word(&out, 2)?, "end")?;
    if root == [0u8; 32] || end < start {
        return Err(rejected(format!("Ethereum has no Polygon checkpoint {id}")));
    }
    if number < start || number > end {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is not in checkpoint {id} (blocks {start} to {end})"),
        ));
    }
    let levels = depth(end - start + 1);
    if levels > MAX_DEPTH {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("checkpoint {id} spans over 2^{MAX_DEPTH} blocks"),
        ));
    }


    let rlp = bytes_of(
        answer
            .get("header")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("the anchor has no header"))?,
        "header",
    )?;
    let f = rlp_items(&rlp)?;
    if f.len() < 15 {
        return Err(bad("a header with too few fields"));
    }
    let b32 = |i: usize, what: &str| -> Result<[u8; 32], Failure> {
        f[i].try_into()
            .map_err(|_| bad(format!("{what} is not 32 bytes")))
    };
    let tx_root = b32(4, "transactionsRoot")?;
    let receipts_root = b32(5, "receiptsRoot")?;
    let timestamp = uint(f[11], "timestamp")?;
    if uint(f[8], "number")? != number {
        return Err(rejected(format!(
            "the anchor's header is not block {number}"
        )));
    }

    let siblings = answer
        .get("proof")
        .and_then(Value::as_array)
        .filter(|p| p.len() == levels)
        .ok_or_else(|| bad(format!("the checkpoint proof is not {levels} hashes")))?
        .iter()
        .map(|s| {
            bytes_of(
                s.as_str().ok_or_else(|| bad("a proof hash is not hex"))?,
                "a proof hash",
            )?
            .try_into()
            .map_err(|_| bad("a proof hash is not 32 bytes"))
        })
        .collect::<Result<Vec<[u8; 32]>, _>>()?;
    let l = leaf(number, timestamp, &tx_root, &receipts_root);
    if root_from(l, number - start, &siblings) != root {
        return Err(rejected(format!(
            "block {number}'s header is not the one checkpoint {id} holds"
        )));
    }
    Ok(Checkpointed {
        level: "final",
        header_hash: keccak256(&rlp).0,
        transactions_root: tx_root,
        receipts_root,
        timestamp,
        checkpoint: id,
        start,
        end,
        l1_block: l1.number,
    })
}
