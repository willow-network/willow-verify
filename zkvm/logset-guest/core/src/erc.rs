//! The ERC's hashing rules, reproduced byte-for-byte.
//!
//! Every constant and every preimage here is pinned by a KAT against the Solidity
//! reference in the ERC's `assets/` (`forge test -vvvv` on
//! `test/Vectors.t.sol` prints them; see `tests::erc_vectors_*`). If Solidity and
//! this file ever disagree, an on-chain consumer would reject a proof that is
//! internally consistent, so the vectors are the contract between the two.
//!
//! ```text
//! filterHash    = keccak256(abi.encode(FILTER_TYPEHASH, keccak(packed addresses),
//!                                      keccak(packed topics0..3)))
//! record        = txIndex u32 BE | logIndex u32 BE | emitter 20 | nTopics u8
//!                 | topics 32 each | dataLen u32 BE | data
//! blockLogsHash = keccak256(BLOCK_TAG | number u64 BE | k u32 BE | r_1 | .. | r_k)
//! node          = keccak256(NODE_TAG | left | right)
//! statementHash = keccak256(abi.encode(STATEMENT_TYPEHASH, .. 8 fields ..))
//! ```

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

/// Parse a 64-character lowercase hex literal at compile time.
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

/// `keccak256("LogSetStatement(uint256 chainId,uint64 startBlock,uint64 endBlock,bytes32 startParentHash,bytes32 endBlockHash,bytes32 filterHash,bytes32 logSetRoot,uint64 logCount)")`
pub const STATEMENT_TYPEHASH: [u8; 32] =
    b32("f2391921543743dfef3936c0ae97d382aa712cbdef6b85a02b365436e6ac6b8c");

/// `keccak256("LogFilter(address[] addresses,bytes32[] topics0,bytes32[] topics1,bytes32[] topics2,bytes32[] topics3)")`
pub const FILTER_TYPEHASH: [u8; 32] =
    b32("7da516d88ff342c0bec127d022bbb8bc6b014bb733e56f557d8bea57e8a50d70");

/// `bytes32("COMPLETE_LOG_SET_BLOCK_V1")` — right-padded, the Solidity literal.
pub const BLOCK_TAG: [u8; 32] =
    b32("434f4d504c4554455f4c4f475f5345545f424c4f434b5f563100000000000000");

/// `bytes32("COMPLETE_LOG_SET_NODE_V1")` — right-padded.
pub const NODE_TAG: [u8; 32] =
    b32("434f4d504c4554455f4c4f475f5345545f4e4f44455f56310000000000000000");

/// The ERC's cap on a log's topic count.
pub const MAX_TOPICS: usize = 4;

/// The claim, one per filter. Field order is the Solidity struct's declaration
/// order, which is also the order `abi.encode` lays out.
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

/// Bytes of one ABI word holding a `uint64`.
fn word_u64(v: u64) -> [u8; 32] {
    let mut w = [0u8; 32];
    w[24..].copy_from_slice(&v.to_be_bytes());
    w
}

/// `keccak256(abi.encodePacked(list))` where each element occupies one 32-byte
/// left-padded word. Solidity pads array elements even in `encodePacked`.
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

/// The ERC `filterHash`. The caller must have already established that every list
/// is canonical — the guest does that at decode time so a non-canonical filter is
/// rejected before any work is done, not silently hashed.
pub fn filter_hash(addresses: &[[u8; 20]], topics: &[Vec<[u8; 32]>; 4]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(6 * 32);
    buf.extend_from_slice(&FILTER_TYPEHASH);
    buf.extend_from_slice(&packed_hash_20(addresses));
    for list in topics {
        buf.extend_from_slice(&packed_hash_32(list));
    }
    keccak256(&buf)
}

/// Append one log record to a `blockLogsHash` preimage.
///
/// `data.len()` must fit `u32`; the caller checks and returns
/// [`Error::LogDataTooLong`] rather than truncating.
pub fn push_log_record(
    out: &mut Vec<u8>,
    tx_index: u32,
    log_index: u32,
    emitter: &[u8; 20],
    topics: &[[u8; 32]],
    data: &[u8],
) {
    out.extend_from_slice(&tx_index.to_be_bytes());
    out.extend_from_slice(&log_index.to_be_bytes());
    out.extend_from_slice(emitter);
    out.push(topics.len() as u8);
    for t in topics {
        out.extend_from_slice(t);
    }
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(data);
}

/// The fixed 44-byte prefix of a `blockLogsHash` preimage, with `k` left at zero
/// for the caller to patch once the record count is known.
pub fn block_digest_prefix(block_number: u64) -> Vec<u8> {
    let mut buf = Vec::with_capacity(64);
    buf.extend_from_slice(&BLOCK_TAG);
    buf.extend_from_slice(&block_number.to_be_bytes());
    buf.extend_from_slice(&0u32.to_be_bytes());
    buf
}

/// Byte offset of the `k` field inside a `blockLogsHash` preimage.
pub const BLOCK_DIGEST_K_OFFSET: usize = 40;

/// Finish a preimage built with [`block_digest_prefix`] + [`push_log_record`].
pub fn finish_block_digest(preimage: &mut [u8], k: u32) -> [u8; 32] {
    preimage[BLOCK_DIGEST_K_OFFSET..BLOCK_DIGEST_K_OFFSET + 4].copy_from_slice(&k.to_be_bytes());
    keccak256(preimage)
}

/// `blockLogsHash` for a block in which nothing matched: `k = 0`, no records.
/// Depends only on the block number, so one value serves every filter.
pub fn empty_block_logs_hash(block_number: u64) -> [u8; 32] {
    keccak256(&block_digest_prefix(block_number))
}

/// One log, as the ERC records it. Used by the tests and by the `synth` builder;
/// the guest itself streams records straight into the digest preimage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRecord {
    pub tx_index: u32,
    pub log_index: u32,
    pub emitter: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

/// `blockLogsHash(blockNumber, records)`. `records` must be in ascending
/// `logIndex` order.
pub fn block_logs_hash(block_number: u64, records: &[LogRecord]) -> [u8; 32] {
    let mut buf = block_digest_prefix(block_number);
    for r in records {
        push_log_record(
            &mut buf,
            r.tx_index,
            r.log_index,
            &r.emitter,
            &r.topics,
            &r.data,
        );
    }
    finish_block_digest(&mut buf, records.len() as u32)
}

/// An interior node of the log-set tree.
pub fn node_hash(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = [0u8; 96];
    buf[..32].copy_from_slice(&NODE_TAG);
    buf[32..64].copy_from_slice(left);
    buf[64..].copy_from_slice(right);
    keccak256(&buf)
}

/// The log-set root: a complete binary tree over the next power of two, padded on
/// the right with ZERO-VALUED leaves (`0x00..00`, not a hash of anything), hashing
/// every interior position including the all-padding ones. `N == 1` ⇒ the leaf.
///
/// All-zero subtrees are folded from a per-level cache instead of being re-hashed.
/// The cached value is by definition `node(zero_l, zero_l)`, so the output is the
/// same tree the Solidity reference builds — this is arithmetic sharing, not a
/// different rule.
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

/// The statement's EIP-712 `hashStruct`, with no domain separator — the id the
/// registry records it under.
pub fn statement_hash(s: &LogSetStatement) -> [u8; 32] {
    let mut buf = Vec::with_capacity(9 * 32);
    buf.extend_from_slice(&STATEMENT_TYPEHASH);
    buf.extend_from_slice(&word_u64(s.chain_id));
    buf.extend_from_slice(&word_u64(s.start_block));
    buf.extend_from_slice(&word_u64(s.end_block));
    buf.extend_from_slice(&s.start_parent_hash);
    buf.extend_from_slice(&s.end_block_hash);
    buf.extend_from_slice(&s.filter_hash);
    buf.extend_from_slice(&s.log_set_root);
    buf.extend_from_slice(&word_u64(s.log_count));
    keccak256(&buf)
}

/// `abi.encode(LogSetStatement[])` — the guest's public values.
///
/// `LogSetStatement` is a STATIC struct (every field is a value type), so the
/// array encodes as `offset(0x20) | length | elements inline`, eight words per
/// element in declaration order. Total length is `64 + 256 * n`.
pub fn abi_encode_statements(statements: &[LogSetStatement]) -> Vec<u8> {
    let mut out = Vec::with_capacity(64 + 256 * statements.len());
    out.extend_from_slice(&word_u64(0x20));
    out.extend_from_slice(&word_u64(statements.len() as u64));
    for s in statements {
        out.extend_from_slice(&word_u64(s.chain_id));
        out.extend_from_slice(&word_u64(s.start_block));
        out.extend_from_slice(&word_u64(s.end_block));
        out.extend_from_slice(&s.start_parent_hash);
        out.extend_from_slice(&s.end_block_hash);
        out.extend_from_slice(&s.filter_hash);
        out.extend_from_slice(&s.log_set_root);
        out.extend_from_slice(&word_u64(s.log_count));
    }
    out
}

/// Bytes one statement occupies inside `abi.encode(LogSetStatement[])`.
pub const STATEMENT_WORDS: usize = 8;
/// Bytes the array header occupies (offset word + length word).
pub const ARRAY_HEADER_LEN: usize = 64;
