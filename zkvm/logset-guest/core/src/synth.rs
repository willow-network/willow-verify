//! Synthetic fixture builder — real-shaped headers and receipts, built natively.
//!
//! Off by default so the guest ELF never contains it (`cargo test` turns it on
//! via `cfg(any(test, feature = "synth"))`; `execrun` asks for it by feature to
//! generate the cycles-per-header and cycles-per-opened-block blobs).
//!
//! Everything here is the HONEST encoder: headers carry the 21-field Prague
//! shape, receipt values carry the EIP-2718 type byte when typed, `logsBloom` is
//! the real OR of the receipts' blooms, and `receiptsRoot` is
//! `willow_receipts_trie::build_trie(values).root`. An adversarial test builds an
//! honest witness, edits one field, and re-encodes.

use crate::blob::{encode_blob, LogFilter, OpenedBlock, Witness};
use crate::bloom::{probe, FilterProbes};
use crate::keccak256;
use alloc::vec;
use alloc::vec::Vec;

// ---------------------------------------------------------------------------
// RLP encoding
// ---------------------------------------------------------------------------

pub fn rlp_str(bytes: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(bytes.len() + 9);
    if bytes.len() == 1 && bytes[0] <= 0x7f {
        out.push(bytes[0]);
    } else if bytes.len() <= 55 {
        out.push(0x80 + bytes.len() as u8);
        out.extend_from_slice(bytes);
    } else {
        let le = bytes.len().to_be_bytes();
        let first = le.iter().position(|&b| b != 0).unwrap();
        out.push(0xb7 + (le.len() - first) as u8);
        out.extend_from_slice(&le[first..]);
        out.extend_from_slice(bytes);
    }
    out
}

pub fn rlp_list_of(items: &[Vec<u8>]) -> Vec<u8> {
    let payload: Vec<u8> = items.concat();
    let mut out = Vec::with_capacity(payload.len() + 9);
    if payload.len() <= 55 {
        out.push(0xc0 + payload.len() as u8);
    } else {
        let le = payload.len().to_be_bytes();
        let first = le.iter().position(|&b| b != 0).unwrap();
        out.push(0xf7 + (le.len() - first) as u8);
        out.extend_from_slice(&le[first..]);
    }
    out.extend_from_slice(&payload);
    out
}

/// Minimal-length big-endian, the RLP integer form. Zero is the empty string.
pub fn rlp_u64(v: u64) -> Vec<u8> {
    if v == 0 {
        return vec![0x80];
    }
    let be = v.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap();
    rlp_str(&be[first..])
}

// ---------------------------------------------------------------------------
// blooms
// ---------------------------------------------------------------------------

/// Set a value's three bits in a bloom filter.
pub fn bloom_add(bloom: &mut [u8; 256], value: &[u8]) {
    for idx in probe(value).bits {
        bloom[255 - (idx >> 3) as usize] |= 1u8 << (idx & 7);
    }
}

// ---------------------------------------------------------------------------
// receipts
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct SynthLog {
    pub emitter: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

impl SynthLog {
    pub fn rlp(&self) -> Vec<u8> {
        let topics: Vec<Vec<u8>> = self.topics.iter().map(|t| rlp_str(t)).collect();
        rlp_list_of(&[
            rlp_str(&self.emitter),
            rlp_list_of(&topics),
            rlp_str(&self.data),
        ])
    }
    pub fn bloom_into(&self, bloom: &mut [u8; 256]) {
        bloom_add(bloom, &self.emitter);
        for t in &self.topics {
            bloom_add(bloom, t);
        }
    }
}

#[derive(Clone, Debug)]
pub struct SynthReceipt {
    /// `0` = legacy (no type byte); `1..=4` = EIP-2718 typed; `0x7E` = OP deposit.
    pub tx_type: u8,
    pub status: bool,
    pub cum_gas: u64,
    pub logs: Vec<SynthLog>,
    /// Items appended after `logs`: a deposit's `[depositNonce, depositReceiptVersion]`.
    pub deposit_tail: Vec<u64>,
}

impl SynthReceipt {
    pub fn bloom(&self) -> [u8; 256] {
        let mut b = [0u8; 256];
        for l in &self.logs {
            l.bloom_into(&mut b);
        }
        b
    }
    /// The trie VALUE for key `rlp(i)`.
    pub fn value(&self) -> Vec<u8> {
        let logs: Vec<Vec<u8>> = self.logs.iter().map(|l| l.rlp()).collect();
        let mut items = vec![
            rlp_u64(self.status as u64),
            rlp_u64(self.cum_gas),
            rlp_str(&self.bloom()),
            rlp_list_of(&logs),
        ];
        items.extend(self.deposit_tail.iter().map(|&v| rlp_u64(v)));
        let body = rlp_list_of(&items);
        if self.tx_type == 0 {
            body
        } else {
            let mut v = Vec::with_capacity(body.len() + 1);
            v.push(self.tx_type);
            v.extend_from_slice(&body);
            v
        }
    }
}

// ---------------------------------------------------------------------------
// headers
// ---------------------------------------------------------------------------

/// A 21-field post-Prague header: the real shape, so the hashed byte count and
/// the RLP walk cost are the ones mainnet pays.
pub fn encode_header(
    parent_hash: &[u8; 32],
    number: u64,
    receipts_root: &[u8; 32],
    logs_bloom: &[u8; 256],
) -> Vec<u8> {
    let f = |b: &[u8]| rlp_str(b);
    rlp_list_of(&[
        f(parent_hash),
        f(&[0x1du8; 32]), // ommersHash
        f(&[0xaau8; 20]), // beneficiary
        f(&[0x33u8; 32]), // stateRoot
        f(&[0x44u8; 32]), // transactionsRoot
        f(receipts_root),
        f(logs_bloom),
        rlp_u64(0),                      // difficulty
        rlp_u64(number),                 //
        rlp_u64(36_000_000),             // gasLimit
        rlp_u64(17_123_456),             // gasUsed
        rlp_u64(1_760_000_000 + number), // timestamp
        f(b"willow-logset-m1"),          // extraData
        f(&[0x55u8; 32]),                // mixHash
        f(&[0u8; 8]),                    // nonce
        rlp_u64(3_141_592_653),          // baseFeePerGas
        f(&[0x66u8; 32]),                // withdrawalsRoot
        rlp_u64(786_432),                // blobGasUsed
        rlp_u64(0),                      // excessBlobGas
        f(&[0x77u8; 32]),                // parentBeaconBlockRoot
        f(&[0x88u8; 32]),                // requestsHash
    ])
}

// ---------------------------------------------------------------------------
// witness assembly
// ---------------------------------------------------------------------------

/// One synthetic block. The overrides exist so a lean-range fixture can carry a
/// realistically DENSE bloom without paying to build the receipts behind it —
/// a lean block's receipts are never witnessed and its `receiptsRoot` is never
/// read, so nothing about the measurement is faked by supplying them directly.
#[derive(Clone, Debug, Default)]
pub struct SynthBlock {
    pub receipts: Vec<SynthReceipt>,
    pub bloom_override: Option<[u8; 256]>,
    pub receipts_root_override: Option<[u8; 32]>,
}

/// An owned mirror of [`Witness`] a test can mutate field by field.
#[derive(Clone, Debug)]
pub struct OwnedWitness {
    pub chain_id: u64,
    pub start_block: u64,
    pub n_blocks: u32,
    pub filters: Vec<LogFilter>,
    pub headers: Vec<Vec<u8>>,
    pub opened: Vec<(u32, Vec<Vec<u8>>)>,
}

impl OwnedWitness {
    pub fn view(&self) -> Witness<'_> {
        Witness {
            chain_id: self.chain_id,
            start_block: self.start_block,
            n_blocks: self.n_blocks,
            filters: self.filters.clone(),
            headers: self.headers.iter().map(|h| h.as_slice()).collect(),
            opened: self
                .opened
                .iter()
                .map(|(i, rs)| OpenedBlock {
                    index: *i,
                    receipts: rs.iter().map(|r| r.as_slice()).collect(),
                })
                .collect(),
        }
    }
    pub fn encode(&self) -> Vec<u8> {
        encode_blob(&self.view())
    }
    /// Re-hash the header chain from `first_parent` so an edit to a header's own
    /// bytes stays a valid chain. Used to build honest controls, never to repair
    /// an attack.
    pub fn relink(&mut self, first_parent: [u8; 32]) {
        let mut parent = first_parent;
        for (i, h) in self.headers.iter_mut().enumerate() {
            let mut fixed = h.clone();
            // The parentHash is the first item of the list; its payload starts
            // right after the list header and the 0xa0 string header.
            let off = list_payload_offset(&fixed) + 1;
            fixed[off..off + 32].copy_from_slice(&parent);
            parent = keccak256(&fixed);
            *h = fixed;
            let _ = i;
        }
    }
    /// keccak256 of the last header — the statement's `endBlockHash`.
    pub fn end_block_hash(&self) -> [u8; 32] {
        keccak256(self.headers.last().expect("at least one header"))
    }
}

fn list_payload_offset(rlp: &[u8]) -> usize {
    let b = rlp[0];
    if b <= 0xf7 {
        1
    } else {
        1 + (b - 0xf7) as usize
    }
}

/// Build an honest witness over `blocks`, classifying each block with the very
/// predicate the guest uses.
pub fn build_witness(
    chain_id: u64,
    start_block: u64,
    first_parent: [u8; 32],
    filters: Vec<LogFilter>,
    blocks: &[SynthBlock],
) -> OwnedWitness {
    let probes: Vec<FilterProbes> = filters.iter().map(FilterProbes::of).collect();
    let mut headers = Vec::with_capacity(blocks.len());
    let mut opened = Vec::new();
    let mut parent = first_parent;
    for (i, b) in blocks.iter().enumerate() {
        let values: Vec<Vec<u8>> = b.receipts.iter().map(|r| r.value()).collect();
        let root = b
            .receipts_root_override
            .unwrap_or_else(|| willow_receipts_trie::build_trie(&values).root);
        let bloom = b.bloom_override.unwrap_or_else(|| {
            let mut acc = [0u8; 256];
            for r in &b.receipts {
                for (x, y) in acc.iter_mut().zip(r.bloom().iter()) {
                    *x |= *y;
                }
            }
            acc
        });
        let header = encode_header(&parent, start_block + i as u64, &root, &bloom);
        parent = keccak256(&header);
        headers.push(header);
        if probes.iter().any(|p| p.possible(&bloom)) {
            opened.push((i as u32, values));
        }
    }
    OwnedWitness {
        chain_id,
        start_block,
        n_blocks: blocks.len() as u32,
        filters,
        headers,
        opened,
    }
}

// ---------------------------------------------------------------------------
// deterministic pseudorandom filler
// ---------------------------------------------------------------------------

/// splitmix64 — a deterministic stream so every fixture is reproducible.
#[derive(Clone, Debug)]
pub struct Rng(pub u64);

impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn bytes<const N: usize>(&mut self) -> [u8; N] {
        let mut out = [0u8; N];
        for c in out.chunks_mut(8) {
            let v = self.next_u64().to_le_bytes();
            let n = c.len();
            c.copy_from_slice(&v[..n]);
        }
        out
    }
}

/// A bloom at mainnet-like density: `n_values` distinct 32-byte values folded in.
/// A busy mainnet block sets well over half the 2,048 bits, which is exactly why
/// the rule-out has to be measured rather than assumed.
pub fn dense_bloom(rng: &mut Rng, n_values: usize) -> [u8; 256] {
    let mut b = [0u8; 256];
    for _ in 0..n_values {
        let v: [u8; 32] = rng.bytes();
        bloom_add(&mut b, &v);
    }
    b
}

/// A LEAN-ONLY range: `n` blocks whose blooms are dense but rule the filter out.
/// This is the fixture that prices a header — keccak over ~600 header bytes, the
/// RLP walk, and the filter's bloom probes — with no receipts anywhere.
pub fn lean_range(n: usize, filter: LogFilter, bloom_values: usize) -> OwnedWitness {
    let probes = FilterProbes::of(&filter);
    let mut rng = Rng(0x10_9E_75_E7);
    let mut blocks = Vec::with_capacity(n);
    for _ in 0..n {
        // Reroll on a bloom false positive so the fixture is genuinely lean.
        let (bloom, root) = loop {
            let b = dense_bloom(&mut rng, bloom_values);
            if !probes.possible(&b) {
                break (b, rng.bytes::<32>());
            }
        };
        blocks.push(SynthBlock {
            receipts: Vec::new(),
            bloom_override: Some(bloom),
            receipts_root_override: Some(root),
        });
    }
    build_witness(1, 25_000_000, [0x9a; 32], vec![filter], &blocks)
}

/// A block of `n_receipts` receipts, each with `logs_per_receipt` three-topic
/// logs and 64 bytes of data — the shape of a busy mainnet block. Every
/// `match_every`-th log is made to match `matching` so the scan has real work.
pub fn busy_block(
    rng: &mut Rng,
    n_receipts: usize,
    logs_per_receipt: usize,
    matching: Option<(&[u8; 20], &[u8; 32])>,
    match_every: usize,
) -> SynthBlock {
    let mut receipts = Vec::with_capacity(n_receipts);
    let mut counter = 0usize;
    for t in 0..n_receipts {
        let mut logs = Vec::with_capacity(logs_per_receipt);
        for _ in 0..logs_per_receipt {
            counter += 1;
            let hit = matching.is_some() && match_every != 0 && counter.is_multiple_of(match_every);
            let (emitter, topic0) = match (hit, matching) {
                (true, Some((a, t0))) => (*a, *t0),
                _ => (rng.bytes::<20>(), rng.bytes::<32>()),
            };
            logs.push(SynthLog {
                emitter,
                topics: vec![topic0, rng.bytes::<32>(), rng.bytes::<32>()],
                data: rng.bytes::<64>().to_vec(),
            });
        }
        receipts.push(SynthReceipt {
            tx_type: if t % 3 == 0 { 0 } else { 2 },
            status: true,
            cum_gas: 21_000 * (t as u64 + 1),
            logs,
            deposit_tail: vec![],
        });
    }
    SynthBlock {
        receipts,
        bloom_override: None,
        receipts_root_override: None,
    }
}

/// A range in which every block is OPENED: the fixture that prices a trie
/// rebuild plus a full log scan.
pub fn busy_range(
    n_blocks: usize,
    n_receipts: usize,
    logs_per_receipt: usize,
    match_every: usize,
) -> OwnedWitness {
    let emitter = [0xa0u8; 20];
    let topic0 = [0xddu8; 32];
    let filter = LogFilter {
        addresses: vec![emitter],
        topics: [vec![topic0], Vec::new(), Vec::new(), Vec::new()],
    };
    let mut rng = Rng(0xB0_05_1E);
    let blocks: Vec<SynthBlock> = (0..n_blocks)
        .map(|_| {
            busy_block(
                &mut rng,
                n_receipts,
                logs_per_receipt,
                Some((&emitter, &topic0)),
                match_every,
            )
        })
        .collect();
    build_witness(1, 25_800_000, [0x5c; 32], vec![filter], &blocks)
}
