





use crate::call::{self, calldata, word};
use crate::history::{self, ETHEREUM_RING};
use crate::l2::{header, nitro_code_hash, walk_back, MAX_HOPS, NITRO_RING};
use crate::lc::{bytes_of, Execution, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, Signature};
use serde_json::Value;



pub const MAX_SIGNED_AGE_SECS: u64 = 3 * 3600;




pub enum KeyOn {
    SystemConfig(Address),
    SequencerInbox { inbox: Address, first_block: u64 },
}

pub struct Sequenced {
    pub chain_id: u64,
    pub name: &'static str,
    pub key: KeyOn,
}



pub const SEQUENCED: [Sequenced; 15] = [
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
    Sequenced {
        chain_id: 1868,
        name: "Soneium",
        key: KeyOn::SystemConfig(address!("7A8Ed66B319911A0F3E7288BDdAB30d9c0C875c3")),
    },
    Sequenced {
        chain_id: 7777777,
        name: "Zora",
        key: KeyOn::SystemConfig(address!("A3cAB0126d5F504B071b81a3e8A2BBBF17930d86")),
    },
    Sequenced {
        chain_id: 480,
        name: "World Chain",
        key: KeyOn::SystemConfig(address!("6ab0777fD0e609CE58F939a7F70Fe41F5Aa6300A")),
    },
    Sequenced {
        chain_id: 11155420,
        name: "OP Sepolia",
        key: KeyOn::SystemConfig(address!("034edD2A225f7f429A63E0f1D2084B9E0A93b538")),
    },
    Sequenced {
        chain_id: 84532,
        name: "Base Sepolia",
        key: KeyOn::SystemConfig(address!("f272670eb55e895584501d564AfEB048bEd26194")),
    },
    Sequenced {
        chain_id: 1301,
        name: "Unichain Sepolia",
        key: KeyOn::SystemConfig(address!("aeE94b9aB7752D3F7704bDE212c0C6A0b701571D")),
    },
    Sequenced {
        chain_id: 763373,
        name: "Ink Sepolia",
        key: KeyOn::SystemConfig(address!("05C993e60179f28bF649a2Bb5b00b5F4283bD525")),
    },
    Sequenced {
        chain_id: 421614,
        name: "Arbitrum Sepolia",
        key: KeyOn::SequencerInbox {
            inbox: address!("6c97864CE4bEf387dE0b3310A44230f7E3F1be0D"),
            first_block: 0,
        },
    },
    Sequenced {
        chain_id: 46630,
        name: "Robinhood Chain testnet",
        key: KeyOn::SequencerInbox {
            inbox: address!("A0D9dB3DC9791D54b5183C1C1866eFe1eCA7D414"),
            first_block: 0,
        },
    },
];


const PAYLOAD_SIZES: [usize; 4] = [508, 512, 528, 560];

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedConfirmed {
    pub hash: [u8; 32],

    pub anchor_block: u64,
    pub anchor_time: u64,
    pub signer: Address,

    pub l1_block: u64,
}




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


    if u32::from_le_bytes(p[436..440].try_into().unwrap()) as usize != fixed {
        return Err(rejected("the signed payload does not match its version"));
    }
    let number = u64::from_le_bytes(p[404..412].try_into().unwrap());
    Ok((number, p[472..504].try_into().unwrap(), signer))
}

pub use crate::feed::{feed_signature_hash, nitro_signed_block};




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
        KeyOn::SequencerInbox { inbox, first_block } => crate::feed::nitro_sequenced_block(
            chain_id,
            chain.name,
            inbox,
            first_block,
            signed,
            &|to: Address, data: alloy_primitives::Bytes| call::call(&state, &l1, to, data),
        )?,
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


    let (ring, code) = match chain.key {
        KeyOn::SystemConfig(_) => (ETHEREUM_RING, history::ethereum_code_hash()),
        KeyOn::SequencerInbox { .. } => (NITRO_RING, nitro_code_hash()),
    };
    let hash = walk_back(&top, number, headers, answer, ring, code)?;
    Ok(SignedConfirmed {
        hash,
        anchor_block: top.number,
        anchor_time: top.timestamp,
        signer,
        l1_block: l1.number,
    })
}
