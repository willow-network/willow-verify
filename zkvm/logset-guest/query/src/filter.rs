//! The `eth_getLogs` filter, and the index key ranges that hold its answer.

use logset_core::index::{make_key, parse_record, Key, Order};

/// An `eth_getLogs` filter with its block range resolved. Matching follows the
/// log RPC: `addresses: None` and a `None` (or missing) topic position match
/// anything; a log with fewer topics than a constrained position does not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LogFilter {
    pub from_block: u64,
    pub to_block: u64,
    pub addresses: Option<Vec<[u8; 20]>>,
    pub topics: Vec<Option<Vec<[u8; 32]>>>,
}

/// One log decoded from its ERC record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Log {
    pub block: u64,
    pub tx_index: u32,
    /// Block-wide, as in `eth_getLogs`.
    pub log_index: u32,
    pub address: [u8; 20],
    pub topics: Vec<[u8; 32]>,
    pub data: Vec<u8>,
}

impl Log {
    /// Decode and fully validate one record.
    pub fn decode(block: u64, record: &[u8]) -> Option<Self> {
        let head = parse_record(record).ok()?;
        let n = record[28] as usize;
        let topics = (0..n)
            .map(|i| record[29 + 32 * i..61 + 32 * i].try_into().unwrap())
            .collect();
        Some(Self {
            block,
            tx_index: head.tx,
            log_index: head.log,
            address: head.emitter,
            topics,
            data: record[29 + 32 * n + 4..].to_vec(),
        })
    }
}

impl LogFilter {
    /// Address and topic criteria from an `eth_getLogs` filter object, read with
    /// the log RPC's rules, over blocks `[from_block, to_block]` (the caller
    /// resolves `fromBlock`/`toBlock`, which may be tags).
    pub fn from_eth_filter(
        v: &serde_json::Value,
        from_block: u64,
        to_block: u64,
    ) -> Result<Self, String> {
        use serde_json::Value;
        fn fixed<const N: usize>(v: &Value) -> Result<[u8; N], String> {
            let s = v
                .as_str()
                .ok_or_else(|| format!("expected a hex string, got {v}"))?;
            let mut out = [0u8; N];
            hex::decode_to_slice(s.strip_prefix("0x").unwrap_or(s), &mut out)
                .map_err(|e| format!("{s}: {e}"))?;
            Ok(out)
        }
        let addresses = match v.get("address") {
            None | Some(Value::Null) => None,
            Some(Value::Array(list)) if list.is_empty() => None,
            Some(Value::Array(list)) => {
                Some(list.iter().map(fixed::<20>).collect::<Result<_, _>>()?)
            }
            Some(one) => Some(vec![fixed::<20>(one)?]),
        };
        let mut topics = Vec::new();
        if let Some(list) = v.get("topics") {
            let list = list.as_array().ok_or("topics must be an array")?;
            if list.len() > 4 {
                return Err("at most 4 topic positions".into());
            }
            for t in list {
                topics.push(match t {
                    Value::Null => None,
                    Value::Array(any) if any.is_empty() => None,
                    Value::Array(any) => {
                        Some(any.iter().map(fixed::<32>).collect::<Result<_, _>>()?)
                    }
                    one => Some(vec![fixed::<32>(one)?]),
                });
            }
        }
        Ok(Self {
            from_block,
            to_block,
            addresses,
            topics,
        })
    }

    pub fn matches(&self, log: &Log) -> bool {
        if log.block < self.from_block || log.block > self.to_block {
            return false;
        }
        if let Some(addrs) = &self.addresses {
            if !addrs.contains(&log.address) {
                return false;
            }
        }
        self.topics.iter().enumerate().all(|(i, want)| match want {
            None => true,
            Some(want) => log.topics.get(i).is_some_and(|t| want.contains(t)),
        })
    }
}

/// Most key ranges one segment's answer may need; a wider filter is refused.
pub const MAX_RANGES: usize = 1024;

/// The key ranges whose union holds every log of blocks `[start, end]` the
/// filter can match, in a canonical order:
/// - address and topic0 given: one `ByEmitter` range per pair, bounded to the
///   filter's blocks (the block number follows topic0 in the key);
/// - address only: each address's whole `ByEmitter` run;
/// - topic0 only: each topic0's whole `ByTopic0` run;
/// - neither: the whole index.
///
/// The rest of the filter (blocks outside a pair range, later topics) is
/// applied to the proven records natively.
pub fn plan(f: &LogFilter, start: u64, end: u64) -> Vec<(Order, Key, Key)> {
    let (lo, hi) = (f.from_block.max(start), f.to_block.min(end));
    if lo > hi {
        return Vec::new();
    }
    let addrs = f.addresses.as_deref().map(sorted);
    let topic0 = f.topics.first().and_then(|t| t.as_deref()).map(sorted);
    let (e, t) = (Order::ByEmitter, Order::ByTopic0);
    let (m32, m64) = (u32::MAX, u64::MAX);
    match (addrs, topic0) {
        (Some(addrs), Some(topics)) => addrs
            .iter()
            .flat_map(|a| {
                topics.iter().map(move |t0| {
                    (
                        e,
                        make_key(e, a, Some(t0), lo, 0, 0),
                        make_key(e, a, Some(t0), hi, m32, m32),
                    )
                })
            })
            .collect(),
        (Some(addrs), None) => addrs
            .iter()
            .map(|a| {
                (
                    e,
                    make_key(e, a, None, 0, 0, 0),
                    make_key(e, a, Some(&[0xff; 32]), m64, m32, m32),
                )
            })
            .collect(),
        (None, Some(topics)) => topics
            .iter()
            .map(|t0| {
                (
                    t,
                    make_key(t, &[0; 20], Some(t0), 0, 0, 0),
                    make_key(t, &[0xff; 20], Some(t0), m64, m32, m32),
                )
            })
            .collect(),
        (None, None) => vec![(e, [0; 9], [m64; 9])],
    }
}

fn sorted<T: Ord + Copy>(v: &[T]) -> Vec<T> {
    let mut v = v.to_vec();
    v.sort_unstable();
    v.dedup();
    v
}
