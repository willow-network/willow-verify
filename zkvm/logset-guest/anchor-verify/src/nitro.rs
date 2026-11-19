











use crate::call::{self, calldata, word};
use crate::history;
use crate::l2::{as_bool, descend, ethereum_at, header};
use crate::lc::{bytes_of, Execution, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, B256};
use serde_json::Value;


pub struct NitroChain {
    pub chain_id: u64,
    pub name: &'static str,
    pub rollup: Address,
}


pub const NITRO_CHAINS: [NitroChain; 4] = [
    NitroChain {
        chain_id: 42161,
        name: "Arbitrum One",
        rollup: address!("4DCeB440657f21083db8aDd07665f8ddBe1DCfc0"),
    },
    NitroChain {
        chain_id: 4663,
        name: "Robinhood Chain",
        rollup: address!("23A19d23e89166adedbDcB432518AB01e4272D94"),
    },
    NitroChain {
        chain_id: 421614,
        name: "Arbitrum Sepolia",
        rollup: address!("042B2E6C5E99d4c521bd49beeD5E99651D9B0Cf4"),
    },
    NitroChain {
        chain_id: 46630,
        name: "Robinhood Chain testnet",
        rollup: address!("dc5F8E399DBd8a9F5F87AeC4C23Beb12431b386D"),
    },
];


pub const NITRO_RING: u64 = 393_168;

pub const NITRO_HISTORY_CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe1460605760203603605c575f3563a3b1b31d5f5260205f6004601c60645afa15605c575f51600181038211605c57816205ffd0910311605c576205ffd09006545f5260205ff35b5f5ffd5b5f356205ffd0600163a3b1b31d5f5260205f6004601c60645afa15605c575f5103065500";


const MAX_ANCESTORS: usize = 1024;

const PENDING: u8 = 1;
const CONFIRMED: u8 = 2;

const FINISHED: [u8; 32] = {
    let mut w = [0u8; 32];
    w[31] = 1;
    w
};

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}


pub fn nitro_code_hash() -> B256 {
    keccak256(hex::decode(NITRO_HISTORY_CODE).expect("constant hex"))
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NitroConfirmed {
    pub level: &'static str,
    pub hash: [u8; 32],
    pub anchor_block: u64,
    pub assertion: [u8; 32],
    pub l1_block: u64,
}



fn hex_word(v: Option<&Value>, what: &str) -> Result<[u8; 32], Failure> {
    let s = v
        .and_then(Value::as_str)
        .ok_or_else(|| bad(format!("{what} is not hex")))?;
    bytes_of(s, what)?
        .try_into()
        .map_err(|_| bad(format!("{what} is not 32 bytes")))
}


fn hash3(a: &[u8; 32], b: &[u8; 32], c: &[u8; 32]) -> [u8; 32] {
    let mut pre = [0u8; 96];
    pre[..32].copy_from_slice(a);
    pre[32..64].copy_from_slice(b);
    pre[64..].copy_from_slice(c);
    keccak256(pre).0
}


fn on_rollup(
    state: &call::State,
    l1: &Execution,
    chain: &NitroChain,
    sig: &str,
    arg: [u8; 32],
) -> Result<alloy_primitives::Bytes, Failure> {
    call::call(state, l1, chain.rollup, calldata(sig, &[arg]))
}


fn status_of(
    state: &call::State,
    l1: &Execution,
    chain: &NitroChain,
    h: [u8; 32],
) -> Result<u8, Failure> {
    let out = on_rollup(state, l1, chain, "getAssertion(bytes32)", h)?;
    let w = word(&out, 4)?;
    if w[..31].iter().any(|b| *b != 0) {
        return Err(rejected("an assertion status out of range"));
    }
    Ok(w[31])
}


pub fn confirm_nitro(
    lc: &LightClient,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<NitroConfirmed, Failure> {
    let chain = NITRO_CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            Failure::new(
                Kind::Internal,
                format!("chain {chain_id} is not a Nitro chain here"),
            )
        })?;
    let (l1, state) = ethereum_at(lc, number, answer, now)?;


    let a = answer
        .get("assertion")
        .ok_or_else(|| bad("the anchor names no assertion"))?;
    let parent = hex_word(a.get("parent"), "the parent assertion")?;
    let inbox_acc = hex_word(a.get("inboxAcc"), "the inbox accumulator")?;
    let words = a
        .get("state")
        .and_then(Value::as_array)
        .filter(|s| s.len() == 6)
        .ok_or_else(|| bad("the assertion state is not 6 words"))?;
    let mut after = [[0u8; 32]; 6];
    for (w, v) in after.iter_mut().zip(words) {
        *w = hex_word(Some(v), "an assertion state word")?;
    }
    let state_hash = keccak256(after.concat()).0;
    let assertion = hash3(&parent, &state_hash, &inbox_acc);
    if after[4] != FINISHED {
        return Err(rejected("the assertion did not finish its block"));
    }


    let level = match status_of(&state, &l1, chain, assertion)? {
        CONFIRMED => "final",
        PENDING => "committed",
        _ => {
            return Err(rejected(format!(
                "{} has no assertion 0x{} on Ethereum",
                chain.name,
                hex::encode(assertion)
            )))
        }
    };
    if level == "committed" {
        let ancestors = a
            .get("ancestors")
            .and_then(Value::as_array)
            .filter(|l| l.len() <= MAX_ANCESTORS)
            .ok_or_else(|| bad("the anchor has no list of the claims it builds on"))?;
        pending_chain_stands(&state, &l1, chain, number, assertion, parent, ancestors)?;
    }


    let headers = answer
        .get("headers")
        .and_then(Value::as_array)
        .filter(|h| h.len() == 1)
        .ok_or_else(|| bad("the anchor has no L2 header"))?;
    let top = header(
        headers[0]
            .as_str()
            .ok_or_else(|| bad("a header is not hex"))?,
    )?;
    if top.hash != after[0] || top.extra_data[..] != after[1][..] {
        return Err(rejected(
            "the asserted block's header is not the assertion's",
        ));
    }
    if number > top.number {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is after the asserted block"),
        ));
    }


    let low = descend(&top, number, answer)?;
    let hash = if low.number == number {
        low.hash
    } else {
        let proof = answer
            .get("proofs")
            .and_then(Value::as_array)
            .and_then(|p| p.first())
            .ok_or_else(|| bad("the anchor has no L2 history proof"))?;
        history::block_hash(
            low.state_root,
            low.number,
            number,
            NITRO_RING,
            nitro_code_hash(),
            proof,
        )?
    };
    Ok(NitroConfirmed {
        level,
        hash,
        anchor_block: top.number,
        assertion,
        l1_block: l1.number,
    })
}




pub(crate) fn pending_chain_stands(
    state: &call::State,
    l1: &Execution,
    chain: &NitroChain,
    number: u64,
    assertion: [u8; 32],
    parent: [u8; 32],
    ancestors: &[Value],
) -> Result<(), Failure> {
    let contested = || {
        Failure::new(
            Kind::Disputed,
            format!(
                "another claim on Ethereum contests {}'s claim for block {number}",
                chain.name
            ),
        )
    };
    let (mut x, mut p) = (assertion, parent);
    let mut sent = ancestors.iter();
    loop {

        if !as_bool(&on_rollup(state, l1, chain, "isFirstChild(bytes32)", x)?)? {
            return Err(contested());
        }
        let second = word(
            &on_rollup(state, l1, chain, "getSecondChildCreationBlock(bytes32)", p)?,
            0,
        )?;
        if second.iter().any(|b| *b != 0) {
            return Err(contested());
        }

        if !as_bool(&on_rollup(state, l1, chain, "isPending(bytes32)", p)?)? {
            if status_of(state, l1, chain, p)? != CONFIRMED {
                return Err(rejected("a claim builds on one Ethereum does not have"));
            }
            return Ok(());
        }

        let pre = sent.next().ok_or_else(|| {
            Failure::new(
                Kind::NotCovered,
                format!(
                    "Willow did not send every claim {}'s claim for block {number} builds on",
                    chain.name
                ),
            )
        })?;
        let grand = hex_word(pre.get("parent"), "an earlier claim's parent")?;
        let state_hash = hex_word(pre.get("stateHash"), "an earlier claim's state")?;
        let inbox = hex_word(pre.get("inboxAcc"), "an earlier claim's inbox")?;
        if hash3(&grand, &state_hash, &inbox) != p {
            return Err(rejected(
                "an earlier claim does not hash to its child's parent",
            ));
        }
        x = p;
        p = grand;
    }
}
