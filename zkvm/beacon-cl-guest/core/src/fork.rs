//! Per-fork `BeaconState` generalized indices, and the guards that make them
//! survive the next hard fork. A port of the gindex table and KATs of an
//! earlier Willow circuit crate (the superseded Binius64 one),
//! carried over unchanged,
//! guard for guard.
//!
//! # Why this module exists
//!
//! A `BeaconState` field's generalized index is
//! `next_power_of_two(field_count) + position`. Both halves move: forks append
//! fields, and when the count crosses a power of two *every* index doubles.
//! Electra crossing 32 → 37 fields moved `validators` from 43 to 75. A guest that
//! hardcodes one fork's numbers silently proves the wrong subtree after the next
//! fork — the branch still verifies, it just authenticates a different field.
//!
//! So nothing here is a magic number. [`state_gindex`] *computes* the index, and
//! three guards keep its two inputs honest:
//!
//! 1. **Compile-time, against the declaration.** [`ELECTRA_STATE_FIELDS`] and
//!    [`FULU_STATE_FIELDS`] are checked against the actual Rust `BeaconState`
//!    declarations in `crates/network/src/beacon/{electra_final,fulu_final}.rs` —
//!    the same types `state_fetcher.rs` merkleizes real mainnet states with — by
//!    `include_str!`-ing those files and counting fields in a `const fn`. Append a
//!    field there and this crate **stops compiling** (E0080). The field *names* at
//!    the pinned positions are checked the same way, so a reordering is caught too.
//! 2. **Compile-time, against the fork enum.** Every `const fn` here matches
//!    [`ForkVersion`] exhaustively with no wildcard arm. Adding a variant for the
//!    next fork is a compile error in each table. *Never write a `_ =>` arm here.*
//! 3. **Test-time.** [`tests`] re-derives the pinned indices and checks depths and
//!    branch round-trips.

use crate::ssz::{hash_pair, Bytes32};

/// Beacon forks whose `BeaconState` shape this crate knows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkVersion {
    Capella,
    Deneb,
    Electra,
    Fulu,
}

/// Every fork this crate has a table for, for exhaustive iteration in tests.
pub const ALL_FORKS: [ForkVersion; 4] = [
    ForkVersion::Capella,
    ForkVersion::Deneb,
    ForkVersion::Electra,
    ForkVersion::Fulu,
];

/// Top-level `BeaconState` fields at Capella (phase0's 21 + Altair's 3 +
/// Bellatrix's 1 + Capella's 3). Not reachable by the compile-time guard —
/// `ethereum-consensus` owns the type — pinned by test only.
pub const CAPELLA_STATE_FIELDS: usize = 28;

/// Deneb adds no `BeaconState` field; same count, same indices as Capella.
pub const DENEB_STATE_FIELDS: usize = 28;

/// Electra's 37 fields (Deneb's 28 + nine EIP-6110/7002/7251 fields). Crossing 32
/// is what doubled every index at this fork.
pub const ELECTRA_STATE_FIELDS: usize = 37;

/// Fulu's 38 fields (Electra's 37 + EIP-7917 `proposer_lookahead`). Still ≤ 64, so
/// indices are unchanged from Electra — the guard below tells us when that stops.
pub const FULU_STATE_FIELDS: usize = 38;

/// A top-level `BeaconState` field this crate can anchor to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateField {
    /// Field 2. Bound so a consumer can match the proof against a
    /// `BeaconAnchor { beacon_state_root, slot }` on both halves, not just the root.
    Slot,
    /// Field 11. `hash_tree_root(state.validators)`, A2's registry root.
    Validators,
    /// Field 12. `hash_tree_root(state.balances)`, A1's list root.
    Balances,
    /// Field 34, Electra and later only. The post-Pectra deposit queue.
    PendingDeposits,
    /// Field 24. `hash_tree_root(state.latest_execution_payload_header)`; its
    /// `state_root` subfield is the EL anchor the stage-2 join reads at composed
    /// gindex 2818. Carried so the guard covers it too.
    LatestExecutionPayloadHeader,
}

/// 0-based declaration position of a field in the `BeaconState` container.
pub const fn field_position(field: StateField) -> usize {
    match field {
        StateField::Slot => 2,
        StateField::Validators => 11,
        StateField::Balances => 12,
        StateField::PendingDeposits => 34,
        StateField::LatestExecutionPayloadHeader => 24,
    }
}

/// Number of top-level `BeaconState` fields at `fork`.
pub const fn state_field_count(fork: ForkVersion) -> usize {
    match fork {
        ForkVersion::Capella => CAPELLA_STATE_FIELDS,
        ForkVersion::Deneb => DENEB_STATE_FIELDS,
        ForkVersion::Electra => ELECTRA_STATE_FIELDS,
        ForkVersion::Fulu => FULU_STATE_FIELDS,
    }
}

/// Leaf slots in the `BeaconState` container tree: `next_power_of_two(fields)`.
pub const fn state_leaf_slots(fork: ForkVersion) -> usize {
    state_field_count(fork).next_power_of_two()
}

/// Depth of the `BeaconState` container tree, and so of every top-level field's
/// Merkle branch.
pub const fn state_tree_depth(fork: ForkVersion) -> usize {
    state_leaf_slots(fork).trailing_zeros() as usize
}

/// Generalized index of `field` in `fork`'s `BeaconState`, or `None` if the field
/// does not exist at that fork. `next_power_of_two(field_count) + position`.
pub const fn state_gindex(fork: ForkVersion, field: StateField) -> Option<u64> {
    match (fork, field) {
        (ForkVersion::Capella | ForkVersion::Deneb, StateField::PendingDeposits) => None,
        _ => Some((state_leaf_slots(fork) + field_position(field)) as u64),
    }
}

/// [`state_gindex`], panicking rather than returning `None`.
pub const fn require_state_gindex(fork: ForkVersion, field: StateField) -> u64 {
    match state_gindex(fork, field) {
        Some(g) => g,
        None => panic!("field does not exist at this fork"),
    }
}

/// Branch depth implied by a generalized index: `floor(log2(gindex))`.
pub const fn depth_for_gindex(gindex: u64) -> usize {
    assert!(gindex >= 1, "generalized index is 1-based");
    (63 - gindex.leading_zeros()) as usize
}

// ---------------------------------------------------------------------
// Guard 1: the declaration this crate's numbers claim to describe
// ---------------------------------------------------------------------

const ELECTRA_STATE_SRC: &str =
    include_str!("../../../../crates/network/src/beacon/electra_final.rs");
const FULU_STATE_SRC: &str = include_str!("../../../../crates/network/src/beacon/fulu_final.rs");

/// Offset just past the first occurrence of `needle` at or after `from`, or
/// `usize::MAX` if there is none.
const fn find_from(hay: &[u8], needle: &[u8], from: usize) -> usize {
    let mut i = from;
    while i + needle.len() <= hay.len() {
        let mut j = 0;
        while j < needle.len() && hay[i + j] == needle[j] {
            j += 1;
        }
        if j == needle.len() {
            return i + needle.len();
        }
        i += 1;
    }
    usize::MAX
}

/// Offset of the newline that opens `pub struct BeaconState`'s body.
const fn state_body_start(src: &str) -> usize {
    let s = src.as_bytes();
    let named = find_from(s, b"pub struct BeaconState<", 0);
    assert!(
        named != usize::MAX,
        "no `pub struct BeaconState<` in the included source — the file moved or \
         was renamed, so this crate's generalized indices are unverified"
    );
    let opened = find_from(s, b"\n> {\n", named);
    assert!(
        opened != usize::MAX,
        "could not find the end of `BeaconState`'s generic parameter list — the \
         declaration was reformatted and the field scan below is unreliable"
    );
    opened - 1
}

/// Offset just past the `"\n}"` that closes `BeaconState`'s body.
const fn state_body_end(src: &str, start: usize) -> usize {
    let end = find_from(src.as_bytes(), b"\n}", start);
    assert!(end != usize::MAX, "unterminated `BeaconState` body");
    end
}

/// Number of `pub` fields declared in the included file's `BeaconState`.
const fn declared_field_count(src: &str) -> usize {
    let s = src.as_bytes();
    let start = state_body_start(src);
    let end = state_body_end(src, start);
    let mut n = 0;
    let mut i = start;
    loop {
        let hit = find_from(s, b"\n    pub ", i);
        if hit == usize::MAX || hit >= end {
            return n;
        }
        n += 1;
        i = hit;
    }
}

/// Whether the field at 0-based `index` in the included `BeaconState` is `name`.
const fn declared_field_is(src: &str, index: usize, name: &str) -> bool {
    let s = src.as_bytes();
    let nm = name.as_bytes();
    let start = state_body_start(src);
    let end = state_body_end(src, start);
    let mut n = 0;
    let mut i = start;
    loop {
        let hit = find_from(s, b"\n    pub ", i);
        if hit == usize::MAX || hit >= end {
            return false;
        }
        if n == index {
            let mut j = 0;
            while j < nm.len() {
                if hit + j >= s.len() || s[hit + j] != nm[j] {
                    return false;
                }
                j += 1;
            }
            return hit + j < s.len() && s[hit + j] == b':';
        }
        n += 1;
        i = hit;
    }
}

const _: () = assert!(
    declared_field_count(ELECTRA_STATE_SRC) == ELECTRA_STATE_FIELDS,
    "electra_final.rs's BeaconState no longer has 37 fields. Every generalized \
     index in this crate is now suspect: recount, update ELECTRA_STATE_FIELDS, \
     and re-pin the KATs in tests."
);
const _: () = assert!(
    declared_field_count(FULU_STATE_SRC) == FULU_STATE_FIELDS,
    "fulu_final.rs's BeaconState no longer has 38 fields. Recount, update \
     FULU_STATE_FIELDS, and re-pin the KATs."
);
const _: () = assert!(
    ELECTRA_STATE_FIELDS.next_power_of_two() == FULU_STATE_FIELDS.next_power_of_two(),
    "Electra and Fulu no longer share a container depth, so they can no longer \
     share a generalized-index table. Split state_field_count's match arms."
);

const _: () = assert!(
    declared_field_is(ELECTRA_STATE_SRC, field_position(StateField::Slot), "slot"),
    "electra_final.rs's BeaconState field 2 is not `slot` — the fields were \
     reordered and field_position is wrong."
);
const _: () = assert!(
    declared_field_is(
        ELECTRA_STATE_SRC,
        field_position(StateField::Validators),
        "validators"
    ),
    "electra_final.rs's BeaconState field 11 is not `validators`."
);
const _: () = assert!(
    declared_field_is(
        ELECTRA_STATE_SRC,
        field_position(StateField::Balances),
        "balances"
    ),
    "electra_final.rs's BeaconState field 12 is not `balances`."
);
const _: () = assert!(
    declared_field_is(
        ELECTRA_STATE_SRC,
        field_position(StateField::PendingDeposits),
        "pending_deposits"
    ),
    "electra_final.rs's BeaconState field 34 is not `pending_deposits`."
);
const _: () = assert!(
    declared_field_is(
        ELECTRA_STATE_SRC,
        field_position(StateField::LatestExecutionPayloadHeader),
        "latest_execution_payload_header"
    ),
    "electra_final.rs's BeaconState field 24 is not `latest_execution_payload_header`."
);

const _: () = assert!(
    declared_field_is(FULU_STATE_SRC, field_position(StateField::Slot), "slot"),
    "fulu_final.rs's BeaconState field 2 is not `slot`."
);
const _: () = assert!(
    declared_field_is(
        FULU_STATE_SRC,
        field_position(StateField::Validators),
        "validators"
    ),
    "fulu_final.rs's BeaconState field 11 is not `validators`."
);
const _: () = assert!(
    declared_field_is(
        FULU_STATE_SRC,
        field_position(StateField::Balances),
        "balances"
    ),
    "fulu_final.rs's BeaconState field 12 is not `balances`."
);

// ---------------------------------------------------------------------
// Native Merkle-branch reference
// ---------------------------------------------------------------------

/// Fold `siblings` into the root they imply for `leaf` at `gindex`. Bit `i` of
/// `gindex` (LSB first) says which side `leaf`'s ancestor sits on at level `i`:
/// 1 = right child, so the sibling hashes on the left.
///
/// Returns `None` if `siblings.len()` is not `depth_for_gindex(gindex)` — the
/// depth is a property of the index, never of the caller's data, and a
/// mismatched branch is rejected rather than folded (or panicked on: in-guest
/// a panic aborts the whole prove).
pub fn branch_root(leaf: &Bytes32, siblings: &[Bytes32], gindex: u64) -> Option<Bytes32> {
    if siblings.len() != depth_for_gindex(gindex) {
        return None;
    }
    let mut value = *leaf;
    for (i, sibling) in siblings.iter().enumerate() {
        value = if (gindex >> i) & 1 == 1 {
            hash_pair(sibling, &value)
        } else {
            hash_pair(&value, sibling)
        };
    }
    Some(value)
}

/// Whether `leaf` at `gindex` with `siblings` reproduces `root`. False —
/// fail closed — when the branch length doesn't match the index depth.
pub fn verify_branch(leaf: &Bytes32, siblings: &[Bytes32], gindex: u64, root: &Bytes32) -> bool {
    branch_root(leaf, siblings, gindex).as_ref() == Some(root)
}

/// A `BeaconState` as its top-level field leaves — all the anchoring needs.
#[derive(Clone, Debug)]
pub struct StateLeaves {
    pub fork: ForkVersion,
    leaves: Vec<Bytes32>,
}

impl StateLeaves {
    /// An all-zero state of `fork`'s shape.
    pub fn new(fork: ForkVersion) -> Self {
        Self {
            fork,
            leaves: vec![[0u8; 32]; state_leaf_slots(fork)],
        }
    }

    /// Set the leaf at a 0-based declaration position.
    pub fn set_position(&mut self, position: usize, leaf: Bytes32) {
        assert!(
            position < state_field_count(self.fork),
            "position {position} is past {:?}'s {} fields",
            self.fork,
            state_field_count(self.fork)
        );
        self.leaves[position] = leaf;
    }

    /// Set a named field's leaf.
    pub fn set(&mut self, field: StateField, leaf: Bytes32) {
        assert!(
            state_gindex(self.fork, field).is_some(),
            "{field:?} does not exist at {:?}",
            self.fork
        );
        self.set_position(field_position(field), leaf);
    }

    /// A named field's leaf.
    pub fn leaf(&self, field: StateField) -> Bytes32 {
        self.leaves[field_position(field)]
    }

    /// The container tree, 1-indexed by generalized index.
    fn tree(&self) -> Vec<Bytes32> {
        let slots = self.leaves.len();
        let mut tree = vec![[0u8; 32]; 2 * slots];
        tree[slots..].copy_from_slice(&self.leaves);
        for g in (1..slots).rev() {
            tree[g] = hash_pair(&tree[2 * g], &tree[2 * g + 1]);
        }
        tree
    }

    /// `hash_tree_root(BeaconState)`.
    pub fn root(&self) -> Bytes32 {
        self.tree()[1]
    }

    /// The Merkle branch for `gindex`, leaf-side first.
    pub fn branch_at_gindex(&self, gindex: u64) -> Vec<Bytes32> {
        assert_eq!(
            depth_for_gindex(gindex),
            state_tree_depth(self.fork),
            "gindex {gindex} is not a top-level field of {:?}'s BeaconState",
            self.fork
        );
        let tree = self.tree();
        let mut g = gindex;
        let mut branch = Vec::with_capacity(state_tree_depth(self.fork));
        while g > 1 {
            branch.push(tree[(g ^ 1) as usize]);
            g >>= 1;
        }
        branch
    }

    /// The Merkle branch for a named field.
    pub fn branch(&self, field: StateField) -> Vec<Bytes32> {
        self.branch_at_gindex(require_state_gindex(self.fork, field))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A branch whose length disagrees with the gindex depth must be
    /// rejected, never folded and never a panic (in-guest a panic aborts
    /// the whole prove).
    #[test]
    fn a_short_branch_is_none_not_a_panic() {
        let g = require_state_gindex(ForkVersion::Electra, StateField::Balances);
        assert_eq!(branch_root(&[7u8; 32], &[[0u8; 32]; 5], g), None);
        assert!(!verify_branch(&[7u8; 32], &[[0u8; 32]; 5], g, &[0u8; 32]));
    }
    use crate::ssz::uint64_leaf;

    #[test]
    fn derived_gindices_match_the_pinned_values() {
        assert_eq!(
            state_gindex(ForkVersion::Electra, StateField::Validators),
            Some(75)
        );
        assert_eq!(
            state_gindex(ForkVersion::Electra, StateField::Balances),
            Some(76)
        );
        assert_eq!(
            state_gindex(ForkVersion::Electra, StateField::Slot),
            Some(66)
        );
        assert_eq!(
            state_gindex(ForkVersion::Electra, StateField::PendingDeposits),
            Some(98)
        );
        assert_eq!(
            state_gindex(ForkVersion::Fulu, StateField::Validators),
            Some(75)
        );
        assert_eq!(
            state_gindex(ForkVersion::Fulu, StateField::Balances),
            Some(76)
        );
        // Capella/Deneb: 28 fields → 32 leaf slots → 32 + position.
        assert_eq!(
            state_gindex(ForkVersion::Capella, StateField::Validators),
            Some(43)
        );
        assert_eq!(
            state_gindex(ForkVersion::Deneb, StateField::Balances),
            Some(44)
        );
        assert_eq!(
            state_gindex(ForkVersion::Deneb, StateField::PendingDeposits),
            None
        );
    }

    #[test]
    fn depth_is_five_pre_electra_and_six_after() {
        assert_eq!(state_tree_depth(ForkVersion::Capella), 5);
        assert_eq!(state_tree_depth(ForkVersion::Deneb), 5);
        assert_eq!(state_tree_depth(ForkVersion::Electra), 6);
        assert_eq!(state_tree_depth(ForkVersion::Fulu), 6);
    }

    #[test]
    fn branch_from_leaves_reproduces_the_root() {
        for fork in ALL_FORKS {
            let mut state = StateLeaves::new(fork);
            state.set(StateField::Slot, uint64_leaf(11_649_024));
            state.set(StateField::Validators, [0x11; 32]);
            state.set(StateField::Balances, [0x22; 32]);
            let root = state.root();
            for field in [
                StateField::Slot,
                StateField::Validators,
                StateField::Balances,
            ] {
                let g = require_state_gindex(fork, field);
                assert!(
                    verify_branch(&state.leaf(field), &state.branch(field), g, &root),
                    "{fork:?} {field:?}"
                );
            }
        }
    }

    #[test]
    fn the_declaration_scanner_reads_the_real_field_list() {
        assert_eq!(declared_field_count(ELECTRA_STATE_SRC), 37);
        assert_eq!(declared_field_count(FULU_STATE_SRC), 38);
        assert!(declared_field_is(ELECTRA_STATE_SRC, 0, "genesis_time"));
        assert!(declared_field_is(FULU_STATE_SRC, 37, "proposer_lookahead"));
        assert!(!declared_field_is(ELECTRA_STATE_SRC, 12, "validators"));
        // `balance` is a prefix of `balances`; the trailing `:` check stops it.
        assert!(!declared_field_is(ELECTRA_STATE_SRC, 12, "balance"));
    }
}
