//! Witness blob v1 — the byte layout the host writes and the guest reads.
//!
//! Hand-rolled little-endian; no serde, no bincode, nothing that can be coaxed
//! into allocating from a length field it has not bounds-checked. Decoding is
//! ZERO-COPY: headers and receipt values are borrowed out of the input, so a
//! 10,000-block blob is not duplicated in guest memory.
//!
//! ```text
//!  off  size  field
//!    0     4  magic u32 LE = 0x574C5331   (bytes 31 53 4c 57)
//!    4     8  chain_id u64 LE
//!   12     8  start_block u64 LE
//!   20     4  n_blocks u32 LE             (>= 1)
//!   24     1  n_filters u8                (>= 1)
//!   25   ...  filters   n_filters x { n_addr u16 LE, 20 B each,
//!                                     4 x { n_topics u16 LE, 32 B each } }
//!        ...  headers   n_blocks x { len u32 LE, header rlp bytes }
//!          4  n_opened u32 LE
//!        ...  opened    n_opened x { index u32 LE, n_receipts u32 LE,
//!                                    n_receipts x { len u32 LE, value bytes } }
//! ```
//!
//! A receipt VALUE is what the receipts trie stores under key `rlp(i)`: for a
//! legacy receipt `rlp(receipt)`, for a typed one `type_byte | rlp(receipt)`.
//!
//! Fail-closed: bad magic, a short read, a byte left over, a non-canonical filter
//! list, an opened index out of range or not strictly ascending, or more receipts
//! than the trie can key all return `Err`.

use crate::error::Error;
use alloc::vec::Vec;

/// `0x574C5331`, read as a LITTLE-ENDIAN `u32`, so the blob's first four bytes are
/// `31 53 4c 57`.
pub const BLOB_MAGIC: u32 = 0x574C_5331;

/// `willow_receipts_trie::build_trie` keys `rlp(0..=65535)` and PANICS above that.
pub const MAX_RECEIPTS: usize = 0x1_0000;

/// A filter in the ERC's canonical form: every list strictly ascending as an
/// unsigned integer, no duplicates. An empty list is a wildcard.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogFilter {
    pub addresses: Vec<[u8; 20]>,
    /// `topics[i]` constrains the log's topic at position `i`.
    pub topics: [Vec<[u8; 32]>; 4],
}

impl LogFilter {
    /// Does `log` match, with the semantics of `eth_getLogs`?
    pub fn matches(&self, emitter: &[u8; 20], topics: &[[u8; 32]]) -> bool {
        if !self.addresses.is_empty() && !self.addresses.iter().any(|a| a == emitter) {
            return false;
        }
        for (i, allowed) in self.topics.iter().enumerate() {
            if allowed.is_empty() {
                continue;
            }
            match topics.get(i) {
                None => return false,
                Some(t) => {
                    if !allowed.iter().any(|a| a == t) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

/// One opened block: the receipts-trie values for `rlp(0..n)`, in transaction order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedBlock<'a> {
    /// Offset from `start_block`.
    pub index: u32,
    pub receipts: Vec<&'a [u8]>,
}

/// The decoded witness. Public so the tests (and a host) can build one, mutate a
/// single field, and re-encode — the shape every adversarial case is one edit away
/// from a passing control.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Witness<'a> {
    pub chain_id: u64,
    pub start_block: u64,
    pub n_blocks: u32,
    pub filters: Vec<LogFilter>,
    /// Header RLP for `start_block + i`, exactly the bytes that get hashed.
    pub headers: Vec<&'a [u8]>,
    /// Strictly ascending by `index`.
    pub opened: Vec<OpenedBlock<'a>>,
}

struct Reader<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize, field: &'static str) -> Result<&'a [u8], Error> {
        let end = self.pos.checked_add(n).ok_or(Error::Truncated {
            field,
            need: n,
            have: 0,
        })?;
        if end > self.buf.len() {
            return Err(Error::Truncated {
                field,
                need: n,
                have: self.buf.len() - self.pos,
            });
        }
        let out = &self.buf[self.pos..end];
        self.pos = end;
        Ok(out)
    }
    fn u8(&mut self, field: &'static str) -> Result<u8, Error> {
        Ok(self.take(1, field)?[0])
    }
    fn u16(&mut self, field: &'static str) -> Result<u16, Error> {
        let b = self.take(2, field)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }
    fn u32(&mut self, field: &'static str) -> Result<u32, Error> {
        let b = self.take(4, field)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }
    fn u64(&mut self, field: &'static str) -> Result<u64, Error> {
        let b = self.take(8, field)?;
        let mut a = [0u8; 8];
        a.copy_from_slice(b);
        Ok(u64::from_le_bytes(a))
    }
    fn blob(&mut self, field: &'static str) -> Result<&'a [u8], Error> {
        let len = self.u32(field)? as usize;
        self.take(len, field)
    }
}

fn ascending<const N: usize>(list: &[[u8; N]], filter: usize, which: u8) -> Result<(), Error> {
    for i in 1..list.len() {
        // Fixed-width big-endian byte strings order the same as the unsigned
        // integers they encode, so lexicographic comparison IS the ERC's rule.
        if list[i] <= list[i - 1] {
            return Err(Error::FilterNotCanonical {
                filter,
                list: which,
                index: i,
            });
        }
    }
    Ok(())
}

/// Decode a v1 blob. Borrows every variable-length field out of `buf`.
pub fn decode_blob(buf: &[u8]) -> Result<Witness<'_>, Error> {
    let mut r = Reader { buf, pos: 0 };
    let magic = r.u32("magic")?;
    if magic != BLOB_MAGIC {
        if magic.swap_bytes() == BLOB_MAGIC {
            return Err(Error::MagicByteOrder);
        }
        return Err(Error::BadMagic { got: magic });
    }
    let chain_id = r.u64("chain_id")?;
    let start_block = r.u64("start_block")?;
    let n_blocks = r.u32("n_blocks")?;
    if n_blocks == 0 {
        return Err(Error::ZeroBlocks);
    }
    // `end_block` has to exist before anything is committed about it.
    start_block
        .checked_add(n_blocks as u64 - 1)
        .ok_or(Error::RangeOverflow)?;

    let n_filters = r.u8("n_filters")?;
    if n_filters == 0 {
        return Err(Error::ZeroFilters);
    }
    let mut filters = Vec::with_capacity(n_filters as usize);
    for fi in 0..n_filters as usize {
        let n_addr = r.u16("filter.n_addr")? as usize;
        let mut addresses = Vec::with_capacity(n_addr.min(1024));
        for _ in 0..n_addr {
            let mut a = [0u8; 20];
            a.copy_from_slice(r.take(20, "filter.address")?);
            addresses.push(a);
        }
        ascending(&addresses, fi, 0)?;
        let mut topics: [Vec<[u8; 32]>; 4] = Default::default();
        for (ti, slot) in topics.iter_mut().enumerate() {
            let n = r.u16("filter.n_topics")? as usize;
            slot.reserve(n.min(1024));
            for _ in 0..n {
                let mut t = [0u8; 32];
                t.copy_from_slice(r.take(32, "filter.topic")?);
                slot.push(t);
            }
            ascending(slot, fi, ti as u8 + 1)?;
        }
        filters.push(LogFilter { addresses, topics });
    }

    let mut headers = Vec::with_capacity((n_blocks as usize).min(1 << 16));
    for _ in 0..n_blocks {
        headers.push(r.blob("header")?);
    }

    let n_opened = r.u32("n_opened")?;
    let mut opened: Vec<OpenedBlock> = Vec::with_capacity((n_opened as usize).min(1 << 16));
    let mut prev: Option<u32> = None;
    for _ in 0..n_opened {
        let index = r.u32("opened.index")?;
        if index >= n_blocks {
            return Err(Error::OpenedIndexOutOfRange { index, n_blocks });
        }
        if let Some(p) = prev {
            if index <= p {
                return Err(Error::OpenedIndexNotAscending {
                    prev: p,
                    got: index,
                });
            }
        }
        prev = Some(index);
        let n_receipts = r.u32("opened.n_receipts")? as usize;
        if n_receipts > MAX_RECEIPTS {
            return Err(Error::TooManyReceipts {
                index,
                n: n_receipts,
            });
        }
        let mut receipts = Vec::with_capacity(n_receipts.min(MAX_RECEIPTS));
        for _ in 0..n_receipts {
            receipts.push(r.blob("opened.receipt")?);
        }
        opened.push(OpenedBlock { index, receipts });
    }

    if r.pos != buf.len() {
        return Err(Error::TrailingBytes {
            extra: buf.len() - r.pos,
        });
    }

    Ok(Witness {
        chain_id,
        start_block,
        n_blocks,
        filters,
        headers,
        opened,
    })
}

/// Encode a witness in the v1 layout. The inverse of [`decode_blob`] for every
/// well-formed witness; the tests round-trip it and then corrupt one field.
pub fn encode_blob(w: &Witness<'_>) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&BLOB_MAGIC.to_le_bytes());
    out.extend_from_slice(&w.chain_id.to_le_bytes());
    out.extend_from_slice(&w.start_block.to_le_bytes());
    out.extend_from_slice(&w.n_blocks.to_le_bytes());
    out.push(w.filters.len() as u8);
    for f in &w.filters {
        out.extend_from_slice(&(f.addresses.len() as u16).to_le_bytes());
        for a in &f.addresses {
            out.extend_from_slice(a);
        }
        for list in &f.topics {
            out.extend_from_slice(&(list.len() as u16).to_le_bytes());
            for t in list {
                out.extend_from_slice(t);
            }
        }
    }
    for h in &w.headers {
        out.extend_from_slice(&(h.len() as u32).to_le_bytes());
        out.extend_from_slice(h);
    }
    out.extend_from_slice(&(w.opened.len() as u32).to_le_bytes());
    for o in &w.opened {
        out.extend_from_slice(&o.index.to_le_bytes());
        out.extend_from_slice(&(o.receipts.len() as u32).to_le_bytes());
        for v in &o.receipts {
            out.extend_from_slice(&(v.len() as u32).to_le_bytes());
            out.extend_from_slice(v);
        }
    }
    out
}
