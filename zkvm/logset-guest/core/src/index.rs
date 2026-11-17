

















































use crate::error::Error;

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


pub const INDEX_LEAF_TAG: [u8; 32] = tag(b"COMPLETE_LOG_INDEX_LEAF_V1");

pub const INDEX_NODE_TAG: [u8; 32] = tag(b"COMPLETE_LOG_INDEX_NODE_V1");











pub type Key = [u64; 9];


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Order {

    ByEmitter = 0,

    ByTopic0 = 1,
}

#[inline(always)]
fn be64(b: &[u8]) -> u64 {
    u64::from_be_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
}


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


pub fn leaf_hash(block: u64, record: &[u8]) -> [u8; 32] {
    let mut k = Keccak256::new();
    k.update(&INDEX_LEAF_TAG);
    k.update(&block.to_be_bytes());
    k.update(record);
    k.finalize()
}



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






















































#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IndexCommitment {
    pub entry_count: u64,
    pub root_by_emitter: [u8; 32],
    pub root_by_topic0: [u8; 32],
}

impl IndexCommitment {








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













































































































































































































































#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RangeProof {
    pub lo: u64,
    pub records: Vec<(u64, Vec<u8>)>,
    pub siblings: Vec<[u8; 32]>,
}













fn cap_of(n: u64) -> u64 {
    let mut cap = 1u64;
    while cap < n {
        cap <<= 1;
    }
    cap
}


























































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








































































































