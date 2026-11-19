


use anyhow::{ensure, Result};





const _: fn(&[u8]) -> Vec<u8> = willow_receipts_trie::rlp_string_header;



pub const MAX_VALUES: usize = 0x10000;


const PARALLEL_VALUES: usize = 64;










pub fn ordered_root(values: &[Vec<u8>]) -> Result<[u8; 32]> {
    ensure!(
        values.len() <= MAX_VALUES,
        "{} trie values, at most {MAX_VALUES} supported",
        values.len()
    );
    let alloy = || alloy_trie::root::ordered_trie_root_encoded(values).0;
    let willow = || willow_receipts_trie::build_trie(values).root;

    let (alloy_root, willow_root): ([u8; 32], [u8; 32]) = if values.len() >= PARALLEL_VALUES {
        crate::par::join(alloy, willow)
    } else {
        (alloy(), willow())
    };
    ensure!(
        alloy_root == willow_root,
        "trie implementations disagree: alloy-trie 0x{} vs willow-receipts-trie 0x{}",
        hex::encode(alloy_root),
        hex::encode(willow_root)
    );
    Ok(alloy_root)
}
