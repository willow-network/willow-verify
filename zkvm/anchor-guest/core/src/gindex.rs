//! Fork-dependent generalized indices into the ATTESTED `BeaconState` — derived
//! from the fork, never witnessed (the aggregate's anchor discipline). Values and
//! provenance match `crates/network/src/beacon/light_client.rs`, which carries the
//! real-mainnet KATs for both.

use beacon_cl_core::fork::ForkVersion;

/// `finalized_checkpoint.root` (`finalized_checkpoint` is field 20, a
/// `Checkpoint`; `.root` is its field 1): 105 through Deneb (depth 6), 169 from
/// Electra (depth 7).
pub const fn finalized_root_gindex(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella | ForkVersion::Deneb => 105,
        ForkVersion::Electra | ForkVersion::Fulu => 169,
    }
}

/// `next_sync_committee` (field 23): 55 through Deneb, 87 from Electra.
pub const fn next_sync_committee_gindex(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella | ForkVersion::Deneb => 55,
        ForkVersion::Electra | ForkVersion::Fulu => 87,
    }
}
