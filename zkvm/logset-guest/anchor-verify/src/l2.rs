




use crate::call::{self, address_word, calldata, word};
use crate::history::{self, ETHEREUM_RING};
use crate::lc::{bytes_of, Execution, Finality, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, Bytes};
use serde_json::Value;




pub struct OpChain {
    pub chain_id: u64,
    pub name: &'static str,
    pub registry: Address,

    pub proposers: &'static [Address],
    pub super_root: bool,
}











pub const OP_CHAINS: [OpChain; 13] = [
    OpChain {
        chain_id: 8453,
        name: "Base",
        registry: address!("909f6cf47ed12f010A796527f562bFc26C7F4E72"),
        proposers: &[address!("c1366Fabe614d42D367A1ecE61821238A1d31cF5")],
        super_root: false,
    },
    OpChain {
        chain_id: 10,
        name: "OP Mainnet",
        registry: address!("23B2C62946350F4246f9f9D027e071f0264FD113"),
        proposers: &[address!("473300df21D047806A082244b417f96b32f13A33")],
        super_root: true,
    },
    OpChain {
        chain_id: 130,
        name: "Unichain",
        registry: address!("27Cf508E4E3Aa8d30b3226aC3b5Ea0e8bcaCAFF9"),
        proposers: &[address!("86b392ca30ad7453c8d88b56eb5a04fd0e2175b7")],
        super_root: true,
    },
    OpChain {
        chain_id: 57073,
        name: "Ink",
        registry: address!("Ee018bAf058227872540AC60eFbd38b023d9dAe2"),
        proposers: &[address!("3832bfbeF03173E4C49a00ec0DD178817A02D177")],
        super_root: true,
    },
    OpChain {
        chain_id: 1868,
        name: "Soneium",
        registry: address!("4890928941e62e273dA359374b105F803329F473"),
        proposers: &[address!("400c164C4a8cA84385B70EEd6eB03ea847c8E1b8")],
        super_root: true,
    },
    OpChain {
        chain_id: 7777777,
        name: "Zora",
        registry: address!("54027b388330415a34b2dBa9E6d25895649eEFf1"),
        proposers: &[address!("48247032092e7b0ecf5dEF611ad89eaf3fC888Dd")],
        super_root: false,
    },
    OpChain {
        chain_id: 480,
        name: "World Chain",
        registry: address!("90BB48fe3310499Db36437dCAE642F721e32d094"),
        proposers: &[address!("2307278fC8aB0005974A6DeD2FA6d1187333a223")],
        super_root: false,
    },
    OpChain {
        chain_id: 42220,
        name: "Celo",
        registry: address!("8fE58d2168b5412Cf1Bd212cE6137f8b7300222d"),
        proposers: &[address!("79D14553D6B3484F5612272B43c219A882415d33")],
        super_root: false,
    },
    OpChain {
        chain_id: 196,
        name: "X Layer",
        registry: address!("000590BB65ab1864a7AD46d6B957cC9a4F2C149d"),
        proposers: &[address!("E43944421681170648E10007f73816e04F74394F")],
        super_root: false,
    },
    OpChain {
        chain_id: 11155420,
        name: "OP Sepolia",
        registry: address!("a1Cec548926eb5d69aa3B7B57d371EdBdD03e64b"),
        proposers: &[address!("9884C33C65111DA7Bbed0e63cD2392F503b135F5")],
        super_root: true,
    },
    OpChain {
        chain_id: 84532,
        name: "Base Sepolia",
        registry: address!("2fF5cC82dBf333Ea30D8ee462178ab1707315355"),
        proposers: &[address!("db84125F2F4229c81c579F41BC129C71B174Eb58")],
        super_root: false,
    },
    OpChain {
        chain_id: 1301,
        name: "Unichain Sepolia",
        registry: address!("Bb6cA820978442750B682663efA851AD4131127b"),
        proposers: &[address!("97E74f93DF7Fb8eb2542d0Cc42DFB21AaBB40800")],
        super_root: true,
    },
    OpChain {
        chain_id: 763373,
        name: "Ink Sepolia",
        registry: address!("299D7Ea9f0B584cfaF2a5341D151B44967594cA9"),
        proposers: &[address!("2282d49d805333D8cd6ddda52B32aC07d6e4e51B")],
        super_root: true,
    },
];


pub const SEPOLIA_CHAINS: [u64; 6] = [11_155_420, 84_532, 1_301, 763_373, 421_614, 46_630];

pub fn on_sepolia(chain_id: u64) -> bool {
    SEPOLIA_CHAINS.contains(&chain_id)
}



pub const MAX_L1_AGE_SECS: u64 = 3600;

pub(crate) const MAX_HOPS: usize = 16;

pub const MAX_DESCENT: usize = 32_768;
pub const MAX_DESCENT_BYTES: usize = 16 << 20;

const CHALLENGER_WINS: u8 = 1;

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2Header {
    pub hash: [u8; 32],
    pub parent_hash: [u8; 32],
    pub state_root: [u8; 32],
    pub number: u64,
    pub timestamp: u64,

    pub withdrawals_root: Option<[u8; 32]>,

    pub extra_data: Vec<u8>,
}


pub(crate) fn rlp_items(mut b: &[u8]) -> Result<Vec<&[u8]>, Failure> {
    let h = alloy_rlp::Header::decode(&mut b).map_err(|_| bad("a header is not RLP"))?;
    if !h.list || h.payload_length != b.len() {
        return Err(bad("a header is not one RLP list"));
    }
    let mut items = Vec::new();
    while !b.is_empty() {
        let h = alloy_rlp::Header::decode(&mut b).map_err(|_| bad("a header field is not RLP"))?;
        if h.list || h.payload_length > b.len() {
            return Err(bad("a header field is not a byte string"));
        }
        items.push(&b[..h.payload_length]);
        b = &b[h.payload_length..];
    }
    Ok(items)
}

fn b32(item: &[u8], what: &str) -> Result<[u8; 32], Failure> {
    item.try_into()
        .map_err(|_| bad(format!("{what} is not 32 bytes")))
}

pub(crate) fn uint(item: &[u8], what: &str) -> Result<u64, Failure> {
    if item.len() > 8 || item.first() == Some(&0) {
        return Err(bad(format!("{what} is not a canonical uint64")));
    }
    Ok(item.iter().fold(0u64, |a, &x| (a << 8) | x as u64))
}

pub fn header(rlp_hex: &str) -> Result<L2Header, Failure> {
    let rlp = bytes_of(rlp_hex, "header")?;
    let f = rlp_items(&rlp)?;
    if f.len() < 15 {
        return Err(bad("a header with too few fields"));
    }
    Ok(L2Header {
        hash: keccak256(&rlp).0,
        parent_hash: b32(f[0], "parentHash")?,
        state_root: b32(f[3], "stateRoot")?,
        number: uint(f[8], "number")?,
        timestamp: uint(f[11], "timestamp")?,
        withdrawals_root: f.get(16).map(|w| b32(w, "withdrawalsRoot")).transpose()?,
        extra_data: f[12].to_vec(),
    })
}





pub fn descend(top: &L2Header, number: u64, answer: &Value) -> Result<L2Header, Failure> {
    let Some(d) = answer.get("descent") else {
        return Ok(top.clone());
    };
    let list = d
        .as_array()
        .ok_or_else(|| bad("the descent is not a list of headers"))?;
    let bytes: usize = list
        .iter()
        .map(|h| h.as_str().map_or(0, str::len) / 2)
        .sum();
    if list.len() > MAX_DESCENT || bytes > MAX_DESCENT_BYTES {
        return Err(Failure::new(
            Kind::NotCovered,
            format!(
                "the descent from block {} is over {MAX_DESCENT} headers or {} MB",
                top.number,
                MAX_DESCENT_BYTES >> 20
            ),
        ));
    }
    let mut current = top.clone();
    for h in list {
        if current.number <= number {
            return Err(rejected(format!(
                "the descent goes on below block {number}"
            )));
        }
        let older = header(
            h.as_str()
                .ok_or_else(|| bad("a descent header is not hex"))?,
        )?;
        if older.hash != current.parent_hash || older.number + 1 != current.number {
            return Err(rejected(format!(
                "the descent's header of block {} is not block {}'s parent",
                older.number, current.number
            )));
        }
        current = older;
    }
    Ok(current)
}





pub(crate) fn walk_back(
    top: &L2Header,
    number: u64,
    headers: &[Value],
    answer: &Value,
    ring: u64,
    code: alloy_primitives::B256,
) -> Result<[u8; 32], Failure> {
    let mut current = descend(top, number, answer)?;
    if current.number == number {
        return Ok(current.hash);
    }
    let proofs = answer
        .get("proofs")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("the anchor has no L2 history proofs"))?;
    let mut next_header = 1;
    let mut proofs = proofs.iter();
    loop {
        if current.number == number {
            return Ok(current.hash);
        }
        let proof = proofs
            .next()
            .ok_or_else(|| bad("too few L2 history proofs"))?;
        if current.number - number <= ring {
            return history::block_hash(
                current.state_root,
                current.number,
                number,
                ring,
                code,
                proof,
            );
        }
        let back = current.number - ring;
        let h = history::block_hash(current.state_root, current.number, back, ring, code, proof)?;
        let older = header(
            headers
                .get(next_header)
                .and_then(Value::as_str)
                .ok_or_else(|| bad("too few L2 headers"))?,
        )?;
        next_header += 1;
        if older.hash != h || older.number != back {
            return Err(rejected(format!(
                "the header of block {back} is not the one the history holds"
            )));
        }
        current = older;
    }
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2Confirmed {


    pub level: &'static str,
    pub hash: [u8; 32],

    pub anchor_block: u64,
    pub game: Address,

    pub l1_block: u64,
}


fn returned_bytes(out: &Bytes) -> Result<Vec<u8>, Failure> {
    let off = u64::from_be_bytes(word(out, 0)?[24..].try_into().unwrap()) as usize;
    let len_word = out
        .get(off..off + 32)
        .ok_or_else(|| rejected("a call returned too little"))?;
    let len = u64::from_be_bytes(len_word[24..].try_into().unwrap()) as usize;
    out.get(off + 32..off + 32 + len)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| rejected("a call returned too little"))
}

pub(crate) fn as_bool(out: &Bytes) -> Result<bool, Failure> {
    match word(out, 0)? {
        w if w[..31].iter().all(|&b| b == 0) && w[31] <= 1 => Ok(w[31] == 1),
        _ => Err(rejected("a call did not return a bool")),
    }
}



pub(crate) fn ethereum_at(
    lc: &LightClient,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<(Execution, call::State), Failure> {
    lc.check_age(now)?;
    let asked = answer
        .get("blockNumber")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| bad("the anchor has no blockNumber"))?;
    if asked != number {
        return Err(rejected(format!(
            "the anchor is for block {asked}, not {number}"
        )));
    }
    let update = answer
        .get("update")
        .ok_or_else(|| bad("the anchor has no update"))?;
    let f: Finality = lc.finality(update, now)?;
    let l1 = f.finalized.execution;
    if now.saturating_sub(l1.timestamp) > MAX_L1_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleHead,
            format!(
                "Ethereum's finalized block that Willow sent is {} minutes old",
                now.saturating_sub(l1.timestamp) / 60
            ),
        ));
    }
    let state = call::prove_state(
        l1.state_root.into(),
        answer
            .get("l1")
            .ok_or_else(|| bad("the anchor has no Ethereum state"))?,
    )?;
    Ok((l1, state))
}


pub fn confirm_op(
    lc: &LightClient,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<L2Confirmed, Failure> {
    let chain = OP_CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id)
        .ok_or_else(|| {
            Failure::new(
                Kind::Internal,
                format!("chain {chain_id} is not an OP-stack chain here"),
            )
        })?;
    confirm_op_on(lc, chain, number, answer, now)
}



pub(crate) fn confirm_op_on(
    lc: &LightClient,
    chain: &OpChain,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<L2Confirmed, Failure> {
    let chain_id = chain.chain_id;
    let (l1, state) = ethereum_at(lc, number, answer, now)?;
    let game: Address = answer
        .get("game")
        .and_then(Value::as_str)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| bad("the anchor names no game"))?;
    let at = |to: Address, sig: &str, args: &[[u8; 32]]| {
        call::call(&state, &l1, to, calldata(sig, args))
    };
    let g = address_word(game);
    let level = if as_bool(&at(chain.registry, "isGameClaimValid(address)", &[g])?)? {
        "final"
    } else {


        let proper = as_bool(&at(chain.registry, "isGameProper(address)", &[g])?)?;
        let respected = as_bool(&at(chain.registry, "isGameRespected(address)", &[g])?)?;
        let status = word(&at(game, "status()", &[])?, 0)?[31];
        let creator = Address::from_slice(&word(&at(game, "gameCreator()", &[])?, 0)?[12..]);

        if !proper || !respected || status == CHALLENGER_WINS || !chain.proposers.contains(&creator)
        {
            return Err(Failure::new(
                Kind::Declined,
                format!(
                    "game {game} is neither valid on Ethereum nor a standing claim by {}'s \
                     proposer",
                    chain.name
                ),
            ));
        }
        "committed"
    };
    let root_claim = word(&at(game, "rootClaim()", &[])?, 0)?;
    let sequence = u64::try_from(alloy_primitives::U256::from_be_bytes(word(
        &at(game, "l2SequenceNumber()", &[])?,
        0,
    )?))
    .map_err(|_| rejected("the game's L2 position is over 64 bits"))?;


    let output_root = if chain.super_root {
        let pre = returned_bytes(&at(game, "extraData()", &[])?)?;
        if keccak256(&pre).0 != root_claim
            || pre.first() != Some(&1)
            || pre.len() < 9
            || (pre.len() - 9) % 64 != 0
        {
            return Err(rejected("the game's super root is not its claim"));
        }
        if u64::from_be_bytes(pre[1..9].try_into().unwrap()) != sequence {
            return Err(rejected("the super root's time is not the game's"));
        }
        pre[9..]
            .chunks_exact(64)
            .find(|p| p[..32] == alloy_primitives::U256::from(chain_id).to_be_bytes::<32>())
            .map(|p| p[32..].try_into().unwrap())
            .ok_or_else(|| {
                rejected(format!(
                    "the super root has no output root for {}",
                    chain.name
                ))
            })?
    } else {
        root_claim
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
    let withdrawals = top
        .withdrawals_root
        .ok_or_else(|| rejected("the claimed block's header has no message-passer root"))?;
    let mut pre = [0u8; 128];
    pre[32..64].copy_from_slice(&top.state_root);
    pre[64..96].copy_from_slice(&withdrawals);
    pre[96..].copy_from_slice(&top.hash);
    if keccak256(pre).0 != output_root {
        return Err(rejected(
            "the claimed block's header is not the game's output root",
        ));
    }
    if (chain.super_root && top.timestamp != sequence)
        || (!chain.super_root && top.number != sequence)
    {
        return Err(rejected(
            "the claimed block is not at the game's L2 position",
        ));
    }
    if number > top.number {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is after the block game {game} claims"),
        ));
    }

    let hash = walk_back(
        &top,
        number,
        headers,
        answer,
        ETHEREUM_RING,
        history::ethereum_code_hash(),
    )?;
    Ok(L2Confirmed {
        level,
        hash,
        anchor_block: top.number,
        game,
        l1_block: l1.number,
    })
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Settles {


    AfterPeriod,


    AtOnce,
}



pub struct OracleChain {
    pub chain_id: u64,
    pub name: &'static str,

    pub l1: u64,
    pub oracle: Address,
    pub settles: Settles,

    pub portal: Option<Address>,

    pub proposers: &'static [Address],

    pub history: bool,
}






pub const OPBNB: OracleChain = OracleChain {
    chain_id: 204,
    name: "opBNB",
    l1: crate::bnb::CHAIN_ID,
    oracle: address!("153CAB79f4767E2ff862C94aa49573294B13D169"),
    settles: Settles::AfterPeriod,
    portal: Some(address!("1876EA7702C0ad0C6A2ae6036DE7733edfBca519")),
    proposers: &[address!("c235c904ad9efcabff4628e3279994a4c0a9d591")],
    history: false,
};





pub const ORACLE_CHAINS: [OracleChain; 3] = [
    OracleChain {
        chain_id: 5000,
        name: "Mantle",
        l1: 1,
        oracle: address!("31d543e7BE1dA6eFDc2206Ef7822879045B9f481"),
        settles: Settles::AfterPeriod,
        portal: None,
        proposers: &[],
        history: true,
    },
    OracleChain {
        chain_id: 747474,
        name: "Katana",
        l1: 1,
        oracle: address!("100d3ca4f97776A40A7D93dB4AbF0FEA34230666"),
        settles: Settles::AtOnce,
        portal: None,
        proposers: &[],
        history: true,
    },
    OPBNB,
];


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleConfirmed {


    pub level: &'static str,
    pub hash: [u8; 32],
    pub anchor_block: u64,
    pub output_index: u64,
    pub l1_block: u64,
}

fn word_u64(w: [u8; 32], what: &str) -> Result<u64, Failure> {
    u64::try_from(alloy_primitives::U256::from_be_bytes(w))
        .map_err(|_| rejected(format!("{what} is over 64 bits")))
}


pub fn confirm_oracle(
    lc: &LightClient,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<OracleConfirmed, Failure> {
    let chain = ORACLE_CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id && c.l1 == 1)
        .ok_or_else(|| {
            Failure::new(
                Kind::Internal,
                format!("chain {chain_id} has no output oracle on Ethereum here"),
            )
        })?;
    let (l1, state) = ethereum_at(lc, number, answer, now)?;
    confirm_oracle_at(&l1, &state, chain, number, answer)
}




pub fn confirm_oracle_on_bnb(
    bnb: &crate::bnb::Bnb,
    chain_id: u64,
    number: u64,
    answer: &Value,
    now: u64,
) -> Result<OracleConfirmed, Failure> {
    let chain = ORACLE_CHAINS
        .iter()
        .find(|c| c.chain_id == chain_id && c.l1 == crate::bnb::CHAIN_ID)
        .ok_or_else(|| {
            Failure::new(
                Kind::Internal,
                format!("chain {chain_id} has no output oracle on BNB here"),
            )
        })?;
    crate::asked_block(number, answer)?;
    let a = answer
        .get("bnb")
        .ok_or_else(|| bad("the anchor has no BNB block"))?;
    let b = a
        .get("blockNumber")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .and_then(|h| u64::from_str_radix(h, 16).ok())
        .ok_or_else(|| bad("the BNB anchor has no blockNumber"))?;
    let (c, h) = bnb.confirm_header(b, a, now)?;

    if c.level != "final" {
        return Err(Failure::new(
            Kind::Declined,
            format!("BNB block {b} is not final"),
        ));
    }
    if now.saturating_sub(h.time) > MAX_L1_AGE_SECS {
        return Err(Failure::new(
            Kind::StaleHead,
            format!(
                "BNB's finalized block that Willow sent is {} minutes old",
                now.saturating_sub(h.time) / 60
            ),
        ));
    }
    let l1 = crate::bnb::execution(&h);
    let state = call::prove_state(
        l1.state_root.into(),
        answer
            .get("l1")
            .ok_or_else(|| bad("the anchor has no BNB state"))?,
    )?;
    confirm_oracle_at(&l1, &state, chain, number, answer)
}



pub fn confirm_oracle_at(
    l1: &Execution,
    state: &call::State,
    chain: &OracleChain,
    number: u64,
    answer: &Value,
) -> Result<OracleConfirmed, Failure> {
    let index = answer
        .get("outputIndex")
        .and_then(Value::as_u64)
        .ok_or_else(|| bad("the anchor names no output"))?;

    let ask = |to: Address, sig: &str, args: &[[u8; 32]]| {
        call::call_on(state, l1, chain.l1, to, calldata(sig, args))
    };
    let at = |sig: &str, args: &[[u8; 32]]| ask(chain.oracle, sig, args);
    if let Some(portal) = chain.portal {
        let paid_on = Address::from_slice(&word(&ask(portal, "L2_ORACLE()", &[])?, 0)?[12..]);
        if paid_on != chain.oracle {
            return Err(Failure::new(
                Kind::Declined,
                format!(
                    "{}'s portal no longer pays withdrawals on its output oracle",
                    chain.name
                ),
            ));
        }
    }

    let out = at(
        "getL2Output(uint256)",
        &[alloy_primitives::U256::from(index).to_be_bytes::<32>()],
    )?;
    let root = word(&out, 0)?;
    let posted = word_u64(word(&out, 1)?, "the output's time")?;
    let block = word_u64(word(&out, 2)?, "the output's block")?;
    let level = match chain.settles {
        Settles::AtOnce => "final",
        Settles::AfterPeriod => {
            let period = word_u64(
                word(&at("FINALIZATION_PERIOD_SECONDS()", &[])?, 0)?,
                "the finalization period",
            )?;

            if l1.timestamp > posted.saturating_add(period) {
                "final"
            } else {
                if !chain.proposers.is_empty() {
                    let by = Address::from_slice(&word(&at("PROPOSER()", &[])?, 0)?[12..]);
                    if !chain.proposers.contains(&by) {
                        return Err(Failure::new(
                            Kind::Declined,
                            format!(
                                "{}'s outputs are posted by {by}, not the proposer this page pins",
                                chain.name
                            ),
                        ));
                    }
                }
                "committed"
            }
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


    let withdrawals = match answer.get("messagePasserRoot") {
        Some(v) => bytes_of(
            v.as_str()
                .ok_or_else(|| bad("the message-passer root is not hex"))?,
            "the message-passer root",
        )?
        .try_into()
        .map_err(|_| bad("the message-passer root is not 32 bytes"))?,
        None => top
            .withdrawals_root
            .ok_or_else(|| rejected("the output's block header has no message-passer root"))?,
    };
    let mut pre = [0u8; 128];
    pre[32..64].copy_from_slice(&top.state_root);
    pre[64..96].copy_from_slice(&withdrawals);
    pre[96..].copy_from_slice(&top.hash);
    if keccak256(pre).0 != root || top.number != block {
        return Err(rejected(format!(
            "the header is not {}'s output {index}",
            chain.name
        )));
    }
    if number > top.number {
        return Err(Failure::new(
            Kind::NotCovered,
            format!("block {number} is after output {index}'s block"),
        ));
    }
    let hash = if chain.history {
        walk_back(
            &top,
            number,
            headers,
            answer,
            ETHEREUM_RING,
            history::ethereum_code_hash(),
        )?
    } else {
        let low = descend(&top, number, answer)?;
        if low.number != number {
            return Err(Failure::new(
                Kind::NotCovered,
                format!(
                    "{} keeps no block-hash history: the headers sent stop at block {}, above \
                     block {number}",
                    chain.name, low.number
                ),
            ));
        }
        low.hash
    };
    Ok(OracleConfirmed {
        level,
        hash,
        anchor_block: top.number,
        output_index: index,
        l1_block: l1.number,
    })
}

#[allow(unused_imports)]
pub(crate) use crate::nitro::pending_chain_stands;
pub use crate::nitro::{
    confirm_nitro, nitro_code_hash, NitroChain, NitroConfirmed, NITRO_CHAINS, NITRO_HISTORY_CODE,
    NITRO_RING,
};
