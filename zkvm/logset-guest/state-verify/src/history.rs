//! Block hashes from the call block's own state (EIP-2935): after block B, the history account's
//! slot `n mod 8191` holds the hash of block n, for B-8191 ≤ n < B. That covers BLOCKHASH's
//! B-256 … B-1, so a call proof carries the slots a call reads instead of the headers.

use crate::call::Witness;
use alloy_primitives::{address, keccak256, Address, B256, U256};

/// The history account, at the same address on every chain that runs EIP-2935.
pub const HISTORY_ADDRESS: Address = address!("0000F90827F1C53a10cb7A02335B175320002935");
/// Ethereum's ring: the hashes of the 8,191 blocks before the state's block.
pub const RING: u64 = 8191;
/// EIP-2935's runtime code (the EIP's, deployed on Ethereum by Prague).
pub const CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500";

/// keccak256 of [`CODE`].
pub fn code_hash() -> B256 {
    keccak256(hex::decode(CODE).expect("constant hex"))
}

/// The slot that holds block `number`'s hash.
pub fn slot(number: u64) -> B256 {
    B256::from(U256::from(number % RING))
}

/// Block `number`'s hash as the state of block `at` holds it in `witness`: `None` when the
/// witness lacks the history account or the slot, the account is not EIP-2935's, the slot is
/// empty, or `number` is not among the ring's blocks.
pub fn block_hash(witness: &Witness, at: u64, number: u64) -> Option<B256> {
    if number >= at || at - number > RING {
        return None;
    }
    let account = witness.account(&HISTORY_ADDRESS)?;
    if !account.exists || account.code_hash != code_hash() {
        return None;
    }
    let value = witness.slot(&HISTORY_ADDRESS, slot(number))?;
    (!value.is_zero()).then(|| B256::from(value))
}
