
















use crate::error::Error;
pub use beacon_cl_core::fork::ForkVersion;




pub const CAPELLA_SLOT: u64 = 6_209_536;

pub const DENEB_SLOT: u64 = 8_626_176;


pub const ELECTRA_SLOT: u64 = 11_649_024;

pub const FULU_SLOT: u64 = 13_164_544;



pub const SLOTS_PER_HISTORICAL_ROOT: u64 = 8192;



pub const BLOCK_ROOTS_TREE_DEPTH: usize = SLOTS_PER_HISTORICAL_ROOT.trailing_zeros() as usize;



pub const HISTORICAL_ROOTS_LIMIT_DEPTH: usize = 24;






const _: () = assert!(
    CAPELLA_SLOT.is_multiple_of(SLOTS_PER_HISTORICAL_ROOT),
    "CAPELLA_SLOT is no longer a multiple of SLOTS_PER_HISTORICAL_ROOT; \
     `chunk = (slot - CAPELLA_SLOT)/8192` and `idx = slot % 8192` no longer \
     describe the same period. Rework state_to_block_root_gindex."
);
const _: () = assert!(
    CAPELLA_SLOT < DENEB_SLOT && DENEB_SLOT < ELECTRA_SLOT && ELECTRA_SLOT < FULU_SLOT,
    "the fork schedule is not monotonic; `fork_at`'s descending ladder is wrong"
);





pub const CAPELLA_PAYLOAD_FIELDS: usize = 15;




pub const DENEB_PAYLOAD_FIELDS: usize = 17;



pub const PAYLOAD_BLOCK_NUMBER_POSITION: usize = 6;




pub const PAYLOAD_BLOCK_HASH_POSITION: usize = 12;

const _: () = assert!(
    PAYLOAD_BLOCK_NUMBER_POSITION < CAPELLA_PAYLOAD_FIELDS
        && PAYLOAD_BLOCK_HASH_POSITION < CAPELLA_PAYLOAD_FIELDS,
    "block_number/block_hash must exist in the SMALLEST payload shape too, \
     otherwise the Capella path indexes past its leaf list"
);



pub fn fork_at(slot: u64) -> Result<ForkVersion, Error> {
    if slot >= FULU_SLOT {
        Ok(ForkVersion::Fulu)
    } else if slot >= ELECTRA_SLOT {
        Ok(ForkVersion::Electra)
    } else if slot >= DENEB_SLOT {
        Ok(ForkVersion::Deneb)
    } else if slot >= CAPELLA_SLOT {
        Ok(ForkVersion::Capella)
    } else {
        Err(Error::PreCapellaSlot { slot })
    }
}




pub const fn payload_field_count(fork: ForkVersion) -> usize {
    match fork {
        ForkVersion::Capella => CAPELLA_PAYLOAD_FIELDS,
        ForkVersion::Deneb => DENEB_PAYLOAD_FIELDS,
        ForkVersion::Electra => DENEB_PAYLOAD_FIELDS,
        ForkVersion::Fulu => DENEB_PAYLOAD_FIELDS,
    }
}





pub const fn payload_tree_depth(fork: ForkVersion) -> usize {
    payload_field_count(fork)
        .next_power_of_two()
        .trailing_zeros() as usize
}










