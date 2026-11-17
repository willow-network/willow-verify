






























use crate::ssz::{hash_pair, Bytes32};


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForkVersion {
    Capella,
    Deneb,
    Electra,
    Fulu,
}












pub const CAPELLA_STATE_FIELDS: usize = 28;


pub const DENEB_STATE_FIELDS: usize = 28;



pub const ELECTRA_STATE_FIELDS: usize = 37;



pub const FULU_STATE_FIELDS: usize = 38;


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StateField {


    Slot,

    Validators,

    Balances,

    PendingDeposits,



    LatestExecutionPayloadHeader,
}


pub const fn field_position(field: StateField) -> usize {
    match field {
        StateField::Slot => 2,
        StateField::Validators => 11,
        StateField::Balances => 12,
        StateField::PendingDeposits => 34,
        StateField::LatestExecutionPayloadHeader => 24,
    }
}


pub const fn state_field_count(fork: ForkVersion) -> usize {
    match fork {
        ForkVersion::Capella => CAPELLA_STATE_FIELDS,
        ForkVersion::Deneb => DENEB_STATE_FIELDS,
        ForkVersion::Electra => ELECTRA_STATE_FIELDS,
        ForkVersion::Fulu => FULU_STATE_FIELDS,
    }
}


pub const fn state_leaf_slots(fork: ForkVersion) -> usize {
    state_field_count(fork).next_power_of_two()
}



pub const fn state_tree_depth(fork: ForkVersion) -> usize {
    state_leaf_slots(fork).trailing_zeros() as usize
}



















pub const fn depth_for_gindex(gindex: u64) -> usize {
    assert!(gindex >= 1, "generalized index is 1-based");
    (63 - gindex.leading_zeros()) as usize
}





const ELECTRA_STATE_SRC: &str =
    include_str!("../../../../crates/network/src/beacon/electra_final.rs");
const FULU_STATE_SRC: &str = include_str!("../../../../crates/network/src/beacon/fulu_final.rs");



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


const fn state_body_end(src: &str, start: usize) -> usize {
    let end = find_from(src.as_bytes(), b"\n}", start);
    assert!(end != usize::MAX, "unterminated `BeaconState` body");
    end
}


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



pub fn verify_branch(leaf: &Bytes32, siblings: &[Bytes32], gindex: u64, root: &Bytes32) -> bool {
    branch_root(leaf, siblings, gindex).as_ref() == Some(root)
}




















































































































































































