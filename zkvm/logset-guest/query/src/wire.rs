//! The JSON a query proof travels in (`willow_getLogsProof`). Numbers are
//! `0x` quantities and bytes `0x` hex, as in Ethereum JSON-RPC.

use logset_core::index::RangeProof;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Everything a client needs to check one `eth_getLogs` answer except the SP1
/// proofs, which it fetches by id (`willow_getProof`) and can cache: one
/// segment proof serves every filter over that segment.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsProof {
    #[serde(with = "quantity")]
    pub chain_id: u64,
    #[serde(with = "quantity")]
    pub from_block: u64,
    #[serde(with = "quantity")]
    pub to_block: u64,
    /// Contiguous, in block order.
    pub parts: Vec<ProofPart>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ProofPart {
    Segment(SegmentProof),
    Block(BlockProof),
    Leaf(LeafProof),
}

/// A final segment: its L1 public values and one range proof per [`plan`](crate::plan) range.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SegmentProof {
    #[serde(with = "bytes")]
    pub public_values: Vec<u8>,
    pub proof: String,
    pub ranges: Vec<RangeProofWire>,
}

/// A block not yet in a final segment: its leaf public values and every record.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockProof {
    #[serde(with = "bytes")]
    pub public_values: Vec<u8>,
    pub proof: String,
    #[serde(with = "bytes")]
    pub records: Vec<u8>,
}

/// A multi-block leaf not yet in a final segment (a fast chain's `S`-block
/// segment): its public values and every record of each of its blocks.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeafProof {
    #[serde(with = "bytes")]
    pub public_values: Vec<u8>,
    pub proof: String,
    /// One match-all record stream per block of the leaf, in block order.
    #[serde(with = "bytes_list")]
    pub records: Vec<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeProofWire {
    #[serde(with = "quantity")]
    pub lo: u64,
    pub records: Vec<RecordWire>,
    #[serde(with = "words")]
    pub siblings: Vec<[u8; 32]>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecordWire {
    #[serde(with = "quantity")]
    pub block: u64,
    #[serde(with = "bytes")]
    pub record: Vec<u8>,
}

impl From<RangeProof> for RangeProofWire {
    fn from(p: RangeProof) -> Self {
        Self {
            lo: p.lo,
            records: p
                .records
                .into_iter()
                .map(|(block, record)| RecordWire { block, record })
                .collect(),
            siblings: p.siblings,
        }
    }
}

impl From<&RangeProofWire> for RangeProof {
    fn from(w: &RangeProofWire) -> Self {
        Self {
            lo: w.lo,
            records: w
                .records
                .iter()
                .map(|r| (r.block, r.record.clone()))
                .collect(),
            siblings: w.siblings.clone(),
        }
    }
}

/// Which stored proof a part refers to. The text form is the proof store's
/// file stem (`l1/<start>-<end>`, `leaf/<number>-<hash>`, and for a leaf of
/// more than one block `leaf/<start>-<end>-<end hash>`), parsed strictly.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProofId {
    Segment {
        start: u64,
        end: u64,
    },
    Block {
        number: u64,
        hash: [u8; 32],
    },
    Leaf {
        start: u64,
        end: u64,
        hash: [u8; 32],
    },
}

impl fmt::Display for ProofId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProofId::Segment { start, end } => write!(f, "l1/{start}-{end}"),
            ProofId::Block { number, hash } => write!(f, "leaf/{number}-{}", hex::encode(hash)),
            ProofId::Leaf { start, end, hash } => {
                write!(f, "leaf/{start}-{end}-{}", hex::encode(hash))
            }
        }
    }
}

impl FromStr for ProofId {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || format!("bad proof id {s:?}");
        let (kind, rest) = s.split_once('/').ok_or_else(bad)?;
        let number = |x: &str| -> Result<u64, String> {
            if x.is_empty() || x.len() > 20 || !x.bytes().all(|c| c.is_ascii_digit()) {
                return Err(bad());
            }
            x.parse().map_err(|_| bad())
        };
        let hash = |x: &str| -> Result<[u8; 32], String> {
            if x.len() != 64 || !x.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f')) {
                return Err(bad());
            }
            let mut out = [0u8; 32];
            hex::decode_to_slice(x, &mut out).map_err(|_| bad())?;
            Ok(out)
        };
        let fields: Vec<&str> = rest.split('-').collect();
        match (kind, fields.as_slice()) {
            ("l1", [a, b]) => Ok(ProofId::Segment {
                start: number(a)?,
                end: number(b)?,
            }),
            ("leaf", [a, h]) => Ok(ProofId::Block {
                number: number(a)?,
                hash: hash(h)?,
            }),
            // A one-block leaf has only the two-field name.
            ("leaf", [a, b, h]) => {
                let (start, end) = (number(a)?, number(b)?);
                if start >= end {
                    return Err(bad());
                }
                Ok(ProofId::Leaf {
                    start,
                    end,
                    hash: hash(h)?,
                })
            }
            _ => Err(bad()),
        }
    }
}

pub(crate) mod quantity {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &u64, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("0x{v:x}"))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<u64, D::Error> {
        let s = String::deserialize(d)?;
        let h = s
            .strip_prefix("0x")
            .ok_or_else(|| D::Error::custom("expected 0x"))?;
        u64::from_str_radix(h, 16).map_err(D::Error::custom)
    }
}

pub(crate) mod bytes {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&format!("0x{}", hex::encode(v)))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        let h = s
            .strip_prefix("0x")
            .ok_or_else(|| D::Error::custom("expected 0x"))?;
        hex::decode(h).map_err(D::Error::custom)
    }
}

pub(crate) mod words {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[[u8; 32]], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(v.iter().map(|w| format!("0x{}", hex::encode(w))))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<[u8; 32]>, D::Error> {
        Vec::<String>::deserialize(d)?
            .iter()
            .map(|s| {
                let mut w = [0u8; 32];
                let h = s
                    .strip_prefix("0x")
                    .ok_or_else(|| D::Error::custom("expected 0x"))?;
                hex::decode_to_slice(h, &mut w).map_err(D::Error::custom)?;
                Ok(w)
            })
            .collect()
    }
}

pub(crate) mod bytes_list {
    use serde::{de::Error, Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[Vec<u8>], s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(v.iter().map(|b| format!("0x{}", hex::encode(b))))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<Vec<u8>>, D::Error> {
        Vec::<String>::deserialize(d)?
            .iter()
            .map(|s| {
                let h = s
                    .strip_prefix("0x")
                    .ok_or_else(|| D::Error::custom("expected 0x"))?;
                hex::decode(h).map_err(D::Error::custom)
            })
            .collect()
    }
}
