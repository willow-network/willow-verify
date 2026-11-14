//! The precompiles of Nitro's EVM at ArbOS 60 and on: Osaka's, and ArbOS's own at 0x64–0x74,
//! 0xff and 0xa4b05. ArbSys and ArbGasInfo are rebuilt from Nitro's source, gas included (800 to
//! open ArbOS's state, 800 a slot read, 3 a word of arguments and of results); a call to any
//! other ArbOS precompile, or to a method of these not rebuilt, is refused.

use crate::evm::{arbos_slot, refuse, Ctx};
use crate::state::{signed, SLOTS};
use crate::ROBINHOOD;
use alloy_primitives::{address, keccak256, Address, B256, U256};
use revm::context::LocalContextTr;
use revm::context_interface::{ContextTr, JournalTr};
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::interpreter::{CallInput, Gas, InputsImpl, InstructionResult, InterpreterResult};
use revm::primitives::hardfork::SpecId;
use revm::Database;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

const ARBSYS: Address = address!("0x0000000000000000000000000000000000000064");
const ARBGASINFO: Address = address!("0x000000000000000000000000000000000000006c");

/// ArbOS's precompiles (`gethhook`: all of them from ArbOS 60).
pub(crate) const ARBOS: [Address; 18] = [
    ARBSYS,
    address!("0x0000000000000000000000000000000000000065"),
    address!("0x0000000000000000000000000000000000000066"),
    address!("0x0000000000000000000000000000000000000067"),
    address!("0x0000000000000000000000000000000000000068"),
    address!("0x0000000000000000000000000000000000000069"),
    address!("0x000000000000000000000000000000000000006b"),
    ARBGASINFO,
    address!("0x000000000000000000000000000000000000006d"),
    address!("0x000000000000000000000000000000000000006e"),
    address!("0x000000000000000000000000000000000000006f"),
    address!("0x0000000000000000000000000000000000000070"),
    address!("0x0000000000000000000000000000000000000071"),
    address!("0x0000000000000000000000000000000000000072"),
    address!("0x0000000000000000000000000000000000000073"),
    address!("0x0000000000000000000000000000000000000074"),
    address!("0x00000000000000000000000000000000000000ff"),
    address!("0x00000000000000000000000000000000000a4b05"),
];

/// Gas to open ArbOS's state and per slot read (`StorageReadCost`), per word copied (`CopyGas`).
const READ: u64 = 800;
const COPY: u64 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Purity {
    Pure,
    View,
    Write,
    Payable,
}

use Purity::*;

/// Every method of ArbSys and ArbGasInfo (their ABIs, nitro-precompile-interfaces 7e88c8c).
const METHODS: [(Address, &str, Purity); 41] = [
    (ARBSYS, "arbBlockNumber()", View),
    (ARBSYS, "arbBlockHash(uint256)", View),
    (ARBSYS, "arbChainID()", View),
    (ARBSYS, "arbOSVersion()", View),
    (ARBSYS, "getStorageGasAvailable()", View),
    (ARBSYS, "isTopLevelCall()", View),
    (
        ARBSYS,
        "mapL1SenderContractAddressToL2Alias(address,address)",
        Pure,
    ),
    (ARBSYS, "wasMyCallersAddressAliased()", View),
    (ARBSYS, "myCallersAddressWithoutAliasing()", View),
    (ARBSYS, "withdrawEth(address)", Payable),
    (ARBSYS, "sendTxToL1(address,bytes)", Payable),
    (ARBSYS, "sendMerkleTreeState()", View),
    (ARBGASINFO, "getPricesInWeiWithAggregator(address)", View),
    (ARBGASINFO, "getPricesInWei()", View),
    (ARBGASINFO, "getPricesInArbGasWithAggregator(address)", View),
    (ARBGASINFO, "getPricesInArbGas()", View),
    (ARBGASINFO, "getGasAccountingParams()", View),
    (ARBGASINFO, "getMaxTxGasLimit()", View),
    (ARBGASINFO, "getMinimumGasPrice()", View),
    (ARBGASINFO, "getL1BaseFeeEstimate()", View),
    (ARBGASINFO, "getL1BaseFeeEstimateInertia()", View),
    (ARBGASINFO, "getL1RewardRate()", View),
    (ARBGASINFO, "getL1RewardRecipient()", View),
    (ARBGASINFO, "getL1GasPriceEstimate()", View),
    (ARBGASINFO, "getCurrentTxL1GasFees()", View),
    (ARBGASINFO, "getGasBacklog()", View),
    (ARBGASINFO, "getPricingInertia()", View),
    (ARBGASINFO, "getGasBacklogTolerance()", View),
    (ARBGASINFO, "getL1PricingSurplus()", View),
    (ARBGASINFO, "getPerBatchGasCharge()", View),
    (ARBGASINFO, "getAmortizedCostCapBips()", View),
    (ARBGASINFO, "getL1FeesAvailable()", View),
    (ARBGASINFO, "getL1PricingEquilibrationUnits()", View),
    (ARBGASINFO, "getLastL1PricingUpdateTime()", View),
    (ARBGASINFO, "getL1PricingFundsDueForRewards()", View),
    (ARBGASINFO, "getL1PricingUnitsSinceUpdate()", View),
    (ARBGASINFO, "getLastL1PricingSurplus()", View),
    (ARBGASINFO, "getMaxBlockGasLimit()", View),
    (ARBGASINFO, "getGasPricingConstraints()", View),
    (ARBGASINFO, "getMultiGasPricingConstraints()", View),
    (ARBGASINFO, "getMultiGasBaseFee()", View),
];

fn selector(signature: &str) -> [u8; 4] {
    keccak256(signature.as_bytes())[..4].try_into().unwrap()
}

/// A method by its precompile and selector: its signature and purity.
type Methods = HashMap<(Address, [u8; 4]), (&'static str, Purity)>;

static BY_SELECTOR: LazyLock<Methods> = LazyLock::new(|| {
    METHODS
        .iter()
        .map(|(a, sig, p)| ((*a, selector(sig)), (*sig, *p)))
        .collect()
});

/// Osaka's precompiles and ArbOS's, less those the caller's state overrides (Nitro drops a
/// precompile whose account is overridden).
pub(crate) struct Precompiles {
    eth: EthPrecompiles,
    overridden: HashSet<Address>,
}

impl Precompiles {
    pub fn new(spec: SpecId, overridden: &HashSet<Address>) -> Self {
        let eth = EthPrecompiles {
            precompiles: revm::precompile::Precompiles::new(
                revm::precompile::PrecompileSpecId::from_spec_id(spec),
            ),
            spec,
        };
        Self {
            eth,
            overridden: overridden.clone(),
        }
    }
}

impl<DB: Database> PrecompileProvider<Ctx<DB>> for Precompiles {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: SpecId) -> bool {
        <EthPrecompiles as PrecompileProvider<Ctx<DB>>>::set_spec(&mut self.eth, spec)
    }

    fn run(
        &mut self,
        ctx: &mut Ctx<DB>,
        address: &Address,
        inputs: &InputsImpl,
        is_static: bool,
        gas_limit: u64,
    ) -> Result<Option<InterpreterResult>, String> {
        if self.overridden.contains(address) {
            return Ok(None);
        }
        if ARBOS.contains(address) {
            return Ok(Some(arbos(ctx, *address, inputs, is_static, gas_limit)));
        }
        self.eth.run(ctx, address, inputs, is_static, gas_limit)
    }

    /// Warm from the start of the call, overridden or not (Nitro's `ActivePrecompiles`).
    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        Box::new(self.eth.warm_addresses().chain(ARBOS))
    }

    fn contains(&self, address: &Address) -> bool {
        !self.overridden.contains(address)
            && (ARBOS.contains(address) || self.eth.contains(address))
    }
}

/// Why a precompile method didn't answer.
enum Fail {
    /// Its gas ran out (`BurnOut`): reverted with none left.
    OutOfGas,
    /// One of its Solidity errors: reverted with the error's data.
    Sol(Vec<u8>),
    /// Any other error: reverted with the gas left.
    Plain,
    /// Not rebuilt here.
    Refuse(String),
    /// A read failed (the context holds why).
    Db,
}

/// The gas a precompile call has left, burnt as Nitro's `Context.Burn` burns it.
struct Meter(u64);

impl Meter {
    fn burn(&mut self, n: u64) -> Result<(), Fail> {
        match self.0.checked_sub(n) {
            Some(left) => {
                self.0 = left;
                Ok(())
            }
            None => {
                self.0 = 0;
                Err(Fail::OutOfGas)
            }
        }
    }
}

fn words(n: usize) -> u64 {
    (n as u64).div_ceil(32)
}

fn result(r: InstructionResult, out: Vec<u8>, limit: u64, left: u64) -> InterpreterResult {
    let mut gas = Gas::new(limit);
    let _ = gas.record_cost(limit - left);
    InterpreterResult {
        result: r,
        output: out.into(),
        gas,
    }
}

fn word(v: U256) -> Vec<u8> {
    v.to_be_bytes::<32>().to_vec()
}

fn int_word(negative: bool, magnitude: U256) -> Vec<u8> {
    match negative {
        true => word((!magnitude).wrapping_add(U256::from(1))),
        false => word(magnitude),
    }
}

/// A call to an ArbOS precompile, as Nitro's `Precompile.Call` runs it.
fn arbos<DB: Database>(
    ctx: &mut Ctx<DB>,
    address: Address,
    inputs: &InputsImpl,
    is_static: bool,
    limit: u64,
) -> InterpreterResult {
    let input: Vec<u8> = match &inputs.input {
        CallInput::SharedBuffer(range) => ctx
            .local()
            .shared_memory_buffer_slice(range.clone())
            .map(|s| s.to_vec())
            .unwrap_or_default(),
        CallInput::Bytes(b) => b.to_vec(),
    };
    let fatal = || result(InstructionResult::FatalExternalError, vec![], limit, 0);
    let all_gas = |r| result(r, vec![], limit, 0);
    if address != ARBSYS && address != ARBGASINFO {
        refuse(ctx, format!("it calls the ArbOS precompile {address}"));
        return fatal();
    }
    // Every ArbOS precompile is active from ArbOS 60: a call without a method's selector, or
    // with one its ABI lacks, reverts with all its gas.
    let Some(sel) = input.get(..4).map(|s| <[u8; 4]>::try_from(s).unwrap()) else {
        return all_gas(InstructionResult::Revert);
    };
    let Some(&(method, purity)) = BY_SELECTOR.get(&(address, sel)) else {
        return all_gas(InstructionResult::Revert);
    };
    let value = inputs.call_value;
    if (purity >= View && inputs.target_address != address)
        || (purity >= Write && is_static)
        || (purity < Payable && !value.is_zero())
    {
        return all_gas(InstructionResult::Revert);
    }
    let mut m = Meter(limit);
    // Opening ArbOS's state reads its version; out of gas there fails the call, not a revert.
    if purity != Pure && m.burn(READ).is_err() {
        return all_gas(InstructionResult::PrecompileOOG);
    }
    let args = &input[4..];
    if m.burn(COPY * words(args.len())).is_err() {
        return all_gas(InstructionResult::Revert);
    }
    let body = match address {
        ARBSYS => arbsys(ctx, &mut m, method, args, inputs),
        _ => arbgasinfo(ctx, &mut m, method, args),
    };
    match body {
        Ok(out) => match m.burn(COPY * words(out.len())) {
            Ok(()) => result(InstructionResult::Return, out, limit, m.0),
            Err(_) => all_gas(InstructionResult::Revert),
        },
        Err(Fail::Sol(data)) => match m.burn(COPY * words(data.len())) {
            Ok(()) => result(InstructionResult::Revert, data, limit, m.0),
            Err(_) => all_gas(InstructionResult::Revert),
        },
        Err(Fail::Plain) => result(InstructionResult::Revert, vec![], limit, m.0),
        Err(Fail::OutOfGas) => all_gas(InstructionResult::Revert),
        Err(Fail::Refuse(why)) => {
            refuse(ctx, why);
            fatal()
        }
        Err(Fail::Db) => fatal(),
    }
}

/// Argument `i` of a call's arguments, a word each; too short a call reverts with all its gas.
fn arg(args: &[u8], i: usize) -> Result<U256, Fail> {
    args.get(i * 32..i * 32 + 32)
        .map(U256::from_be_slice)
        .ok_or(Fail::OutOfGas)
}

fn read<DB: Database>(ctx: &mut Ctx<DB>, m: &mut Meter, slot: B256) -> Result<U256, Fail> {
    m.burn(READ)?;
    arbos_slot(ctx, slot).ok_or(Fail::Db)
}

/// The L2 gas price ArbGasInfo reports: the block's base fee, even where the call runs at zero.
fn l2_price<DB: Database>(ctx: &Ctx<DB>) -> U256 {
    ctx.chain
        .base_fee_in_block
        .unwrap_or(U256::from(ctx.block.basefee))
}

fn arbsys<DB: Database>(
    ctx: &mut Ctx<DB>,
    _m: &mut Meter,
    method: &str,
    args: &[u8],
    inputs: &InputsImpl,
) -> Result<Vec<u8>, Fail> {
    let depth = ctx.journaled_state.depth();
    let refuse = |what: &str| Err(Fail::Refuse(format!("it calls ArbSys.{what}")));
    Ok(match method {
        "arbBlockNumber()" => word(ctx.chain.block_number),
        "arbBlockHash(uint256)" => {
            let n = arg(args, 0)?;
            let current = ctx.chain.block_number;
            let (now, asked) = (current.as_limbs()[0], u64::try_from(n));
            let invalid = || {
                let mut data = selector("InvalidBlockNumber(uint256,uint256)").to_vec();
                data.extend(word(n));
                data.extend(word(current));
                Err(Fail::Sol(data))
            };
            let Ok(asked) = asked else {
                return invalid();
            };
            if asked >= now || asked.wrapping_add(256) < now {
                return invalid();
            }
            // The block's own hashes: none at or past it (a number the caller raised).
            if asked >= ctx.chain.header_number {
                word(U256::ZERO)
            } else {
                match ctx.journaled_state.database.block_hash(asked) {
                    Ok(h) => h.to_vec(),
                    Err(e) => {
                        ctx.error = Err(revm::context_interface::context::ContextError::Db(e));
                        return Err(Fail::Db);
                    }
                }
            }
        }
        "arbChainID()" => word(U256::from(ROBINHOOD)),
        "arbOSVersion()" => word(U256::from(55 + ctx.chain.version)),
        "getStorageGasAvailable()" => word(U256::ZERO),
        "isTopLevelCall()" => word(U256::from(u8::from(depth <= 2))),
        "mapL1SenderContractAddressToL2Alias(address,address)" => {
            let sender = arg(args, 0)?;
            arg(args, 1)?;
            let offset = U256::from_be_slice(
                address!("0x1111000000000000000000000000000000001111").as_slice(),
            );
            let mask: U256 = (U256::from(1) << 160) - U256::from(1);
            let aliased: U256 = (sender & mask).wrapping_add(offset);
            word(aliased & mask)
        }
        // Only a transaction of an aliasing type (none in a call) is aliased.
        "wasMyCallersAddressAliased()" => word(U256::ZERO),
        // A direct call has no caller's caller; past that, the caller's frames aren't kept here.
        "myCallersAddressWithoutAliasing()" if depth <= 1 => word(U256::ZERO),
        "myCallersAddressWithoutAliasing()" => return refuse("myCallersAddressWithoutAliasing"),
        "sendMerkleTreeState()" if inputs.caller_address != Address::ZERO => {
            return Err(Fail::Plain)
        }
        m => return refuse(m.split('(').next().unwrap_or(m)),
    })
}

fn arbgasinfo<DB: Database>(
    ctx: &mut Ctx<DB>,
    m: &mut Meter,
    method: &str,
    args: &[u8],
) -> Result<Vec<u8>, Fail> {
    let s = &*SLOTS;
    let words_of = |v: &[U256]| v.iter().flat_map(|x| word(*x)).collect::<Vec<u8>>();
    let l2 = l2_price(ctx);
    let mul = |a: U256, b: U256| a.checked_mul(b).ok_or(Fail::Plain);
    Ok(match method {
        "getPricesInWeiWithAggregator(address)" | "getPricesInWei()" => {
            if method.ends_with("(address)") {
                arg(args, 0)?;
            }
            let l1 = read(ctx, m, s.l1_price_per_unit)?;
            let for_l1_calldata = mul(l1, U256::from(16))?;
            let per_l2_tx = mul(for_l1_calldata, U256::from(140))?;
            let base = read(ctx, m, s.l2_min_base_fee)?.min(l2);
            let storage = mul(l2, U256::from(20_000))?;
            words_of(&[per_l2_tx, for_l1_calldata, storage, base, l2 - base, l2])
        }
        "getPricesInArbGasWithAggregator(address)" | "getPricesInArbGas()" => {
            if method.ends_with("(address)") {
                arg(args, 0)?;
            }
            let l1 = read(ctx, m, s.l1_price_per_unit)?;
            let for_l1_calldata = mul(l1, U256::from(16))?;
            let per_l2_tx = mul(for_l1_calldata, U256::from(140))?;
            let (gas_for_l1, gas_per_tx) = match l2.is_zero() {
                true => (U256::ZERO, U256::ZERO),
                false => (for_l1_calldata / l2, per_l2_tx / l2),
            };
            words_of(&[gas_per_tx, gas_for_l1, U256::from(20_000)])
        }
        "getGasAccountingParams()" => {
            let speed = read(ctx, m, s.l2_speed_limit)?;
            let max = read(ctx, m, s.l2_per_block_gas_limit)?;
            words_of(&[low64(speed), low64(max), low64(max)])
        }
        "getMaxTxGasLimit()" => word(low64(read(ctx, m, s.l2_per_tx_gas_limit)?)),
        "getMinimumGasPrice()" => word(read(ctx, m, s.l2_min_base_fee)?),
        "getL1BaseFeeEstimate()" | "getL1GasPriceEstimate()" => {
            word(read(ctx, m, s.l1_price_per_unit)?)
        }
        "getL1BaseFeeEstimateInertia()" => word(low64(read(ctx, m, s.l1_inertia)?)),
        "getL1RewardRate()" => word(low64(read(ctx, m, s.l1_per_unit_reward)?)),
        "getL1RewardRecipient()" => {
            let v = read(ctx, m, s.l1_pay_rewards_to)?;
            word(v & ((U256::from(1) << 160) - U256::from(1)))
        }
        "getCurrentTxL1GasFees()" => word(ctx.chain.poster_fee),
        "getGasBacklog()" => word(low64(read(ctx, m, s.l2_gas_backlog)?)),
        "getPricingInertia()" => word(low64(read(ctx, m, s.l2_pricing_inertia)?)),
        "getGasBacklogTolerance()" => word(low64(read(ctx, m, s.l2_backlog_tolerance)?)),
        "getL1PricingSurplus()" => {
            let refunds = signed(read(ctx, m, s.l1_total_funds_due)?);
            let rewards = signed(read(ctx, m, s.l1_funds_due_for_rewards)?);
            let have = read(ctx, m, s.l1_fees_available)?;
            surplus(have, refunds, rewards)?
        }
        "getPerBatchGasCharge()" => {
            let v = low64(read(ctx, m, s.l1_per_batch_gas_cost)?).to::<u64>() as i64;
            int_word(v < 0, U256::from(v.unsigned_abs()))
        }
        "getAmortizedCostCapBips()" => word(low64(read(ctx, m, s.l1_amortized_cost_cap_bips)?)),
        "getL1FeesAvailable()" => word(read(ctx, m, s.l1_fees_available)?),
        "getL1PricingEquilibrationUnits()" => word(read(ctx, m, s.l1_equilibration_units)?),
        "getLastL1PricingUpdateTime()" => word(low64(read(ctx, m, s.l1_last_update_time)?)),
        "getL1PricingFundsDueForRewards()" => {
            // A uint256 in the ABI: a negative amount fails to encode, and the call reverts.
            match signed(read(ctx, m, s.l1_funds_due_for_rewards)?) {
                (true, v) if !v.is_zero() => return Err(Fail::Plain),
                (_, v) => word(v),
            }
        }
        "getL1PricingUnitsSinceUpdate()" => {
            // As this call's own L1 data left it.
            let v = low64(read(ctx, m, s.l1_units_since_update)?).to::<u64>();
            word(U256::from(v.wrapping_add(ctx.chain.units_added)))
        }
        "getLastL1PricingSurplus()" => {
            let (neg, v) = signed(read(ctx, m, s.l1_last_surplus)?);
            int_word(neg, v)
        }
        "getMaxBlockGasLimit()" => word(low64(read(ctx, m, s.l2_per_block_gas_limit)?)),
        m => {
            let name = m.split('(').next().unwrap_or(m);
            return Err(Fail::Refuse(format!("it calls ArbGasInfo.{name}")));
        }
    })
}

/// A storage word as `StorageBackedUint64` reads it.
fn low64(v: U256) -> U256 {
    U256::from(v.as_limbs()[0])
}

/// `L1FeesAvailable - (TotalFundsDue + FundsDueForRewards)`, as an int256.
fn surplus(have: U256, refunds: (bool, U256), rewards: (bool, U256)) -> Result<Vec<u8>, Fail> {
    let as_i = |(neg, v): (bool, U256)| -> alloy_primitives::I256 {
        let v = alloy_primitives::I256::from_raw(v);
        if neg {
            -v
        } else {
            v
        }
    };
    let need = as_i(refunds)
        .checked_add(as_i(rewards))
        .ok_or(Fail::Plain)?;
    let out = alloy_primitives::I256::from_raw(have)
        .checked_sub(need)
        .ok_or(Fail::Plain)?;
    Ok(word(out.into_raw()))
}

/// Nitro's reason for a revert with one of ArbOS's Solidity errors (`RenderRPCError`): "error
/// Name(args)", the arguments as Go prints them; `None` for any other data or one that doesn't
/// decode.
pub(crate) fn render_error(data: &[u8]) -> Option<String> {
    #[derive(Clone, Copy)]
    enum T {
        U(u32),
        Str,
        Bool,
    }
    type Errors = Vec<([u8; 4], &'static str, Vec<T>)>;
    static ERRORS: LazyLock<Errors> = LazyLock::new(|| {
        let e = |name: &'static str, types: Vec<T>| {
            let sig: Vec<&str> = types
                .iter()
                .map(|t| match t {
                    T::U(256) => "uint256",
                    T::U(64) => "uint64",
                    T::U(16) => "uint16",
                    T::U(_) => unreachable!(),
                    T::Str => "string",
                    T::Bool => "bool",
                })
                .collect();
            (selector(&format!("{name}({})", sig.join(","))), name, types)
        };
        vec![
            e("CallerNotArbOS", vec![]),
            e("Custom", vec![T::U(64), T::Str, T::Bool]),
            e("Unused", vec![]),
            e("NoTicketWithID", vec![]),
            e("NotCallable", vec![]),
            e("InvalidBlockNumber", vec![T::U(256), T::U(256)]),
            e("ProgramNotWasm", vec![]),
            e("ProgramNotActivated", vec![]),
            e("ProgramNeedsUpgrade", vec![T::U(16), T::U(16)]),
            e("ProgramExpired", vec![T::U(64)]),
            e("ProgramUpToDate", vec![]),
            e("ProgramKeepaliveTooSoon", vec![T::U(64)]),
            e("ProgramInsufficientValue", vec![T::U(256), T::U(256)]),
        ]
    });
    let (sel, body) = (data.get(..4)?, &data[4..]);
    let (_, name, types) = ERRORS.iter().find(|(s, _, _)| s == sel)?;
    let w = |i: usize| body.get(i * 32..i * 32 + 32).map(U256::from_be_slice);
    let mut shown = Vec::new();
    for (i, t) in types.iter().enumerate() {
        let v = w(i)?;
        shown.push(match t {
            T::U(256) => v.to_string(),
            T::U(bits) => {
                if v.bit_len() > *bits as usize {
                    return None;
                }
                v.to_string()
            }
            T::Bool => match v {
                v if v == U256::ZERO => "false".into(),
                v if v == U256::from(1) => "true".into(),
                _ => return None,
            },
            T::Str => {
                let at = usize::try_from(v).ok()?;
                let len = usize::try_from(U256::from_be_slice(body.get(at..at.checked_add(32)?)?))
                    .ok()?;
                let start = at.checked_add(32)?;
                let bytes = body.get(start..start.checked_add(len)?)?;
                String::from_utf8_lossy(bytes).into_owned()
            }
        });
    }
    Some(format!("error {name}({})", shown.join(", ")))
}
