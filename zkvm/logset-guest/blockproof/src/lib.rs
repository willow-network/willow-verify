





















pub mod block;
pub mod compare;
pub mod derive;
pub mod fault;
pub mod fees;
pub mod mpt;
pub mod opfee;
pub mod proof;
pub mod render;
pub mod txs;






















#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReceiptExtras {


    pub gas_used_for_l1: Option<u64>,


    pub timeboosted: Option<bool>,


    pub deposit_nonce: Option<u64>,
}


pub fn keccak(b: &[u8]) -> [u8; 32] {
    alloy_primitives::keccak256(b).0
}














