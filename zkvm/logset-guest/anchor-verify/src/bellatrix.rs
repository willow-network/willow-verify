











use beacon_cl_core::fork::{branch_root, depth_for_gindex, state_tree_depth, verify_branch};
use beacon_cl_core::ssz::{merkleize, Bytes32};
use window_anchor_core::blob::{BLOB_MAGIC, OFF_PATH_KIND};
use window_anchor_core::{
    fork_at, Statement, ANCHOR_KIND, BLOCK_STATE_ROOT_GINDEX, CAPELLA_SLOT,
    PAYLOAD_TO_BLOCK_ROOT_GINDEX, SLOTS_PER_HISTORICAL_ROOT,
};


pub const HISTORICAL_ROOTS_PATH: u8 = 2;


pub const BELLATRIX_SLOT: u64 = 4_636_672;
pub const MERGE_BLOCK: u64 = 15_537_394;

pub const BELLATRIX_PAYLOAD_FIELDS: usize = 14;

const HISTORICAL_ROOTS_POSITION: u64 = 7;
const HISTORICAL_ROOTS_DEPTH: u32 = 24;
const BLOCK_ROOTS_DEPTH: u32 = 13;


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    pub anchor_slot: u64,
    pub anchor_state_root: Bytes32,
    pub branch_state_root: [Bytes32; 3],
    pub target_slot: u64,

    pub siblings: Vec<Bytes32>,
    pub payload_leaves: Vec<Bytes32>,
    pub branch_payload: [Bytes32; 7],
}


#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Malformed(&'static str),

    Slots,
    PayloadLeafCount(usize),
    SiblingCount {
        expected: usize,
        got: usize,
    },
    Branch,
    BlockNumberNotCanonical,

    NotAfterTheMerge(u64),
}


pub fn is_historical_roots(blob: &[u8]) -> bool {
    blob.get(OFF_PATH_KIND) == Some(&HISTORICAL_ROOTS_PATH)
}


























fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8], Error> {
    if b.len() < n {
        return Err(Error::Malformed("the witness is cut short"));
    }
    let (head, rest) = b.split_at(n);
    *b = rest;
    Ok(head)
}

fn node(b: &mut &[u8]) -> Result<Bytes32, Error> {
    Ok(take(b, 32)?.try_into().unwrap())
}

fn u64_of(b: &mut &[u8]) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(take(b, 8)?.try_into().unwrap()))
}

pub fn decode(blob: &[u8]) -> Result<Witness, Error> {
    let mut b = blob;
    if u32::from_le_bytes(take(&mut b, 4)?.try_into().unwrap()) != BLOB_MAGIC {
        return Err(Error::Malformed("not a WAN1 witness"));
    }
    let anchor_slot = u64_of(&mut b)?;
    let anchor_state_root = node(&mut b)?;
    let branch_state_root = [node(&mut b)?, node(&mut b)?, node(&mut b)?];
    let target_slot = u64_of(&mut b)?;
    if take(&mut b, 1)?[0] != HISTORICAL_ROOTS_PATH {
        return Err(Error::Malformed("not the historical-roots path"));
    }
    let n = u16::from_le_bytes(take(&mut b, 2)?.try_into().unwrap()) as usize;
    let siblings = (0..n).map(|_| node(&mut b)).collect::<Result<_, _>>()?;
    let m = take(&mut b, 1)?[0] as usize;
    let payload_leaves = (0..m).map(|_| node(&mut b)).collect::<Result<_, _>>()?;
    let mut branch_payload = [[0u8; 32]; 7];
    for x in branch_payload.iter_mut() {
        *x = node(&mut b)?;
    }
    if !b.is_empty() {
        return Err(Error::Malformed("bytes left over after the witness"));
    }
    Ok(Witness {
        anchor_slot,
        anchor_state_root,
        branch_state_root,
        target_slot,
        siblings,
        payload_leaves,
        branch_payload,
    })
}



pub fn gindex(depth: usize, target_slot: u64) -> u64 {
    let period = target_slot / SLOTS_PER_HISTORICAL_ROOT;
    let mut g = (1u64 << depth) + HISTORICAL_ROOTS_POSITION;
    g *= 2; // the list's data, beside its length
    g = (g << HISTORICAL_ROOTS_DEPTH) | period;
    g *= 2; // HistoricalBatch.block_roots, beside state_roots
    (g << BLOCK_ROOTS_DEPTH) | (target_slot % SLOTS_PER_HISTORICAL_ROOT)
}



pub fn run(blob: &[u8]) -> Result<Statement, Error> {
    let w = decode(blob)?;
    if !(BELLATRIX_SLOT..CAPELLA_SLOT).contains(&w.target_slot) || w.target_slot >= w.anchor_slot {
        return Err(Error::Slots);
    }
    let fork = fork_at(w.anchor_slot).map_err(|_| Error::Slots)?;
    if w.payload_leaves.len() != BELLATRIX_PAYLOAD_FIELDS {
        return Err(Error::PayloadLeafCount(w.payload_leaves.len()));
    }
    let payload_root = merkleize(&w.payload_leaves, 4);
    let target_root = branch_root(
        &payload_root,
        &w.branch_payload,
        PAYLOAD_TO_BLOCK_ROOT_GINDEX,
    )
    .ok_or(Error::Branch)?;
    let depth = state_tree_depth(fork);
    let g = gindex(depth, w.target_slot);
    let expected = depth_for_gindex(g);
    if w.siblings.len() != expected {
        return Err(Error::SiblingCount {
            expected,
            got: w.siblings.len(),
        });
    }
    if !verify_branch(&target_root, &w.siblings, g, &w.anchor_state_root) {
        return Err(Error::Branch);
    }
    let anchor_beacon_root = branch_root(
        &w.anchor_state_root,
        &w.branch_state_root,
        BLOCK_STATE_ROOT_GINDEX,
    )
    .ok_or(Error::Branch)?;
    let number_leaf = w.payload_leaves[6];
    if number_leaf[8..] != [0u8; 24] {
        return Err(Error::BlockNumberNotCanonical);
    }
    let el_block_number = u64::from_le_bytes(number_leaf[..8].try_into().unwrap());
    if el_block_number < MERGE_BLOCK {
        return Err(Error::NotAfterTheMerge(el_block_number));
    }
    Ok(Statement {
        kind: ANCHOR_KIND,
        anchor_beacon_root,
        el_block_number,
        el_block_hash: w.payload_leaves[12],
        anchor_slot: w.anchor_slot,
        target_slot: w.target_slot,
    })
}

















































































































































