//! Block hashes from the call block's own state (EIP-2935): after block B, the history account's
//! slot `n mod ring` holds the hash of block n, for B-ring ≤ n < B. On Ethereum and the OP stack
//! the ring is 8,191 blocks, which covers BLOCKHASH's B-256 … B-1; Arbitrum's history account
//! (ArbOS 40 on) keeps 393,168 L2 blocks, which covers `ArbSys.arbBlockHash`. So a call proof
//! carries the slots a call reads instead of the headers.

use crate::call::Witness;
use alloy_primitives::{address, keccak256, Address, B256, U256};

/// The history account, at the same address on every chain that runs EIP-2935.
pub const HISTORY_ADDRESS: Address = address!("0000F90827F1C53a10cb7A02335B175320002935");
/// Ethereum's ring: the hashes of the 8,191 blocks before the state's block.
pub const RING: u64 = 8191;
/// EIP-2935's runtime code (the EIP's, deployed on Ethereum by Prague, on the OP stack by Isthmus).
pub const CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500";
/// Arbitrum's: 393,168 L2 blocks, the block number from `ArbSys.arbBlockNumber()`.
pub const ARBITRUM_CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe1460605760203603605c575f3563a3b1b31d5f5260205f6004601c60645afa15605c575f51600181038211605c57816205ffd0910311605c576205ffd09006545f5260205ff35b5f5ffd5b5f356205ffd0600163a3b1b31d5f5260205f6004601c60645afa15605c575f5103065500";

/// One chain's history account: its code and its ring.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct History {
    pub code: &'static str,
    pub ring: u64,
}

/// Ethereum's and the OP stack's.
pub const ETHEREUM: History = History {
    code: CODE,
    ring: RING,
};
/// Arbitrum's (Robinhood Chain's).
pub const ARBITRUM: History = History {
    code: ARBITRUM_CODE,
    ring: 393_168,
};

impl History {
    /// The history account of a chain whose calls are proven.
    pub fn of(chain_id: u64) -> History {
        match chain_id {
            willow_arb_exec::ROBINHOOD => ARBITRUM,
            _ => ETHEREUM,
        }
    }

    pub fn code_bytes(&self) -> Vec<u8> {
        hex::decode(self.code).expect("constant hex")
    }

    /// keccak256 of the code.
    pub fn code_hash(&self) -> B256 {
        keccak256(self.code_bytes())
    }

    /// The slot that holds block `number`'s hash.
    pub fn slot(&self, number: u64) -> B256 {
        B256::from(U256::from(number % self.ring))
    }

    /// Block `number`'s hash as the state of block `at` holds it in `witness`: `None` when the
    /// witness lacks the history account or the slot, the account is not this history's, the
    /// slot is empty, or `number` is not among the ring's blocks.
    pub fn block_hash(&self, witness: &Witness, at: u64, number: u64) -> Option<B256> {
        if number >= at || at - number > self.ring {
            return None;
        }
        let account = witness.account(&HISTORY_ADDRESS)?;
        if !account.exists || account.code_hash != self.code_hash() {
            return None;
        }
        let value = witness.slot(&HISTORY_ADDRESS, self.slot(number))?;
        (!value.is_zero()).then(|| B256::from(value))
    }
}

/// keccak256 of [`CODE`].
pub fn code_hash() -> B256 {
    ETHEREUM.code_hash()
}

/// The slot that holds block `number`'s hash (Ethereum's ring).
pub fn slot(number: u64) -> B256 {
    ETHEREUM.slot(number)
}

/// [`History::block_hash`] on Ethereum.
pub fn block_hash(witness: &Witness, at: u64, number: u64) -> Option<B256> {
    ETHEREUM.block_hash(witness, at, number)
}
