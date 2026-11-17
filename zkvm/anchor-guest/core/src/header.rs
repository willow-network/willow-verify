





use beacon_cl_core::ssz::{hash_pair, uint64_leaf, Bytes32};


pub const SYNC_COMMITTEE_SIZE: usize = 512;


pub const DOMAIN_SYNC_COMMITTEE: [u8; 4] = [0x07, 0x00, 0x00, 0x00];



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



fn bytes48_root(bytes: &[u8; 48]) -> Bytes32 {
    let mut chunk0 = [0u8; 32];
    chunk0.copy_from_slice(&bytes[..32]);
    let mut chunk1 = [0u8; 32];
    chunk1[..16].copy_from_slice(&bytes[32..]);
    hash_pair(&chunk0, &chunk1)
}






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





pub fn sync_committee_hash_tree_root(
    pubkeys: &[[u8; 48]; SYNC_COMMITTEE_SIZE],
    aggregate_pubkey: &[u8; 48],
) -> Bytes32 {
    let pubkey_roots: Vec<Bytes32> = pubkeys.iter().map(bytes48_root).collect();
    let pubkeys_root = merkleize_pow2(pubkey_roots);
    hash_pair(&pubkeys_root, &bytes48_root(aggregate_pubkey))
}


fn compute_fork_data_root(fork_version: [u8; 4], genesis_validators_root: &Bytes32) -> Bytes32 {
    let mut version_chunk = [0u8; 32];
    version_chunk[..4].copy_from_slice(&fork_version);
    hash_pair(&version_chunk, genesis_validators_root)
}


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



pub fn compute_signing_root(object_root: &Bytes32, domain: &Bytes32) -> Bytes32 {
    hash_pair(object_root, domain)
}


pub fn bit_set(bits: &[u8], i: usize) -> bool {
    bits[i / 8] & (1 << (i % 8)) != 0
}
