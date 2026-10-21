//! **Sorted log index** (feature `index`) — every log of the range, committed a
//! second time as a SORTED Merkle list, so that the complete result of a filter
//! is one contiguous, Merkle-provable slice.
//!
//! The default guest commits `logSetRoot`: per-block digests in block order. That
//! answers "what are ALL the logs", but answering "what are all the USDC
//! Transfers" from it means opening every block. The index commits the same logs
//! sorted by a key, so a filter's answer is a run of adjacent leaves plus the two
//! neighbours that prove the run cannot be extended. No new SP1 proof per query.
//!
//! ```text
//!   leaf     = keccak256(INDEX_LEAF_TAG | blockNumber u64 BE | record)
//!              record = the ERC log record, byte-for-byte what logSetRoot hashes:
//!                       txIndex u32 | logIndex u32 | emitter 20 | nTopics u8 | topics | dataLen u32 | data
//!   keyE     = emitter 20 | hasTopic0 u8 | topic0 32 | block u64 | tx u32 | log u32   (69 B)
//!   keyT     = hasTopic0 u8 | topic0 32 | emitter 20 | block u64 | tx u32 | log u32   (69 B)
//!   rootE    = padded Merkle root (INDEX_NODE_TAG) of the leaves in ascending keyE
//!   rootT    = the same leaves in ascending keyT
//!   committed: entryCount, rootE, rootT      (appended after abi.encode(LogSetStatement[]))
//! ```
//!
//! # Where the soundness lives: index multiset == log set
//!
//! The index is only useful if it holds EXACTLY the logs `logSetRoot` covers:
//! no log dropped (a filter answer would silently miss it), none duplicated, none
//! forged. Three facts give that, each enforced in-guest:
//!
//! 1. **One entry per scanned log, from the same bytes.** The guest appends an
//!    entry for every log record it writes into the match-all digest preimage,
//!    and builds both the leaf and the keys from THOSE bytes, never from the
//!    witness directly. The index is only produced under the single match-all
//!    filter (`Error::IndexNeedsMatchAll` otherwise), so every block is opened and
//!    every log is scanned. `entryCount == logCount` is re-checked at the end
//!    (`Error::IndexCountMismatch`).
//! 2. **The sort is a permutation.** Sorting in the zkVM costs O(n log n)
//!    comparisons; checking a host-supplied order costs O(n). So the host sends a
//!    permutation `perm` per ordering (the HINT) and the guest checks it is a
//!    bijection on `0..n` — declared length equals `n`, every index `< n`
//!    (`IndexPermOutOfRange`), no index twice (`IndexPermDuplicate`, a bitmap).
//!    A bijection over the guest's own entry list IS multiset equality; nothing
//!    the host sends can enter the index, it can only choose an order.
//! 3. **The order is the sorted order.** `key(perm[j]) < key(perm[j+1])` STRICTLY
//!    for every `j` (`IndexNotAscending`). Keys end in the log's unique position
//!    `(block, tx, log)`, so strict order is achievable, and a strictly ascending
//!    arrangement of a set is unique — the committed root is a function of the
//!    log set alone, independent of the hint.
//!
//! An empty hint makes the guest sort by itself (same roots, O(n log n)); that
//! mode exists to measure what the hint saves.

use crate::error::Error;
use alloc::vec;
use alloc::vec::Vec;

const fn tag(s: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < s.len() {
        out[i] = s[i];
        i += 1;
    }
    out
}

/// `bytes32("COMPLETE_LOG_INDEX_LEAF_V1")`.
pub const INDEX_LEAF_TAG: [u8; 32] = tag(b"COMPLETE_LOG_INDEX_LEAF_V1");
/// `bytes32("COMPLETE_LOG_INDEX_NODE_V1")`.
pub const INDEX_NODE_TAG: [u8; 32] = tag(b"COMPLETE_LOG_INDEX_NODE_V1");

/// A sort key: nine big-endian `u64` words, compared lexicographically (the
/// derived `Ord`). Word-aligned so the guest builds it without a byte buffer;
/// with the emitter split as `a = em[0..8]`, `b = em[8..16]`, `c = em[16..20]`:
///
/// ```text
/// ByEmitter: a | b | c<<32 | flag  | topic0 (4 words) | block | tx<<32 | log
/// ByTopic0:  topic0 (4 words) | flag<<32 | c | a | b | block | tx<<32 | log
/// ```
/// `flag` = 1 if the log has a topic0, else 0 (and topic0 words = 0), so a
/// zero-topic log never shares a slice with a topic0 of `0x00..00`.
pub type Key = [u64; 9];

/// The two orderings the index commits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {
    /// Emitter first: one slice per (address, topic0) pair, or per address.
    ByEmitter = 0,
    /// Topic0 first: one slice per topic0 for filters with no address.
    ByTopic0 = 1,
}

#[inline(always)]
fn be64(b: &[u8]) -> u64 {
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}

/// Build a key. `topic0 = None` for a log with no topics.
#[inline]
pub fn make_key(
    order: Order,
    emitter: &[u8; 20],
    topic0: Option<&[u8; 32]>,
    block: u64,
    tx: u32,
    log: u32,
) -> Key {
    let a = be64(&emitter[0..8]);
    let b = be64(&emitter[8..16]);
    let c = u32::from_be_bytes([emitter[16], emitter[17], emitter[18], emitter[19]]) as u64;
    let (flag, t) = match topic0 {
        Some(t) => (
            1u64,
            [
                be64(&t[0..8]),
                be64(&t[8..16]),
                be64(&t[16..24]),
                be64(&t[24..32]),
            ],
        ),
        None => (0u64, [0u64; 4]),
    };
    let pos = ((tx as u64) << 32) | log as u64;
    match order {
        Order::ByEmitter => [a, b, (c << 32) | flag, t[0], t[1], t[2], t[3], block, pos],
        Order::ByTopic0 => [t[0], t[1], t[2], t[3], (flag << 32) | c, a, b, block, pos],
    }
}

/// The `ByTopic0` key rearranged from the `ByEmitter` one (same words, no bytes).
#[inline(always)]
pub fn topic0_key_from(e: &Key) -> Key {
    let (c, flag) = (e[2] >> 32, e[2] & 0xffff_ffff);
    [
        e[3],
        e[4],
        e[5],
        e[6],
        (flag << 32) | c,
        e[0],
        e[1],
        e[7],
        e[8],
    ]
}

/// The smallest and largest key a filter slice can hold.
pub fn key_range(order: Order, emitter: &[u8; 20], topic0: Option<&[u8; 32]>) -> (Key, Key) {
    (
        make_key(order, emitter, topic0, 0, 0, 0),
        make_key(order, emitter, topic0, u64::MAX, u32::MAX, u32::MAX),
    )
}

/// The fields of one ERC log record the keys need, read from the record bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordHead {
    pub tx: u32,
    pub log: u32,
    pub emitter: [u8; 20],
    pub topic0: Option<[u8; 32]>,
}

fn be_u32(b: &[u8]) -> u32 {
    let mut x = [0u8; 4];
    x.copy_from_slice(&b[..4]);
    u32::from_be_bytes(x)
}

/// Parse and fully validate one ERC log record (fixed fields, nTopics <= 4, and
/// `dataLen` matching the remaining length exactly). The guest only parses
/// records it serialized itself; a query VERIFIER parses untrusted ones.
pub fn parse_record(r: &[u8]) -> Result<RecordHead, Error> {
    const FIXED: usize = 4 + 4 + 20 + 1;
    if r.len() < FIXED + 4 {
        return Err(Error::IndexRecordMalformed);
    }
    let n_topics = r[28] as usize;
    if n_topics > crate::erc::MAX_TOPICS {
        return Err(Error::IndexRecordMalformed);
    }
    let data_len_off = FIXED + 32 * n_topics;
    if r.len() < data_len_off + 4 {
        return Err(Error::IndexRecordMalformed);
    }
    let data_len = be_u32(&r[data_len_off..]) as usize;
    if r.len() - (data_len_off + 4) != data_len {
        return Err(Error::IndexRecordMalformed);
    }
    let mut emitter = [0u8; 20];
    emitter.copy_from_slice(&r[8..28]);
    let topic0 = if n_topics > 0 {
        let mut t = [0u8; 32];
        t.copy_from_slice(&r[29..61]);
        Some(t)
    } else {
        None
    };
    Ok(RecordHead {
        tx: be_u32(&r[0..]),
        log: be_u32(&r[4..]),
        emitter,
        topic0,
    })
}

/// keccak256 with a lane-wise absorb around `tiny_keccak::keccakf` (the SP1
/// `keccak_permute` precompile in the zkVM). `tiny_keccak::Keccak` XORs input
/// byte by byte; this XORs whole 8-byte lanes. Tested equal to it.
pub struct Keccak256 {
    s: [u64; 25],
    pos: usize,
}

const RATE: usize = 136;

impl Keccak256 {
    #[inline(always)]
    pub fn new() -> Self {
        Self {
            s: [0u64; 25],
            pos: 0,
        }
    }
    #[inline(always)]
    fn byte(&mut self, x: u8) {
        self.s[self.pos >> 3] ^= (x as u64) << (8 * (self.pos & 7));
        self.pos += 1;
        if self.pos == RATE {
            tiny_keccak::keccakf(&mut self.s);
            self.pos = 0;
        }
    }
    pub fn update(&mut self, mut data: &[u8]) {
        while self.pos & 7 != 0 && !data.is_empty() {
            self.byte(data[0]);
            data = &data[1..];
        }
        while data.len() >= 8 {
            let lane = u64::from_le_bytes([
                data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
            ]);
            self.s[self.pos >> 3] ^= lane;
            self.pos += 8;
            if self.pos == RATE {
                tiny_keccak::keccakf(&mut self.s);
                self.pos = 0;
            }
            data = &data[8..];
        }
        for &x in data {
            self.byte(x);
        }
    }
    pub fn finalize(mut self) -> [u8; 32] {
        self.s[self.pos >> 3] ^= 0x01u64 << (8 * (self.pos & 7));
        self.s[(RATE - 1) >> 3] ^= 0x80u64 << 56;
        tiny_keccak::keccakf(&mut self.s);
        let mut out = [0u8; 32];
        for i in 0..4 {
            out[8 * i..8 * i + 8].copy_from_slice(&self.s[i].to_le_bytes());
        }
        out
    }
}

impl Default for Keccak256 {
    fn default() -> Self {
        Self::new()
    }
}

/// `keccak256(INDEX_LEAF_TAG | block u64 BE | record)`, streamed (no copy).
pub fn leaf_hash(block: u64, record: &[u8]) -> [u8; 32] {
    let mut k = Keccak256::new();
    k.update(&INDEX_LEAF_TAG);
    k.update(&block.to_be_bytes());
    k.update(record);
    k.finalize()
}

/// An interior node of an index tree: `keccak256(INDEX_NODE_TAG | left | right)`,
/// 96 bytes = 12 whole lanes, one permutation.
#[inline]
pub fn index_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut s = [0u64; 25];
    for i in 0..4 {
        s[i] = u64::from_le_bytes([
            INDEX_NODE_TAG[8 * i],
            INDEX_NODE_TAG[8 * i + 1],
            INDEX_NODE_TAG[8 * i + 2],
            INDEX_NODE_TAG[8 * i + 3],
            INDEX_NODE_TAG[8 * i + 4],
            INDEX_NODE_TAG[8 * i + 5],
            INDEX_NODE_TAG[8 * i + 6],
            INDEX_NODE_TAG[8 * i + 7],
        ]);
        s[4 + i] = u64::from_le_bytes(left[8 * i..8 * i + 8].try_into().unwrap());
        s[8 + i] = u64::from_le_bytes(right[8 * i..8 * i + 8].try_into().unwrap());
    }
    s[12] ^= 0x01;
    s[16] ^= 0x80u64 << 56;
    tiny_keccak::keccakf(&mut s);
    let mut out = [0u8; 32];
    for i in 0..4 {
        out[8 * i..8 * i + 8].copy_from_slice(&s[i].to_le_bytes());
    }
    out
}

/// Root of the padded tree over `leaves` (the ERC tree's shape with the index
/// node tag): next power of two, zero-valued padding leaves, every interior
/// position hashed (all-zero subtrees from a per-level cache). `n = 1` ⇒ the
/// leaf; `n = 0` ⇒ `0x00..00` (a range with no logs at all).
pub fn index_root(leaves: &[[u8; 32]]) -> [u8; 32] {
    let n = leaves.len();
    if n == 0 {
        return [0u8; 32];
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
        let next_zero = index_node(&zero, &zero);
        for i in 0..cap {
            let (l, r) = (level[2 * i], level[2 * i + 1]);
            level[i] = if l == zero && r == zero {
                next_zero
            } else {
                index_node(&l, &r)
            };
        }
        zero = next_zero;
        level.truncate(cap);
    }
    level[0]
}

/// One index entry: both keys and the leaf.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub key_e: Key,
    pub key_t: Key,
    pub leaf: [u8; 32],
}

impl Entry {
    #[inline(always)]
    pub fn key(&self, o: Order) -> &Key {
        match o {
            Order::ByEmitter => &self.key_e,
            Order::ByTopic0 => &self.key_t,
        }
    }
}

/// What the index guest commits after the statements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexCommitment {
    pub entry_count: u64,
    pub root_by_emitter: [u8; 32],
    pub root_by_topic0: [u8; 32],
}

impl IndexCommitment {
    /// Three ABI words: `uint64 entryCount | bytes32 rootE | bytes32 rootT`.
    pub fn encode(&self) -> [u8; 96] {
        let mut out = [0u8; 96];
        out[24..32].copy_from_slice(&self.entry_count.to_be_bytes());
        out[32..64].copy_from_slice(&self.root_by_emitter);
        out[64..96].copy_from_slice(&self.root_by_topic0);
        out
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if b.len() != 96 || b[..24].iter().any(|x| *x != 0) {
            return None;
        }
        let mut c = [0u8; 8];
        c.copy_from_slice(&b[24..32]);
        let mut e = [0u8; 32];
        e.copy_from_slice(&b[32..64]);
        let mut t = [0u8; 32];
        t.copy_from_slice(&b[64..96]);
        Some(Self {
            entry_count: u64::from_be_bytes(c),
            root_by_emitter: e,
            root_by_topic0: t,
        })
    }
    pub fn root(&self, o: Order) -> &[u8; 32] {
        match o {
            Order::ByEmitter => &self.root_by_emitter,
            Order::ByTopic0 => &self.root_by_topic0,
        }
    }
}

/// Accumulates entries during the scan. `keep_records` is a HOST-side option
/// (query demo); the guest leaves it off.
#[derive(Default)]
pub struct IndexBuilder {
    pub entries: Vec<Entry>,
    pub keep_records: bool,
    /// `(block, record bytes)` in scan order, only when `keep_records`.
    pub records: Vec<(u64, Vec<u8>)>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// The keys of every record of one opened block. `buf` is the block's
    /// match-all digest preimage and `starts[j]` the offset of record `j` in it;
    /// record `j` ends where `j+1` starts (the last one at `buf.len()`).
    pub fn push_keys(&mut self, number: u64, buf: &[u8], starts: &[u32]) -> Result<(), Error> {
        self.entries.reserve(starts.len());
        for j in 0..starts.len() {
            let r = record_at(buf, starts, j);
            let h = parse_record(r)?;
            let key_e = make_key(
                Order::ByEmitter,
                &h.emitter,
                h.topic0.as_ref(),
                number,
                h.tx,
                h.log,
            );
            self.entries.push(Entry {
                key_t: topic0_key_from(&key_e),
                key_e,
                leaf: [0u8; 32],
            });
            if self.keep_records {
                self.records.push((number, r.to_vec()));
            }
        }
        Ok(())
    }

    /// Fill the leaves of the block's entries just pushed by [`push_keys`].
    pub fn fill_leaves(&mut self, number: u64, buf: &[u8], starts: &[u32]) {
        let base = self.entries.len() - starts.len();
        for j in 0..starts.len() {
            self.entries[base + j].leaf = leaf_hash(number, record_at(buf, starts, j));
        }
    }

    /// Verify (or, with an empty hint, compute) both orderings and commit.
    pub fn finish(&self, hint: &[u8]) -> Result<IndexCommitment, Error> {
        let n = self.entries.len();
        span_start!("idx_hint");
        let perms = parse_hint(hint, n)?;
        span_end!("idx_hint");
        let (root_e, root_t) = match perms {
            Some((pe, pt)) => {
                span_start!("idx_perm_e");
                let le = check_perm(&self.entries, pe, Order::ByEmitter)?;
                span_end!("idx_perm_e");
                span_start!("idx_tree_e");
                let re = index_root(&le);
                span_end!("idx_tree_e");
                drop(le);
                span_start!("idx_perm_t");
                let lt = check_perm(&self.entries, pt, Order::ByTopic0)?;
                span_end!("idx_perm_t");
                span_start!("idx_tree_t");
                let rt = index_root(&lt);
                span_end!("idx_tree_t");
                (re, rt)
            }
            None => {
                span_start!("idx_sort_e");
                let le = guest_sort(&self.entries, Order::ByEmitter)?;
                span_end!("idx_sort_e");
                span_start!("idx_tree_e");
                let re = index_root(&le);
                span_end!("idx_tree_e");
                drop(le);
                span_start!("idx_sort_t");
                let lt = guest_sort(&self.entries, Order::ByTopic0)?;
                span_end!("idx_sort_t");
                span_start!("idx_tree_t");
                let rt = index_root(&lt);
                span_end!("idx_tree_t");
                (re, rt)
            }
        };
        Ok(IndexCommitment {
            entry_count: n as u64,
            root_by_emitter: root_e,
            root_by_topic0: root_t,
        })
    }
}

fn record_at<'b>(buf: &'b [u8], starts: &[u32], j: usize) -> &'b [u8] {
    let s = starts[j] as usize;
    let e = if j + 1 < starts.len() {
        starts[j + 1] as usize
    } else {
        buf.len()
    };
    &buf[s..e]
}

/// Hint layout: empty ⇒ the guest sorts. Otherwise
/// `mode u8 = 1 | n u32 LE | permE n × u32 LE | permT n × u32 LE`, exact length.
pub const HINT_MODE_PERM: u8 = 1;

/// The two permutation byte strings (`ByEmitter`, `ByTopic0`).
type Perms<'a> = (&'a [u8], &'a [u8]);

fn parse_hint(hint: &[u8], n: usize) -> Result<Option<Perms<'_>>, Error> {
    if hint.is_empty() {
        return Ok(None);
    }
    if hint[0] != HINT_MODE_PERM {
        return Err(Error::IndexHintMalformed { what: "mode" });
    }
    if hint.len() < 5 {
        return Err(Error::IndexHintMalformed { what: "short" });
    }
    let declared = be_or_le_u32(&hint[1..5]) as u64;
    if declared != n as u64 {
        return Err(Error::IndexHintCount {
            declared,
            entries: n as u64,
        });
    }
    let need = 5 + 8 * n;
    if hint.len() != need {
        return Err(Error::IndexHintMalformed { what: "length" });
    }
    let body = &hint[5..];
    Ok(Some((&body[..4 * n], &body[4 * n..])))
}

#[inline(always)]
fn be_or_le_u32(b: &[u8]) -> u32 {
    u32::from_le_bytes([b[0], b[1], b[2], b[3]])
}

/// The permutation check: `perm` (n × u32 LE) must be a bijection on `0..n` and
/// list the entries in STRICTLY ascending key order. Returns the leaves in that
/// order. `perm.len() == 4n` is guaranteed by [`parse_hint`].
pub fn check_perm(entries: &[Entry], perm: &[u8], o: Order) -> Result<Vec<[u8; 32]>, Error> {
    let n = entries.len();
    if perm.len() != 4 * n {
        return Err(Error::IndexHintMalformed {
            what: "perm length",
        });
    }
    let mut seen = vec![0u64; n.div_ceil(64)];
    let mut leaves = Vec::with_capacity(n);
    let mut prev: Option<&Key> = None;
    for j in 0..n {
        let p = be_or_le_u32(&perm[4 * j..]) as usize;
        if p >= n {
            return Err(Error::IndexPermOutOfRange {
                order: o as u8,
                at: j as u32,
            });
        }
        let (w, bit) = (p >> 6, 1u64 << (p & 63));
        if seen[w] & bit != 0 {
            return Err(Error::IndexPermDuplicate {
                order: o as u8,
                at: j as u32,
            });
        }
        seen[w] |= bit;
        let e = &entries[p];
        let k = e.key(o);
        if let Some(pk) = prev {
            if pk >= k {
                return Err(Error::IndexNotAscending {
                    order: o as u8,
                    at: j as u32,
                });
            }
        }
        prev = Some(k);
        leaves.push(e.leaf);
    }
    Ok(leaves)
}

/// The no-hint path: sort in the guest. Same output as a valid hint.
pub fn guest_sort(entries: &[Entry], o: Order) -> Result<Vec<[u8; 32]>, Error> {
    let perm = sorted_perm(entries, o);
    let mut leaves = Vec::with_capacity(entries.len());
    for j in 0..perm.len() {
        if j > 0 && entries[perm[j - 1] as usize].key(o) >= entries[perm[j] as usize].key(o) {
            return Err(Error::IndexNotAscending {
                order: o as u8,
                at: j as u32,
            });
        }
        leaves.push(entries[perm[j] as usize].leaf);
    }
    Ok(leaves)
}

/// The sorting permutation (host: to build the hint; guest: the no-hint path).
pub fn sorted_perm(entries: &[Entry], o: Order) -> Vec<u32> {
    let mut perm: Vec<u32> = (0..entries.len() as u32).collect();
    perm.sort_unstable_by(|&a, &b| entries[a as usize].key(o).cmp(entries[b as usize].key(o)));
    perm
}

/// HOST: the hint for `entries`.
pub fn build_hint(entries: &[Entry]) -> Vec<u8> {
    let n = entries.len();
    let mut h = Vec::with_capacity(5 + 8 * n);
    h.push(HINT_MODE_PERM);
    h.extend_from_slice(&(n as u32).to_le_bytes());
    for o in [Order::ByEmitter, Order::ByTopic0] {
        for p in sorted_perm(entries, o) {
            h.extend_from_slice(&p.to_le_bytes());
        }
    }
    h
}

// ===========================================================================
// Queries: a filter's complete result as one contiguous slice + a proof
// ===========================================================================

/// A contiguous run of leaves `lo ..= lo + records.len() - 1` of one committed
/// ordering, given as full records (the verifier re-hashes them), plus the
/// Merkle siblings to the root. Siblings that are all-padding subtrees are
/// omitted: the verifier derives them from `entryCount`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeProof {
    pub lo: u64,
    pub records: Vec<(u64, Vec<u8>)>,
    pub siblings: Vec<[u8; 32]>,
}

impl RangeProof {
    /// Bytes on the wire: 8 (lo) + per record (8 block + 4 len + record) + 32 per sibling.
    pub fn wire_size(&self) -> usize {
        8 + self
            .records
            .iter()
            .map(|(_, r)| 12 + r.len())
            .sum::<usize>()
            + 32 * self.siblings.len()
    }
}

fn cap_of(n: u64) -> u64 {
    let mut cap = 1u64;
    while cap < n {
        cap <<= 1;
    }
    cap
}

/// HOST: all tree levels over `leaves` (level 0 = padded leaves).
pub fn tree_levels(leaves: &[[u8; 32]]) -> Vec<Vec<[u8; 32]>> {
    let n = leaves.len();
    let cap = cap_of(n as u64) as usize;
    let mut lv = leaves.to_vec();
    lv.resize(cap, [0u8; 32]);
    let mut out = vec![lv];
    let mut zero = [0u8; 32];
    while out.last().unwrap().len() > 1 {
        let prev = out.last().unwrap();
        let next_zero = index_node(&zero, &zero);
        let next: Vec<[u8; 32]> = prev
            .chunks(2)
            .map(|c| {
                if c[0] == zero && c[1] == zero {
                    next_zero
                } else {
                    index_node(&c[0], &c[1])
                }
            })
            .collect();
        zero = next_zero;
        out.push(next);
    }
    out
}

/// HOST: the siblings for positions `lo..=hi`, in the order [`verify_range`]
/// consumes them (per level: left sibling if needed, then right sibling if
/// needed and not all-padding).
pub fn range_siblings(levels: &[Vec<[u8; 32]>], n: u64, lo: u64, hi: u64) -> Vec<[u8; 32]> {
    let mut out = Vec::new();
    let (mut a, mut b) = (lo, hi);
    for (l, lv) in levels.iter().enumerate() {
        if lv.len() == 1 {
            break;
        }
        if a & 1 == 1 {
            out.push(lv[(a - 1) as usize]);
        }
        if b & 1 == 0 {
            let s = b + 1;
            if (s << l) < n {
                out.push(lv[s as usize]);
            }
        }
        a >>= 1;
        b >>= 1;
    }
    out
}

/// Verify a range proof for keys in `[kmin, kmax]` against a committed ordering.
/// Returns the matching records (boundary neighbours stripped). The result is
/// COMPLETE: the committed ordering is strictly sorted (proved in-guest), the
/// proof shows a contiguous run of positions, and each end is either the end of
/// the list or a neighbour whose key lies outside the range.
pub fn verify_range(
    root: &[u8; 32],
    n: u64,
    order: Order,
    kmin: &Key,
    kmax: &Key,
    proof: &RangeProof,
) -> Result<Vec<(u64, Vec<u8>)>, Error> {
    let bad = |what| Error::IndexProof { what };
    if kmin > kmax {
        return Err(bad("empty key range"));
    }
    if n == 0 {
        if !proof.records.is_empty() || !proof.siblings.is_empty() || *root != [0u8; 32] {
            return Err(bad("n = 0 needs an empty proof and a zero root"));
        }
        return Ok(Vec::new());
    }
    let m = proof.records.len() as u64;
    if m == 0 {
        return Err(bad("no records"));
    }
    let lo = proof.lo;
    let hi = lo.checked_add(m - 1).ok_or(bad("overflow"))?;
    if hi >= n {
        return Err(bad("range past entryCount"));
    }
    let mut keys = Vec::with_capacity(m as usize);
    let mut leaves = Vec::with_capacity(m as usize);
    for (block, r) in &proof.records {
        let h = parse_record(r)?;
        keys.push(make_key(
            order,
            &h.emitter,
            h.topic0.as_ref(),
            *block,
            h.tx,
            h.log,
        ));
        leaves.push(leaf_hash(*block, r));
    }
    for j in 1..keys.len() {
        if keys[j - 1] >= keys[j] {
            return Err(bad("records not strictly ascending"));
        }
    }
    // Each end of the run is either a neighbour whose key lies outside the
    // range (stripped) or the end of the committed list.
    let start = if keys[0] < *kmin {
        1
    } else if lo == 0 {
        0
    } else {
        return Err(bad("run does not start at a left neighbour or position 0"));
    };
    let end = if keys[m as usize - 1] > *kmax {
        m as usize - 1
    } else if hi == n - 1 {
        m as usize
    } else {
        return Err(bad("run does not end at a right neighbour or position n-1"));
    };
    if start > end {
        return Err(bad("one record cannot be both neighbours"));
    }
    for k in &keys[start..end] {
        if k < kmin || k > kmax {
            return Err(bad("interior record outside the range"));
        }
    }
    // Merkle: fold the contiguous run up to the root.
    let cap = cap_of(n);
    let mut sib = proof.siblings.iter();
    let (mut a, mut b) = (lo, hi);
    let mut cur = leaves;
    let mut level = 0usize;
    let mut width = cap;
    let mut zero = [0u8; 32];
    while width > 1 {
        let mut row: Vec<[u8; 32]> = Vec::with_capacity(cur.len() + 2);
        if a & 1 == 1 {
            row.push(*sib.next().ok_or(bad("missing left sibling"))?);
        }
        row.extend_from_slice(&cur);
        if b & 1 == 0 {
            let s = b + 1;
            if (s << level) < n {
                row.push(*sib.next().ok_or(bad("missing right sibling"))?);
            } else {
                row.push(zero);
            }
        }
        let next_zero = index_node(&zero, &zero);
        cur = row
            .chunks(2)
            .map(|c| {
                if c[0] == zero && c[1] == zero {
                    next_zero
                } else {
                    index_node(&c[0], &c[1])
                }
            })
            .collect();
        zero = next_zero;
        a >>= 1;
        b >>= 1;
        level += 1;
        width >>= 1;
    }
    if sib.next().is_some() {
        return Err(bad("unused siblings"));
    }
    if cur.len() != 1 || cur[0] != *root {
        return Err(bad("root mismatch"));
    }
    Ok(proof.records[start..end].to_vec())
}

/// HOST: the proof of the slice `[kmin, kmax]` over one ordering, given the
/// ordered keys, the ordered records and the tree levels.
pub fn prove_range(
    keys: &[Key],
    records: &[(u64, Vec<u8>)],
    levels: &[Vec<[u8; 32]>],
    kmin: &Key,
    kmax: &Key,
) -> RangeProof {
    let n = keys.len() as u64;
    if n == 0 {
        return RangeProof {
            lo: 0,
            records: Vec::new(),
            siblings: Vec::new(),
        };
    }
    let i = keys.partition_point(|k| k < kmin) as u64; // first >= kmin
    let j = keys.partition_point(|k| k <= kmax) as u64; // first > kmax
    let lo = i.saturating_sub(1);
    let hi = if j < n { j } else { n - 1 };
    let lo = if i == 0 { 0 } else { lo };
    RangeProof {
        lo,
        records: records[lo as usize..=hi as usize].to_vec(),
        siblings: range_siblings(levels, n, lo, hi),
    }
}

// ===========================================================================
// The index guest's orchestration
// ===========================================================================
//
// There is ONE orchestration body, `crate::scan`. The index guest runs it with
// an `IndexBuilder` as the sink; the default guest with `crate::NoSink`. (Until
// the tip-guest freeze this was a verbatim copy of `run_statements` with hooks
// added, kept so the default ELF's vkey would not move; the freeze collapsed it.)

use crate::erc::{abi_encode_statements, LogSetStatement};
use crate::{LogSink, NoSink};

impl LogSink for IndexBuilder {
    const ACTIVE: bool = true;
    fn admit(filters: &[crate::LogFilter]) -> Result<(), Error> {
        let f = &filters[0];
        if filters.len() != 1 || !f.addresses.is_empty() || f.topics.iter().any(|t| !t.is_empty()) {
            return Err(Error::IndexNeedsMatchAll);
        }
        Ok(())
    }
    fn block(&mut self, number: u64, buf: &[u8], starts: &[u32]) -> Result<(), Error> {
        span_start!("idx_key");
        self.push_keys(number, buf, starts)?;
        span_end!("idx_key");
        span_start!("idx_leaf");
        self.fill_leaves(number, buf, starts);
        span_end!("idx_leaf");
        Ok(())
    }
}

/// The index guest: [`crate::run`] plus the sorted index, verified against `hint`
/// (module docs). Public values = `abi.encode(LogSetStatement[])` followed by
/// the 96-byte [`IndexCommitment`].
pub fn run_indexed(blob: &[u8], hint: &[u8]) -> Result<Vec<u8>, Error> {
    let (statements, com) = run_statements_indexed(blob, hint, &mut IndexBuilder::new())?;
    span_start!("commit");
    let mut pv = abi_encode_statements(&statements);
    pv.extend_from_slice(&com.encode());
    span_end!("commit");
    Ok(pv)
}

/// [`run_indexed`] before encoding; `ix` comes back holding the entries (and the
/// records, if the host asked for them).
pub fn run_statements_indexed(
    blob: &[u8],
    hint: &[u8],
    ix: &mut IndexBuilder,
) -> Result<(Vec<LogSetStatement>, IndexCommitment), Error> {
    let statements = crate::scan(blob, &mut *ix)?;
    // Binding check: one entry per log of the match-all statement.
    if ix.entries.len() as u64 != statements[0].log_count {
        return Err(Error::IndexCountMismatch {
            entries: ix.entries.len() as u64,
            log_count: statements[0].log_count,
        });
    }
    let com = ix.finish(hint)?;
    Ok((statements, com))
}

/// The scan with an optional index sink (host tools and tests). `None` is
/// exactly [`crate::run_statements`].
pub fn run_statements_core(
    blob: &[u8],
    idx: Option<&mut IndexBuilder>,
) -> Result<Vec<LogSetStatement>, Error> {
    match idx {
        Some(ix) => crate::scan(blob, ix),
        None => crate::scan(blob, &mut NoSink),
    }
}
