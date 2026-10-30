//! Transactions read back from their stored trie values — the bytes
//! `logset_blockcheck::transactions` encoded from the node's JSON and
//! `verify_body` rebuilt transactionsRoot from — into typed fields for
//! rendering.
//!
//! Ethereum's types (legacy, 2930, 1559, 4844 without blobs, 7702) decode
//! through alloy. The OP-stack deposit (`0x7E`) and the Arbitrum Nitro types
//! (`0x64..=0x6A`) have no alloy arms: one field table per type ([`layout`]),
//! the struct order of op-geth's `DepositTx` and OffchainLabs/go-ethereum's
//! `arb_types.go` (the order the blockcheck encoder writes), names them. Each
//! is gated per chain like its receipts.

use alloy::consensus::transaction::SignerRecoverable;
use alloy::consensus::{Transaction as _, TxEnvelope};
use alloy::eips::eip2718::Decodable2718;
use alloy::eips::Typed2718;
use alloy::primitives::U256;
use anyhow::{bail, ensure, Context, Result};
use logset_blockcheck::receipts::{
    deposits_allowed, nitro_allowed, rlp_item, rlp_list_items, DEPOSIT_TX_TYPE, NITRO_TX_TYPES,
};

/// ArbOS's own address: the sender and recipient of a Nitro internal transaction.
pub const ARBOS_ADDRESS: [u8; 20] = {
    let mut a = [0u8; 20];
    a[17] = 0x0a;
    a[18] = 0x4b;
    a[19] = 0x05;
    a
};
/// The ArbRetryableTx precompile: the recipient of a submit-retryable.
pub const ARB_RETRYABLE_ADDRESS: [u8; 20] = {
    let mut a = [0u8; 20];
    a[19] = 0x6e;
    a
};

pub use logset_blockcheck::transactions::{
    ARB_CONTRACT, ARB_DEPOSIT, ARB_INTERNAL, ARB_RETRY, ARB_SUBMIT_RETRYABLE, ARB_UNSIGNED,
};

/// How one field of a table-driven type is encoded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A scalar: minimal big-endian bytes, zero as the empty string.
    Uint,
    /// A 0/1 scalar.
    Bool,
    Address,
    /// An address, or the empty string for none (`null` in JSON).
    OptAddress,
    Hash,
    Bytes,
}

use Kind::*;

/// The field table of a deposit or Nitro type: `(JSON name, kind)` in RLP order.
pub fn layout(ty: u8) -> Option<&'static [(&'static str, Kind)]> {
    Some(match ty {
        DEPOSIT_TX_TYPE => &[
            ("sourceHash", Hash),
            ("from", Address),
            ("to", OptAddress),
            ("mint", Uint),
            ("value", Uint),
            ("gas", Uint),
            ("isSystemTx", Bool),
            ("input", Bytes),
        ],
        ARB_DEPOSIT => &[
            ("chainId", Uint),
            ("requestId", Hash),
            ("from", Address),
            ("to", Address),
            ("value", Uint),
        ],
        ARB_UNSIGNED => &[
            ("chainId", Uint),
            ("from", Address),
            ("nonce", Uint),
            ("maxFeePerGas", Uint),
            ("gas", Uint),
            ("to", OptAddress),
            ("value", Uint),
            ("input", Bytes),
        ],
        ARB_CONTRACT => &[
            ("chainId", Uint),
            ("requestId", Hash),
            ("from", Address),
            ("maxFeePerGas", Uint),
            ("gas", Uint),
            ("to", OptAddress),
            ("value", Uint),
            ("input", Bytes),
        ],
        ARB_RETRY => &[
            ("chainId", Uint),
            ("nonce", Uint),
            ("from", Address),
            ("maxFeePerGas", Uint),
            ("gas", Uint),
            ("to", OptAddress),
            ("value", Uint),
            ("input", Bytes),
            ("ticketId", Hash),
            ("refundTo", Address),
            ("maxRefund", Uint),
            ("submissionFeeRefund", Uint),
        ],
        ARB_SUBMIT_RETRYABLE => &[
            ("chainId", Uint),
            ("requestId", Hash),
            ("from", Address),
            ("l1BaseFee", Uint),
            ("depositValue", Uint),
            ("maxFeePerGas", Uint),
            ("gas", Uint),
            ("retryTo", OptAddress),
            ("retryValue", Uint),
            ("beneficiary", Address),
            ("maxSubmissionFee", Uint),
            ("refundTo", Address),
            ("retryData", Bytes),
        ],
        ARB_INTERNAL => &[("chainId", Uint), ("input", Bytes)],
        _ => return None,
    })
}

/// One decoded field of a table-driven type.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    Uint(U256),
    Bool(bool),
    Address(Option<[u8; 20]>),
    Hash([u8; 32]),
    Bytes(Vec<u8>),
}

/// A stored transaction, decoded.
#[derive(Clone, Debug)]
pub enum Decoded {
    /// Ethereum's types `0x00..=0x04`.
    Eth(Box<TxEnvelope>),
    /// An OP-stack deposit or a Nitro type: its fields in table order.
    Table { ty: u8, fields: Vec<Field> },
}

impl Decoded {
    pub fn ty(&self) -> u8 {
        match self {
            Decoded::Eth(env) => env.ty(),
            Decoded::Table { ty, .. } => *ty,
        }
    }

    /// A table-driven type's field by JSON name.
    pub fn field(&self, name: &str) -> Option<&Field> {
        let Decoded::Table { ty, fields } = self else {
            return None;
        };
        layout(*ty)?
            .iter()
            .position(|(n, _)| *n == name)
            .map(|i| &fields[i])
    }

    pub fn uint(&self, name: &str) -> Option<U256> {
        match self.field(name)? {
            Field::Uint(v) => Some(*v),
            _ => None,
        }
    }

    pub fn address(&self, name: &str) -> Option<[u8; 20]> {
        match self.field(name)? {
            Field::Address(a) => *a,
            _ => None,
        }
    }

    /// The recipient; `None` for a contract creation.
    pub fn to(&self) -> Option<[u8; 20]> {
        match self {
            Decoded::Eth(env) => env.to().map(|a| a.0 .0),
            Decoded::Table { ty, .. } => match *ty {
                ARB_INTERNAL => Some(ARBOS_ADDRESS),
                ARB_SUBMIT_RETRYABLE => Some(ARB_RETRYABLE_ADDRESS),
                _ => self.address("to"),
            },
        }
    }

    /// The nonce a node reports (a deposit's comes from its receipt).
    pub fn nonce(&self) -> u64 {
        match self {
            Decoded::Eth(env) => env.nonce(),
            _ => self.uint("nonce").map_or(0, |n| n.saturating_to()),
        }
    }

    pub fn gas(&self) -> u64 {
        match self {
            Decoded::Eth(env) => env.gas_limit(),
            _ => self.uint("gas").map_or(0, |n| n.saturating_to()),
        }
    }

    pub fn value(&self) -> U256 {
        match self {
            Decoded::Eth(env) => env.value(),
            Decoded::Table { ty, .. } if *ty == ARB_SUBMIT_RETRYABLE => U256::ZERO,
            _ => self.uint("value").unwrap_or_default(),
        }
    }

    /// The call data, as a node reports it (a submit-retryable's is the ABI
    /// call to ArbRetryableTx that it stands for).
    pub fn input(&self) -> Vec<u8> {
        match self {
            Decoded::Eth(env) => env.input().to_vec(),
            Decoded::Table { ty, .. } if *ty == ARB_SUBMIT_RETRYABLE => {
                submit_retryable_input(self)
            }
            _ => match self.field("input") {
                Some(Field::Bytes(b)) => b.clone(),
                _ => vec![],
            },
        }
    }
}

/// `submitRetryable(...)` calldata, as Nitro's `ArbitrumSubmitRetryableTx.data()` builds it.
fn submit_retryable_input(tx: &Decoded) -> Vec<u8> {
    let word = |v: U256| v.to_be_bytes::<32>();
    let addr = |a: Option<[u8; 20]>| {
        let mut w = [0u8; 32];
        w[12..].copy_from_slice(&a.unwrap_or_default());
        w
    };
    let hash = match tx.field("requestId") {
        Some(Field::Hash(h)) => *h,
        _ => [0; 32],
    };
    let data = match tx.field("retryData") {
        Some(Field::Bytes(b)) => b.clone(),
        _ => vec![],
    };
    let u = |n: &str| tx.uint(n).unwrap_or_default();
    let mut out = vec![0xc9, 0xf9, 0x5d, 0x32];
    out.extend_from_slice(&hash);
    for n in [
        "l1BaseFee",
        "depositValue",
        "retryValue",
        "maxFeePerGas",
        "gas",
        "maxSubmissionFee",
    ] {
        out.extend_from_slice(&word(u(n)));
    }
    for n in ["refundTo", "beneficiary", "retryTo"] {
        out.extend_from_slice(&addr(tx.address(n)));
    }
    out.extend_from_slice(&word(U256::from(11 * 32)));
    out.extend_from_slice(&word(U256::from(data.len())));
    out.extend_from_slice(&data);
    out.resize(out.len() + (32 - data.len() % 32) % 32, 0);
    out
}

/// A stored transaction's value → its fields. Values are trie values that
/// rebuilt a transactionsRoot, so a failure here is a bug, not bad input.
pub fn decode(value: &[u8], chain_id: u64) -> Result<Decoded> {
    let first = *value.first().context("empty transaction value")?;
    let table = layout(first).filter(|_| {
        (first == DEPOSIT_TX_TYPE && deposits_allowed(chain_id))
            || (NITRO_TX_TYPES.contains(&first) && nitro_allowed(chain_id))
    });
    let Some(table) = table else {
        let env = TxEnvelope::decode_2718(&mut &value[..]).context("transaction decode")?;
        return Ok(Decoded::Eth(Box::new(env)));
    };
    let (payload, total, is_list) = rlp_item(&value[1..])?;
    ensure!(
        is_list && total == value.len() - 1,
        "malformed 0x{first:02x} transaction"
    );
    let items = rlp_list_items(payload)?;
    ensure!(
        items.len() == table.len(),
        "0x{first:02x} transaction has {} fields, expected {}",
        items.len(),
        table.len()
    );
    let fields = items
        .iter()
        .zip(table)
        .map(|((b, is_list), (name, kind))| {
            ensure!(!is_list, "field {name} is a list");
            Ok(match kind {
                Uint => {
                    ensure!(b.len() <= 32 && b.first() != Some(&0), "{name}: bad scalar");
                    Field::Uint(U256::from_be_slice(b))
                }
                Bool => match *b {
                    [] => Field::Bool(false),
                    [1] => Field::Bool(true),
                    _ => bail!("{name}: bad bool"),
                },
                Address => Field::Address(Some((*b).try_into().context("address")?)),
                OptAddress => match b.len() {
                    0 => Field::Address(None),
                    _ => Field::Address(Some((*b).try_into().context("address")?)),
                },
                Hash => Field::Hash((*b).try_into().context("hash")?),
                Bytes => Field::Bytes(b.to_vec()),
            })
        })
        .collect::<Result<_>>()?;
    Ok(Decoded::Table { ty: first, fields })
}

/// Who sent a transaction: recovered from its signature, or the `from` its
/// bytes carry, or ArbOS for an internal transaction.
pub fn sender(tx: &Decoded) -> Result<[u8; 20]> {
    match tx {
        // Before Homestead (EIP-2) a legacy signature's s could be above n/2.
        Decoded::Eth(env) if env.is_legacy() && env.chain_id().is_none() => {
            let from = env
                .recover_signer_unchecked()
                .context("signature recovery")?;
            Ok(from.0 .0)
        }
        Decoded::Eth(env) => Ok(env.recover_signer().context("signature recovery")?.0 .0),
        Decoded::Table { ty, .. } if *ty == ARB_INTERNAL => Ok(ARBOS_ADDRESS),
        _ => tx.address("from").context("no from"),
    }
}
