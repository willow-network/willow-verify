//! Calls on Unichain (OP stack, chain 130) run as op-reth v2 runs `eth_call`: under the OP rules
//! in force at the block (op-revm 20, Karst included), over a state the caller of this crate
//! proves ([`State`]: every account, slot, code and block hash read comes from it, the L1Block
//! contract's fee slots included), with the caller's state and block overrides, and with no fee
//! charged (no L1 data fee, operator fee or gas fee, op-reth's).

pub mod block;

use alloy_primitives::{address, keccak256, Address, Bytes, B256, U256};
pub use alloy_rpc_types_eth::{state::StateOverride, BlockOverrides};
use op_revm::{DefaultOp, OpBuilder, OpHaltReason, OpSpecId, OpTransaction, OpTransactionError};
use revm::context::result::{EVMError, ExecutionResult, HaltReason, InvalidTransaction, Output};
use revm::context::{BlockEnv, CfgEnv, Context, TxEnv};
use revm::context_interface::block::BlobExcessGasAndPrice;
use revm::context_interface::transaction::{AccessList, AccessListItem};
use revm::database::CacheDB;
use revm::database_interface::DBErrorMarker;
use revm::primitives::{AddressMap, TxKind};
use revm::state::{Account as EvmAccount, AccountInfo, AccountStatus, Bytecode, EvmStorageSlot};
use revm::{Database, DatabaseCommit, DatabaseRef, ExecuteEvm};

/// Unichain's chain id.
pub const UNICHAIN: u64 = 130;
/// The gas a call gets when it names none, and the most it may name.
pub const GAS_CAP: u64 = 50_000_000;

/// Unichain's forks by time (superchain-registry `unichain.toml`): Canyon to Granite at genesis.
const FORKS: [(u64, OpSpecId); 5] = [
    (0, OpSpecId::GRANITE),
    (1_736_445_601, OpSpecId::HOLOCENE),
    (1_746_806_401, OpSpecId::ISTHMUS),
    (1_764_691_201, OpSpecId::JOVIAN),
    (1_783_526_401, OpSpecId::KARST),
];

/// The OP rules in force on Unichain at `timestamp`.
pub fn spec(timestamp: u64) -> OpSpecId {
    FORKS
        .iter()
        .rev()
        .find(|(at, _)| timestamp >= *at)
        .map_or(OpSpecId::GRANITE, |f| f.1)
}

/// The L1 fee parameters' contract; the base fee, L1 fee and operator fee vaults.
const L1_BLOCK: Address = address!("0x4200000000000000000000000000000000000015");
const VAULTS: [Address; 3] = [
    address!("0x4200000000000000000000000000000000000019"),
    address!("0x420000000000000000000000000000000000001a"),
    address!("0x420000000000000000000000000000000000001b"),
];

/// What every call at `timestamp` reads besides its own state, so it is proven in the first
/// round: L1Block's fee slots (L1 base fee, fee scalars, blob base fee; the operator fee's from
/// Isthmus) and the fee vaults (the operator fee's from Isthmus).
pub fn always_read(timestamp: u64) -> Vec<(Address, Vec<B256>)> {
    let isthmus = spec(timestamp).is_enabled_in(OpSpecId::ISTHMUS);
    let mut slots: Vec<u64> = vec![1, 3, 7];
    if isthmus {
        slots.push(8);
    }
    let slots = slots
        .into_iter()
        .map(|s| B256::from(U256::from(s)))
        .collect();
    let vaults = VAULTS[..if isthmus { 3 } else { 2 }].iter();
    std::iter::once((L1_BLOCK, slots))
        .chain(vaults.map(|v| (*v, Vec::new())))
        .collect()
}

/// An account as the proven state holds it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Account {
    pub balance: U256,
    pub nonce: u64,
    pub code_hash: B256,
    /// Its code, where the state holds it with the account.
    pub code: Option<Bytes>,
}

/// The state a call reads, proven by whoever supplies it: an account that is absent, or that it
/// can't prove yet (it notes those itself), is `None`; a slot it can't prove yet reads zero.
pub trait State {
    fn basic(&self, address: Address) -> Option<Account>;
    fn storage(&self, address: Address, slot: U256) -> U256;
    fn code(&self, hash: B256) -> Option<Bytes>;
    fn block_hash(&self, number: u64) -> Option<B256>;
}

/// The block a call runs in: its verified header's fields.
#[derive(Clone, Debug)]
pub struct Block {
    pub number: u64,
    pub timestamp: u64,
    pub gas_limit: u64,
    pub base_fee: u64,
    pub mix_hash: B256,
    pub beneficiary: Address,
}

/// An `eth_call` transaction object.
#[derive(Clone, Debug, Default)]
pub struct Call {
    pub from: Address,
    pub to: Option<Address>,
    pub gas: Option<u64>,
    pub gas_price: Option<u128>,
    pub max_fee_per_gas: Option<u128>,
    pub max_priority_fee_per_gas: Option<u128>,
    pub value: U256,
    pub data: Bytes,
    pub access_list: Vec<(Address, Vec<B256>)>,
}

impl Call {
    /// The price reth reads: a legacy price, or a 1559 price at `base_fee`, or zero.
    fn price(&self, base_fee: u64) -> u128 {
        if let Some(p) = self.gas_price {
            return p;
        }
        match (self.max_fee_per_gas, self.max_priority_fee_per_gas) {
            (None, None) => 0,
            (cap, tip) => cap
                .unwrap_or(u128::MAX)
                .min(u128::from(base_fee).saturating_add(tip.unwrap_or(0))),
        }
    }
}

/// What a call came to.
#[derive(Debug, PartialEq)]
pub enum Outcome {
    Returned(Bytes),
    Reverted(Bytes),
    /// Stopped by the EVM: geth's message for why.
    Halted(String),
    /// Not run: the transaction is invalid at this block (geth's message).
    Invalid(String),
    /// It read the hash of a block the state doesn't hold.
    NoBlockHash(u64),
    /// The EVM refused the block, the overrides or itself.
    Internal(String),
}

/// What the EVM asked for that can't be read as empty.
#[derive(Debug)]
pub(crate) enum Unreadable {
    BlockHash(u64),
    Code(B256),
}

impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BlockHash(n) => write!(f, "the hash of block {n}"),
            Self::Code(h) => write!(f, "code {h}"),
        }
    }
}
impl std::error::Error for Unreadable {}
impl DBErrorMarker for Unreadable {}

/// [`State`] as revm 38 reads it.
pub(crate) struct Db<'a>(pub(crate) &'a dyn State);

impl std::fmt::Debug for Db<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Db")
    }
}

impl DatabaseRef for Db<'_> {
    type Error = Unreadable;

    fn basic_ref(&self, address: Address) -> Result<Option<AccountInfo>, Unreadable> {
        Ok(self.0.basic(address).map(|a| AccountInfo {
            balance: a.balance,
            nonce: a.nonce,
            code_hash: a.code_hash,
            account_id: None,
            code: a.code.map(Bytecode::new_raw),
        }))
    }

    fn code_by_hash_ref(&self, code_hash: B256) -> Result<Bytecode, Unreadable> {
        self.0
            .code(code_hash)
            .map(Bytecode::new_raw)
            .ok_or(Unreadable::Code(code_hash))
    }

    fn storage_ref(&self, address: Address, index: U256) -> Result<U256, Unreadable> {
        Ok(self.0.storage(address, index))
    }

    fn block_hash_ref(&self, number: u64) -> Result<B256, Unreadable> {
        self.0
            .block_hash(number)
            .ok_or(Unreadable::BlockHash(number))
    }
}

/// `m`, unless it holds words that have a router bench an upstream for every method (venn:
/// "limit", "rate") or mark the method unsupported there (erpc: "not supported"): then
/// `instead`. (The log RPC's Ethereum calls word theirs the same way.)
fn routable(m: &str, instead: &str) -> String {
    let lower = m.to_lowercase();
    match ["limit", "rate", "not supported", "unsupported"]
        .iter()
        .any(|w| lower.contains(w))
    {
        true => instead.to_string(),
        false => m.to_string(),
    }
}

/// Geth's message for an EVM halt.
fn halted(reason: &OpHaltReason) -> String {
    let base = match reason {
        OpHaltReason::Base(h) => h,
        OpHaltReason::FailedDeposit => return "failed deposit".into(),
    };
    match base {
        HaltReason::OutOfGas(_) => "out of gas".into(),
        HaltReason::OpcodeNotFound | HaltReason::InvalidFEOpcode | HaltReason::NotActivated => {
            "invalid opcode".into()
        }
        HaltReason::InvalidJump => "invalid jump destination".into(),
        HaltReason::StackUnderflow => "stack underflow".into(),
        HaltReason::StackOverflow => "stack overflow (1024 entries)".into(),
        HaltReason::OutOfOffset => "return data out of bounds".into(),
        HaltReason::CreateCollision => "contract address collision".into(),
        HaltReason::PrecompileError | HaltReason::PrecompileErrorWithContext(_) => {
            "precompile failed".into()
        }
        HaltReason::NonceOverflow => "nonce uint64 overflow".into(),
        HaltReason::CreateContractSizeLimit => "max code size exceeded".into(),
        HaltReason::CreateContractStartingWithEF => "invalid code: must not begin with 0xef".into(),
        HaltReason::CreateInitCodeSizeLimit => "max initcode size exceeded".into(),
        HaltReason::CallTooDeep => "max call depth exceeded".into(),
        HaltReason::OutOfFunds => "insufficient balance for transfer".into(),
        other => routable(&format!("{other:?}"), "the call halted"),
    }
}

/// Geth's message for a transaction the EVM won't run.
fn invalid(e: &InvalidTransaction) -> String {
    use InvalidTransaction as I;
    let m = match e {
        I::CallGasCostMoreThanGasLimit {
            initial_gas,
            gas_limit,
        } => format!("intrinsic gas too low: have {gas_limit}, want {initial_gas}"),
        I::GasFloorMoreThanGasLimit {
            gas_floor,
            gas_limit,
        } => {
            format!("insufficient gas for floor data gas cost: have {gas_limit}, want {gas_floor}")
        }
        I::LackOfFundForMaxFee { fee, balance } => {
            return format!("insufficient funds for gas * price + value: have {balance} want {fee}")
        }
        I::PriorityFeeGreaterThanMaxFee => {
            "max priority fee per gas higher than max fee per gas".into()
        }
        I::GasPriceLessThanBasefee => "max fee per gas less than block base fee".into(),
        I::CreateInitCodeSizeLimit => "max initcode size exceeded".into(),
        other => routable(
            &other.to_string(),
            "the transaction can't run at this block",
        ),
    };
    format!("invalid transaction: {m}")
}

/// A call's state and block overrides (`eth_call`'s third and fourth parameters), as the caller of
/// this crate read and checked them.
#[derive(Clone, Copy, Debug)]
pub struct Overrides<'a> {
    pub state: &'a StateOverride,
    pub block: &'a BlockOverrides,
    /// `blockOverrides.blobBaseFee`, which this alloy's `BlockOverrides` lacks.
    pub blob_base_fee: Option<U256>,
}

/// `call` run at `block` on Unichain over `state`, under the caller's `overrides`.
pub fn call(state: &dyn State, block: &Block, call: &Call, overrides: Overrides<'_>) -> Outcome {
    measured(state, block, call, overrides).outcome
}

/// A run's outcome with the gas it took, for a gas estimate's search: `used` after refunds,
/// `peak` before them (geth's `MaxUsedGas`, raised to the EIP-7623 floor), `out_of_gas` when it
/// halted for lack of gas (not for an invalid operand), `intrinsic` when it was not run for a gas
/// limit below its intrinsic gas.
#[derive(Debug, PartialEq)]
pub struct Measured {
    pub outcome: Outcome,
    pub used: u64,
    pub peak: u64,
    pub out_of_gas: bool,
    pub intrinsic: bool,
}

/// [`call`], with the gas the run took.
pub fn measured(
    state: &dyn State,
    block: &Block,
    call: &Call,
    overrides: Overrides<'_>,
) -> Measured {
    let not_run = |outcome| Measured {
        outcome,
        used: 0,
        peak: 0,
        out_of_gas: false,
        intrinsic: false,
    };
    let spec = spec(block.timestamp);
    let mut cfg = CfgEnv::new()
        .with_chain_id(UNICHAIN)
        .with_spec_and_mainnet_gas_params(spec);
    cfg.disable_nonce_check = true;
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    cfg.disable_base_fee = true;
    // op-reth's calls: no L1 data, operator or gas fee charged.
    cfg.disable_fee_charge = true;
    cfg.tx_gas_limit_cap = Some(u64::MAX);
    let mut block_env = BlockEnv {
        number: U256::from(block.number),
        beneficiary: block.beneficiary,
        timestamp: U256::from(block.timestamp),
        gas_limit: block.gas_limit,
        basefee: block.base_fee,
        difficulty: U256::ZERO,
        prevrandao: Some(block.mix_hash),
        // The OP stack's blob fee: none paid, the lowest price.
        blob_excess_gas_and_price: Some(BlobExcessGasAndPrice {
            excess_blob_gas: 0,
            blob_gasprice: 1,
        }),
        slot_num: 0,
    };
    let mut db = CacheDB::new(Db(state));
    apply_block(
        overrides.block,
        overrides.blob_base_fee,
        &mut db,
        &mut block_env,
    );
    for (address, o) in overrides.state {
        if let Err(e) = apply_account(*address, o, &mut db) {
            return not_run(Outcome::Internal(e));
        }
    }
    let price = call.price(block_env.basefee);
    if price == 0 {
        block_env.basefee = 0;
    }
    let eip1559 = call.max_fee_per_gas.is_some() || call.max_priority_fee_per_gas.is_some();
    if call
        .max_fee_per_gas
        .is_some_and(|cap| block_env.basefee > 0 && cap < u128::from(block_env.basefee))
    {
        return not_run(Outcome::Invalid(invalid(
            &InvalidTransaction::GasPriceLessThanBasefee,
        )));
    }
    let gas_price = match eip1559 {
        true => call
            .max_fee_per_gas
            .unwrap_or(match call.max_priority_fee_per_gas {
                Some(_) => u128::MAX,
                None => 0,
            }),
        false => call.gas_price.unwrap_or(0),
    };
    let mut gas_limit = call.gas.unwrap_or(GAS_CAP).min(GAS_CAP);
    // With a price and no gas: what the caller's balance less the value pays for (reth's).
    if call.gas.is_none() && gas_price > 0 {
        let balance = revm::Database::basic(&mut db, call.from)
            .ok()
            .flatten()
            .map_or(U256::ZERO, |a| a.balance);
        let Some(left) = balance.checked_sub(call.value) else {
            return not_run(Outcome::Invalid(invalid(
                &InvalidTransaction::LackOfFundForMaxFee {
                    fee: Box::new(call.value),
                    balance: Box::new(balance),
                },
            )));
        };
        let allowance: u64 = (left / U256::from(gas_price)).saturating_to();
        gas_limit = gas_limit.min(allowance).min(block_env.gas_limit);
    }
    let base = TxEnv {
        tx_type: match (eip1559, call.access_list.is_empty()) {
            (true, _) => 2,
            (false, false) => 1,
            (false, true) => 0,
        },
        caller: call.from,
        gas_limit,
        gas_price,
        gas_priority_fee: eip1559.then(|| call.max_priority_fee_per_gas.unwrap_or(0)),
        kind: call.to.map_or(TxKind::Create, TxKind::Call),
        value: call.value,
        data: call.data.clone(),
        chain_id: Some(UNICHAIN),
        access_list: AccessList(
            call.access_list
                .iter()
                .map(|(address, keys)| AccessListItem {
                    address: *address,
                    storage_keys: keys.clone(),
                })
                .collect(),
        ),
        ..Default::default()
    };
    let tx = OpTransaction {
        base,
        enveloped_tx: Some(Bytes::new()),
        deposit: Default::default(),
    };
    let mut evm = Context::op()
        .with_db(db)
        .with_block(block_env)
        .with_cfg(cfg)
        .build_op();
    match evm.transact(tx) {
        Ok(r) => {
            let gas = r.result.gas();
            let (used, peak) = (
                gas.tx_gas_used(),
                gas.total_gas_spent().max(gas.floor_gas()),
            );
            let out_of_gas = matches!(&r.result, ExecutionResult::Halt {
                reason: OpHaltReason::Base(HaltReason::OutOfGas(e)), ..
            } if *e != revm::context::result::OutOfGasError::InvalidOperand);
            let outcome = match r.result {
                ExecutionResult::Success { output, .. } => Outcome::Returned(match output {
                    Output::Call(b) | Output::Create(b, _) => b,
                }),
                ExecutionResult::Revert { output, .. } => Outcome::Reverted(output),
                ExecutionResult::Halt { reason, .. } => Outcome::Halted(halted(&reason)),
            };
            Measured {
                outcome,
                used,
                peak,
                out_of_gas,
                intrinsic: false,
            }
        }
        Err(EVMError::Database(Unreadable::BlockHash(n))) => not_run(Outcome::NoBlockHash(n)),
        Err(EVMError::Database(e)) => not_run(Outcome::Internal(e.to_string())),
        Err(EVMError::Transaction(OpTransactionError::Base(e))) => Measured {
            intrinsic: matches!(e, InvalidTransaction::CallGasCostMoreThanGasLimit { .. }),
            ..not_run(Outcome::Invalid(invalid(&e)))
        },
        Err(e) => not_run(Outcome::Internal(e.to_string())),
    }
}

/// A block's fields as the caller sets them (reth's: the fork stays the block's own).
fn apply_block(
    o: &BlockOverrides,
    blob_base_fee: Option<U256>,
    db: &mut CacheDB<Db<'_>>,
    env: &mut BlockEnv,
) {
    for (n, h) in o.block_hash.iter().flatten() {
        db.cache.block_hashes.insert(U256::from(*n), *h);
    }
    if let Some(n) = o.number {
        env.number = n;
    }
    if let Some(d) = o.difficulty {
        env.difficulty = d;
    }
    if let Some(t) = o.time {
        env.timestamp = U256::from(t);
    }
    if let Some(g) = o.gas_limit {
        env.gas_limit = g;
    }
    if let Some(c) = o.coinbase {
        env.beneficiary = c;
    }
    if let Some(r) = o.random {
        env.prevrandao = Some(r);
    }
    if let Some(b) = o.base_fee {
        env.basefee = b.saturating_to();
    }
    if let Some(b) = blob_base_fee {
        env.blob_excess_gas_and_price = Some(BlobExcessGasAndPrice {
            excess_blob_gas: 0,
            blob_gasprice: b.saturating_to(),
        });
    }
}

/// One account's override laid over the state, as op-reth v2 lays it (alloy-evm 0.33's
/// `apply_account_override`, on revm 38).
fn apply_account(
    address: Address,
    o: &alloy_rpc_types_eth::state::AccountOverride,
    db: &mut CacheDB<Db<'_>>,
) -> Result<(), String> {
    let mut info = db
        .basic(address)
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    if let Some(nonce) = o.nonce {
        info.nonce = nonce;
    }
    if let Some(code) = &o.code {
        info.code_hash = keccak256(code);
        info.code = Some(Bytecode::new_raw_checked(code.clone()).map_err(|e| e.to_string())?);
    }
    if let Some(balance) = o.balance {
        info.balance = balance;
    }
    let mut account = EvmAccount {
        info: info.clone(),
        original_info: Box::new(info),
        status: AccountStatus::Touched,
        storage: Default::default(),
        transaction_id: 0,
    };
    let slots = match (&o.state, &o.state_diff) {
        (Some(_), Some(_)) => {
            return Err(format!(
                "Both 'state' and 'stateDiff' fields are set for account {address}"
            ))
        }
        (None, None) => None,
        // The whole storage replaced: the account's slots cleared first.
        (Some(state), None) => {
            db.commit(AddressMap::from_iter([(
                address,
                EvmAccount {
                    status: AccountStatus::SelfDestructed | AccountStatus::Touched,
                    ..Default::default()
                },
            )]));
            account.mark_created();
            Some(state)
        }
        (None, Some(diff)) => {
            if account.info.is_empty() && !diff.is_empty() {
                account.mark_created();
            }
            Some(diff)
        }
    };
    for (slot, value) in slots.into_iter().flatten() {
        account.storage.insert(
            (*slot).into(),
            EvmStorageSlot {
                // Unlike the present value, so the slot reads as changed.
                original_value: (!*value).into(),
                present_value: (*value).into(),
                is_cold: false,
                transaction_id: 0,
            },
        );
    }
    db.commit(AddressMap::from_iter([(address, account)]));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_call_reads_the_l1_fee_slots_in_force() {
        let (l1, vaults) = (
            address!("0x4200000000000000000000000000000000000015"),
            [0x19, 0x1a, 0x1b],
        );
        let slots = |v: &[u64]| {
            v.iter()
                .map(|s| B256::from(U256::from(*s)))
                .collect::<Vec<_>>()
        };
        let at = |t, n: usize, s: &[u64]| {
            let mut want = vec![(l1, slots(s))];
            for v in &vaults[..n] {
                let mut a = [0u8; 20];
                (a[0], a[19]) = (0x42, *v);
                want.push((Address::new(a), vec![]));
            }
            assert_eq!(always_read(t), want, "{t}");
        };
        at(1_746_806_400, 2, &[1, 3, 7]);
        at(1_746_806_401, 3, &[1, 3, 7, 8]);
    }

    #[test]
    fn unichains_forks_by_time() {
        assert_eq!(spec(1_730_748_359), OpSpecId::GRANITE);
        assert_eq!(spec(1_736_445_601), OpSpecId::HOLOCENE);
        assert_eq!(spec(1_746_806_401), OpSpecId::ISTHMUS);
        assert_eq!(spec(1_764_691_200), OpSpecId::ISTHMUS);
        assert_eq!(spec(1_764_691_201), OpSpecId::JOVIAN);
        assert_eq!(spec(1_783_526_401), OpSpecId::KARST);
        assert_eq!(spec(1_800_000_000), OpSpecId::KARST);
    }
}
