//! SSZ hashing for the anchor child: the 5-field `BeaconBlockHeader` root, the
//! `SyncCommittee` root, and the signing-domain computation. Ports of the repo's
//! native light client (`crates/network/src/beacon/{bls,light_client}.rs`), on
//! `beacon_cl_core::ssz::hash_pair` so the guest build rides the same patched
//! sha256 precompile as every sibling guest.

use beacon_cl_core::ssz::{hash_pair, uint64_leaf, Bytes32};

/// Mainnet `SYNC_COMMITTEE_SIZE`.
pub const SYNC_COMMITTEE_SIZE: usize = 512;

/// `DOMAIN_SYNC_COMMITTEE` — the domain type for sync-committee signatures.
pub const DOMAIN_SYNC_COMMITTEE: [u8; 4] = [0x07, 0x00, 0x00, 0x00];

/// Merkleize a power-of-two list of 32-byte leaves (SSZ binary tree, no length
/// mixing — `Vector`/`Container` roots, not `List`).
fn merkleize_pow2(mut layer: Vec<Bytes32>) -> Bytes32 {
    debug_assert!(layer.len().is_power_of_two() && !layer.is_empty());
    while layer.len() > 1 {
        layer = layer
            .chunks_exact(2)
            .map(|pair| hash_pair(&pair[0], &pair[1]))
            .collect();
    }
    layer[0]
}

/// `hash_tree_root` of a `Bytes48` (a compressed BLS pubkey): two 32-byte chunks
/// (the second right-padded), merkleized.
fn bytes48_root(bytes: &[u8; 48]) -> Bytes32 {
    let mut chunk0 = [0u8; 32];
    chunk0.copy_from_slice(&bytes[..32]);
    let mut chunk1 = [0u8; 32];
    chunk1[..16].copy_from_slice(&bytes[32..]);
    hash_pair(&chunk0, &chunk1)
}

/// `hash_tree_root(BeaconBlockHeader{slot, proposer_index, parent_root,
/// state_root, body_root})` — the value the sync committee signs (combined with
/// the domain via [`compute_signing_root`]). Five fields padded to eight leaves;
/// `state_root` and `slot` are DIRECT fields, so recomputing this root from
/// witnessed fields is the whole binding.
pub fn beacon_block_header_root(
    slot: u64,
    proposer_index: u64,
    parent_root: &Bytes32,
    state_root: &Bytes32,
    body_root: &Bytes32,
) -> Bytes32 {
    merkleize_pow2(vec![
        uint64_leaf(slot),
        uint64_leaf(proposer_index),
        *parent_root,
        *state_root,
        *body_root,
        [0u8; 32],
        [0u8; 32],
        [0u8; 32],
    ])
}

/// `hash_tree_root` of an Altair `SyncCommittee`
/// `{pubkeys: Vector[Bytes48, 512], aggregate_pubkey: Bytes48}` — the committee's
/// identity, committed in the statement (and matched by the verifier contract's
/// stored committee chain).
pub fn sync_committee_hash_tree_root(
    pubkeys: &[[u8; 48]; SYNC_COMMITTEE_SIZE],
    aggregate_pubkey: &[u8; 48],
) -> Bytes32 {
    let pubkey_roots: Vec<Bytes32> = pubkeys.iter().map(bytes48_root).collect();
    let pubkeys_root = merkleize_pow2(pubkey_roots);
    hash_pair(&pubkeys_root, &bytes48_root(aggregate_pubkey))
}

/// `hash_tree_root(ForkData{current_version, genesis_validators_root})`.
fn compute_fork_data_root(fork_version: [u8; 4], genesis_validators_root: &Bytes32) -> Bytes32 {
    let mut version_chunk = [0u8; 32];
    version_chunk[..4].copy_from_slice(&fork_version);
    hash_pair(&version_chunk, genesis_validators_root)
}

/// `compute_domain` = `domain_type (4) || compute_fork_data_root(...)[..28]`.
pub fn compute_domain(
    domain_type: [u8; 4],
    fork_version: [u8; 4],
    genesis_validators_root: &Bytes32,
) -> Bytes32 {
    let fork_data_root = compute_fork_data_root(fork_version, genesis_validators_root);
    let mut domain = [0u8; 32];
    domain[..4].copy_from_slice(&domain_type);
    domain[4..].copy_from_slice(&fork_data_root[..28]);
    domain
}

/// `compute_signing_root(object_root, domain)` =
/// `hash_tree_root(SigningData{object_root, domain})`.
pub fn compute_signing_root(object_root: &Bytes32, domain: &Bytes32) -> Bytes32 {
    hash_pair(object_root, domain)
}

/// Whether bit `i` of an SSZ `Bitvector` is set (little-endian within each byte).
pub fn bit_set(bits: &[u8], i: usize) -> bool {
    bits[i / 8] & (1 << (i % 8)) != 0
}
