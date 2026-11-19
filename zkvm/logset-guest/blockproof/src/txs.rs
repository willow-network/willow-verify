













use alloy::consensus::transaction::SignerRecoverable;
use alloy::consensus::{Transaction as _, TxEnvelope};
use alloy::eips::eip2718::Decodable2718;
use alloy::eips::Typed2718;
use alloy::primitives::U256;
use anyhow::{bail, ensure, Context, Result};
use logset_blockcheck::receipts::{
    deposits_allowed, fee_currency_allowed, nitro_allowed, pol_allowed, rlp_item, rlp_list_items,
    state_sync_allowed, DEPOSIT_TX_TYPE, FEE_CURRENCY_TX_TYPE, NITRO_TX_TYPES, STATE_SYNC_TX_TYPE,
};


pub use crate::nitro::{ARBOS_ADDRESS, ARB_RETRYABLE_ADDRESS};

pub use logset_blockcheck::transactions::{
    ARB_CONTRACT, ARB_DEPOSIT, ARB_INTERNAL, ARB_RETRY, ARB_SUBMIT_RETRYABLE, ARB_UNSIGNED,
};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {

    Uint,

    Bool,
    Address,

    OptAddress,
    Hash,
    Bytes,
}

use Kind::*;


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
        _ if NITRO_TX_TYPES.contains(&ty) => return crate::nitro::layout(ty),
        _ => return None,
    })
}



pub const POL_LAYOUT: &[(&str, Kind)] = &[
    ("chainId", Uint),
    ("from", Address),
    ("to", Address),
    ("nonce", Uint),
    ("gas", Uint),
    ("gasPrice", Uint),
    ("input", Bytes),
];




pub const MANTLE_DEPOSIT_LAYOUT: &[(&str, Kind)] = &[
    ("sourceHash", Hash),
    ("from", Address),
    ("to", OptAddress),
    ("mint", Uint),
    ("value", Uint),
    ("gas", Uint),
    ("isSystemTx", Bool),
    ("ethValue", Uint),
    ("input", Bytes),
    ("ethTxValue", Uint),
];


#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field {
    Uint(U256),
    Bool(bool),
    Address(Option<[u8; 20]>),
    Hash([u8; 32]),
    Bytes(Vec<u8>),
}


#[derive(Clone, Debug)]
pub enum Decoded {

    Eth(Box<TxEnvelope>),


    Table {
        ty: u8,
        fields: Vec<Field>,
        pol: bool,
    },

    FeeCurrency(Box<FeeCurrencyTx>),
}






#[derive(Clone, Debug)]
pub struct FeeCurrencyTx {



    pub fields: TxEnvelope,

    pub fee_currency: Option<[u8; 20]>,

    pub signing_hash: [u8; 32],
}

impl FeeCurrencyTx {

    pub fn decode(value: &[u8]) -> Result<Self> {
        ensure!(
            value.first() == Some(&FEE_CURRENCY_TX_TYPE),
            "not a 0x7B transaction"
        );
        let (payload, total, is_list) = rlp_item(&value[1..])?;
        ensure!(
            is_list && total == value.len() - 1,
            "malformed 0x7B transaction"
        );
        let items = raw_items(payload)?;
        ensure!(
            items.len() == 13,
            "0x7B transaction has {} fields, expected 13",
            items.len()
        );
        let (fee_currency, fc_list) = {
            let (b, _, l) = rlp_item(items[9])?;
            (b, l)
        };
        ensure!(!fc_list, "feeCurrency is a list");
        let fee_currency: Option<[u8; 20]> = match fee_currency.len() {
            0 => None,
            20 => Some(fee_currency.try_into().expect("20 bytes")),
            n => bail!("feeCurrency of {n} bytes"),
        };

        let eip1559: Vec<&[u8]> = items[..9].iter().chain(&items[10..]).copied().collect();
        let mut bytes = vec![0x02];
        bytes.extend(rlp_list(&eip1559));
        let fields = TxEnvelope::decode_2718(&mut &bytes[..]).context("0x7B fields")?;
        ensure!(fields.is_eip1559(), "0x7B fields decode as another type");
        let mut signed = vec![FEE_CURRENCY_TX_TYPE];
        signed.extend(rlp_list(&items[..10]));
        Ok(Self {
            fields,
            fee_currency,
            signing_hash: alloy::primitives::keccak256(&signed).0,
        })
    }



    pub fn sender(&self) -> Result<[u8; 20]> {
        let TxEnvelope::Eip1559(s) = &self.fields else {
            bail!("0x7B fields are not EIP-1559's");
        };
        let from = alloy::consensus::crypto::secp256k1::recover_signer(
            s.signature(),
            self.signing_hash.into(),
        )
        .context("signature recovery")?;
        Ok(from.0 .0)
    }
}


fn raw_items(payload: &[u8]) -> Result<Vec<&[u8]>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    while at < payload.len() {
        let (_, total, _) = rlp_item(&payload[at..])?;
        out.push(&payload[at..at + total]);
        at += total;
    }
    Ok(out)
}


fn rlp_list(items: &[&[u8]]) -> Vec<u8> {
    let payload: usize = items.iter().map(|i| i.len()).sum();
    let mut out = Vec::with_capacity(payload + 9);
    alloy_rlp::Header {
        list: true,
        payload_length: payload,
    }
    .encode(&mut out);
    for i in items {
        out.extend_from_slice(i);
    }
    out
}

impl Decoded {










    pub fn ty(&self) -> u8 {
        match self {
            Decoded::Eth(env) => env.ty(),
            Decoded::Table { ty, .. } => *ty,
            Decoded::FeeCurrency(_) => FEE_CURRENCY_TX_TYPE,
        }
    }


    pub fn field(&self, name: &str) -> Option<&Field> {
        let Decoded::Table { ty, fields, pol } = self else {
            return None;
        };

        let table = if *pol {
            Some(POL_LAYOUT)
        } else if *ty == DEPOSIT_TX_TYPE && fields.len() > 8 {
            Some(MANTLE_DEPOSIT_LAYOUT)
        } else {
            layout(*ty)
        };
        table?
            .iter()
            .position(|(n, _)| *n == name)
            .and_then(|i| fields.get(i))
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


    pub fn to(&self) -> Option<[u8; 20]> {
        match self {
            Decoded::Eth(env) => env.to().map(|a| a.0 .0),
            Decoded::FeeCurrency(c) => c.fields.to().map(|a| a.0 .0),
            Decoded::Table { ty, .. } => match *ty {
                t if NITRO_TX_TYPES.contains(&t) => crate::nitro::to(self),

                STATE_SYNC_TX_TYPE => Some([0; 20]),
                _ => self.address("to"),
            },
        }
    }


    pub fn nonce(&self) -> u64 {
        match self {
            Decoded::Eth(env) => env.nonce(),
            Decoded::FeeCurrency(c) => c.fields.nonce(),
            _ => self.uint("nonce").map_or(0, |n| n.saturating_to()),
        }
    }

    pub fn gas(&self) -> u64 {
        match self {
            Decoded::Eth(env) => env.gas_limit(),
            Decoded::FeeCurrency(c) => c.fields.gas_limit(),
            _ => self.uint("gas").map_or(0, |n| n.saturating_to()),
        }
    }

    pub fn value(&self) -> U256 {
        match self {
            Decoded::Eth(env) => env.value(),
            Decoded::FeeCurrency(c) => c.fields.value(),
            Decoded::Table { ty, .. } if NITRO_TX_TYPES.contains(ty) => crate::nitro::value(self),
            _ => self.uint("value").unwrap_or_default(),
        }
    }



    pub fn input(&self) -> Vec<u8> {
        match self {
            Decoded::Eth(env) => env.input().to_vec(),
            Decoded::FeeCurrency(c) => c.fields.input().to_vec(),
            Decoded::Table { ty, .. } if NITRO_TX_TYPES.contains(ty) => crate::nitro::input(self),
            _ => match self.field("input") {
                Some(Field::Bytes(b)) => b.clone(),
                _ => vec![],
            },
        }
    }
}



pub fn decode(value: &[u8], chain_id: u64) -> Result<Decoded> {
    let first = *value.first().context("empty transaction value")?;
    if first == FEE_CURRENCY_TX_TYPE && fee_currency_allowed(chain_id) {
        let tx = FeeCurrencyTx::decode(value)?;
        return Ok(Decoded::FeeCurrency(Box::new(tx)));
    }


    if first == STATE_SYNC_TX_TYPE && state_sync_allowed(chain_id) {
        logset_blockcheck::transactions::state_sync_records(value)?;
        return Ok(Decoded::Table {
            ty: STATE_SYNC_TX_TYPE,
            fields: vec![],
            pol: false,
        });
    }
    let pol = first == DEPOSIT_TX_TYPE && pol_allowed(chain_id);
    let mantle = first == DEPOSIT_TX_TYPE
        && deposits_allowed(chain_id)
        && logset_blockcheck::header::mantle(chain_id);
    let table = if pol {
        Some(POL_LAYOUT)
    } else if mantle {
        Some(MANTLE_DEPOSIT_LAYOUT)
    } else {
        layout(first).filter(|_| {
            (first == DEPOSIT_TX_TYPE && deposits_allowed(chain_id))
                || (NITRO_TX_TYPES.contains(&first) && nitro_allowed(chain_id))
        })
    };
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
        items.len() == table.len() || (mantle && items.len() + 1 == table.len()),
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
    Ok(Decoded::Table {
        ty: first,
        fields,
        pol,
    })
}



pub fn sender(tx: &Decoded) -> Result<[u8; 20]> {
    match tx {

        Decoded::Eth(env) if env.is_legacy() && env.chain_id().is_none() => {
            let from = env
                .recover_signer_unchecked()
                .context("signature recovery")?;
            Ok(from.0 .0)
        }
        Decoded::Eth(env) => Ok(env.recover_signer().context("signature recovery")?.0 .0),
        Decoded::FeeCurrency(c) => c.sender(),
        Decoded::Table { ty, .. } if NITRO_TX_TYPES.contains(ty) => crate::nitro::sender(tx),

        Decoded::Table { ty, .. } if *ty == STATE_SYNC_TX_TYPE => Ok([0; 20]),
        _ => tx.address("from").context("no from"),
    }
}
