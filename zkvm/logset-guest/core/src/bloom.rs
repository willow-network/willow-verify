//! The `logsBloom` rule-out predicate — the whole reason a 10,000-block range is
//! affordable.
//!
//! Ethereum's bloom is m=2048 bits, k=3. A value `v` sets three bits derived from
//! `keccak256(v)`: for `j in 0..3`, `idx = ((h[2j] << 8) | h[2j+1]) & 2047`, and
//! `idx` names bit `idx & 7` of byte `255 - (idx >> 3)` (the filter is stored
//! big-endian, bit 0 of the number at the LAST byte).
//!
//! A filter can only match a log in a block if EVERY non-empty list of the filter
//! has some member whose three bits are all set. Bloom membership has false
//! positives but no false negatives, so a negative answer PROVES the block holds
//! no matching log — which is exactly the statement a lean block needs.
//!
//! The probes are derived once per filter member at decode time and reused for
//! every block in the range: one keccak per filter member for the whole walk,
//! versus three byte tests per member per block.

use crate::blob::LogFilter;
use crate::keccak256;
use alloc::vec::Vec;

/// The three bloom bit positions a value occupies, `0..2048`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Probe {
    pub bits: [u16; 3],
}

/// Derive a value's three bloom bit positions.
pub fn probe(value: &[u8]) -> Probe {
    let h = keccak256(value);
    Probe {
        bits: [
            (((h[0] as u16) << 8) | h[1] as u16) & 2047,
            (((h[2] as u16) << 8) | h[3] as u16) & 2047,
            (((h[4] as u16) << 8) | h[5] as u16) & 2047,
        ],
    }
}

/// Is bit `idx` set in a 256-byte bloom filter?
pub fn bit_set(bloom: &[u8; 256], idx: u16) -> bool {
    let byte = 255 - (idx >> 3) as usize;
    bloom[byte] & (1u8 << (idx & 7)) != 0
}

impl Probe {
    /// All three bits present.
    pub fn present_in(&self, bloom: &[u8; 256]) -> bool {
        bit_set(bloom, self.bits[0]) && bit_set(bloom, self.bits[1]) && bit_set(bloom, self.bits[2])
    }
}

/// A filter's probes, precomputed once for the whole range.
#[derive(Clone, Debug)]
pub struct FilterProbes {
    pub addresses: Vec<Probe>,
    pub topics: [Vec<Probe>; 4],
}

impl FilterProbes {
    pub fn of(filter: &LogFilter) -> Self {
        FilterProbes {
            addresses: filter.addresses.iter().map(|a| probe(a)).collect(),
            topics: [
                filter.topics[0].iter().map(|t| probe(t)).collect(),
                filter.topics[1].iter().map(|t| probe(t)).collect(),
                filter.topics[2].iter().map(|t| probe(t)).collect(),
                filter.topics[3].iter().map(|t| probe(t)).collect(),
            ],
        }
    }

    /// Could a log matching this filter exist in a block with this bloom?
    ///
    /// An empty list is a wildcard and constrains nothing. A non-empty list is an
    /// OR: some member must be present. A wildcard-everywhere filter (all five
    /// lists empty) is possible in every block, which is right — it matches every
    /// log, so every block has to be opened.
    pub fn possible(&self, bloom: &[u8; 256]) -> bool {
        if !any_present(&self.addresses, bloom) {
            return false;
        }
        for list in &self.topics {
            if !any_present(list, bloom) {
                return false;
            }
        }
        true
    }
}

fn any_present(list: &[Probe], bloom: &[u8; 256]) -> bool {
    list.is_empty() || list.iter().any(|p| p.present_in(bloom))
}
