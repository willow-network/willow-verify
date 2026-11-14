//! ArbOS's state: the storage of the account at 0xA4B05FFF…FFFF, laid out in nested spaces
//! (`arbos/storage`): a space's key is keccak(parent key ++ id), and offset `k` in a space lives at
//! keccak(space key ++ k[..31])[..31] ++ k[31].

use alloy_primitives::{address, keccak256, Address, B256, U256};
use std::sync::LazyLock;

/// ArbOS's state account.
pub const ARBOS_STATE: Address = address!("0xA4B05FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");

/// A storage space by its key.
#[derive(Clone, Debug)]
pub struct Space(Vec<u8>);

impl Space {
    pub fn root() -> Self {
        Space(Vec::new())
    }

    /// The space `id` within this one.
    pub fn sub(&self, id: &[u8]) -> Self {
        let mut k = self.0.clone();
        k.extend_from_slice(id);
        Space(keccak256(&k).to_vec())
    }

    /// The account slot of offset `key` in this space.
    pub fn slot_of(&self, key: B256) -> B256 {
        let mut pre = self.0.clone();
        pre.extend_from_slice(&key[..31]);
        let mut out = keccak256(&pre);
        out[31] = key[31];
        out
    }

    pub fn slot(&self, offset: u64) -> B256 {
        self.slot_of(B256::from(U256::from(offset)))
    }
}

/// The slots eth_call reads, by name.
pub struct Slots {
    pub version: B256,
    pub network_fee_account: B256,
    pub brotli_level: B256,
    pub collect_tips: B256,
    pub l1_pay_rewards_to: B256,
    pub l1_equilibration_units: B256,
    pub l1_inertia: B256,
    pub l1_per_unit_reward: B256,
    pub l1_last_update_time: B256,
    pub l1_funds_due_for_rewards: B256,
    pub l1_units_since_update: B256,
    pub l1_price_per_unit: B256,
    pub l1_last_surplus: B256,
    pub l1_per_batch_gas_cost: B256,
    pub l1_amortized_cost_cap_bips: B256,
    pub l1_fees_available: B256,
    pub l1_total_funds_due: B256,
    pub l2_speed_limit: B256,
    pub l2_per_block_gas_limit: B256,
    pub l2_min_base_fee: B256,
    pub l2_gas_backlog: B256,
    pub l2_pricing_inertia: B256,
    pub l2_backlog_tolerance: B256,
    pub l2_per_tx_gas_limit: B256,
    pub features: B256,
    pub l1_block_number: B256,
    blockhashes: Space,
}

impl Slots {
    /// The slot holding the hash ArbOS keeps for L1 block `n` (`Blockhashes.BlockHash`).
    pub fn l1_block_hash(&self, n: u64) -> B256 {
        self.blockhashes.slot(1 + n % 256)
    }
}

pub static SLOTS: LazyLock<Slots> = LazyLock::new(|| {
    let root = Space::root();
    let l1 = root.sub(&[0]);
    let l2 = root.sub(&[1]);
    let blockhashes = root.sub(&[6]);
    Slots {
        version: root.slot(0),
        network_fee_account: root.slot(3),
        brotli_level: root.slot(7),
        collect_tips: root.slot(11),
        l1_pay_rewards_to: l1.slot(0),
        l1_equilibration_units: l1.slot(1),
        l1_inertia: l1.slot(2),
        l1_per_unit_reward: l1.slot(3),
        l1_last_update_time: l1.slot(4),
        l1_funds_due_for_rewards: l1.slot(5),
        l1_units_since_update: l1.slot(6),
        l1_price_per_unit: l1.slot(7),
        l1_last_surplus: l1.slot(8),
        l1_per_batch_gas_cost: l1.slot(9),
        l1_amortized_cost_cap_bips: l1.slot(10),
        l1_fees_available: l1.slot(11),
        l1_total_funds_due: l1.sub(&[0]).slot(0),
        l2_speed_limit: l2.slot(0),
        l2_per_block_gas_limit: l2.slot(1),
        l2_min_base_fee: l2.slot(3),
        l2_gas_backlog: l2.slot(4),
        l2_pricing_inertia: l2.slot(5),
        l2_backlog_tolerance: l2.slot(6),
        l2_per_tx_gas_limit: l2.slot(7),
        features: root.sub(&[9]).slot(0),
        l1_block_number: blockhashes.slot(0),
        blockhashes,
    }
});

/// A storage word as a signed number (`StorageBackedBigInt`): two's complement.
pub fn signed(v: U256) -> (bool, U256) {
    match v.bit(255) {
        true => (true, (!v).wrapping_add(U256::from(1))),
        false => (false, v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::b256;

    /// As read off Robinhood Chain with eth_getProof (version 61, chain id 4663 at these slots).
    #[test]
    fn arbos_slots_are_where_nitro_keeps_them() {
        let s = &*SLOTS;
        assert_eq!(
            s.version,
            b256!("0x15fed0451499512d95f3ec5a41c878b9de55f21878b5b4e190d4667ec709b400")
        );
        assert_eq!(
            s.l1_price_per_unit,
            b256!("0xa9f6f085d78d1d37c5819e5c16c9e03198bd14e08cd1f6f8191bc6207b9e9707")
        );
        assert_eq!(
            s.l2_min_base_fee,
            b256!("0xe54de2a4cdacc0a0059d2b6e16348103df8c4aff409c31e40ec73d11926c8203")
        );
        assert_eq!(
            s.l1_block_number,
            b256!("0x3c79da47f96b0f39664f73c0a1f350580be90742947dddfa21ba64d578dfe600")
        );
        // Offset 256 carries into the 31-byte page: 1 + 255 % 256.
        let page = s.l1_block_hash(255);
        assert_eq!(page[31], 0);
        assert_ne!(page, s.l1_block_hash(254));
    }
}
