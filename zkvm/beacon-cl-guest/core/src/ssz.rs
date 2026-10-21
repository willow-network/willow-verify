//! In-guest SSZ merkleization for the two beacon lists we scan.
//!
//! A byte-faithful twin of `crates/beacon-ssz-binius/src/native.rs` (the oracle
//! the retired Binius64 circuits were differentially tested against, itself a
//! second implementation of what `crates/network`'s `state_fetcher.rs` gets from
//! `ethereum-consensus`). `tests::ssz_matches_the_shipping_crate` asserts this
//! port is byte-exact against that crate.
//!
//! Everything here is `f(list contents)` only — the SHA-256 is the SP1 precompile
//! under the zkVM (workspace patch) and software on the host, but the values are
//! identical either way, so a root computed in a `#[test]` is the root the guest
//! commits.

use sha2::{Digest, Sha256};

/// A 32-byte SSZ node.
pub type Bytes32 = [u8; 32];

/// `VALIDATOR_REGISTRY_LIMIT` for the mainnet preset (`2^40`).
pub const VALIDATOR_REGISTRY_LIMIT: u64 = 1 << 40;

/// Merkle depth of the `validators` List chunk tree: `List[Validator, 2^40]` has
/// one 32-byte chunk per element, so `log2(2^40) = 40`.
pub const VALIDATORS_TREE_DEPTH: usize = 40;

/// Merkle depth of the `balances` List chunk tree: `List[Gwei, 2^40]` packs four
/// `uint64` per 32-byte chunk, so `chunk_count = 2^40 * 8 / 32 = 2^38` and the
/// tree is **38** deep, not 40. The off-by-two is two padding levels of the wrong
/// root — getting it wrong matches nothing.
pub const BALANCES_TREE_DEPTH: usize = 38;

/// Balances packed per 32-byte SSZ chunk.
pub const BALANCES_PER_CHUNK: usize = 4;

/// SHA-256 of two concatenated 32-byte nodes — the SSZ internal-node hash. A
/// 64-byte message is two SHA-256 compressions (data block + padding block).
pub fn hash_pair(left: &Bytes32, right: &Bytes32) -> Bytes32 {
    let mut h = Sha256::new();
    h.update(left);
    h.update(right);
    h.finalize().into()
}

/// `zero_hashes()[d]` is the root of an all-zero subtree of depth `d`.
pub fn zero_hashes(max_depth: usize) -> Vec<Bytes32> {
    let mut z = Vec::with_capacity(max_depth + 1);
    z.push([0u8; 32]);
    for d in 1..=max_depth {
        let prev = z[d - 1];
        z.push(hash_pair(&prev, &prev));
    }
    z
}

/// Merkleize `chunks` left-packed into a `2^depth`-leaf tree, padding with the
/// zero subtree.
pub fn merkleize(chunks: &[Bytes32], depth: usize) -> Bytes32 {
    let z = zero_hashes(depth);
    let mut layer = chunks.to_vec();
    for zl in z.iter().take(depth) {
        if layer.is_empty() {
            break;
        }
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            let right = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                *zl
            };
            next.push(hash_pair(&layer[i], &right));
            i += 2;
        }
        layer = next;
    }
    if layer.is_empty() {
        z[depth]
    } else {
        layer[0]
    }
}

/// Merkleize `nodes` (each already a subtree root at height `leaf_depth`) up a
/// further `levels`, padding right with the zero subtree of the appropriate
/// height. Used by the top to lift slice roots to the two list roots.
pub fn merkleize_above(nodes: &[Bytes32], levels: usize, leaf_depth: usize) -> Bytes32 {
    let z = zero_hashes(leaf_depth + levels);
    let mut layer = nodes.to_vec();
    for l in 0..levels {
        if layer.is_empty() {
            break;
        }
        let zl = z[leaf_depth + l];
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            let right = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                zl
            };
            next.push(hash_pair(&layer[i], &right));
            i += 2;
        }
        layer = next;
    }
    if layer.is_empty() {
        z[leaf_depth + levels]
    } else {
        layer[0]
    }
}

/// The SSZ length mixin: `sha256(root ‖ uint256_le(length))`.
///
/// This single hash is the whole completeness argument for a List — it binds the
/// element count into the root, so a truncated list cannot reach the same anchored
/// root.
pub fn mix_in_length(root: &Bytes32, length: u64) -> Bytes32 {
    let mut len_leaf = [0u8; 32];
    len_leaf[..8].copy_from_slice(&length.to_le_bytes());
    hash_pair(root, &len_leaf)
}

/// Pack `balances` four little-endian `uint64` per 32-byte chunk, zero-filling the
/// tail of the final partial chunk.
pub fn pack_balances(balances: &[u64]) -> Vec<Bytes32> {
    balances
        .chunks(BALANCES_PER_CHUNK)
        .map(|c| {
            let mut chunk = [0u8; 32];
            for (i, b) in c.iter().enumerate() {
                chunk[i * 8..i * 8 + 8].copy_from_slice(&b.to_le_bytes());
            }
            chunk
        })
        .collect()
}

/// `hash_tree_root(state.balances)` — `List[Gwei, VALIDATOR_REGISTRY_LIMIT]`.
pub fn balances_hash_tree_root(balances: &[u64]) -> Bytes32 {
    let chunks = pack_balances(balances);
    let root = merkleize(&chunks, BALANCES_TREE_DEPTH);
    mix_in_length(&root, balances.len() as u64)
}

/// Sum of a balance list, checked for u64 wraparound the way the circuit checks it.
pub fn total_balance(balances: &[u64]) -> u64 {
    balances
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b))
        .expect("balance total overflows u64")
}

/// The eight fields of the `Validator` container, in SSZ declaration order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Validator {
    pub pubkey: [u8; 48],
    pub withdrawal_credentials: Bytes32,
    pub effective_balance: u64,
    pub slashed: bool,
    pub activation_eligibility_epoch: u64,
    pub activation_epoch: u64,
    pub exit_epoch: u64,
    pub withdrawable_epoch: u64,
}

impl Validator {
    /// The eight 32-byte field leaves, before the container's 3-level tree.
    pub fn field_leaves(&self) -> [Bytes32; 8] {
        [
            pubkey_hash_tree_root(&self.pubkey),
            self.withdrawal_credentials,
            uint64_leaf(self.effective_balance),
            bool_leaf(self.slashed),
            uint64_leaf(self.activation_eligibility_epoch),
            uint64_leaf(self.activation_epoch),
            uint64_leaf(self.exit_epoch),
            uint64_leaf(self.withdrawable_epoch),
        ]
    }
}

/// `uint64` leaf: little-endian in the low 8 bytes, zero-padded to 32.
pub fn uint64_leaf(v: u64) -> Bytes32 {
    let mut leaf = [0u8; 32];
    leaf[..8].copy_from_slice(&v.to_le_bytes());
    leaf
}

/// `boolean` leaf: `0x01`/`0x00` in byte 0, zero-padded to 32.
pub fn bool_leaf(v: bool) -> Bytes32 {
    let mut leaf = [0u8; 32];
    leaf[0] = v as u8;
    leaf
}

/// `hash_tree_root(Vector[byte, 48])` = one hash over two zero-padded chunks. This
/// is also the D2 join key: the CL half exposes it free as credential-branch
/// `siblings[0]`, and the stage-2 join intersects it with the EL registered set.
pub fn pubkey_hash_tree_root(pubkey: &[u8; 48]) -> Bytes32 {
    let mut a = [0u8; 32];
    a.copy_from_slice(&pubkey[..32]);
    let mut b = [0u8; 32];
    b[..16].copy_from_slice(&pubkey[32..]);
    hash_pair(&a, &b)
}

/// `hash_tree_root(Validator)` — a fixed 8-field container, depth 3, no mixin.
pub fn validator_hash_tree_root(v: &Validator) -> Bytes32 {
    merkleize(&v.field_leaves(), 3)
}

/// The three sibling nodes on the path from `withdrawal_credentials` (field 1) to
/// the `Validator` container root:
/// `[htr(pubkey), hash(effective_balance, slashed), hash(the four epoch leaves)]`.
///
/// A credential scan reads only field 1, so it can witness these three nodes and
/// verify a depth-3 branch instead of recomputing all eight field leaves — three
/// hashes per validator instead of eight (the [`ScanMode::CredentialBranch`] win).
/// `siblings[0]` is `htr(pubkey)`, the free join key.
pub fn credential_branch_siblings(v: &Validator) -> [Bytes32; 3] {
    let l = v.field_leaves();
    [
        l[0],
        hash_pair(&l[2], &l[3]),
        hash_pair(&hash_pair(&l[4], &l[5]), &hash_pair(&l[6], &l[7])),
    ]
}

/// Recombine a credential with its branch siblings into the container root.
pub fn apply_credential_branch(credential: &Bytes32, siblings: &[Bytes32; 3]) -> Bytes32 {
    let n01 = hash_pair(&siblings[0], credential);
    let n0123 = hash_pair(&n01, &siblings[1]);
    hash_pair(&n0123, &siblings[2])
}

/// `hash_tree_root(state.validators)` — `List[Validator, VALIDATOR_REGISTRY_LIMIT]`.
pub fn validators_hash_tree_root(validators: &[Validator]) -> Bytes32 {
    let chunks: Vec<Bytes32> = validators.iter().map(validator_hash_tree_root).collect();
    let root = merkleize(&chunks, VALIDATORS_TREE_DEPTH);
    mix_in_length(&root, validators.len() as u64)
}

/// Sum of `balances[i]` over every `i` whose validator carries `target`, and the
/// count — the reference for A2's public outputs `(matched_total, matched_count)`.
pub fn credential_scan(validators: &[Validator], balances: &[u64], target: &Bytes32) -> (u64, u64) {
    assert_eq!(
        validators.len(),
        balances.len(),
        "parallel-list invariant: validators and balances must be the same length"
    );
    let mut total = 0u64;
    let mut count = 0u64;
    for (v, b) in validators.iter().zip(balances) {
        if &v.withdrawal_credentials == target {
            total = total.checked_add(*b).expect("matched total overflows u64");
            count += 1;
        }
    }
    (total, count)
}
