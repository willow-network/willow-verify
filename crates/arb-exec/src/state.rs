




use alloy_primitives::{address, keccak256, Address, B256, U256};
use std::sync::LazyLock;


pub const ARBOS_STATE: Address = address!("0xA4B05FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF");


#[derive(Clone, Debug)]
pub struct Space(Vec<u8>);

impl Space {

    pub fn root() -> Self {
        Space(Vec::new())
    }


    pub fn sub(&self, id: &[u8]) -> Self {
        let mut joined = self.0.clone();
        joined.extend_from_slice(id);
        Space(keccak256(&joined).to_vec())
    }



    pub fn slot_of(&self, key: B256) -> B256 {
        let mut joined = self.0.clone();
        joined.extend_from_slice(&key[..31]);
        let mut slot = keccak256(&joined);
        slot.0[31] = key[31];
        slot
    }


    pub fn slot(&self, offset: u64) -> B256 {
        self.slot_of(B256::from(U256::from(offset)))
    }
}


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

    l1_blocks: Space,
}

impl Slots {


    pub fn l1_block_hash(&self, n: u64) -> B256 {
        self.l1_blocks.slot(1 + n % 256)
    }
}


pub static SLOTS: LazyLock<Slots> = LazyLock::new(|| {
    let root = Space::root();
    let l1 = root.sub(&[0]);
    let l1_funds = l1.sub(&[0]);
    let l2 = root.sub(&[1]);
    let l1_blocks = root.sub(&[6]);
    let features = root.sub(&[9]);
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
        l1_total_funds_due: l1_funds.slot(0),
        l2_speed_limit: l2.slot(0),
        l2_per_block_gas_limit: l2.slot(1),
        l2_min_base_fee: l2.slot(3),
        l2_gas_backlog: l2.slot(4),
        l2_pricing_inertia: l2.slot(5),
        l2_backlog_tolerance: l2.slot(6),
        l2_per_tx_gas_limit: l2.slot(7),
        features: features.slot(0),
        l1_block_number: l1_blocks.slot(0),
        l1_blocks,
    }
});


pub fn signed(v: U256) -> (bool, U256) {
    match v.bit(255) {
        true => (true, U256::ZERO.wrapping_sub(v)),
        false => (false, v),
    }
}


pub(crate) fn low64(v: U256) -> u64 {
    v.as_limbs()[0]
}


pub(crate) fn low160(v: U256) -> Address {
    Address::from_word(B256::from(v))
}
