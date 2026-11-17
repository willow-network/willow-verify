












use sha2::{Digest, Sha256};


pub type Bytes32 = [u8; 32];



















pub fn hash_pair(left: &Bytes32, right: &Bytes32) -> Bytes32 {
    let mut h = Sha256::new();
    h.update(left);
    h.update(right);
    h.finalize().into()
}


pub fn zero_hashes(max_depth: usize) -> Vec<Bytes32> {
    let mut z = Vec::with_capacity(max_depth + 1);
    z.push([0u8; 32]);
    for d in 1..=max_depth {
        let prev = z[d - 1];
        z.push(hash_pair(&prev, &prev));
    }
    z
}



pub fn merkleize(chunks: &[Bytes32], depth: usize) -> Bytes32 {
    let z = zero_hashes(depth);
    let mut layer = chunks.to_vec();
    for zl in z.iter().take(depth) {
        if layer.is_empty() {
            break;
        }
        let mut next = Vec::with_capacity(layer.len().div_ceil(2));
        let mut i = 0;
        while i < layer.len() {
            let right = if i + 1 < layer.len() {
                layer[i + 1]
            } else {
                *zl
            };
            next.push(hash_pair(&layer[i], &right));
            i += 2;
        }
        layer = next;
    }
    if layer.is_empty() {
        z[depth]
    } else {
        layer[0]
    }
}





































pub fn mix_in_length(root: &Bytes32, length: u64) -> Bytes32 {
    let mut len_leaf = [0u8; 32];
    len_leaf[..8].copy_from_slice(&length.to_le_bytes());
    hash_pair(root, &len_leaf)
}





























































pub fn uint64_leaf(v: u64) -> Bytes32 {
    let mut leaf = [0u8; 32];
    leaf[..8].copy_from_slice(&v.to_le_bytes());
    leaf
}









































































