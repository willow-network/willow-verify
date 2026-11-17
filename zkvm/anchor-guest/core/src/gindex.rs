




use beacon_cl_core::fork::ForkVersion;




pub const fn finalized_root_gindex(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella | ForkVersion::Deneb => 105,
        ForkVersion::Electra | ForkVersion::Fulu => 169,
    }
}


pub const fn next_sync_committee_gindex(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella | ForkVersion::Deneb => 55,
        ForkVersion::Electra | ForkVersion::Fulu => 87,
    }
}
