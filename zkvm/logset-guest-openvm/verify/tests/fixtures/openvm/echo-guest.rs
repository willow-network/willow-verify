// Test fixture for willow-logs-verify's OvmCheck: reveals sha256 of its input, the way the
// log-set guests reveal sha256(pv).
use sha2::{Digest, Sha256};

openvm::entry!(main);

fn main() {
    let input = openvm::io::read_vec();
    let d: [u8; 32] = Sha256::digest(&input).into();
    openvm::io::reveal_bytes32(d);
}
