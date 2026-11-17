






use alloc::vec::Vec;
use tiny_keccak::Hasher;


pub fn native_keccak256(msg: &[u8]) -> [u8; 32] {
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(msg);
    let mut out = [0u8; 32];
    hasher.finalize(&mut out);
    out
}





pub fn rlp_long_len_bytes(len: usize) -> Vec<u8> {
    let be = len.to_be_bytes();
    let first = be.iter().position(|&b| b != 0).unwrap_or(be.len() - 1);
    be[first..].to_vec()
}



















pub fn rlp_string_header(bytes: &[u8]) -> Vec<u8> {
    if bytes.len() == 1 && bytes[0] < 0x80 {
        return Vec::new();
    }
    if bytes.len() < 56 {
        return alloc::vec![0x80 + bytes.len() as u8];
    }
    let lb = rlp_long_len_bytes(bytes.len());
    let mut out = Vec::with_capacity(1 + lb.len());
    out.push(0xb7 + lb.len() as u8);
    out.extend_from_slice(&lb);
    out
}




pub fn rlp_list_header(payload_len: usize) -> Vec<u8> {
    if payload_len < 56 {
        return alloc::vec![0xc0 + payload_len as u8];
    }
    let lb = rlp_long_len_bytes(payload_len);
    let mut out = Vec::with_capacity(1 + lb.len());
    out.push(0xf7 + lb.len() as u8);
    out.extend_from_slice(&lb);
    out
}







pub fn native_leaf_hash(hp_str: &[u8], value: &[u8]) -> [u8; 32] {
    let v_hdr = rlp_string_header(value);
    let payload = hp_str.len() + v_hdr.len() + value.len();
    let p_hdr = rlp_list_header(payload);
    let mut node = Vec::with_capacity(p_hdr.len() + payload);
    node.extend_from_slice(&p_hdr);
    node.extend_from_slice(hp_str);
    node.extend_from_slice(&v_hdr);
    node.extend_from_slice(value);
    native_keccak256(&node)
}
