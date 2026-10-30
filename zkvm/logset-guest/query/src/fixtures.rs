//! TEST FIXTURES (feature `fixtures`): a synthetic chain whose segment public
//! values come from the aggregator's own native code (`aggregate_index`), so
//! they are what the L1 guest commits. Proofs are stood in for by
//! `sha256(vkey | pv)` behind the system's first bytes: an SP1 compressed proof's
//! bincode tag ([`fake_proof`]) or the OpenVM magic ([`fake_openvm_proof`]);
//! [`FakeCheck`] accepts either.

use crate::verify::{Keys, ProofCheck, Trust};
use crate::{frame, pack_streams, Log, ProofId, OPENVM_PROOF_MAGIC};
use logset_agg_core::index_agg::{
    aggregate_index, encode_idx_input_blocks, host_block_records, host_hint,
};
use logset_core::index::{run_statements_core, IndexBuilder};
use logset_core::synth::{build_witness, SynthBlock, SynthLog, SynthReceipt};
use logset_core::{decode_blob, encode_blob, keccak256, run, LogFilter as CoreFilter, Witness};
use std::path::Path;

pub const LEAF_VK: [u32; 8] = [0x687c_1233, 1, 2, 3, 4, 5, 6, 0x5db6_4b1d];
pub const L1_VK: [u8; 32] = [0x0b; 32];
/// The stand-in OpenVM leaf and L1 program keys (other than SP1's).
pub const OVM_LEAF_VK: [u32; 8] = [0x28b8_1964, 7, 8, 9, 10, 11, 12, 0x0849_0119];
pub const OVM_L1_VK: [u8; 32] = [0x3b; 32];

pub fn sha256(b: &[u8]) -> [u8; 32] {
    use sha2::Digest;
    sha2::Sha256::digest(b).into()
}

pub fn addr(i: u8) -> [u8; 20] {
    [i; 20]
}

pub fn topic(i: u8) -> [u8; 32] {
    [i; 32]
}

/// Deterministic blocks over emitters `addr(1..=3)` and signatures
/// `topic(1..=3)`; every fifth block is empty, some logs have no topics or a
/// second topic (`topic(10..=11)`).
pub fn blocks(n: usize, seed: u64) -> Vec<SynthBlock> {
    let mut x = seed;
    let mut next = move |m: u64| {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (x >> 33) % m
    };
    (0..n)
        .map(|i| {
            if i % 5 == 2 {
                return SynthBlock::default();
            }
            let receipts = (0..1 + next(3))
                .map(|r| {
                    let logs = (0..next(4))
                        .map(|_| {
                            let emitter = addr(1 + next(3) as u8);
                            let topics = match next(6) {
                                0 => vec![],
                                1 | 2 => vec![topic(1 + next(3) as u8)],
                                _ => vec![topic(1 + next(3) as u8), topic(10 + next(2) as u8)],
                            };
                            let data = (0..next(40)).map(|_| next(256) as u8).collect();
                            SynthLog {
                                emitter,
                                topics,
                                data,
                            }
                        })
                        .collect();
                    SynthReceipt {
                        tx_type: 2,
                        status: true,
                        cum_gas: 21_000 * (r + 1),
                        logs,
                        deposit_tail: vec![],
                    }
                })
                .collect();
            SynthBlock {
                receipts,
                ..Default::default()
            }
        })
        .collect()
}

fn match_all() -> CoreFilter {
    CoreFilter {
        addresses: vec![],
        topics: [vec![], vec![], vec![], vec![]],
    }
}

/// `n` blocks from `start` on chain `chain_id`: one-block leaf pvs, record
/// streams and block hashes.
pub struct Chain {
    pub chain_id: u64,
    pub start: u64,
    /// One-block leaf pvs (empty for a chain read from a blob: use [`Chain::leaf_pv`]).
    pub leaf_pvs: Vec<Vec<u8>>,
    pub records: Vec<Vec<u8>>,
    pub hashes: Vec<[u8; 32]>,
    source: Source,
}

/// What a leaf over any run of the chain's blocks is built from.
enum Source {
    Synth {
        blocks: Vec<SynthBlock>,
        parent: [u8; 32],
    },
    /// A real `genblob` witness over the whole chain (match-all, every block opened).
    Blob(Vec<u8>),
}

pub fn chain(chain_id: u64, start: u64, n: usize, seed: u64) -> Chain {
    let bs = blocks(n, seed);
    let parent = [0x42; 32];
    let all = build_witness(chain_id, start, parent, vec![match_all()], &bs);
    let hashes: Vec<[u8; 32]> = all.headers.iter().map(|h| keccak256(h)).collect();
    let leaf_pvs = (0..n)
        .map(|i| {
            let p = if i == 0 { parent } else { hashes[i - 1] };
            let w = build_witness(
                chain_id,
                start + i as u64,
                p,
                vec![match_all()],
                &bs[i..i + 1],
            );
            run(&w.encode()).expect("leaf")
        })
        .collect();
    let mut ix = IndexBuilder::new();
    ix.keep_records = true;
    run_statements_core(&all.encode(), Some(&mut ix)).expect("match-all scan");
    let mut records = vec![Vec::new(); n];
    for (b, r) in &ix.records {
        records[(b - start) as usize].extend_from_slice(r);
    }
    Chain {
        chain_id,
        start,
        leaf_pvs,
        records,
        hashes,
        source: Source::Synth { blocks: bs, parent },
    }
}

/// A chain read from a real `genblob` witness (one match-all filter, every
/// block opened), e.g. `<chain>/seg-*.blob`.
pub fn chain_from_blob(blob: Vec<u8>) -> Result<Chain, String> {
    let w = decode_blob(&blob).map_err(|e| format!("blob: {e:?}"))?;
    let all_open = w.opened.len() == w.n_blocks as usize
        && w.opened
            .iter()
            .enumerate()
            .all(|(i, o)| o.index == i as u32);
    if w.filters != vec![match_all()] || !all_open {
        return Err("not a match-all witness with every block opened".into());
    }
    let records = host_block_records(&blob).map_err(|e| format!("records: {e:?}"))?;
    Ok(Chain {
        chain_id: w.chain_id,
        start: w.start_block,
        leaf_pvs: Vec::new(),
        records,
        hashes: w.headers.iter().map(|h| keccak256(h)).collect(),
        source: Source::Blob(blob.clone()),
    })
}

impl Chain {
    /// The leaf pv over blocks `i0..i1` (offsets from `start`), from the leaf's
    /// native code over a witness of exactly those blocks.
    pub fn leaf_pv(&self, i0: usize, i1: usize) -> Vec<u8> {
        match &self.source {
            Source::Synth { blocks, parent } => {
                let p = if i0 == 0 {
                    *parent
                } else {
                    self.hashes[i0 - 1]
                };
                let start = self.start + i0 as u64;
                let w = build_witness(self.chain_id, start, p, vec![match_all()], &blocks[i0..i1]);
                run(&w.encode()).expect("leaf")
            }
            Source::Blob(blob) => {
                let w = decode_blob(blob).expect("blob");
                let part = Witness {
                    chain_id: w.chain_id,
                    start_block: w.start_block + i0 as u64,
                    n_blocks: (i1 - i0) as u32,
                    filters: w.filters.clone(),
                    headers: w.headers[i0..i1].to_vec(),
                    opened: w.opened[i0..i1]
                        .iter()
                        .map(|o| logset_core::OpenedBlock {
                            index: o.index - i0 as u32,
                            receipts: o.receipts.clone(),
                        })
                        .collect(),
                };
                run(&encode_blob(&part)).expect("leaf")
            }
        }
    }

    /// The L1 pv over blocks `i0..i1` (offsets from `start`), one-block children.
    pub fn l1(&self, i0: usize, i1: usize) -> Vec<u8> {
        self.l1_over(1, i0, i1)
    }

    /// The L1 pv over blocks `i0..i1` from children of `span` blocks (the last may be shorter).
    pub fn l1_over(&self, span: usize, i0: usize, i1: usize) -> Vec<u8> {
        self.l1_over_key(&LEAF_VK, span, i0, i1)
    }

    /// [`Chain::l1`] as the OpenVM L1 commits it: its leaves' key is the OpenVM leaf's.
    pub fn ovm_l1(&self, i0: usize, i1: usize) -> Vec<u8> {
        self.l1_over_key(&OVM_LEAF_VK, 1, i0, i1)
    }

    /// [`Chain::l1_over`] for leaves of program key `leaf_vk`.
    pub fn l1_over_key(&self, leaf_vk: &[u32; 8], span: usize, i0: usize, i1: usize) -> Vec<u8> {
        let kids: Vec<(Vec<u8>, Vec<Vec<u8>>)> = (i0..i1)
            .step_by(span)
            .map(|a| {
                let b = (a + span).min(i1);
                let pv = match self.leaf_pvs.get(a) {
                    Some(pv) if b == a + 1 => pv.clone(),
                    _ => self.leaf_pv(a, b),
                };
                (pv, self.records[a..b].to_vec())
            })
            .collect();
        let input = encode_idx_input_blocks(leaf_vk, &kids);
        let hint = host_hint(&input).expect("hint");
        aggregate_index(&input, &hint, &sha256, &mut |_, _| {})
            .expect("aggregate")
            .3
    }

    /// The leaf program key as an L1 commits it.
    pub fn leaf_vkey(&self) -> [u8; 32] {
        logset_agg_core::vkey::vkey_words_to_bytes(&LEAF_VK)
    }

    /// The stand-in OpenVM leaf program key as an OpenVM L1 commits it.
    pub fn ovm_leaf_vkey(&self) -> [u8; 32] {
        logset_agg_core::vkey::vkey_words_to_bytes(&OVM_LEAF_VK)
    }

    /// Both systems' stand-in keys; no cutover block.
    pub fn trust(&self) -> Trust {
        Trust {
            chain_id: self.chain_id,
            sp1: Keys {
                segment: vec![L1_VK],
                leaf: vec![self.leaf_vkey()],
            },
            openvm: Keys {
                segment: vec![OVM_L1_VK],
                leaf: vec![self.ovm_leaf_vkey()],
            },
            openvm_from: None,
        }
    }

    pub fn block_id(&self, i: usize) -> ProofId {
        ProofId::Block {
            number: self.start + i as u64,
            hash: self.hashes[i],
        }
    }

    /// The store id of the leaf over blocks `i0..i1`: a block id for one block.
    pub fn leaf_id(&self, i0: usize, i1: usize) -> ProofId {
        if i1 == i0 + 1 {
            return self.block_id(i0);
        }
        ProofId::Leaf {
            start: self.start + i0 as u64,
            end: self.start + i1 as u64 - 1,
            hash: self.hashes[i1 - 1],
        }
    }

    /// The leaf over blocks `i0..i1` as its store files: pv, proof, `.rec`.
    pub fn leaf_files(&self, i0: usize, i1: usize) -> (Vec<u8>, Vec<u8>, Vec<u8>) {
        let pv = match self.leaf_pvs.get(i0) {
            Some(pv) if i1 == i0 + 1 => pv.clone(),
            _ => self.leaf_pv(i0, i1),
        };
        let rec = if i1 == i0 + 1 {
            self.records[i0].clone()
        } else {
            pack_streams(&self.records[i0..i1])
        };
        let proof = fake_proof(&self.leaf_vkey(), &pv);
        (pv, proof, rec)
    }

    pub fn segment_id(&self, i0: usize, i1: usize) -> ProofId {
        ProofId::Segment {
            start: self.start + i0 as u64,
            end: self.start + i1 as u64 - 1,
        }
    }

    pub fn all_logs(&self) -> Vec<Log> {
        let mut out = Vec::new();
        for (i, rec) in self.records.iter().enumerate() {
            let block = self.start + i as u64;
            let (buf, starts) = frame(block, rec).unwrap();
            for (j, &at) in starts.iter().enumerate() {
                let end = starts.get(j + 1).map_or(buf.len(), |&n| n as usize);
                out.push(Log::decode(block, &buf[at as usize..end]).unwrap());
            }
        }
        out
    }

    /// Write blocks `leaves` and segments `segments` in the segment tier's
    /// store layout (`leaf/<n>-<hash>.{pv,proof,rec}`, `l1/<s>-<e>.{pv,proof}`).
    pub fn write_store(
        &self,
        root: &Path,
        leaves: impl IntoIterator<Item = usize>,
        segments: &[(usize, usize)],
    ) {
        std::fs::create_dir_all(root.join("leaf")).unwrap();
        std::fs::create_dir_all(root.join("l1")).unwrap();
        let leaf_vk = self.leaf_vkey();
        for i in leaves {
            let stem = root.join(self.block_id(i).to_string());
            let pv = &self.leaf_pvs[i];
            std::fs::write(stem.with_extension("rec"), &self.records[i]).unwrap();
            std::fs::write(stem.with_extension("proof"), fake_proof(&leaf_vk, pv)).unwrap();
            std::fs::write(stem.with_extension("pv"), pv).unwrap();
        }
        for &(i0, i1) in segments {
            let stem = root.join(self.segment_id(i0, i1).to_string());
            let pv = self.l1(i0, i1);
            std::fs::write(stem.with_extension("proof"), fake_proof(&L1_VK, &pv)).unwrap();
            std::fs::write(stem.with_extension("pv"), pv).unwrap();
        }
    }

    /// Write leaves of `span` blocks starting at offsets `firsts`, and L1
    /// segments `segments = (i0, i1)` over children of `span` blocks, in the
    /// store layout (`leaf/<s>-<e>-<hash>.{pv,proof,rec}` for `span > 1`).
    pub fn write_leaves(
        &self,
        root: &Path,
        span: usize,
        firsts: impl IntoIterator<Item = usize>,
        segments: &[(usize, usize)],
    ) {
        std::fs::create_dir_all(root.join("leaf")).unwrap();
        std::fs::create_dir_all(root.join("l1")).unwrap();
        for a in firsts {
            let b = a + span;
            let stem = root.join(self.leaf_id(a, b).to_string());
            let (pv, proof, rec) = self.leaf_files(a, b);
            std::fs::write(stem.with_extension("rec"), rec).unwrap();
            std::fs::write(stem.with_extension("proof"), proof).unwrap();
            std::fs::write(stem.with_extension("pv"), pv).unwrap();
        }
        for &(i0, i1) in segments {
            let stem = root.join(self.segment_id(i0, i1).to_string());
            let pv = self.l1_over(span, i0, i1);
            std::fs::write(stem.with_extension("proof"), fake_proof(&L1_VK, &pv)).unwrap();
            std::fs::write(stem.with_extension("pv"), pv).unwrap();
        }
    }
}

/// A stand-in SP1 proof: an SP1 compressed proof's first bytes, then `sha256(vk | pv)`.
pub fn fake_proof(vk: &[u8; 32], pv: &[u8]) -> Vec<u8> {
    [&[1u8, 0, 0, 0][..], &sha256(&[vk.as_slice(), pv].concat())].concat()
}

/// A stand-in OpenVM proof: the OpenVM magic, then `sha256(vk | pv)`.
pub fn fake_openvm_proof(vk: &[u8; 32], pv: &[u8]) -> Vec<u8> {
    [
        &OPENVM_PROOF_MAGIC[..],
        &sha256(&[vk.as_slice(), pv].concat()),
    ]
    .concat()
}

/// Accepts [`fake_proof`] and [`fake_openvm_proof`] of exactly `(vk, pv)`.
pub struct FakeCheck;

impl ProofCheck for FakeCheck {
    fn check(&mut self, vk: &[u8; 32], proof: &[u8], pv: &[u8]) -> Result<(), String> {
        (proof == fake_proof(vk, pv) || proof == fake_openvm_proof(vk, pv))
            .then_some(())
            .ok_or_else(|| "bad proof".into())
    }
}
