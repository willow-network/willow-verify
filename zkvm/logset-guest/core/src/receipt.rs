//! Receipt and log parsing, and the positional indices the ERC record form uses.
//!
//! A receipts-trie VALUE under key `rlp(i)` is either `rlp(receipt)` (legacy) or
//! `type_byte | rlp(receipt)` (EIP-2718). Either way the receipt body is
//! `[status_or_state, cumulativeGas, logsBloom, logs]` and a log is
//! `[emitter, topics, data]`.
//!
//! **`txIndex` and `logIndex` are never witnessed.** `txIndex` is the receipt's
//! position in the trie — the key the value had to be stored under for the root to
//! match — and `logIndex` is a running block-wide counter over every log the block
//! emitted, matching `eth_getLogs`. A prover has no field to lie in: to move a log
//! it must move the receipt, and moving a receipt changes the root.
//!
//! Pre-Byzantium receipts carry a 32-byte `postState` instead of a status byte.
//! Item 0 is not interpreted here, so both shapes parse.
//!
//! # Chain-gated receipt types: the seven chains' formats, nothing more
//!
//! The leaf is frozen once for all seven chains Willow proves (one program
//! for all of them), so it carries every receipt format they commit, each
//! accepted ONLY on the chains that use it ([`profile_of`] the witness's
//! `chain_id`). Any other chain gets the L1 set alone; a further chain is a new
//! leaf key appended to the verifier's table, not a change to this one.
//!
//! | profile | chains (chainId) | extra types | body |
//! |---|---|---|---|
//! | L1 | Ethereum (1), any unlisted chain | none | 4 items |
//! | OP stack | Base 8453, OP Mainnet 10, Unichain 130, Ink 57073 | `0x7E` deposit | 4 items + `depositNonce` (Regolith) + `depositReceiptVersion` (Canyon) |
//! | Nitro | Arbitrum One 42161, Robinhood Chain 4663 | `0x64` Deposit, `0x65` Unsigned, `0x66` Contract, `0x68` Retry, `0x69` SubmitRetryable, `0x6A` Internal | 4 items |
//!
//! All L1 types (legacy, `0x01`..`0x04`) are accepted everywhere. Nitro has no
//! `0x67`; `ArbitrumLegacyTxType` is `0x78` and its receipts are committed in the
//! legacy (untyped) encoding, so neither byte is ever a Nitro type prefix and
//! both stay refused. Isthmus, Holocene and Jovian add RPC receipt fields (L1
//! and operator fee) but no consensus-encoding items: real post-Jovian Base, OP,
//! Unichain and Ink receipts are the 6-item deposit / 4-item typed forms
//! (`fixtures/base`, `fixtures/fast-chains`).
//!
//! The logs are item 3 in every form and are parsed identically; a deposit's
//! tail items carry no logs and are only shape-checked (non-list, canonical
//! u64). Why this cannot change what a statement about another chain means:
//!
//! 1. Each extra type is accepted only under its profile's chain ids; on every
//!    other chain, Ethereum included, it is [`Error::ReceiptTypeUnknown`]
//!    exactly as before, so the Ethereum branch of the leaf is unchanged.
//! 2. `chainId` is committed in every statement and every consumer pins it, so
//!    choosing another chain's id in the witness only produces a statement about
//!    that chain, which a consumer of this chain rejects.
//! 3. The receipts trie commits the exact value bytes (the root check runs
//!    before the scan), so a receipt reaches the parser only if the chain's own
//!    trie holds it; the parser's job is to read the logs that chain emitted,
//!    and item 3 is where every one of these encodings puts them.

use crate::erc::MAX_TOPICS;
use crate::error::Error;
use crate::rlp;
use alloc::vec::Vec;

/// EIP-2718 transaction types whose receipts this guest accepts on every chain.
pub const KNOWN_TX_TYPES: [u8; 4] = [0x01, 0x02, 0x03, 0x04];

/// The OP-stack deposit transaction type.
pub const DEPOSIT_TX_TYPE: u8 = 0x7E;

/// OP-stack chains among the seven: Base, OP Mainnet, Unichain, Ink.
pub const OP_STACK_CHAIN_IDS: [u64; 4] = [8453, 10, 130, 57073];

/// Arbitrum Nitro chains among the seven: Arbitrum One, Robinhood Chain (Orbit).
pub const NITRO_CHAIN_IDS: [u64; 2] = [42161, 4663];

/// Nitro's own transaction types (OffchainLabs go-ethereum `core/types`).
pub const NITRO_TX_TYPES: [u8; 6] = [0x64, 0x65, 0x66, 0x68, 0x69, 0x6A];

/// Which extra receipt types a chain's tries may hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptProfile {
    L1,
    OpStack,
    Nitro,
}

pub fn profile_of(chain_id: u64) -> ReceiptProfile {
    if OP_STACK_CHAIN_IDS.contains(&chain_id) {
        ReceiptProfile::OpStack
    } else if NITRO_CHAIN_IDS.contains(&chain_id) {
        ReceiptProfile::Nitro
    } else {
        ReceiptProfile::L1
    }
}

/// Whether `chain_id`'s receipts tries may hold `0x7E` deposit receipts.
pub fn deposits_allowed(chain_id: u64) -> bool {
    profile_of(chain_id) == ReceiptProfile::OpStack
}

/// One log, borrowed from the receipt value. Topics live inline so a busy block
/// does not allocate once per log.
#[derive(Clone, Debug)]
pub struct LogView<'a> {
    pub emitter: [u8; 20],
    topics_buf: [[u8; 32]; MAX_TOPICS],
    n_topics: u8,
    pub data: &'a [u8],
}

impl<'a> LogView<'a> {
    pub fn topics(&self) -> &[[u8; 32]] {
        &self.topics_buf[..self.n_topics as usize]
    }
}

/// Strip an EIP-2718 type byte, if any, and return the RLP body and whether
/// the receipt is a deposit.
fn receipt_body(
    value: &[u8],
    block: u32,
    tx: u32,
    profile: ReceiptProfile,
) -> Result<(&[u8], bool), Error> {
    let first = *value.first().ok_or(Error::ReceiptEmpty { block, tx })?;
    if KNOWN_TX_TYPES.contains(&first) {
        return Ok((&value[1..], false));
    }
    match profile {
        ReceiptProfile::OpStack if first == DEPOSIT_TX_TYPE => return Ok((&value[1..], true)),
        ReceiptProfile::Nitro if NITRO_TX_TYPES.contains(&first) => {
            return Ok((&value[1..], false))
        }
        _ => {}
    }
    // A legacy receipt is a bare RLP list, so it must lead with a list header.
    if first < 0xc0 {
        return Err(Error::ReceiptTypeUnknown {
            block,
            tx,
            ty: first,
        });
    }
    Ok((value, false))
}

/// Parse every log of one receipt value, in emission order. `profile` is
/// [`profile_of`] the witness's chain.
pub fn parse_receipt<'a>(
    value: &'a [u8],
    block: u32,
    tx: u32,
    profile: ReceiptProfile,
) -> Result<Vec<LogView<'a>>, Error> {
    let (body, is_deposit) = receipt_body(value, block, tx, profile)?;
    let fields = rlp::list(body)?;
    let arity_ok = if is_deposit {
        (4..=6).contains(&fields.len())
    } else {
        fields.len() == 4
    };
    if !arity_ok {
        return Err(Error::ReceiptFieldCount {
            block,
            tx,
            got: fields.len(),
        });
    }
    for f in &fields[4..] {
        if f.is_list || rlp::as_u64(f.payload).is_none() {
            return Err(Error::DepositTailMalformed { block, tx });
        }
    }
    if fields[2].is_list || fields[2].payload.len() != 256 {
        return Err(Error::ReceiptBloomLen {
            block,
            tx,
            got: fields[2].payload.len(),
        });
    }
    if !fields[3].is_list {
        return Err(Error::LogsNotAList { block, tx });
    }
    let log_items = rlp::items(fields[3].payload)?;
    let mut out = Vec::with_capacity(log_items.len());
    for (li, item) in log_items.iter().enumerate() {
        let log = li as u32;
        if !item.is_list {
            return Err(Error::LogFieldCount {
                block,
                tx,
                log,
                got: 0,
            });
        }
        let parts = rlp::items(item.payload)?;
        if parts.len() != 3 {
            return Err(Error::LogFieldCount {
                block,
                tx,
                log,
                got: parts.len(),
            });
        }
        if parts[0].is_list || parts[0].payload.len() != 20 {
            return Err(Error::LogEmitterLen {
                block,
                tx,
                log,
                got: parts[0].payload.len(),
            });
        }
        let mut emitter = [0u8; 20];
        emitter.copy_from_slice(parts[0].payload);
        if !parts[1].is_list {
            return Err(Error::LogTopicsNotAList { block, tx, log });
        }
        let topic_items = rlp::items(parts[1].payload)?;
        if topic_items.len() > MAX_TOPICS {
            return Err(Error::TooManyTopics {
                block,
                tx,
                log,
                got: topic_items.len(),
            });
        }
        let mut topics_buf = [[0u8; 32]; MAX_TOPICS];
        for (i, t) in topic_items.iter().enumerate() {
            if t.is_list || t.payload.len() != 32 {
                return Err(Error::LogTopicLen {
                    block,
                    tx,
                    log,
                    topic: i,
                    got: t.payload.len(),
                });
            }
            topics_buf[i].copy_from_slice(t.payload);
        }
        if parts[2].is_list {
            return Err(Error::LogFieldCount {
                block,
                tx,
                log,
                got: 3,
            });
        }
        if u32::try_from(parts[2].payload.len()).is_err() {
            return Err(Error::LogDataTooLong { block, tx, log });
        }
        out.push(LogView {
            emitter,
            topics_buf,
            n_topics: topic_items.len() as u8,
            data: parts[2].payload,
        });
    }
    Ok(out)
}
