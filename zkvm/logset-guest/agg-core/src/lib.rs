








































use logset_core::erc::LogSetStatement;



pub const CHILD_PV_LEN: usize = 64 + 256;







































fn word(b: &[u8], off: usize) -> [u8; 32] {
    let mut w = [0u8; 32];
    w.copy_from_slice(&b[off..off + 32]);
    w
}


fn u64_word(b: &[u8], off: usize) -> Option<u64> {
    if b[off..off + 24].iter().any(|&x| x != 0) {
        return None;
    }
    Some(u64::from_be_bytes(
        b[off + 24..off + 32].try_into().unwrap(),
    ))
}


pub fn decode_child(pv: &[u8]) -> Option<LogSetStatement> {
    if pv.len() != CHILD_PV_LEN {
        return None;
    }
    decode_statements(pv).map(|mut v| v.remove(0))
}



pub fn decode_statements(pv: &[u8]) -> Option<Vec<LogSetStatement>> {
    if pv.len() < CHILD_PV_LEN || (pv.len() - 64) % 256 != 0 {
        return None;
    }
    let m = (pv.len() - 64) / 256;
    if u64_word(pv, 0)? != 0x20 || u64_word(pv, 32)? != m as u64 {
        return None;
    }
    let mut out = Vec::with_capacity(m);
    for j in 0..m {
        let b = 64 + 256 * j;
        out.push(LogSetStatement {
            chain_id: u64_word(pv, b)?,
            start_block: u64_word(pv, b + 32)?,
            end_block: u64_word(pv, b + 64)?,
            start_parent_hash: word(pv, b + 96),
            end_block_hash: word(pv, b + 128),
            filter_hash: word(pv, b + 160),
            log_set_root: word(pv, b + 192),
            log_count: u64_word(pv, b + 224)?,
        });
    }
    Some(out)
}

































































































































































































#[cfg(feature = "index")]
pub mod index_agg;








