//! Ethereum blocks from the merge to Capella (15,537,394 to 17,034,869) through the beacon
//! state's `historical_roots`, frozen since Capella and in every later state: the target block's
//! Bellatrix execution payload up to its block root, then `HistoricalBatch.block_roots` of its
//! period, the batch root (its sibling the period's `state_roots` root), `historical_roots`, the
//! anchor state's field 7, the anchor state root, and the anchor block root that the light client
//! verified as finalized.
//!
//! The witness is `window_anchor_core`'s WAN1 blob with `path_kind` 2 and 14 payload leaves,
//! verified here rather than in that core, so the SP1 window-anchor guest built from it (whose
//! verifying key is pinned) does not change. Every index is computed from the two slots; the
//! sibling count must be the depth it implies.

use beacon_cl_core::fork::{branch_root, depth_for_gindex, state_tree_depth, verify_branch};
use beacon_cl_core::ssz::{merkleize, Bytes32};
use window_anchor_core::blob::{BLOB_MAGIC, OFF_PATH_KIND};
use window_anchor_core::{
    fork_at, Statement, ANCHOR_KIND, BLOCK_STATE_ROOT_GINDEX, CAPELLA_SLOT,
    PAYLOAD_TO_BLOCK_ROOT_GINDEX, SLOTS_PER_HISTORICAL_ROOT,
};

/// `path_kind` of a WAN1 blob proven through `historical_roots`.
pub const HISTORICAL_ROOTS_PATH: u8 = 2;
/// Bellatrix's first slot, and the first proof-of-stake block (slot 4,700,013): a Bellatrix block
/// before it carries an empty payload, block 0.
pub const BELLATRIX_SLOT: u64 = 4_636_672;
pub const MERGE_BLOCK: u64 = 15_537_394;
/// A Bellatrix `ExecutionPayload`'s fields: Capella's less `withdrawals`.
pub const BELLATRIX_PAYLOAD_FIELDS: usize = 14;
/// `BeaconState.historical_roots`, `List[Root, 2^24]`.
const HISTORICAL_ROOTS_POSITION: u64 = 7;
const HISTORICAL_ROOTS_DEPTH: u32 = 24;
const BLOCK_ROOTS_DEPTH: u32 = 13;

/// A WAN1 witness of the historical-roots path.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness {
    pub anchor_slot: u64,
    pub anchor_state_root: Bytes32,
    pub branch_state_root: [Bytes32; 3],
    pub target_slot: u64,
    /// Target block root up to the anchor state root: 13 + 1 + 24 + 1 + the state's depth.
    pub siblings: Vec<Bytes32>,
    pub payload_leaves: Vec<Bytes32>,
    pub branch_payload: [Bytes32; 7],
}

/// Why a historical-roots witness does not verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    Malformed(&'static str),
    /// The target is not a Bellatrix slot with a payload, or the anchor not Capella or later.
    Slots,
    PayloadLeafCount(usize),
    SiblingCount {
        expected: usize,
        got: usize,
    },
    Branch,
    BlockNumberNotCanonical,
    /// A payload before the merge (block number 0, a Bellatrix block's empty payload).
    NotAfterTheMerge(u64),
}

/// Whether a WAN1 blob is of the historical-roots path.
pub fn is_historical_roots(blob: &[u8]) -> bool {
    blob.get(OFF_PATH_KIND) == Some(&HISTORICAL_ROOTS_PATH)
}

/// The WAN1 bytes of `w` (`path_kind` 2).
pub fn encode(w: &Witness) -> Vec<u8> {
    let mut out = Vec::with_capacity(376 + 32 * (w.siblings.len() + w.payload_leaves.len()));
    out.extend_from_slice(&BLOB_MAGIC.to_le_bytes());
    out.extend_from_slice(&w.anchor_slot.to_le_bytes());
    out.extend_from_slice(&w.anchor_state_root);
    for n in &w.branch_state_root {
        out.extend_from_slice(n);
    }
    out.extend_from_slice(&w.target_slot.to_le_bytes());
    out.push(HISTORICAL_ROOTS_PATH);
    out.extend_from_slice(&(w.siblings.len() as u16).to_le_bytes());
    for n in &w.siblings {
        out.extend_from_slice(n);
    }
    out.push(w.payload_leaves.len() as u8);
    for n in &w.payload_leaves {
        out.extend_from_slice(n);
    }
    for n in &w.branch_payload {
        out.extend_from_slice(n);
    }
    out
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

/// The generalized index of the block root at `target_slot` under an anchor state of depth
/// `depth`: field 7, its list's data, the period's batch, the batch's `block_roots`, the slot.
pub fn gindex(depth: usize, target_slot: u64) -> u64 {
    let period = target_slot / SLOTS_PER_HISTORICAL_ROOT;
    let mut g = (1u64 << depth) + HISTORICAL_ROOTS_POSITION;
    g *= 2; // the list's data, beside its length
    g = (g << HISTORICAL_ROOTS_DEPTH) | period;
    g *= 2; // HistoricalBatch.block_roots, beside state_roots
    (g << BLOCK_ROOTS_DEPTH) | (target_slot % SLOTS_PER_HISTORICAL_ROOT)
}

/// Verifies a historical-roots witness: the statement the beacon path gives, for a block from
/// the merge to Capella.
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

#[cfg(test)]
mod tests {
    use super::*;
    use beacon_cl_core::ssz::{hash_pair, mix_in_length, zero_hashes};

    fn n(seed: u64) -> Bytes32 {
        let mut h = [0u8; 32];
        h[..8].copy_from_slice(&seed.to_le_bytes());
        hash_pair(&h, &[7; 32])
    }

    fn leaf(v: u64) -> Bytes32 {
        let mut l = [0u8; 32];
        l[..8].copy_from_slice(&v.to_le_bytes());
        l
    }

    fn branch(leaves: &[Bytes32], depth: usize, index: usize) -> Vec<Bytes32> {
        let z = zero_hashes(depth);
        let (mut layer, mut i, mut out) = (leaves.to_vec(), index, Vec::new());
        for zl in z.iter().take(depth) {
            out.push(*layer.get(i ^ 1).unwrap_or(zl));
            layer = layer
                .chunks(2)
                .map(|p| hash_pair(&p[0], p.get(1).unwrap_or(zl)))
                .collect();
            i /= 2;
        }
        out
    }

    /// A Bellatrix block at `slot` (block `number`) under a Fulu anchor state whose
    /// `historical_roots` hold its period: every root consistent, built from scratch.
    pub(crate) fn scene(slot: u64, number: u64) -> Witness {
        let mut leaves: Vec<Bytes32> = (0..14).map(|i| n(100 + i)).collect();
        leaves[6] = leaf(number);
        leaves[12] = n(12_345);
        let branch_payload: [Bytes32; 7] = core::array::from_fn(|i| n(200 + i as u64));
        let target_root = branch_root(
            &merkleize(&leaves, 4),
            &branch_payload,
            PAYLOAD_TO_BLOCK_ROOT_GINDEX,
        )
        .unwrap();
        let period = slot / 8192;
        let mut block_roots: Vec<Bytes32> = (0..8192).map(|i| n(10_000 + i)).collect();
        block_roots[(slot % 8192) as usize] = target_root;
        let state_roots_root = n(5);
        let mut hr: Vec<Bytes32> = (0..758).map(|i| n(30_000 + i)).collect();
        hr[period as usize] = hash_pair(&merkleize(&block_roots, 13), &state_roots_root);
        let mut fields: Vec<Bytes32> = (0..38).map(|i| n(60_000 + i)).collect();
        fields[7] = mix_in_length(&merkleize(&hr, 24), 758);
        let state_root = merkleize(&fields, 6);
        let mut siblings = branch(&block_roots, 13, (slot % 8192) as usize);
        siblings.push(state_roots_root);
        siblings.extend(branch(&hr, 24, period as usize));
        siblings.push(leaf(758));
        siblings.extend(branch(&fields, 6, 7));
        let header = [leaf(15_000_000), leaf(1), n(1), state_root, n(2)];
        Witness {
            anchor_slot: 15_000_000,
            anchor_state_root: state_root,
            branch_state_root: branch(&header, 3, 3).try_into().unwrap(),
            target_slot: slot,
            siblings,
            payload_leaves: leaves,
            branch_payload,
        }
    }

    #[test]
    fn a_bellatrix_block_verifies_through_historical_roots() {
        let w = scene(4_913_000, 15_748_796);
        let s = run(&encode(&w)).unwrap();
        assert_eq!(
            (s.el_block_number, s.el_block_hash),
            (15_748_796, n(12_345))
        );
        assert_eq!(s.target_slot, 4_913_000);
        assert_eq!(w.siblings.len(), 13 + 1 + 24 + 1 + 6);
        assert_eq!(decode(&encode(&w)).unwrap(), w);
        // The first block after the merge, and the last before Capella.
        assert!(run(&encode(&scene(4_700_013, MERGE_BLOCK))).is_ok());
        assert!(run(&encode(&scene(CAPELLA_SLOT - 1, 17_034_869))).is_ok());
    }

    #[test]
    fn every_tampered_historical_roots_witness_is_refused() {
        let w = scene(4_913_000, 15_748_796);
        let fails = |w: &Witness| run(&encode(w)).unwrap_err();
        // The state-roots sibling, a block_roots sibling, a historical_roots sibling, a field.
        for i in [0, 12, 13, 14, 37, 38, 44] {
            let mut t = w.clone();
            t.siblings[i][3] ^= 1;
            assert_eq!(fails(&t), Error::Branch, "sibling {i}");
        }
        // The slot claims another period (the historical_roots index off by one) or index.
        for slot in [4_913_000 + 8192, 4_913_000 - 8192, 4_913_001] {
            let mut t = w.clone();
            t.target_slot = slot;
            assert_eq!(fails(&t), Error::Branch, "slot {slot}");
        }
        // Capella's or Deneb's payload shape at a Bellatrix slot.
        let mut t = w.clone();
        t.payload_leaves.push(n(9));
        assert_eq!(fails(&t), Error::PayloadLeafCount(15));
        // A sibling short, or one more.
        let mut t = w.clone();
        t.siblings.pop();
        assert!(matches!(fails(&t), Error::SiblingCount { .. }));
        let mut t = w.clone();
        t.siblings.push(n(1));
        assert!(matches!(fails(&t), Error::SiblingCount { .. }));
        // A Capella slot by this path; a target at or after the anchor; an anchor before Capella.
        let mut t = w.clone();
        t.target_slot = CAPELLA_SLOT;
        assert_eq!(fails(&t), Error::Slots);
        let mut t = w.clone();
        t.anchor_slot = 4_913_000;
        assert_eq!(fails(&t), Error::Slots);
        let mut t = w.clone();
        t.anchor_slot = CAPELLA_SLOT - 1;
        assert_eq!(fails(&t), Error::Slots);
        // A Bellatrix slot before the merge, and the empty payload such a block carries.
        let mut t = w.clone();
        t.target_slot = BELLATRIX_SLOT - 1;
        assert_eq!(fails(&t), Error::Slots);
        assert_eq!(
            run(&encode(&scene(4_700_012, 0))).unwrap_err(),
            Error::NotAfterTheMerge(0)
        );
        // Bytes left over, or cut short.
        let mut b = encode(&w);
        b.push(0);
        assert!(matches!(run(&b).unwrap_err(), Error::Malformed(_)));
        b.truncate(b.len() - 33);
        assert!(matches!(run(&b).unwrap_err(), Error::Malformed(_)));
        // The same witness as path 1 is window_anchor_core's, which refuses a Bellatrix slot.
        let mut b = encode(&w);
        b[OFF_PATH_KIND] = 1;
        assert!(window_anchor_core::run(&b).is_err());
        assert!(!is_historical_roots(&b));
    }
}
