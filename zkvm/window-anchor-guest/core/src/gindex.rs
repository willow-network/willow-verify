







































use crate::error::Error;
use crate::fork_schedule::{
    ForkVersion, BLOCK_ROOTS_TREE_DEPTH, CAPELLA_SLOT, HISTORICAL_ROOTS_LIMIT_DEPTH,
    SLOTS_PER_HISTORICAL_ROOT,
};
use beacon_cl_core::fork::state_tree_depth;





pub const BLOCK_ROOTS_POSITION: usize = 5;




pub const HISTORICAL_SUMMARIES_POSITION: usize = 27;



pub const BLOCK_SUMMARY_ROOT_POSITION: u64 = 0;







pub const BLOCK_STATE_ROOT_GINDEX: u64 = 11;











pub const PAYLOAD_TO_BLOCK_ROOT_GINDEX: u64 = 201;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathKind {

    Recent,

    Historical,
}

impl PathKind {









    pub const fn from_u8(b: u8) -> Result<Self, Error> {
        match b {
            0 => Ok(PathKind::Recent),
            1 => Ok(PathKind::Historical),
            got => Err(Error::BadPathKind { got }),
        }
    }
}




pub use beacon_cl_core::fork::depth_for_gindex;







pub fn state_to_block_root_gindex(
    fork: ForkVersion,
    path: PathKind,
    anchor_slot: u64,
    target_slot: u64,
) -> Result<u64, Error> {
    let d = state_tree_depth(fork);

    let slots = 1u64 << d;

    match path {
        PathKind::Recent => {
            if target_slot >= anchor_slot {
                return Err(Error::TargetNotBeforeAnchor {
                    anchor_slot,
                    target_slot,
                });
            }

            if anchor_slot - target_slot > SLOTS_PER_HISTORICAL_ROOT {
                return Err(Error::TargetOutsideRecentRange {
                    anchor_slot,
                    target_slot,
                });
            }
            let g = slots + BLOCK_ROOTS_POSITION as u64;
            Ok((g << BLOCK_ROOTS_TREE_DEPTH) | (target_slot % SLOTS_PER_HISTORICAL_ROOT))
        }
        PathKind::Historical => {


            if target_slot < CAPELLA_SLOT {
                return Err(Error::PreCapellaSlot { slot: target_slot });
            }
            let chunk = (target_slot - CAPELLA_SLOT) / SLOTS_PER_HISTORICAL_ROOT;
            let idx = target_slot % SLOTS_PER_HISTORICAL_ROOT;
            if chunk >= 1u64 << HISTORICAL_ROOTS_LIMIT_DEPTH {
                return Err(Error::HistoricalChunkOutOfRange { chunk });
            }


            let mut g = slots + HISTORICAL_SUMMARIES_POSITION as u64;

            g *= 2;

            g = (g << HISTORICAL_ROOTS_LIMIT_DEPTH) | chunk;

            g = g * 2 + BLOCK_SUMMARY_ROOT_POSITION;

            Ok((g << BLOCK_ROOTS_TREE_DEPTH) | idx)
        }
    }
}


















