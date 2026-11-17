












































use ethereum_consensus::altair::{SyncAggregate, SyncCommittee};
use ethereum_consensus::capella::{HistoricalSummary, SignedBlsToExecutionChange};
use ethereum_consensus::crypto::KzgCommitment;
use ethereum_consensus::phase0::{
    AttestationData, BeaconBlockHeader, Checkpoint, Deposit, Eth1Data, ProposerSlashing,
    SignedVoluntaryExit, Validator, JUSTIFICATION_BITS_LENGTH,
};
use ethereum_consensus::primitives::{
    BlsPublicKey, BlsSignature, Bytes32, Epoch, ExecutionAddress, Gwei, ParticipationFlags, Root,
    Slot, ValidatorIndex, WithdrawalIndex,
};
use ethereum_consensus::ssz::prelude::*;






#[derive(
    Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct Attestation<const MAX_VALIDATORS_PER_SLOT: usize, const MAX_COMMITTEES_PER_SLOT: usize> {
    pub aggregation_bits: Bitlist<MAX_VALIDATORS_PER_SLOT>,
    pub data: AttestationData,
    pub signature: BlsSignature,
    pub committee_bits: Bitvector<MAX_COMMITTEES_PER_SLOT>,
}



#[derive(
    Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct IndexedAttestation<const MAX_VALIDATORS_PER_SLOT: usize> {
    #[serde(with = "ethereum_consensus::serde::seq_of_str")]
    pub attesting_indices: List<ValidatorIndex, MAX_VALIDATORS_PER_SLOT>,
    pub data: AttestationData,
    pub signature: BlsSignature,
}





#[derive(
    Default, Debug, SimpleSerialize, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct AttesterSlashing<const MAX_VALIDATORS_PER_SLOT: usize> {
    pub attestation_1: IndexedAttestation<MAX_VALIDATORS_PER_SLOT>,
    pub attestation_2: IndexedAttestation<MAX_VALIDATORS_PER_SLOT>,
}



#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct DepositRequest {
    #[serde(rename = "pubkey")]
    pub public_key: BlsPublicKey,
    pub withdrawal_credentials: Bytes32,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub amount: Gwei,
    pub signature: BlsSignature,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub index: u64,
}



#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct WithdrawalRequest {
    pub source_address: ExecutionAddress,
    #[serde(rename = "validator_pubkey")]
    pub validator_public_key: BlsPublicKey,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub amount: Gwei,
}



#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct ConsolidationRequest {
    pub source_address: ExecutionAddress,
    #[serde(rename = "source_pubkey")]
    pub source_public_key: BlsPublicKey,
    #[serde(rename = "target_pubkey")]
    pub target_public_key: BlsPublicKey,
}



#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct ExecutionRequests<
    const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
    pub deposits: List<DepositRequest, MAX_DEPOSIT_REQUESTS_PER_PAYLOAD>,
    pub withdrawals: List<WithdrawalRequest, MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD>,
    pub consolidations: List<ConsolidationRequest, MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD>,
}




#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct PendingDeposit {
    #[serde(rename = "pubkey")]
    pub public_key: BlsPublicKey,
    pub withdrawal_credentials: Bytes32,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub amount: Gwei,
    pub signature: BlsSignature,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub slot: Slot,
}




#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct PendingPartialWithdrawal {
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub validator_index: ValidatorIndex,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub amount: Gwei,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub withdrawable_epoch: Epoch,
}




pub use ethereum_consensus::electra::beacon_state::PendingConsolidation;



#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct BeaconBlockBody<
    const MAX_PROPOSER_SLASHINGS: usize,
    const MAX_VALIDATORS_PER_SLOT: usize,
    const MAX_COMMITTEES_PER_SLOT: usize,
    const MAX_ATTESTER_SLASHINGS: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_DEPOSITS: usize,
    const MAX_VOLUNTARY_EXITS: usize,
    const SYNC_COMMITTEE_SIZE: usize,
    const BYTES_PER_LOGS_BLOOM: usize,
    const MAX_EXTRA_DATA_BYTES: usize,
    const MAX_BYTES_PER_TRANSACTION: usize,
    const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
    const MAX_BLS_TO_EXECUTION_CHANGES: usize,
    const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize,
    const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
    pub randao_reveal: BlsSignature,
    pub eth1_data: Eth1Data,
    pub graffiti: Bytes32,
    pub proposer_slashings: List<ProposerSlashing, MAX_PROPOSER_SLASHINGS>,
    pub attester_slashings:
        List<self::AttesterSlashing<MAX_VALIDATORS_PER_SLOT>, MAX_ATTESTER_SLASHINGS>,
    pub attestations:
        List<self::Attestation<MAX_VALIDATORS_PER_SLOT, MAX_COMMITTEES_PER_SLOT>, MAX_ATTESTATIONS>,
    pub deposits: List<Deposit, MAX_DEPOSITS>,
    pub voluntary_exits: List<SignedVoluntaryExit, MAX_VOLUNTARY_EXITS>,
    pub sync_aggregate: SyncAggregate<SYNC_COMMITTEE_SIZE>,
    pub execution_payload: ethereum_consensus::deneb::ExecutionPayload<
        BYTES_PER_LOGS_BLOOM,
        MAX_EXTRA_DATA_BYTES,
        MAX_BYTES_PER_TRANSACTION,
        MAX_TRANSACTIONS_PER_PAYLOAD,
        MAX_WITHDRAWALS_PER_PAYLOAD,
    >,
    pub bls_to_execution_changes: List<SignedBlsToExecutionChange, MAX_BLS_TO_EXECUTION_CHANGES>,
    pub blob_kzg_commitments: List<KzgCommitment, MAX_BLOB_COMMITMENTS_PER_BLOCK>,
    pub execution_requests: ExecutionRequests<
        MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
        MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
        MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
    >,
}

#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct BeaconBlock<
    const MAX_PROPOSER_SLASHINGS: usize,
    const MAX_VALIDATORS_PER_SLOT: usize,
    const MAX_COMMITTEES_PER_SLOT: usize,
    const MAX_ATTESTER_SLASHINGS: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_DEPOSITS: usize,
    const MAX_VOLUNTARY_EXITS: usize,
    const SYNC_COMMITTEE_SIZE: usize,
    const BYTES_PER_LOGS_BLOOM: usize,
    const MAX_EXTRA_DATA_BYTES: usize,
    const MAX_BYTES_PER_TRANSACTION: usize,
    const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
    const MAX_BLS_TO_EXECUTION_CHANGES: usize,
    const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize,
    const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub slot: Slot,
    #[serde(with = "ethereum_consensus::serde::as_str")]
    pub proposer_index: ValidatorIndex,
    pub parent_root: Root,
    pub state_root: Root,
    pub body: BeaconBlockBody<
        MAX_PROPOSER_SLASHINGS,
        MAX_VALIDATORS_PER_SLOT,
        MAX_COMMITTEES_PER_SLOT,
        MAX_ATTESTER_SLASHINGS,
        MAX_ATTESTATIONS,
        MAX_DEPOSITS,
        MAX_VOLUNTARY_EXITS,
        SYNC_COMMITTEE_SIZE,
        BYTES_PER_LOGS_BLOOM,
        MAX_EXTRA_DATA_BYTES,
        MAX_BYTES_PER_TRANSACTION,
        MAX_TRANSACTIONS_PER_PAYLOAD,
        MAX_WITHDRAWALS_PER_PAYLOAD,
        MAX_BLS_TO_EXECUTION_CHANGES,
        MAX_BLOB_COMMITMENTS_PER_BLOCK,
        MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
        MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
        MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
    >,
}

#[derive(
    Default, Debug, Clone, SimpleSerialize, PartialEq, Eq, serde::Serialize, serde::Deserialize,
)]
pub struct SignedBeaconBlock<
    const MAX_PROPOSER_SLASHINGS: usize,
    const MAX_VALIDATORS_PER_SLOT: usize,
    const MAX_COMMITTEES_PER_SLOT: usize,
    const MAX_ATTESTER_SLASHINGS: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_DEPOSITS: usize,
    const MAX_VOLUNTARY_EXITS: usize,
    const SYNC_COMMITTEE_SIZE: usize,
    const BYTES_PER_LOGS_BLOOM: usize,
    const MAX_EXTRA_DATA_BYTES: usize,
    const MAX_BYTES_PER_TRANSACTION: usize,
    const MAX_TRANSACTIONS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWALS_PER_PAYLOAD: usize,
    const MAX_BLS_TO_EXECUTION_CHANGES: usize,
    const MAX_BLOB_COMMITMENTS_PER_BLOCK: usize,
    const MAX_DEPOSIT_REQUESTS_PER_PAYLOAD: usize,
    const MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD: usize,
    const MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD: usize,
> {
    pub message: BeaconBlock<
        MAX_PROPOSER_SLASHINGS,
        MAX_VALIDATORS_PER_SLOT,
        MAX_COMMITTEES_PER_SLOT,
        MAX_ATTESTER_SLASHINGS,
        MAX_ATTESTATIONS,
        MAX_DEPOSITS,
        MAX_VOLUNTARY_EXITS,
        SYNC_COMMITTEE_SIZE,
        BYTES_PER_LOGS_BLOOM,
        MAX_EXTRA_DATA_BYTES,
        MAX_BYTES_PER_TRANSACTION,
        MAX_TRANSACTIONS_PER_PAYLOAD,
        MAX_WITHDRAWALS_PER_PAYLOAD,
        MAX_BLS_TO_EXECUTION_CHANGES,
        MAX_BLOB_COMMITMENTS_PER_BLOCK,
        MAX_DEPOSIT_REQUESTS_PER_PAYLOAD,
        MAX_WITHDRAWAL_REQUESTS_PER_PAYLOAD,
        MAX_CONSOLIDATION_REQUESTS_PER_PAYLOAD,
    >,
    pub signature: BlsSignature,
}
















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
}
