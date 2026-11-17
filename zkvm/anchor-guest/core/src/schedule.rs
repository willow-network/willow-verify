




use beacon_cl_core::fork::ForkVersion;


pub const CAPELLA_SLOT: u64 = 6_209_536;
pub const DENEB_SLOT: u64 = 8_626_176;
pub const ELECTRA_SLOT: u64 = 11_649_024;
pub const FULU_SLOT: u64 = 13_164_544;



pub const SLOTS_PER_SYNC_COMMITTEE_PERIOD: u64 = 32 * 256;



pub const fn fork_at_slot(slot: u64) -> Option<ForkVersion> {
    if slot >= FULU_SLOT {
        Some(ForkVersion::Fulu)
    } else if slot >= ELECTRA_SLOT {
        Some(ForkVersion::Electra)
    } else if slot >= DENEB_SLOT {
        Some(ForkVersion::Deneb)
    } else if slot >= CAPELLA_SLOT {
        Some(ForkVersion::Capella)
    } else {
        None
    }
}


pub const fn fork_version_bytes(fork: ForkVersion) -> [u8; 4] {
    match fork {
        ForkVersion::Capella => [0x03, 0, 0, 0],
        ForkVersion::Deneb => [0x04, 0, 0, 0],
        ForkVersion::Electra => [0x05, 0, 0, 0],
        ForkVersion::Fulu => [0x06, 0, 0, 0],
    }
}










