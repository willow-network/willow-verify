








































































pub mod blob;
pub mod error;
pub mod fork_schedule;
pub mod gindex;









pub use blob::{decode_blob, Witness, BLOB_MAGIC};
pub use error::Error;
pub use fork_schedule::{
    fork_at, payload_field_count, payload_tree_depth, ForkVersion, CAPELLA_SLOT, DENEB_SLOT,
    ELECTRA_SLOT, FULU_SLOT, SLOTS_PER_HISTORICAL_ROOT,
};
pub use gindex::{
    depth_for_gindex, state_to_block_root_gindex, PathKind, BLOCK_STATE_ROOT_GINDEX,
    PAYLOAD_TO_BLOCK_ROOT_GINDEX,
};

use beacon_cl_core::fork::{branch_root, verify_branch};
use beacon_cl_core::ssz::{merkleize, Bytes32};
use fork_schedule::{PAYLOAD_BLOCK_HASH_POSITION, PAYLOAD_BLOCK_NUMBER_POSITION};





pub const ANCHOR_KIND: u64 = 0x4101;







const _: () = assert!(
    depth_for_gindex(BLOCK_STATE_ROOT_GINDEX) == 3,
    "Witness::branch_state_root is [Bytes32; 3]; gindex 11 no longer implies depth 3"
);
const _: () = assert!(
    depth_for_gindex(PAYLOAD_TO_BLOCK_ROOT_GINDEX) == 7,
    "Witness::branch_payload is [Bytes32; 7]; gindex 201 no longer implies depth 7"
);


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Statement {

    pub kind: u64,

    pub anchor_beacon_root: Bytes32,

    pub el_block_number: u64,

    pub el_block_hash: Bytes32,

    pub anchor_slot: u64,


    pub target_slot: u64,
}


















































pub fn run(blob: &[u8]) -> Result<Statement, Error> {
    let w = decode_blob(blob)?;


    let fork_anchor = fork_at(w.anchor_slot)?;
    let fork_target = fork_at(w.target_slot)?;




    let expected_leaves = payload_field_count(fork_target);
    if w.payload_leaves.len() != expected_leaves {
        return Err(Error::PayloadLeafCountMismatch {
            expected: expected_leaves,
            got: w.payload_leaves.len(),
        });
    }
    let payload_root = merkleize(&w.payload_leaves, payload_tree_depth(fork_target));


    let target_block_root = branch_root(
        &payload_root,
        &w.branch_payload,
        PAYLOAD_TO_BLOCK_ROOT_GINDEX,
    )
    .ok_or(Error::SiblingCountMismatch {
        gindex: PAYLOAD_TO_BLOCK_ROOT_GINDEX,
        expected: depth_for_gindex(PAYLOAD_TO_BLOCK_ROOT_GINDEX),
        got: w.branch_payload.len(),
    })?;



    let gindex = state_to_block_root_gindex(fork_anchor, w.path, w.anchor_slot, w.target_slot)?;
    let expected_siblings = depth_for_gindex(gindex);
    if w.siblings.len() != expected_siblings {
        return Err(Error::SiblingCountMismatch {
            gindex,
            expected: expected_siblings,
            got: w.siblings.len(),
        });
    }
    if !verify_branch(
        &target_block_root,
        &w.siblings,
        gindex,
        &w.anchor_state_root,
    ) {
        return Err(Error::BlockRootBranchInvalid { gindex });
    }



    let anchor_beacon_root = branch_root(
        &w.anchor_state_root,
        &w.branch_state_root,
        BLOCK_STATE_ROOT_GINDEX,
    )
    .ok_or(Error::SiblingCountMismatch {
        gindex: BLOCK_STATE_ROOT_GINDEX,
        expected: depth_for_gindex(BLOCK_STATE_ROOT_GINDEX),
        got: w.branch_state_root.len(),
    })?;


    let number_leaf: Bytes32 = w.payload_leaves[PAYLOAD_BLOCK_NUMBER_POSITION];
    if number_leaf[8..] != [0u8; 24] {
        return Err(Error::BlockNumberLeafNotCanonical);
    }
    let mut n = [0u8; 8];
    n.copy_from_slice(&number_leaf[..8]);
    let el_block_number = u64::from_le_bytes(n);
    let el_block_hash = w.payload_leaves[PAYLOAD_BLOCK_HASH_POSITION];

    Ok(Statement {
        kind: ANCHOR_KIND,
        anchor_beacon_root,
        el_block_number,
        el_block_hash,
        anchor_slot: w.anchor_slot,
        target_slot: w.target_slot,
    })
}
