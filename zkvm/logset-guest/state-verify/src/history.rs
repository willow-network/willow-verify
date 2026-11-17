





use crate::call::Witness;
use alloy_primitives::{address, keccak256, Address, B256, U256};


pub const HISTORY_ADDRESS: Address = address!("0000F90827F1C53a10cb7A02335B175320002935");

pub const RING: u64 = 8191;

pub const CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe14604657602036036042575f35600143038111604257611fff81430311604257611fff9006545f5260205ff35b5f5ffd5b5f35611fff60014303065500";

pub const ARBITRUM_CODE: &str = "3373fffffffffffffffffffffffffffffffffffffffe1460605760203603605c575f3563a3b1b31d5f5260205f6004601c60645afa15605c575f51600181038211605c57816205ffd0910311605c576205ffd09006545f5260205ff35b5f5ffd5b5f356205ffd0600163a3b1b31d5f5260205f6004601c60645afa15605c575f5103065500";


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct History {
    pub code: &'static str,
    pub ring: u64,
}


pub const ETHEREUM: History = History {
    code: CODE,
    ring: RING,
};

pub const ARBITRUM: History = History {
    code: ARBITRUM_CODE,
    ring: 393_168,
};

impl History {

    pub fn of(chain_id: u64) -> History {
        match chain_id {
            willow_arb_exec::ROBINHOOD => ARBITRUM,
            _ => ETHEREUM,
        }
    }

    pub fn code_bytes(&self) -> Vec<u8> {
        hex::decode(self.code).expect("constant hex")
    }


    pub fn code_hash(&self) -> B256 {
        keccak256(self.code_bytes())
    }


    pub fn slot(&self, number: u64) -> B256 {
        B256::from(U256::from(number % self.ring))
    }




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


pub fn code_hash() -> B256 {
    ETHEREUM.code_hash()
}


pub fn slot(number: u64) -> B256 {
    ETHEREUM.slot(number)
}


pub fn block_hash(witness: &Witness, at: u64, number: u64) -> Option<B256> {
    ETHEREUM.block_hash(witness, at, number)
}
