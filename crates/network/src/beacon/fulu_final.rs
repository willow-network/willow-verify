//! Locally-defined Fulu (mainnet, Feb 2026) BeaconState type.
//!
//! Fulu (per the consensus-specs PeerDAS spec) adds **one** field
//! to Electra-final's BeaconState: `proposer_lookahead`, a fixed
//! `Vector[ValidatorIndex, SLOTS_PER_EPOCH * (MIN_SEED_LOOKAHEAD +
//! 1)] = Vector[u64, 64]` introduced by EIP-7917 (Beacon Chain
//! Proposer Lookahead). All other state fields are unchanged from
//! Electra-final.
//!
//! `BeaconBlock` / `BeaconBlockBody` / `ExecutionPayload` are
//! **unchanged** from Electra (verified by dumping a Fulu block
//! via a beacon API — same 13 body fields, same 17 payload fields, same
//! `execution_requests` shape). The state_fetcher reuses the
//! `electra_final::SignedBeaconBlock` type for both Fulu and
//! Electra block deserialization.
//!
//! ## SSZ implications
//!
//! With 38 fields, the BeaconState container Merkleizes at depth
//! ceil_log2(38) = 6 — same depth as Electra's 37 fields. So
//! `historical_summaries` (still at position 27) keeps generalized
//! index 64 + 27 = 91. The proof structure carries through
//! verbatim.
//!
//! Verified the field list against live mainnet JSON via a beacon API's
//! `/eth/v2/debug/beacon/states/13164544` (the Fulu activation
//! slot): 38 top-level keys, with `proposer_lookahead` as the
//! sole addition over Electra-final's 37.

use ethereum_consensus::altair::SyncCommittee;
use ethereum_consensus::capella::HistoricalSummary;
use ethereum_consensus::phase0::{
    BeaconBlockHeader, Checkpoint, Eth1Data, Validator, JUSTIFICATION_BITS_LENGTH,
};
use ethereum_consensus::primitives::{
    Bytes32, Epoch, Gwei, ParticipationFlags, Root, Slot, ValidatorIndex, WithdrawalIndex,
};
use ethereum_consensus::ssz::prelude::*;

use super::electra_final::{PendingConsolidation, PendingDeposit, PendingPartialWithdrawal};

/// Fulu adds a fixed-size `Vector[ValidatorIndex,
/// SLOTS_PER_EPOCH * (MIN_SEED_LOOKAHEAD + 1)]`. Mainnet preset:
/// SLOTS_PER_EPOCH = 32, MIN_SEED_LOOKAHEAD = 1 → 64.
pub const PROPOSER_LOOKAHEAD_LIMIT: usize = 64;

/// Mainnet-final Fulu `BeaconState` (= Electra's 37 fields + a
/// trailing `proposer_lookahead`). Field declaration order matches
/// the consensus-specs container layout exactly — order matters for
/// SSZ Merkleization, even though serde JSON parsing is name-based.
#[derive(
    Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct BeaconState<
    const SLOTS_PER_HISTORICAL_ROOT: usize,
    const HISTORICAL_ROOTS_LIMIT: usize,
    const ETH1_DATA_VOTES_BOUND: usize,
    const VALIDATOR_REGISTRY_LIMIT: usize,
    const EPOCHS_PER_HISTORICAL_VECTOR: usize,
    const EPOCHS_PER_SLASHINGS_VECTOR: usize,
    const MAX_VALIDATORS_PER_COMMITTEE: usize,
    const SYNC_COMMITTEE_SIZE: usize,
    const BYTES_PER_LOGS_BLOOM: usize,
    const MAX_EXTRA_DATA_BYTES: usize,
    const PENDING_DEPOSITS_LIMIT: usize,
    const PENDING_PARTIAL_WITHDRAWALS_LIMIT: usize,
    const PENDING_CONSOLIDATIONS_LIMIT: usize,
> {
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub genesis_time: u64,
    pub genesis_validators_root: Root,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub slot: Slot,
    pub fork: ethereum_consensus::phase0::Fork,
    pub latest_block_header: BeaconBlockHeader,
    pub block_roots: Vector<Root, SLOTS_PER_HISTORICAL_ROOT>,
    pub state_roots: Vector<Root, SLOTS_PER_HISTORICAL_ROOT>,
    pub historical_roots: List<Root, HISTORICAL_ROOTS_LIMIT>,
    pub eth1_data: Eth1Data,
    pub eth1_data_votes: List<Eth1Data, ETH1_DATA_VOTES_BOUND>,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub eth1_deposit_index: u64,
    pub validators: List<Validator, VALIDATOR_REGISTRY_LIMIT>,
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub balances: List<Gwei, VALIDATOR_REGISTRY_LIMIT>,
    pub randao_mixes: Vector<Bytes32, EPOCHS_PER_HISTORICAL_VECTOR>,
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub slashings: Vector<Gwei, EPOCHS_PER_SLASHINGS_VECTOR>,
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub previous_epoch_participation: List<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub current_epoch_participation: List<ParticipationFlags, VALIDATOR_REGISTRY_LIMIT>,
    pub justification_bits: Bitvector<JUSTIFICATION_BITS_LENGTH>,
    pub previous_justified_checkpoint: Checkpoint,
    pub current_justified_checkpoint: Checkpoint,
    pub finalized_checkpoint: Checkpoint,
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub inactivity_scores: List<u64, VALIDATOR_REGISTRY_LIMIT>,
    pub current_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
    pub next_sync_committee: SyncCommittee<SYNC_COMMITTEE_SIZE>,
    pub latest_execution_payload_header: ethereum_consensus::deneb::ExecutionPayloadHeader<
        BYTES_PER_LOGS_BLOOM,
        MAX_EXTRA_DATA_BYTES,
    >,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub next_withdrawal_index: WithdrawalIndex,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub next_withdrawal_validator_index: ValidatorIndex,
    pub historical_summaries: List<HistoricalSummary, HISTORICAL_ROOTS_LIMIT>,
    // Electra additions, mainnet-final.
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub deposit_requests_start_index: u64,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub deposit_balance_to_consume: Gwei,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub exit_balance_to_consume: Gwei,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub earliest_exit_epoch: Epoch,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub consolidation_balance_to_consume: Gwei,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub earliest_consolidation_epoch: Epoch,
    pub pending_deposits: List<PendingDeposit, PENDING_DEPOSITS_LIMIT>,
    pub pending_partial_withdrawals:
        List<PendingPartialWithdrawal, PENDING_PARTIAL_WITHDRAWALS_LIMIT>,
    pub pending_consolidations: List<PendingConsolidation, PENDING_CONSOLIDATIONS_LIMIT>,
    // Fulu addition: EIP-7917 proposer lookahead.
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub proposer_lookahead: Vector<ValidatorIndex, PROPOSER_LOOKAHEAD_LIMIT>,
}
