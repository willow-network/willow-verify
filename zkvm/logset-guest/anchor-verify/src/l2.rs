//! L2 blocks against what their chain commits on Ethereum (`TRUSTLESS-ANCHOR.md`): an OP-stack
//! chain's dispute game, read with Ethereum's contract code over state the light client verified
//! (`call.rs`), its output root, the anchor block's header, and the L2's own block-hash history
//! back to the block.

use crate::call::{self, address_word, calldata, word};
use crate::history::{self, ETHEREUM_RING};
use crate::lc::{bytes_of, Execution, Finality, LightClient};
use crate::{Failure, Kind};
use alloy_primitives::{address, keccak256, Address, Bytes};
use serde_json::Value;

/// An OP-stack chain: its AnchorStateRegistry on Ethereum, the address that posts its games, and
/// whether its games claim super roots (OP Mainnet, Unichain, Ink) or output roots (Base).
pub struct OpChain {
    pub chain_id: u64,
    pub name: &'static str,
    pub registry: Address,
    pub proposer: Address,
    pub super_root: bool,
}

/// Read from each chain's contracts on Ethereum on 2026-10-19: the respected game type's games
/// are all posted by these addresses.
pub const OP_CHAINS: [OpChain; 4] = [
    OpChain {
        chain_id: 8453,
        name: "Base",
        registry: address!("909f6cf47ed12f010A796527f562bFc26C7F4E72"),
        proposer: address!("c1366Fabe614d42D367A1ecE61821238A1d31cF5"),
        super_root: false,
    },
    OpChain {
        chain_id: 10,
        name: "OP Mainnet",
        registry: address!("23B2C62946350F4246f9f9D027e071f0264FD113"),
        proposer: address!("473300df21D047806A082244b417f96b32f13A33"),
        super_root: true,
    },
    OpChain {
        chain_id: 130,
        name: "Unichain",
        registry: address!("27Cf508E4E3Aa8d30b3226aC3b5Ea0e8bcaCAFF9"),
        proposer: address!("D5F0E2912C70771C589CD8bB087EDE0Dab4AFA9A"),
        super_root: true,
    },
    OpChain {
        chain_id: 57073,
        name: "Ink",
        registry: address!("Ee018bAf058227872540AC60eFbd38b023d9dAe2"),
        proposer: address!("3832bfbeF03173E4C49a00ec0DD178817A02D177"),
        super_root: true,
    },
];

/// Ethereum state older than this says nothing about a game: a guardian can blacklist one, and a
/// claim can be lost, after it.
pub const MAX_L1_AGE_SECS: u64 = 3600;
/// Hops back through the L2's history one answer may take.
pub(crate) const MAX_HOPS: usize = 16;
/// `GameStatus.CHALLENGER_WINS`.
const CHALLENGER_WINS: u8 = 1;

fn bad(message: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, message)
}

fn rejected(message: impl Into<String>) -> Failure {
    Failure::new(Kind::Rejected, message)
}

/// A block header's fields this needs, from its RLP, and its hash.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2Header {
    pub hash: [u8; 32],
    pub state_root: [u8; 32],
    pub number: u64,
    pub timestamp: u64,
    /// Since Isthmus, the L2ToL1MessagePasser's storage root.
    pub withdrawals_root: Option<[u8; 32]>,
    /// Nitro: the send root.
    pub extra_data: Vec<u8>,
}

/// The RLP items of a list, as byte strings (nested lists are refused: headers have none).
fn rlp_items(mut b: &[u8]) -> Result<Vec<&[u8]>, Failure> {
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

fn uint(item: &[u8], what: &str) -> Result<u64, Failure> {
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
        state_root: b32(f[3], "stateRoot")?,
        number: uint(f[8], "number")?,
        timestamp: uint(f[11], "timestamp")?,
        withdrawals_root: f.get(16).map(|w| b32(w, "withdrawalsRoot")).transpose()?,
        extra_data: f[12].to_vec(),
    })
}

/// Block `number`'s hash, back from `top` (`headers[0]`) through the L2's own block-hash history
/// (`ring` blocks, contract `code`): the anchor's `proofs`, and for each hop of `ring` blocks the
/// header of the block it lands on (`headers[1..]`), checked by the hash the history holds.
pub(crate) fn walk_back(
    top: &L2Header,
    number: u64,
    headers: &[Value],
    answer: &Value,
    ring: u64,
    code: alloy_primitives::B256,
) -> Result<[u8; 32], Failure> {
    let proofs = answer
        .get("proofs")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("the anchor has no L2 history proofs"))?;
    let mut current = top.clone();
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

/// What an L2 anchor showed about block `number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct L2Confirmed {
    /// `final` (the claim is valid on Ethereum) or `committed` (posted by the chain's proposer,
    /// not yet past its challenge period).
    pub level: &'static str,
    pub hash: [u8; 32],
    /// The L2 block the game claims, and the game.
    pub anchor_block: u64,
    pub game: Address,
    /// The Ethereum block whose state was read.
    pub l1_block: u64,
}

/// The `bytes` a call returned.
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

/// The anchor's Ethereum side: that it is for block `number`, the finalized block the light client
/// verified from its update (no older than [`MAX_L1_AGE_SECS`]), and the state proven against it.
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

/// Block `number` of OP-stack chain `chain_id`, from its `willow_getBlockAnchor` result.
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
        // Committed: a proper game of the respected type, posted by the chain's proposer and not
        // lost.
        let proper = as_bool(&at(chain.registry, "isGameProper(address)", &[g])?)?;
        let respected = as_bool(&at(chain.registry, "isGameRespected(address)", &[g])?)?;
        let status = word(&at(game, "status()", &[])?, 0)?[31];
        let creator = Address::from_slice(&word(&at(game, "gameCreator()", &[])?, 0)?[12..]);
        // Not a verdict on the block: a claim this page does not count on, not one shown wrong.
        if !proper || !respected || status == CHALLENGER_WINS || creator != chain.proposer {
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

    // The output root the game claims, alone (Base) or in a super root.
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

    // The claimed block: its header hashes into the output root.
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

/// An Arbitrum Nitro chain: its Rollup (BoLD) on Ethereum.
pub struct NitroChain {
    pub chain_id: u64,
    pub name: &'static str,
    pub rollup: Address,
}

/// Robinhood Chain settles on Ethereum itself: its Rollup, SequencerInbox and Bridge are
/// Ethereum contracts.
pub const NITRO_CHAINS: [NitroChain; 2] = [
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
];

/// Nitro's block-hash history (ArbOS's EIP-2935 contract, at the same address): keyed by L2 block
/// number, a ring of 393,168 blocks.
pub const NITRO_RING: u64 = 393_168;
pub const NITRO_HISTORY_CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe1460605760203603605c575f3563a3b1b31d5f5260205f6004601c60645afa15605c575f51600181038211605c57816205ffd0910311605c576205ffd09006545f5260205ff35b5f5ffd5b5f356205ffd0600163a3b1b31d5f5260205f6004601c60645afa15605c575f5103065500";
/// `AssertionStatus.Pending` and `.Confirmed`; `MachineStatus.FINISHED`.
const PENDING: u8 = 1;
const CONFIRMED: u8 = 2;
const FINISHED: u8 = 1;

/// Pending claims an anchor may walk back through to a confirmed one: Robinhood Chain posts one
/// about every half hour, so its 6.4-day challenge period holds about 300.
const MAX_PENDING: usize = 1024;

/// An `AssertionNode`'s status, from `getAssertion`'s fifth word.
fn status(node: &Bytes) -> Result<u8, Failure> {
    match word(node, 4)? {
        w if w[..31].iter().any(|&b| b != 0) => Err(rejected("an assertion status out of range")),
        w => Ok(w[31]),
    }
}

pub fn nitro_code_hash() -> alloy_primitives::B256 {
    keccak256(hex::decode(NITRO_HISTORY_CODE).expect("constant hex"))
}

/// What a Nitro anchor showed about block `number`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NitroConfirmed {
    /// `final` (the assertion is confirmed) or `committed` (asserted, bonded, not yet confirmed).
    pub level: &'static str,
    pub hash: [u8; 32],
    pub anchor_block: u64,
    pub assertion: [u8; 32],
    pub l1_block: u64,
}

/// Block `number` of Nitro chain `chain_id`, from its `willow_getBlockAnchor` result: the
/// assertion on Ethereum that ends at or after it, and the chain's own history back to it.
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

    // The assertion: its hash from its parent, its end state and the inbox accumulator.
    let a = answer
        .get("assertion")
        .ok_or_else(|| bad("the anchor names no assertion"))?;
    let w32 = |v: &Value, what: &str| -> Result<[u8; 32], Failure> {
        bytes_of(
            v.as_str()
                .ok_or_else(|| bad(format!("{what} is not hex")))?,
            what,
        )?
        .try_into()
        .map_err(|_| bad(format!("{what} is not 32 bytes")))
    };
    let parent = w32(&a["parent"], "the parent assertion")?;
    let acc = w32(&a["inboxAcc"], "the inbox accumulator")?;
    let words = a["state"]
        .as_array()
        .filter(|w| w.len() == 6)
        .ok_or_else(|| bad("the assertion state is not 6 words"))?
        .iter()
        .map(|w| w32(w, "an assertion state word"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut pre = Vec::with_capacity(96);
    pre.extend_from_slice(&parent);
    pre.extend_from_slice(&keccak256(words.concat()).0);
    pre.extend_from_slice(&acc);
    let assertion = keccak256(&pre).0;
    if words[4][..31].iter().any(|&b| b != 0) || words[4][31] != FINISHED {
        return Err(rejected("the assertion did not finish its block"));
    }
    let node = call::call(
        &state,
        &l1,
        chain.rollup,
        calldata("getAssertion(bytes32)", &[assertion]),
    )?;
    let level = match status(&node)? {
        CONFIRMED => "final",
        PENDING => "committed",
        _ => {
            return Err(rejected(format!(
                "{} has no assertion {} on Ethereum",
                chain.name,
                alloy_primitives::B256::from(assertion)
            )))
        }
    };
    // A pending claim counts only while it and every pending claim it builds on, back to a
    // confirmed one, is its parent's only child: a claim on a losing branch stays pending forever.
    if level == "committed" {
        let ancestors = a
            .get("ancestors")
            .and_then(Value::as_array)
            .filter(|v| v.len() <= MAX_PENDING)
            .ok_or_else(|| bad("the anchor has no list of the claims it builds on"))?;
        pending_chain_stands(&state, &l1, chain, number, assertion, parent, ancestors)?;
    }

    // The asserted block: its header hashes to the state's block hash and carries its send root.
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
    if top.hash != words[0] || top.extra_data != words[1] {
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
    let hash = if number == top.number {
        top.hash
    } else {
        let proof = answer
            .get("proofs")
            .and_then(Value::as_array)
            .and_then(|p| p.first())
            .ok_or_else(|| bad("the anchor has no L2 history proof"))?;
        history::block_hash(
            top.state_root,
            top.number,
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

/// That pending assertion `assertion` (parent `parent`) and every pending one it builds on, back
/// to a confirmed one, is its parent's only child, read with the Rollup's own code over `state`:
/// `ancestors` are the pending ones' preimages, parent first. Each link is checked by hash; the
/// Rollup's one-slot getters read the links, `getAssertion` the confirmed end.
pub(crate) fn pending_chain_stands(
    state: &call::State,
    l1: &Execution,
    chain: &NitroChain,
    number: u64,
    assertion: [u8; 32],
    parent: [u8; 32],
    ancestors: &[Value],
) -> Result<(), Failure> {
    let ask = |sig: &str, h: [u8; 32]| call::call(state, l1, chain.rollup, calldata(sig, &[h]));
    let w32 = |v: &Value, what: &str| -> Result<[u8; 32], Failure> {
        bytes_of(
            v.as_str()
                .ok_or_else(|| bad(format!("{what} is not hex")))?,
            what,
        )?
        .try_into()
        .map_err(|_| bad(format!("{what} is not 32 bytes")))
    };
    let (mut x, mut p) = (assertion, parent);
    let mut up = ancestors.iter();
    loop {
        if !as_bool(&ask("isFirstChild(bytes32)", x)?)?
            || word(&ask("getSecondChildCreationBlock(bytes32)", p)?, 0)? != [0u8; 32]
        {
            return Err(Failure::new(
                Kind::Disputed,
                format!(
                    "another claim on Ethereum contests {}'s claim for block {number}",
                    chain.name
                ),
            ));
        }
        if !as_bool(&ask("isPending(bytes32)", p)?)? {
            if status(&ask("getAssertion(bytes32)", p)?)? != CONFIRMED {
                return Err(rejected("a claim builds on one Ethereum does not have"));
            }
            return Ok(());
        }
        let next = up.next().ok_or_else(|| {
            Failure::new(
                Kind::NotCovered,
                format!(
                    "Willow did not send every claim {}'s claim for block {number} builds on",
                    chain.name
                ),
            )
        })?;
        let pp = w32(&next["parent"], "an earlier claim's parent")?;
        let mut pre = Vec::with_capacity(96);
        pre.extend_from_slice(&pp);
        pre.extend_from_slice(&w32(&next["stateHash"], "an earlier claim's state")?);
        pre.extend_from_slice(&w32(&next["inboxAcc"], "an earlier claim's inbox")?);
        if keccak256(&pre).0 != p {
            return Err(rejected(
                "an earlier claim does not hash to its child's parent",
            ));
        }
        (x, p) = (p, pp);
    }
}
