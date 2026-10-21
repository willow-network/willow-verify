//! The root of an ordered trie (keys `rlp(0..n)`) over pre-encoded values: the
//! receipts, transactions and withdrawals roots of a block.

use anyhow::{ensure, Result};
// `build_trie` must encode leaves canonically for values under 256 bytes
// (transactions, withdrawals), which willow-receipts-trie does since the
// release that added it. Naming an item that release added makes a tree with an older
// crate fail to build here, instead of failing every block's
// transactionsRoot/withdrawalsRoot check at run time.
const _: fn(&[u8]) -> Vec<u8> = willow_receipts_trie::rlp_string_header;

/// Most values [`ordered_root`] takes: `willow-receipts-trie` keys up to
/// `rlp(65535)`, far above any real block.
pub const MAX_VALUES: usize = 0x10000;

/// The ordered trie root over `values`, computed TWICE — once with alloy-trie,
/// once with `willow-receipts-trie` (the crate the guest links). Disagreement is
/// a bug in one of them and is reported, never papered over.
///
/// Both encode every leaf canonically, whatever the value's length, and
/// `willow-receipts-trie` hash-references every node: a node under 32 bytes,
/// which the MPT inlines, would make the two disagree. Every real receipt,
/// transaction and withdrawal is long enough that its leaf is at least 32
/// bytes; the disagreement check is what keeps that honest.
pub fn ordered_root(values: &[Vec<u8>]) -> Result<[u8; 32]> {
    ensure!(
        values.len() <= MAX_VALUES,
        "{} trie values, at most {MAX_VALUES} supported",
        values.len()
    );
    let alloy_root: [u8; 32] = alloy_trie::root::ordered_trie_root_encoded(values).0;
    let willow_root = willow_receipts_trie::build_trie(values).root;
    ensure!(
        alloy_root == willow_root,
        "trie implementations disagree: alloy-trie 0x{} vs willow-receipts-trie 0x{}",
        hex::encode(alloy_root),
        hex::encode(willow_root)
    );
    Ok(alloy_root)
}
