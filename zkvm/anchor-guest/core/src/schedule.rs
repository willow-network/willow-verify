//! The mainnet fork schedule and signing-domain versions, DERIVED from slots in
//! circuit — never witnessed. Port of `crates/network/src/beacon/ssz_proof.rs`'s
//! `ForkSchedule::MAINNET` / `fork_version_at_slot` (returning `Err` where the
//! native code panics) and `light_client.rs`'s `fork_version_bytes`.

use beacon_cl_core::fork::ForkVersion;

/// First slot of each mainnet fork (`epoch * 32`). Update on each hard fork.
pub const CAPELLA_SLOT: u64 = 6_209_536;
pub const DENEB_SLOT: u64 = 8_626_176;
pub const ELECTRA_SLOT: u64 = 11_649_024;
pub const FULU_SLOT: u64 = 13_164_544;

/// Slots per sync-committee period: `SLOTS_PER_EPOCH` (32) ×
/// `EPOCHS_PER_SYNC_COMMITTEE_PERIOD` (256); `period(slot) = slot / 8192`.
pub const SLOTS_PER_SYNC_COMMITTEE_PERIOD: u64 = 32 * 256;

/// The fork active at `slot`. Pre-Capella history is out of scope — the caller
/// refuses the blob rather than silently using Capella indices for it.
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

/// Mainnet 4-byte fork version for the sync-committee signing domain.
pub const fn fork_version_bytes(fork: ForkVersion) -> [u8; 4] {
    match fork {
        ForkVersion::Capella => [0x03, 0, 0, 0],
        ForkVersion::Deneb => [0x04, 0, 0, 0],
        ForkVersion::Electra => [0x05, 0, 0, 0],
        ForkVersion::Fulu => [0x06, 0, 0, 0],
    }
}

/// The wire/statement encoding of a fork — the aggregate's numbering.
pub const fn fork_to_u64(fork: ForkVersion) -> u64 {
    match fork {
        ForkVersion::Capella => 0,
        ForkVersion::Deneb => 1,
        ForkVersion::Electra => 2,
        ForkVersion::Fulu => 3,
    }
}
