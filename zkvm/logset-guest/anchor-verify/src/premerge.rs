//! Ethereum blocks before the merge (0 to 15,537,393), through EIP-7643's historical hashes
//! accumulator: every pre-merge block's hash and total difficulty, 8,192 to an epoch record, the
//! 1,897 epoch roots in one list. Its root is pinned here; anyone can recompute it from the
//! pre-merge headers, and era1 files are named by its epoch roots. A proof needs no light client
//! and no upstream read.
//!
//! The proof (`willow_getBlockAnchor [n]`, `proof.kind = "accumulator"`): the block's `hash` and
//! `totalDifficulty` (the `HeaderRecord`), `branch` (13 siblings, the record's place in its epoch)
//! and `epochBranch` (11 siblings, the epoch's place in the list). The epoch's length and the
//! list's are constants here, never read from the proof.

use crate::{Failure, Kind};
use alloy_primitives::U256;
use beacon_cl_core::ssz::{hash_pair, mix_in_length, zero_hashes, Bytes32};
use serde_json::Value;

/// `hash_tree_root(HistoricalHashesAccumulator)` for the blocks before the merge (EIP-7643).
pub const ROOT: Bytes32 = [
    0xec, 0x8e, 0x04, 0x0f, 0xd6, 0xc5, 0x57, 0xb4, 0x1c, 0xa8, 0xdd, 0xd3, 0x8f, 0x7e, 0x9d, 0x58,
    0xa9, 0x28, 0x19, 0x18, 0xdc, 0x92, 0xbd, 0xb7, 0x23, 0x42, 0xa3, 0x8f, 0xb0, 0x85, 0xe7, 0x01,
];
/// The first proof-of-stake block: every block before it is in the accumulator.
pub const MERGE_BLOCK: u64 = 15_537_394;
pub const EPOCH_SIZE: u64 = 8192;
/// Epochs in the list, and the records of the last one (blocks 15,532,032 to 15,537,393).
pub const EPOCHS: u64 = 1897;
pub const LAST_EPOCH_LEN: u64 = MERGE_BLOCK - (EPOCHS - 1) * EPOCH_SIZE;
/// `EPOCH_SIZE` and `MAX_HISTORICAL_EPOCHS` (2,048) as tree depths.
pub const RECORD_DEPTH: usize = 13;
pub const EPOCH_DEPTH: usize = 11;

/// Records in epoch `epoch`.
pub fn epoch_len(epoch: u64) -> u64 {
    if epoch + 1 == EPOCHS {
        LAST_EPOCH_LEN
    } else {
        EPOCH_SIZE
    }
}

/// `hash_tree_root(HeaderRecord)`: the block hash and its total difficulty (uint256, little-endian).
pub fn record_root(hash: &Bytes32, total_difficulty: &U256) -> Bytes32 {
    hash_pair(hash, &total_difficulty.to_le_bytes::<32>())
}

/// The empty `current_epoch` the accumulator was frozen with.
pub fn empty_epoch() -> Bytes32 {
    mix_in_length(&zero_hashes(RECORD_DEPTH)[RECORD_DEPTH], 0)
}

/// `leaf` folded up `siblings` by its `index` (bit `d` set: the node is the right child at `d`).
fn fold(leaf: Bytes32, siblings: &[Bytes32], index: u64) -> Bytes32 {
    siblings.iter().enumerate().fold(leaf, |v, (d, s)| {
        if (index >> d) & 1 == 1 {
            hash_pair(s, &v)
        } else {
            hash_pair(&v, s)
        }
    })
}

/// The accumulator root a proof of block `number` gives.
pub fn root_of(
    number: u64,
    hash: &Bytes32,
    total_difficulty: &U256,
    branch: &[Bytes32],
    epoch_branch: &[Bytes32],
) -> Bytes32 {
    let epoch = number / EPOCH_SIZE;
    let epoch_root = mix_in_length(
        &fold(
            record_root(hash, total_difficulty),
            branch,
            number % EPOCH_SIZE,
        ),
        epoch_len(epoch),
    );
    let epochs = mix_in_length(&fold(epoch_root, epoch_branch, epoch), EPOCHS);
    hash_pair(&epochs, &empty_epoch())
}

fn bad(m: impl Into<String>) -> Failure {
    Failure::new(Kind::MalformedInput, m)
}

fn node(v: &Value, what: &str) -> Result<Bytes32, Failure> {
    let s = v
        .as_str()
        .and_then(|s| s.strip_prefix("0x"))
        .ok_or_else(|| bad(format!("{what} is not 0x hex")))?;
    hex::decode(s)
        .ok()
        .and_then(|b| b.try_into().ok())
        .ok_or_else(|| bad(format!("{what} is not 32 bytes")))
}

fn nodes(v: Option<&Value>, n: usize, what: &str) -> Result<Vec<Bytes32>, Failure> {
    let list = v
        .and_then(Value::as_array)
        .filter(|l| l.len() == n)
        .ok_or_else(|| bad(format!("{what} is not {n} nodes")))?;
    list.iter().map(|x| node(x, what)).collect()
}

/// Block `number`'s hash and total difficulty from an `accumulator` proof: the fold must give
/// [`ROOT`].
pub fn verify(number: u64, proof: &Value) -> Result<(Bytes32, U256), Failure> {
    if number >= MERGE_BLOCK {
        return Err(Failure::new(
            Kind::Rejected,
            format!("block {number} is not before the merge: the accumulator does not hold it"),
        ));
    }
    let hash = node(
        proof.get("hash").unwrap_or(&Value::Null),
        "the accumulator proof's hash",
    )?;
    let td = proof
        .get("totalDifficulty")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .filter(|s| !s.is_empty() && s.len() <= 64)
        .and_then(|s| U256::from_str_radix(s, 16).ok())
        .ok_or_else(|| bad("the accumulator proof's totalDifficulty is not a uint256"))?;
    let branch = nodes(
        proof.get("branch"),
        RECORD_DEPTH,
        "the accumulator proof's branch",
    )?;
    let epoch_branch = nodes(
        proof.get("epochBranch"),
        EPOCH_DEPTH,
        "the accumulator proof's epochBranch",
    )?;
    if root_of(number, &hash, &td, &branch, &epoch_branch) != ROOT {
        return Err(Failure::new(
            Kind::Rejected,
            format!("the accumulator proof of block {number} does not give the pre-merge root"),
        ));
    }
    Ok((hash, td))
}

/// EIP-7643's 1,897 epoch roots (its test cases), in order: what the server checks each epoch's
/// records against before it serves a proof from them. Not in the page's wasm.
#[cfg(not(target_arch = "wasm32"))]
pub const EPOCH_ROOTS: &[u8] = include_bytes!("../data/eip7643-epoch-roots.bin");

#[cfg(test)]
mod tests {
    use super::*;

    fn epoch_roots() -> Vec<Bytes32> {
        EPOCH_ROOTS
            .chunks_exact(32)
            .map(|c| c.try_into().unwrap())
            .collect()
    }

    /// A Portal test vector: a pre-merge block's header and its record's branch.
    struct Vector {
        number: u64,
        hash: Bytes32,
        td: U256,
        branch: Vec<Value>,
    }

    impl Vector {
        /// The `accumulator` proof of this block, its epoch branch from `roots`.
        fn proof(&self, roots: &[Bytes32]) -> Value {
            let eb: Vec<String> = branch(roots, EPOCH_DEPTH, (self.number / EPOCH_SIZE) as usize)
                .iter()
                .map(|n| format!("0x{}", hex::encode(n)))
                .collect();
            serde_json::json!({
                "kind": "accumulator",
                "hash": format!("0x{}", hex::encode(self.hash)),
                "totalDifficulty": format!("{:#x}", self.td),
                "branch": self.branch,
                "epochBranch": eb,
            })
        }
    }

    fn vectors() -> Vec<Vector> {
        let v: Value =
            serde_json::from_str(include_str!("../testdata/premerge-vectors.json")).unwrap();
        v["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| {
                let header = hex::decode(&x["header"].as_str().unwrap()[2..]).unwrap();
                let hash = alloy_primitives::keccak256(&header).0;
                assert_eq!(
                    format!("0x{}", hex::encode(hash)),
                    x["hash"].as_str().unwrap()
                );
                let number = x["number"].as_u64().unwrap();
                assert_eq!(
                    x["length"].as_u64().unwrap(),
                    epoch_len(number / EPOCH_SIZE)
                );
                Vector {
                    number,
                    hash,
                    td: U256::from_str_radix(&x["totalDifficulty"].as_str().unwrap()[2..], 16)
                        .unwrap(),
                    branch: x["branch"].as_array().unwrap().clone(),
                }
            })
            .collect()
    }

    /// The siblings of `leaves[index]` in a `depth` tree, zero-padded.
    fn branch(leaves: &[Bytes32], depth: usize, index: usize) -> Vec<Bytes32> {
        let z = zero_hashes(depth);
        let (mut layer, mut i, mut out) = (leaves.to_vec(), index, Vec::new());
        for zl in z.iter().take(depth) {
            out.push(*layer.get(i ^ 1).unwrap_or(zl));
            layer = layer
                .chunks(2)
                .map(|p| hash_pair(&p[0], p.get(1).unwrap_or(zl)))
                .collect();
            i /= 2;
        }
        out
    }

    #[test]
    fn eip_7643s_epoch_roots_give_its_root() {
        let roots = epoch_roots();
        assert_eq!(roots.len() as u64, EPOCHS);
        let mut layer = roots.clone();
        let z = zero_hashes(EPOCH_DEPTH);
        for zl in z.iter().take(EPOCH_DEPTH) {
            layer = layer
                .chunks(2)
                .map(|p| hash_pair(&p[0], p.get(1).unwrap_or(zl)))
                .collect();
        }
        let list = mix_in_length(&layer[0], EPOCHS);
        assert_eq!(hash_pair(&list, &empty_epoch()), ROOT);
        assert_eq!(LAST_EPOCH_LEN, 5362);
        // An epoch branch from the list folds back for any epoch.
        let b = branch(&roots, EPOCH_DEPTH, 1802);
        assert_eq!(mix_in_length(&fold(roots[1802], &b, 1802), EPOCHS), list);
    }

    #[test]
    fn portals_vectors_verify_and_any_change_is_rejected() {
        let roots = epoch_roots();
        for v in vectors() {
            let p = v.proof(&roots);
            let (hash, td) = verify(v.number, &p).unwrap();
            assert_eq!(hash, v.hash, "block {}", v.number);
            assert_eq!(td, v.td);
            let flip = |p: &Value, path: &[&str], i: Option<usize>| -> Value {
                let mut q = p.clone();
                let mut x = &mut q;
                for k in path {
                    x = &mut x[*k];
                }
                if let Some(i) = i {
                    x = &mut x[i];
                }
                let mut b = node(x, "x").unwrap();
                b[31] ^= 1;
                *x = Value::String(format!("0x{}", hex::encode(b)));
                q
            };
            let kind = |q: &Value| verify(v.number, q).unwrap_err().kind;
            assert_eq!(kind(&flip(&p, &["hash"], None)), Kind::Rejected);
            assert_eq!(kind(&flip(&p, &["branch"], Some(0))), Kind::Rejected);
            assert_eq!(kind(&flip(&p, &["branch"], Some(12))), Kind::Rejected);
            assert_eq!(kind(&flip(&p, &["epochBranch"], Some(3))), Kind::Rejected);
            let mut q = p.clone();
            q["totalDifficulty"] = Value::String(format!("{:#x}", v.td + U256::from(1)));
            assert_eq!(kind(&q), Kind::Rejected);
            // Another block's number with this block's proof.
            assert_eq!(verify(v.number ^ 1, &p).unwrap_err().kind, Kind::Rejected);
            // A branch one sibling short, or long; an epoch branch one short.
            let mut q = p.clone();
            q["branch"].as_array_mut().unwrap().pop();
            assert_eq!(kind(&q), Kind::MalformedInput);
            let mut q = p.clone();
            q["branch"]
                .as_array_mut()
                .unwrap()
                .push(p["branch"][0].clone());
            assert_eq!(kind(&q), Kind::MalformedInput);
            let mut q = p.clone();
            q["epochBranch"].as_array_mut().unwrap().pop();
            assert_eq!(kind(&q), Kind::MalformedInput);
            for bad_td in ["", "0x", "12", "0xzz", &format!("0x1{}", "0".repeat(64))] {
                let mut q = p.clone();
                q["totalDifficulty"] = Value::String(bad_td.into());
                assert_eq!(kind(&q), Kind::MalformedInput, "{bad_td}");
            }
        }
    }

    #[test]
    fn a_premerge_anchor_needs_no_light_client_and_only_the_accumulator_proves_one() {
        let roots = epoch_roots();
        let v = vectors()
            .into_iter()
            .find(|v| v.number == 14_764_013)
            .unwrap();
        let answer = serde_json::json!({
            "blockNumber": format!("{:#x}", v.number),
            "proof": v.proof(&roots),
        });
        // A checkpoint from genesis is far past its 30 days: no matter before the merge.
        let lc = crate::lc::LightClient::new([7; 32], 0);
        let now = 2_000_000_000;
        let c = crate::confirm(&lc, v.number, &answer, now).unwrap();
        assert_eq!((c.level, c.path, c.hash), ("final", "accumulator", v.hash));
        assert_eq!(c.total_difficulty, Some(v.td));
        // The page's face: verified with no bootstrap, and the hash compared.
        let body = serde_json::json!({"jsonrpc": "2.0", "id": 1, "result": answer}).to_string();
        let mut a = crate::Anchor::new(&format!("0x{}", "07".repeat(32)), 0.0);
        let hash = format!("0x{}", hex::encode(v.hash));
        let out: Value =
            serde_json::from_str(&a.verify(1, v.number as f64, &hash, &body, now as f64)).unwrap();
        assert_eq!(out["ok"], true, "{out}");
        assert_eq!(
            (out["level"].as_str(), out["path"].as_str()),
            (Some("final"), Some("accumulator"))
        );
        assert_eq!(out["matches"], true);
        assert_eq!(out["totalDifficulty"], format!("{:#x}", v.td));
        let other = format!("0x{}", "11".repeat(32));
        let out: Value =
            serde_json::from_str(&a.verify(1, v.number as f64, &other, &body, now as f64)).unwrap();
        assert_eq!(
            (out["ok"].as_bool(), out["matches"].as_bool()),
            (Some(true), Some(false))
        );
        // Another proof kind for a pre-merge block, or the anchor of another block: rejected.
        let mut as_beacon = answer.clone();
        as_beacon["proof"] = serde_json::json!({"kind": "beacon", "witness": "0x00"});
        assert_eq!(
            crate::confirm(&lc, v.number, &as_beacon, now)
                .unwrap_err()
                .kind,
            Kind::Rejected
        );
        assert_eq!(
            crate::confirm(&lc, v.number + 1, &answer, now)
                .unwrap_err()
                .kind,
            Kind::Rejected
        );
        // An accumulator proof named for a block after the merge: rejected, not a light-client ask.
        let mut late = answer.clone();
        late["blockNumber"] = Value::String(format!("{MERGE_BLOCK:#x}"));
        assert_eq!(
            crate::confirm(&lc, MERGE_BLOCK, &late, now)
                .unwrap_err()
                .kind,
            Kind::Rejected
        );
        // After the merge the light client is still needed: its stale checkpoint says so.
        let post = serde_json::json!({"blockNumber": format!("{MERGE_BLOCK:#x}"),
            "proof": {"kind": "head"}, "update": {}});
        assert_eq!(
            crate::confirm(&lc, MERGE_BLOCK, &post, now)
                .unwrap_err()
                .kind,
            Kind::StaleCheckpoint
        );
    }

    #[test]
    fn the_last_epochs_length_is_a_constant_not_the_proofs() {
        let roots = epoch_roots();
        let last = vectors()
            .into_iter()
            .find(|v| v.number == MERGE_BLOCK - 1)
            .unwrap();
        let p = last.proof(&roots);
        assert!(verify(last.number, &p).is_ok());
        // The same record folded as a full epoch of 8,192 gives another root.
        let branch = nodes(p.get("branch"), RECORD_DEPTH, "b").unwrap();
        let eb = nodes(p.get("epochBranch"), EPOCH_DEPTH, "e").unwrap();
        let leaf = fold(
            record_root(&last.hash, &last.td),
            &branch,
            last.number % EPOCH_SIZE,
        );
        let as_full = mix_in_length(&leaf, EPOCH_SIZE);
        let root = hash_pair(
            &mix_in_length(&fold(as_full, &eb, EPOCHS - 1), EPOCHS),
            &empty_epoch(),
        );
        assert_ne!(root, ROOT);
        // The merge block itself, with the last pre-merge block's proof: not in the accumulator.
        assert_eq!(verify(MERGE_BLOCK, &p).unwrap_err().kind, Kind::Rejected);
    }
}
