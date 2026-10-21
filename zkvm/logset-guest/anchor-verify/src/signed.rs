//! The "signed by the sequencer" level (`TRUSTLESS-ANCHOR.md`): a block newer than its chain's
//! latest claim on Ethereum, vouched for by the chain's sequencer, whose key Ethereum names. The
//! signature is verified here; the key is read with Ethereum's contract code over state the light
//! client verified (`call.rs`); the signed block's header gives its state root, and the chain's own
//! block-hash history leads back to the block.

use crate::call::{self, address_word, calldata, word};
use crate::history::{self, ETHEREUM_RING};
use crate::l2::{header, nitro_code_hash, walk_back, MAX_HOPS, NITRO_RING};
use crate::lc::{bytes_of, Execution, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, Signature, B256};
use base64::Engine;
use serde_json::Value;

/// A signed block older than this is not used: its chain's claim on Ethereum should cover it by
/// then, and until one does the sequencer could still replace it.
pub const MAX_SIGNED_AGE_SECS: u64 = 3 * 3600;

/// Where each chain's sequencer key is named on Ethereum (read on Ethereum
/// on 2026-10-08): an OP-stack chain's SystemConfig (`unsafeBlockSigner()`), a Nitro chain's
/// SequencerInbox (`isBatchPoster`, `isSequencer`), with the block of feed message 0.
pub enum KeyOn {
    SystemConfig(Address),
    SequencerInbox { inbox: Address, first_block: u64 },
}

pub struct Sequenced {
    pub chain_id: u64,
    pub name: &'static str,
    pub key: KeyOn,
}

pub const SEQUENCED: [Sequenced; 6] = [
    Sequenced {
        chain_id: 8453,
        name: "Base",
        key: KeyOn::SystemConfig(address!("73a79Fab69143498Ed3712e519A88a918e1f4072")),
    },
    Sequenced {
        chain_id: 10,
        name: "OP Mainnet",
        key: KeyOn::SystemConfig(address!("229047fed2591dbec1eF1118d64F7aF3dB9EB290")),
    },
    Sequenced {
        chain_id: 130,
        name: "Unichain",
        key: KeyOn::SystemConfig(address!("c407398d063f942feBbcC6F80a156b47F3f1BDA6")),
    },
    Sequenced {
        chain_id: 57073,
        name: "Ink",
        key: KeyOn::SystemConfig(address!("62C0a111929fA32ceC2F76aDba54C16aFb6E8364")),
    },
    Sequenced {
        chain_id: 42161,
        name: "Arbitrum One",
        key: KeyOn::SequencerInbox {
            inbox: address!("1c479675ad559DC151F6Ec7ed3FbF8ceE79582B6"),
            first_block: 22_207_817,
        },
    },
    Sequenced {
        chain_id: 4663,
        name: "Robinhood Chain",
        key: KeyOn::SequencerInbox {
            inbox: address!("Bd0D173EEb87D57A09521c24388a12789F33ba96"),
            first_block: 0,
        },
    },
];

/// Each OP gossip topic version's SSZ execution payload fixed part.
const PAYLOAD_SIZES: [usize; 4] = [508, 512, 528, 560];

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

/// What a sequencer-signed anchor showed about block `number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedConfirmed {
    pub hash: [u8; 32],
    /// The block the sequencer signed, and its key.
    pub anchor_block: u64,
    pub signer: Address,
    /// The Ethereum block whose state named the key.
    pub l1_block: u64,
}

/// The block an OP sequencer signed (gossip topic `version`): its number and hash, if `signer`
/// signed it for `chain_id`. The message is the signature (65 bytes) and the signed data, which
/// from version 2 opens with the parent beacon block root before the SSZ execution payload.
pub fn op_signed_block(
    chain_id: u64,
    version: u64,
    message: &[u8],
) -> Result<(u64, [u8; 32], Address), Failure> {
    let fixed = *usize::try_from(version)
        .ok()
        .and_then(|v| PAYLOAD_SIZES.get(v))
        .ok_or_else(|| bad("an unknown gossip version"))?;
    let prefix = if version >= 2 { 32 } else { 0 };
    if message.len() < 65 + prefix + fixed {
        return Err(bad("a signed block too short"));
    }
    let sig = Signature::try_from(&message[..65]).map_err(|_| bad("a malformed signature"))?;
    let data = &message[65..];
    let mut pre = [0u8; 96];
    pre[56..64].copy_from_slice(&chain_id.to_be_bytes());
    pre[64..].copy_from_slice(keccak256(data).as_slice());
    let signer = sig
        .recover_address_from_prehash(&keccak256(pre))
        .map_err(|_| rejected("the sequencer's signature does not recover a key"))?;
    let p = &data[prefix..];
    // The signature does not cover the version: the payload's first variable field must start
    // where this version's fixed part ends.
    if u32::from_le_bytes(p[436..440].try_into().unwrap()) as usize != fixed {
        return Err(rejected("the signed payload does not match its version"));
    }
    let number = u64::from_le_bytes(p[404..412].try_into().unwrap());
    Ok((number, p[472..504].try_into().unwrap(), signer))
}

/// A Go `[]byte` field of a feed message: base64, or null for none.
fn go_bytes(v: Option<&Value>) -> Option<Vec<u8>> {
    match v {
        None | Some(Value::Null) => Some(vec![]),
        Some(Value::String(s)) => base64::engine::general_purpose::STANDARD.decode(s).ok(),
        _ => None,
    }
}

/// Nitro's `BroadcastFeedMessage.SignatureHash(chainId)`.
fn feed_signature_hash(chain_id: u64, seq: u64, block_hash: &[u8; 32], m: &Value) -> Option<B256> {
    let with = m.get("message")?;
    let msg = with.get("message")?;
    let h = msg.get("header")?;
    let mut d = b"Arbitrum Nitro Feed:".to_vec();
    d.extend(chain_id.to_be_bytes());
    d.extend(seq.to_be_bytes());
    d.extend(block_hash);
    d.extend(go_bytes(m.get("blockMetadata"))?);
    d.extend(with.get("delayedMessagesRead")?.as_u64()?.to_be_bytes());
    d.push(u8::try_from(h.get("kind")?.as_u64()?).ok()?);
    let poster: Address = h.get("sender")?.as_str()?.parse().ok()?;
    d.extend(poster.as_slice());
    d.extend(h.get("blockNumber")?.as_u64()?.to_be_bytes());
    d.extend(h.get("timestamp")?.as_u64()?.to_be_bytes());
    if let Some(id) = h.get("requestId").filter(|v| !v.is_null()) {
        let id: B256 = id.as_str()?.parse().ok()?;
        d.extend(id.as_slice());
    }
    // A big.Int's minimal big-endian bytes (none for zero).
    if let Some(fee) = h.get("baseFeeL1").filter(|v| !v.is_null()) {
        let fee = fee.as_u64()?.to_be_bytes();
        d.extend(fee.iter().skip_while(|b| **b == 0));
    }
    d.extend(go_bytes(msg.get("l2Msg"))?);
    Some(keccak256(d))
}

/// The block a Nitro feed message signs: its sequence number, block hash and signing key.
pub fn nitro_signed_block(chain_id: u64, m: &Value) -> Result<(u64, [u8; 32], Address), Failure> {
    let seq = m
        .get("sequenceNumber")
        .and_then(Value::as_u64)
        .ok_or_else(|| bad("a feed message without a sequence number"))?;
    let hash: [u8; 32] = m
        .get("blockHash")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<B256>().ok())
        .ok_or_else(|| bad("a feed message without a block hash"))?
        .0;
    let sig = go_bytes(m.get("signatureV2"))
        .filter(|s| s.len() == 65)
        .ok_or_else(|| bad("a feed message without a signature"))?;
    let sig = Signature::try_from(sig.as_slice()).map_err(|_| bad("a malformed signature"))?;
    let digest = feed_signature_hash(chain_id, seq, &hash, m)
        .ok_or_else(|| bad("a malformed feed message"))?;
    let signer = sig
        .recover_address_from_prehash(&digest)
        .map_err(|_| rejected("the sequencer's signature does not recover a key"))?;
    Ok((seq, hash, signer))
}

fn as_true(out: &alloy_primitives::Bytes) -> Result<bool, Failure> {
    match word(out, 0)? {
        w if w[..31].iter().all(|&b| b == 0) && w[31] <= 1 => Ok(w[31] == 1),
        _ => Err(rejected("a call did not return a bool")),
    }
}

/// Block `number` of `chain_id`, from a `willow_getBlockAnchor` result carrying a block its
/// sequencer signed (`signed`), with Ethereum state at the finalized block the light client
/// verified (`l1`), the signed block's header and, for an earlier block, a history proof.
pub fn confirm_signed(
    lc: &LightClient,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<SignedConfirmed, Failure> {
    let chain = SEQUENCED
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            Failure::new(
                Kind::Internal,
                format!("chain {chain_id} has no sequencer here"),
            )
        })?;
    let (l1, state): (Execution, call::State) = crate::l2::ethereum_at(lc, number, answer, now)?;
    let signed = answer
        .get("signed")
        .ok_or_else(|| bad("the anchor has no signed block"))?;
    let at = |to: Address, sig: &str, args: &[[u8; 32]]| {
        call::call(&state, &l1, to, calldata(sig, args))
    };

    let (top_number, top_hash, signer) = match chain.key {
        KeyOn::SystemConfig(config) => {
            let version = signed
                .get("version")
                .and_then(Value::as_u64)
                .ok_or_else(|| bad("the signed block has no gossip version"))?;
            let message = bytes_of(
                signed
                    .get("message")
                    .and_then(Value::as_str)
                    .ok_or_else(|| bad("the signed block has no message"))?,
                "the signed message",
            )?;
            let (n, h, signer) = op_signed_block(chain_id, version, &message)?;
            let key =
                Address::from_slice(&word(&at(config, "unsafeBlockSigner()", &[])?, 0)?[12..]);
            if signer != key {
                return Err(rejected(format!(
                    "block {n} is not signed by the sequencer key Ethereum names for {} ({key})",
                    chain.name
                )));
            }
            (n, h, signer)
        }
        KeyOn::SequencerInbox { inbox, first_block } => {
            let message = signed
                .get("message")
                .ok_or_else(|| bad("the signed block has no feed message"))?;
            let (seq, h, signer) = nitro_signed_block(chain_id, message)?;
            let s = address_word(signer);
            if !as_true(&at(inbox, "isBatchPoster(address)", &[s])?)?
                && !as_true(&at(inbox, "isSequencer(address)", &[s])?)?
            {
                return Err(rejected(format!(
                    "{signer} is not a sequencer or batch poster Ethereum names for {}",
                    chain.name
                )));
            }
            let n = first_block
                .checked_add(seq)
                .ok_or_else(|| bad("a sequence number out of range"))?;
            (n, h, signer)
        }
    };

    let headers = answer
        .get("headers")
        .and_then(Value::as_array)
        .filter(|h| !h.is_empty() && h.len() <= MAX_HOPS + 1)
        .ok_or_else(|| bad("the anchor has no L2 headers"))?;
    let top = header(
        headers[0]
            .as_str()
            .ok_or_else(|| bad("a header is not hex"))?,
    )?;
    if top.hash != top_hash || top.number != top_number {
        return Err(rejected("the signed block's header is not the one signed"));
    }
    if now.saturating_sub(top.timestamp) > MAX_SIGNED_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleHead,
            format!(
                "the block {}'s sequencer signed is {} minutes old",
                chain.name,
                now.saturating_sub(top.timestamp) / 60
            ),
        ));
    }
    if number > top.number {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is after the signed block"),
        ));
    }
    // Back to the block through the chain's own history, hopping as far as a claim's interval
    // (Ink's games are 12 hours apart, its history 2.3 hours).
    let (ring, code) = match chain.key {
        KeyOn::SystemConfig(_) => (ETHEREUM_RING, history::ethereum_code_hash()),
        KeyOn::SequencerInbox { .. } => (NITRO_RING, nitro_code_hash()),
    };
    let hash = walk_back(&top, number, headers, answer, ring, code)?;
    Ok(SignedConfirmed {
        hash,
        anchor_block: top.number,
        signer,
        l1_block: l1.number,
    })
}
