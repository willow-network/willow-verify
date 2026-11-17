

















use crate::error::Error;
use crate::keccak256;
use alloc::vec::Vec;

const fn nib(c: u8) -> u8 {
    match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        _ => panic!("bad hex digit in a compile-time constant"),
    }
}


const fn b32(s: &str) -> [u8; 32] {
    let b = s.as_bytes();
    assert!(b.len() == 64);
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < 32 {
        out[i] = (nib(b[2 * i]) << 4) | nib(b[2 * i + 1]);
        i += 1;
    }
    out
}






pub const FILTER_TYPEHASH: [u8; 32] =
    b32("7da516d88ff342c0bec127d022bbb8bc6b014bb733e56f557d8bea57e8a50d70");


pub const BLOCK_TAG: [u8; 32] =
    b32("434f4d504c4554455f4c4f475f5345545f424c4f434b5f563100000000000000");


pub const NODE_TAG: [u8; 32] =
    b32("434f4d504c4554455f4c4f475f5345545f4e4f44455f56310000000000000000");


pub const MAX_TOPICS: usize = 4;



#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogSetStatement {
    pub chain_id: u64,
    pub start_block: u64,
    pub end_block: u64,
    pub start_parent_hash: [u8; 32],
    pub end_block_hash: [u8; 32],
    pub filter_hash: [u8; 32],
    pub log_set_root: [u8; 32],
    pub log_count: u64,
}










fn packed_hash_20(list: &[[u8; 20]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(list.len() * 32);
    for a in list {
        buf.extend_from_slice(&[0u8; 12]);
        buf.extend_from_slice(a);
    }
    keccak256(&buf)
}

fn packed_hash_32(list: &[[u8; 32]]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(list.len() * 32);
    for t in list {
        buf.extend_from_slice(t);
    }
    keccak256(&buf)
}




pub fn filter_hash(addresses: &[[u8; 20]], topics: &[Vec<[u8; 32]>; 4]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(6 * 32);
    buf.extend_from_slice(&FILTER_TYPEHASH);
    buf.extend_from_slice(&packed_hash_20(addresses));
    for list in topics {
        buf.extend_from_slice(&packed_hash_32(list));
    }
    keccak256(&buf)
}


























pub fn block_digest_prefix(block_number: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&BLOCK_TAG);
    buf.extend_from_slice(&block_number.to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf
}


pub const BLOCK_DIGEST_K_OFFSET: usize = 40;


pub fn finish_block_digest(preimage: &mut [u8], k: u32) -> [u8; 32] {
    preimage[BLOCK_DIGEST_K_OFFSET..BLOCK_DIGEST_K_OFFSET + 4].copy_from_slice(&k.to_be_bytes());
    keccak256(preimage)
}




































pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 96];
    buf[..32].copy_from_slice(&NODE_TAG);
    buf[32..64].copy_from_slice(left);
    buf[64..].copy_from_slice(right);
    keccak256(&buf)
}









pub fn log_set_root(leaves: &[[u8; 32]]) -> Result<[u8; 32], Error> {
    let n = leaves.len();
    if n == 0 {
        return Err(Error::EmptyRange);
    }
    let mut cap = 1usize;
    while cap < n {
        cap <<= 1;
    }
    let mut level: Vec<[u8; 32]> = Vec::with_capacity(cap);
    level.extend_from_slice(leaves);
    level.resize(cap, [0u8; 32]);

    let mut zero = [0u8; 32];
    while cap > 1 {
        cap >>= 1;
        let next_zero = node_hash(&zero, &zero);
        for i in 0..cap {
            let (l, r) = (level[2 * i], level[2 * i + 1]);
            level[i] = if l == zero && r == zero {
                next_zero
            } else {
                node_hash(&l, &r)
            };
        }
        zero = next_zero;
        level.truncate(cap);
    }
    Ok(level[0])
}











































