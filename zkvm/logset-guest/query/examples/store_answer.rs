//! Builds the `willow_getLogsProof` answer a log RPC would give for one stored part, from a
//! segment tier store (`leaf/`, `l1/`), as test data for the clients. It runs the same
//! `SegmentIndex` the server does; the proof bytes stay in the store, fetched by id.
//!
//! ```sh
//! cargo run --example store_answer -- <store dir> <chain id> <from> <to> ['<filter json>']
//! ```
//! `<from>-<to>` is an L1 window (`l1/<from>-<to>.pv`, a segment part, its blocks' records read
//! from the leaves' `.rec`) or one block (`leaf/<from>-<hash>.pv`, a block part). The filter's
//! `address`/`topics` apply; its block range is `<from>-<to>`. Prints
//! `{"filter", "logs": <how many logs match>, "result": <LogsProof>}`.

use logset_query::{
    frame, unpack_streams, BlockProof, Log, LogFilter, LogsProof, ProofPart, SegmentIndex,
    SegmentProof,
};
use serde_json::{json, Value};
use std::path::Path;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let [dir, chain, from, to, rest @ ..] = &a[..] else {
        eprintln!("usage: store_answer <store dir> <chain id> <from> <to> ['<filter json>']");
        std::process::exit(2)
    };
    let dir = Path::new(dir);
    let (chain, from, to): (u64, u64, u64) = (
        chain.parse().unwrap(),
        from.parse().unwrap(),
        to.parse().unwrap(),
    );
    let mut filter: Value = rest
        .first()
        .map(|f| serde_json::from_str(f).expect("filter json"))
        .unwrap_or(json!({}));
    filter["fromBlock"] = json!(format!("0x{from:x}"));
    filter["toBlock"] = json!(format!("0x{to:x}"));
    let f = LogFilter::from_eth_filter(&filter, from, to).expect("filter");
    let streams = block_streams(dir, from, to);
    let part = if from == to && !dir.join(format!("l1/{from}-{to}.pv")).exists() {
        let (id, pv) = leaf_of(dir, from);
        ProofPart::Block(BlockProof {
            public_values: pv,
            proof: id,
            records: streams[0].clone(),
        })
    } else {
        let pv = std::fs::read(dir.join(format!("l1/{from}-{to}.pv"))).expect("l1 pv");
        let ix = SegmentIndex::build(&pv, &streams).expect("the records rebuild the L1's index");
        ProofPart::Segment(SegmentProof {
            ranges: ix
                .prove(&f)
                .expect("ranges")
                .into_iter()
                .map(Into::into)
                .collect(),
            public_values: pv,
            proof: format!("l1/{from}-{to}"),
        })
    };
    let logs = streams
        .iter()
        .enumerate()
        .flat_map(|(i, rec)| {
            let block = from + i as u64;
            let (buf, starts) = frame(block, rec).expect("record stream");
            (0..starts.len())
                .map(|j| {
                    let end = starts.get(j + 1).map_or(buf.len(), |&n| n as usize);
                    Log::decode(block, &buf[starts[j] as usize..end]).expect("record")
                })
                .collect::<Vec<_>>()
        })
        .filter(|l| f.matches(l))
        .count();
    let result = LogsProof {
        chain_id: chain,
        from_block: from,
        to_block: to,
        parts: vec![part],
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({"filter": filter, "logs": logs, "result": result}))
            .unwrap()
    );
}

/// The stored leaf of one block: its proof id and pv.
fn leaf_of(dir: &Path, n: u64) -> (String, Vec<u8>) {
    let stem = std::fs::read_dir(dir.join("leaf"))
        .expect("leaf dir")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter_map(|name| name.strip_suffix(".pv").map(str::to_string))
        .find(|stem| stem.split('-').count() == 2 && stem.starts_with(&format!("{n}-")))
        .unwrap_or_else(|| panic!("no one-block leaf {n}"));
    let pv = std::fs::read(dir.join(format!("leaf/{stem}.pv"))).unwrap();
    (format!("leaf/{stem}"), pv)
}

/// Each block's match-all record stream, from the leaves (one-block `.rec`, or a multi-block
/// leaf's framed `.rec`) that cover `from..=to`.
fn block_streams(dir: &Path, from: u64, to: u64) -> Vec<Vec<u8>> {
    let mut out: Vec<Option<Vec<u8>>> = vec![None; (to - from + 1) as usize];
    for e in std::fs::read_dir(dir.join("leaf")).expect("leaf dir") {
        let name = e.unwrap().file_name().into_string().unwrap();
        let Some(stem) = name.strip_suffix(".rec") else {
            continue;
        };
        let f: Vec<&str> = stem.split('-').collect();
        let (s, e): (u64, u64) = match f[..] {
            [n, _] => (n.parse().unwrap(), n.parse().unwrap()),
            [s, e, _] => (s.parse().unwrap(), e.parse().unwrap()),
            _ => continue,
        };
        if e < from || s > to {
            continue;
        }
        let rec = std::fs::read(dir.join("leaf").join(&name)).unwrap();
        let streams = if s == e {
            vec![rec]
        } else {
            unpack_streams(&rec, e - s + 1).expect("framed .rec")
        };
        for (i, st) in streams.into_iter().enumerate() {
            let b = s + i as u64;
            if (from..=to).contains(&b) {
                out[(b - from) as usize] = Some(st);
            }
        }
    }
    out.into_iter()
        .enumerate()
        .map(|(i, s)| s.unwrap_or_else(|| panic!("no records for block {}", from + i as u64)))
        .collect()
}
