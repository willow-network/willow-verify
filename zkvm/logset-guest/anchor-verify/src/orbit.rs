











use crate::call::{self, address_word, calldata, word};
use crate::l2::{
    as_bool, confirm_nitro, header, nitro_code_hash, rlp_items, uint, walk_back, MAX_HOPS,
    MAX_L1_AGE_SECS, NITRO_RING,
};
use crate::lc::{bytes_of, Execution, LightClient};
use crate::signed::confirm_signed;
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, Bytes, U256};
use serde_json::Value;


pub struct OrbitChain {
    pub chain_id: u64,
    pub name: &'static str,
    pub parent: u64,
    pub rollup: Address,
    pub proposers: &'static [Address],
}


pub const APECHAIN: OrbitChain = OrbitChain {
    chain_id: 33139,
    name: "ApeChain",
    parent: 42161,
    rollup: address!("374de579AE15aD59eD0519aeAf1A23F348Df259c"),
    proposers: &[address!("AcB7D670bb95144B88a5Cd1883B87bC5021FD10a")],
};


pub const ORBIT_CHAINS: [&OrbitChain; 1] = [&APECHAIN];


pub fn orbit_chain(chain_id: u64) -> Option<&'static OrbitChain> {
    ORBIT_CHAINS
        .iter()
        .copied()
        .find(|c| c.chain_id == chain_id)
}


pub const MAX_PENDING: u64 = 1024;


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrbitConfirmed {
    pub level: &'static str,
    pub hash: [u8; 32],
    pub anchor_block: u64,
    pub node: u64,
    pub parent_block: u64,
    pub parent_level: &'static str,
    pub parent_time: u64,
    pub l1_block: u64,
}

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}


fn rank(level: &str) -> u8 {
    match level {
        "final" => 2,
        "committed" => 1,
        _ => 0,
    }
}


pub fn weaker(a: &'static str, b: &'static str) -> &'static str {
    if rank(a) <= rank(b) {
        a
    } else {
        b
    }
}


pub fn execution_of(rlp_hex: &str) -> Result<Execution, Failure> {
    let rlp = bytes_of(rlp_hex, "the parent header")?;
    let f = rlp_items(&rlp)?;
    if f.len() < 16 {
        return Err(bad("the parent header has too few fields"));
    }
    if f[15].len() > 32 {
        return Err(bad("the parent's base fee is over 32 bytes"));
    }
    let b32 = |item: &[u8], what: &str| -> Result<[u8; 32], Failure> {
        item.try_into()
            .map_err(|_| bad(format!("{what} is not 32 bytes")))
    };
    let number = uint(f[8], "number")?;
    let hash = keccak256(&rlp).0;
    let state_root = b32(f[3], "stateRoot")?;
    let timestamp = uint(f[11], "timestamp")?;
    let gas_limit = uint(f[9], "gasLimit")?;
    let base_fee = U256::from_be_slice(f[15]);
    let prev_randao = b32(f[13], "mixHash")?;
    let fee_recipient: [u8; 20] = f[2]
        .try_into()
        .map_err(|_| bad("the parent's coinbase is not 20 bytes"))?;
    Ok(Execution {
        number,
        hash,
        state_root,
        timestamp,
        gas_limit,
        base_fee,
        prev_randao,
        fee_recipient,
    })
}


fn node_word(k: u64) -> [u8; 32] {
    U256::from(k).to_be_bytes::<32>()
}


fn word_u64(w: [u8; 32], what: &str) -> Result<u64, Failure> {
    u64::try_from(U256::from_be_bytes(w)).map_err(|_| rejected(format!("{what} is over 64 bits")))
}



pub fn confirm_orbit(
    lc: &LightClient,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<OrbitConfirmed, Failure> {
    let chain = orbit_chain(chain_id).ok_or_else(|| {
        Failure::new(
            Kind::Internal,
            format!("chain {chain_id} is not an Orbit chain here"),
        )
    })?;
    crate::asked_block(number, answer)?;


    let parent = answer
        .get("parent")
        .ok_or_else(|| bad("the anchor has no parent block"))?;
    let anchor = parent
        .get("anchor")
        .ok_or_else(|| bad("the parent block has no anchor"))?;
    let a = anchor
        .get("blockNumber")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| bad("the parent's anchor has no blockNumber"))?;
    let (parent_hash, parent_level, l1_block) = if anchor.get("signed").is_some() {
        let c = confirm_signed(lc, chain.parent, a, anchor, now)?;
        (c.hash, "signed", c.l1_block)
    } else {
        let c = confirm_nitro(lc, chain.parent, a, anchor, now)?;
        (c.hash, c.level, c.l1_block)
    };
    let block: Execution = execution_of(
        parent
            .get("header")
            .and_then(Value::as_str)
            .ok_or_else(|| bad("the parent block has no header"))?,
    )?;
    if block.hash != parent_hash || block.number != a {
        return Err(rejected(format!(
            "the parent header is not block {a} as anchored"
        )));
    }
    let state = call::prove_state(
        block.state_root.into(),
        answer
            .get("parentState")
            .ok_or_else(|| bad("the anchor has no parent state"))?,
    )?;
    let rollup = |data: Bytes| call::call_on(&state, &block, chain.parent, chain.rollup, data);
    let get_node = |k: u64| rollup(calldata("getNode(uint64)", &[node_word(k)]));


    let k = answer
        .get("node")
        .and_then(Value::as_u64)
        .ok_or_else(|| bad("the anchor names no node"))?;
    let c = word_u64(
        word(&rollup(calldata("latestConfirmed()", &[]))?, 0)?,
        "a node number",
    )?;
    let node = get_node(k)?;
    let node_level = if k == c {
        "final"
    } else if k > c {
        let age = now.saturating_sub(block.timestamp);
        if age > MAX_L1_AGE_SECS {
            return Err(Failure::new(
                Kind::StaleHead,
                format!(
                    "the block of chain {} that {}'s node was read at is {} minutes old",
                    chain.parent,
                    chain.name,
                    age / 60
                ),
            ));
        }
        if k - c > MAX_PENDING {
            return Err(Failure::new(
                Kind::NotCovered,
                format!("node {k} is more than {MAX_PENDING} nodes past the confirmed one"),
            ));
        }
        let unresolved = word_u64(
            word(&rollup(calldata("firstUnresolvedNode()", &[]))?, 0)?,
            "a node",
        )?;
        let mut staked = false;
        for v in chain.proposers {
            let out = rollup(calldata(
                "nodeHasStaker(uint64,address)",
                &[node_word(k), address_word(*v)],
            ))?;
            staked |= as_bool(&out)?;
        }
        if unresolved != c + 1 || !staked {
            return Err(Failure::new(
                Kind::Declined,
                format!(
                    "node {k} is not a standing claim by {}'s validator",
                    chain.name
                ),
            ));
        }

        let mut before = get_node(c)?;
        for x in c + 1..=k {
            let this = if x == k { node.clone() } else { get_node(x)? };
            if word_u64(word(&this, 3)?, "a node")? != x - 1
                || word_u64(word(&before, 9)?, "a node")? != x
            {
                return Err(Failure::new(
                    Kind::Disputed,
                    format!(
                        "another node on {} contests {}'s node {x}",
                        chain.name, chain.name
                    ),
                ));
            }
            before = this;
        }
        "committed"
    } else {
        return Err(Failure::new(
            Kind::Declined,
            format!(
                "node {k} is neither {}'s confirmed node nor pending",
                chain.name
            ),
        ));
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
    let confirms = top.extra_data.len() == 32 && {
        let mut pre = [0u8; 64];
        pre[..32].copy_from_slice(&top.hash);
        pre[32..].copy_from_slice(&top.extra_data);
        keccak256(pre).0 == word(&node, 2)?
    };
    if !confirms {
        return Err(rejected(format!(
            "block {}'s header is not node {k}'s",
            top.number
        )));
    }
    if number > top.number {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is after node {k}'s block"),
        ));
    }
    let hash = walk_back(&top, number, headers, answer, NITRO_RING, nitro_code_hash())?;
    Ok(OrbitConfirmed {
        level: weaker(parent_level, node_level),
        hash,
        anchor_block: top.number,
        node: k,
        parent_block: a,
        parent_level,
        parent_time: block.timestamp,
        l1_block,
    })
}
