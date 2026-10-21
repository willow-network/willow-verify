//! The PINNED mainnet fork schedule, and the per-fork `ExecutionPayload` shape.
//!
//! Both halves of the anchor derive their tree shapes from *slots*, in-guest:
//!
//! - `fork_at(anchor_slot)` fixes the `BeaconState` container depth, and so the
//!   generalized index of `block_roots` / `historical_summaries`;
//! - `fork_at(target_slot)` fixes the `ExecutionPayload` field count, and so the
//!   depth its leaves merkleize to and the leaf list length the witness must supply.
//!
//! A prover who lies about either slot therefore changes the shape the guest
//! checks, and the branch stops reproducing the root. Nothing about the layout is
//! taken from the witness.
//!
//! `BeaconState` field counts are NOT declared here — they live in
//! `beacon_cl_core::fork`, behind an E0080 compile-time guard that reads the real
//! `crates/network/src/beacon/{electra_final,fulu_final}.rs` declarations.

use crate::error::Error;
pub use beacon_cl_core::fork::ForkVersion;

/// First slot of Capella (2023-04-12, epoch 194,048). `historical_summaries` is
/// introduced empty here, and `historical_roots` stops growing — so this is the
/// earliest target the historical path can reach.
pub const CAPELLA_SLOT: u64 = 6_209_536;
/// First slot of Deneb (epoch 269,568). `ExecutionPayload` grows to 17 fields.
pub const DENEB_SLOT: u64 = 8_626_176;
/// First slot of Electra (epoch 364,032). `BeaconState` crosses 32 fields, so every
/// top-level state gindex doubles.
pub const ELECTRA_SLOT: u64 = 11_649_024;
/// First slot of Fulu (epoch 411,392).
pub const FULU_SLOT: u64 = 13_164_544;

/// `SLOTS_PER_HISTORICAL_ROOT` — the length of `state.block_roots`, and the period
/// each `HistoricalSummary` covers.
pub const SLOTS_PER_HISTORICAL_ROOT: u64 = 8192;

/// Merkle depth of the `block_roots` vector: `log2(8192)`. A `Vector` has no
/// length mixin, so this is the whole subtree.
pub const BLOCK_ROOTS_TREE_DEPTH: usize = SLOTS_PER_HISTORICAL_ROOT.trailing_zeros() as usize;

/// `HISTORICAL_ROOTS_LIMIT = 2^24`, the `List` bound on `historical_summaries`.
/// Each element hashes to one chunk, so the data tree is 24 deep (plus the mixin).
pub const HISTORICAL_ROOTS_LIMIT_DEPTH: usize = 24;

// The historical chunk index is `(target_slot - CAPELLA_SLOT) / 8192` and the
// in-period index is `target_slot % 8192`. Those two agree only because Capella
// began exactly on a period boundary — 6,209,536 = 758 x 8,192. If a future
// re-pin breaks that, the chunk arithmetic in `gindex.rs` silently indexes the
// wrong period, so make it a compile error rather than a comment.
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

/// `ExecutionPayload` fields at Capella: `parent_hash`, `fee_recipient`,
/// `state_root`, `receipts_root`, `logs_bloom`, `prev_randao`, `block_number`,
/// `gas_limit`, `gas_used`, `timestamp`, `extra_data`, `base_fee_per_gas`,
/// `block_hash`, `transactions`, `withdrawals`.
pub const CAPELLA_PAYLOAD_FIELDS: usize = 15;

/// `ExecutionPayload` fields at Deneb and later: Capella's 15 plus EIP-4844's
/// `blob_gas_used` and `excess_blob_gas`. Unchanged at Electra and Fulu — neither
/// touches the payload container.
pub const DENEB_PAYLOAD_FIELDS: usize = 17;

/// 0-based `ExecutionPayload` field position of `block_number`. Same at every fork
/// Capella..Fulu (the Deneb additions append).
pub const PAYLOAD_BLOCK_NUMBER_POSITION: usize = 6;

/// 0-based `ExecutionPayload` field position of `block_hash`. By consensus rules
/// this equals the keccak of the EL header, which is what `WindowVerifier` holds
/// as `anchorBlockHash`.
pub const PAYLOAD_BLOCK_HASH_POSITION: usize = 12;

const _: () = assert!(
    PAYLOAD_BLOCK_NUMBER_POSITION < CAPELLA_PAYLOAD_FIELDS
        && PAYLOAD_BLOCK_HASH_POSITION < CAPELLA_PAYLOAD_FIELDS,
    "block_number/block_hash must exist in the SMALLEST payload shape too, \
     otherwise the Capella path indexes past its leaf list"
);

/// The mainnet fork active at `slot`. Pre-Capella is an error, not a fork: the
/// historical path's `historical_summaries` does not exist before it.
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

/// Number of `ExecutionPayload` fields at `fork`. Exhaustive over [`ForkVersion`]
/// with no wildcard arm, so the next fork variant is a compile error here — the
/// same discipline `beacon_cl_core::fork` uses for the state tables.
pub const fn payload_field_count(fork: ForkVersion) -> usize {
    match fork {
        ForkVersion::Capella => CAPELLA_PAYLOAD_FIELDS,
        ForkVersion::Deneb => DENEB_PAYLOAD_FIELDS,
        ForkVersion::Electra => DENEB_PAYLOAD_FIELDS,
        ForkVersion::Fulu => DENEB_PAYLOAD_FIELDS,
    }
}

/// Depth of the `ExecutionPayload` container tree at `fork`:
/// `log2(next_power_of_two(fields))` — 4 at Capella (15 -> 16), 5 at Deneb+
/// (17 -> 32). Derived, never a literal: crossing 16 fields doubled it once and
/// will again at 32.
pub const fn payload_tree_depth(fork: ForkVersion) -> usize {
    payload_field_count(fork)
        .next_power_of_two()
        .trailing_zeros() as usize
}

/// Short name, for `Display` and test messages.
pub const fn fork_name(fork: ForkVersion) -> &'static str {
    match fork {
        ForkVersion::Capella => "capella",
        ForkVersion::Deneb => "deneb",
        ForkVersion::Electra => "electra",
        ForkVersion::Fulu => "fulu",
    }
}
