






use crate::exec::{arbos_word, fail, refuse, NitroCtx};
use crate::state::{low160, low64, signed, SLOTS};
use alloy_primitives::{hex, Address, Bytes, U256};
use revm::context::LocalContextTr;
use revm::context_interface::JournalTr;
use revm::handler::{EthPrecompiles, PrecompileProvider};
use revm::interpreter::{CallInput, Gas, InputsImpl, InstructionResult, InterpreterResult};
use revm::precompile::{PrecompileSpecId, Precompiles};
use revm::primitives::hardfork::SpecId;
use revm::Database;
use std::collections::HashSet;


const fn small(n: u32) -> Address {
    let mut b = [0u8; 20];
    b[17] = (n >> 16) as u8;
    b[18] = (n >> 8) as u8;
    b[19] = n as u8;
    Address::new(b)
}

const ARBSYS: Address = small(0x64);
const ARBGASINFO: Address = small(0x6c);


pub(crate) const ARBOS_PRECOMPILES: [Address; 18] = [
    small(0x64),
    small(0x65),
    small(0x66),
    small(0x67),
    small(0x68),
    small(0x69),
    small(0x6b),
    small(0x6c),
    small(0x6d),
    small(0x6e),
    small(0x6f),
    small(0x70),
    small(0x71),
    small(0x72),
    small(0x73),
    small(0x74),
    small(0xff),
    small(0xa4b05),
];



const STOPPED: &str = "the run was stopped";


pub(crate) struct ArbPrecompiles {
    base: EthPrecompiles,
    overridden: HashSet<Address>,
}

impl ArbPrecompiles {
    pub(crate) fn new(overridden: HashSet<Address>) -> Self {
        ArbPrecompiles {
            base: EthPrecompiles {
                precompiles: Precompiles::new(PrecompileSpecId::from_spec_id(SpecId::OSAKA)),
                spec: SpecId::OSAKA,
            },
            overridden,
        }
    }
}

impl<DB: Database> PrecompileProvider<NitroCtx<DB>> for ArbPrecompiles {
    type Output = InterpreterResult;

    fn set_spec(&mut self, spec: SpecId) -> bool {
        <EthPrecompiles as PrecompileProvider<NitroCtx<DB>>>::set_spec(&mut self.base, spec)
    }

    fn run(
        &mut self,
        context: &mut NitroCtx<DB>,
        address: &Address,
        inputs: &InputsImpl,
        is_static: bool,
        gas_limit: u64,
    ) -> Result<Option<InterpreterResult>, String> {

        if self.overridden.contains(address) {
            return Ok(None);
        }
        if *address == ARBSYS || *address == ARBGASINFO {
            return arbos_call(context, *address, inputs, is_static, gas_limit).map(Some);
        }
        if ARBOS_PRECOMPILES.contains(address) {
            let why = format!(
                "it calls the ArbOS precompile {}",
                address.to_checksum(None)
            );
            refuse(context, why.clone());
            return Err(why);
        }
        <EthPrecompiles as PrecompileProvider<NitroCtx<DB>>>::run(
            &mut self.base,
            context,
            address,
            inputs,
            is_static,
            gas_limit,
        )
    }

    fn warm_addresses(&self) -> Box<impl Iterator<Item = Address>> {
        Box::new(self.base.warm_addresses().chain(ARBOS_PRECOMPILES))
    }

    fn contains(&self, address: &Address) -> bool {
        !self.overridden.contains(address)
            && (self.base.contains(address) || ARBOS_PRECOMPILES.contains(address))
    }
}


#[derive(Clone, Copy, PartialEq, Eq)]
enum Purity {
    Pure,
    View,
    Payable,
}


#[derive(Clone, Copy)]
enum Method {
    BlockNumber,
    BlockHash,
    ChainId,
    OsVersion,
    StorageGasAvailable,
    IsTopLevelCall,
    AliasOf,
    WasAliased,
    CallerUnaliased,
    WithdrawEth,
    SendTxToL1,
    SendMerkleTreeState,
    PricesInWei {
        aggregator: bool,
    },
    PricesInArbGas {
        aggregator: bool,
    },
    GasAccountingParams,
    MaxTxGasLimit,
    MinimumGasPrice,
    L1BaseFeeEstimate,
    L1BaseFeeEstimateInertia,
    L1RewardRate,
    L1RewardRecipient,
    L1GasPriceEstimate,
    CurrentTxL1GasFees,
    GasBacklog,
    PricingInertia,
    GasBacklogTolerance,
    L1PricingSurplus,
    PerBatchGasCharge,
    AmortizedCostCapBips,
    L1FeesAvailable,
    L1PricingEquilibrationUnits,
    LastL1PricingUpdateTime,
    L1PricingFundsDueForRewards,
    L1PricingUnitsSinceUpdate,
    LastL1PricingSurplus,
    MaxBlockGasLimit,

    NotRebuilt(&'static str),
}

use Method as M;
use Purity::{Payable, Pure, View};

const ARBSYS_METHODS: [([u8; 4], Method, Purity); 12] = [
    (hex!("a3b1b31d"), M::BlockNumber, View),
    (hex!("2b407a82"), M::BlockHash, View),
    (hex!("d127f54a"), M::ChainId, View),
    (hex!("051038f2"), M::OsVersion, View),
    (hex!("a94597ff"), M::StorageGasAvailable, View),
    (hex!("08bd624c"), M::IsTopLevelCall, View),
    (hex!("4dbbd506"), M::AliasOf, Pure),
    (hex!("175a260b"), M::WasAliased, View),
    (hex!("d74523b3"), M::CallerUnaliased, View),
    (hex!("25e16063"), M::WithdrawEth, Payable),
    (hex!("928c169a"), M::SendTxToL1, Payable),
    (hex!("7aeecd2a"), M::SendMerkleTreeState, View),
];

const ARBGASINFO_METHODS: [([u8; 4], Method, Purity); 29] = [
    (hex!("ba9c916e"), M::PricesInWei { aggregator: true }, View),
    (hex!("41b247a8"), M::PricesInWei { aggregator: false }, View),
    (
        hex!("7a1ea732"),
        M::PricesInArbGas { aggregator: true },
        View,
    ),
    (
        hex!("02199f34"),
        M::PricesInArbGas { aggregator: false },
        View,
    ),
    (hex!("612af178"), M::GasAccountingParams, View),
    (hex!("aae1cd4c"), M::MaxTxGasLimit, View),
    (hex!("f918379a"), M::MinimumGasPrice, View),
    (hex!("f5d6ded7"), M::L1BaseFeeEstimate, View),
    (hex!("29eb31ee"), M::L1BaseFeeEstimateInertia, View),
    (hex!("8a5b1d28"), M::L1RewardRate, View),
    (hex!("9e6d7e31"), M::L1RewardRecipient, View),
    (hex!("055f362f"), M::L1GasPriceEstimate, View),
    (hex!("c6f7de0e"), M::CurrentTxL1GasFees, View),
    (hex!("1d5b5c20"), M::GasBacklog, View),
    (hex!("3dfb45b9"), M::PricingInertia, View),
    (hex!("25754f91"), M::GasBacklogTolerance, View),
    (hex!("520acdd7"), M::L1PricingSurplus, View),
    (hex!("6ecca45a"), M::PerBatchGasCharge, View),
    (hex!("7a7d6beb"), M::AmortizedCostCapBips, View),
    (hex!("5b39d23c"), M::L1FeesAvailable, View),
    (hex!("ad26ce90"), M::L1PricingEquilibrationUnits, View),
    (hex!("138b47b4"), M::LastL1PricingUpdateTime, View),
    (hex!("963d6002"), M::L1PricingFundsDueForRewards, View),
    (hex!("eff01306"), M::L1PricingUnitsSinceUpdate, View),
    (hex!("2987d027"), M::LastL1PricingSurplus, View),
    (hex!("0371fdb4"), M::MaxBlockGasLimit, View),
    (
        hex!("232027d1"),
        M::NotRebuilt("getGasPricingConstraints"),
        View,
    ),
    (
        hex!("bbfc0a72"),
        M::NotRebuilt("getMultiGasPricingConstraints"),
        View,
    ),
    (hex!("c0e10bbb"), M::NotRebuilt("getMultiGasBaseFee"), View),
];


const INVALID_BLOCK_NUMBER: [u8; 4] = hex!("d5dc642d");


enum Done {

    Output(Vec<u8>),

    SolError(Vec<u8>),

    Plain,

    AllGas,

    Refused(String),
}


fn word_gas(len: usize) -> u64 {
    (len as u64).div_ceil(32).saturating_mul(3)
}

fn ended(result: InstructionResult, gas_limit: u64) -> InterpreterResult {
    let mut gas = Gas::new(gas_limit);
    gas.spend_all();
    InterpreterResult {
        result,
        output: Bytes::new(),
        gas,
    }
}


fn arbos_call<DB: Database>(
    ctx: &mut NitroCtx<DB>,
    at: Address,
    inputs: &InputsImpl,
    is_static: bool,
    gas_limit: u64,
) -> Result<InterpreterResult, String> {
    let input: Vec<u8> = match &inputs.input {
        CallInput::Bytes(b) => b.to_vec(),
        CallInput::SharedBuffer(range) => ctx
            .local
            .shared_memory_buffer_slice(range.clone())
            .map(|s| s.to_vec())
            .unwrap_or_default(),
    };
    let all_gas = || Ok(ended(InstructionResult::Revert, gas_limit));
    let table: &[([u8; 4], Method, Purity)] = match at == ARBSYS {
        true => &ARBSYS_METHODS,
        false => &ARBGASINFO_METHODS,
    };
    let Some(&(_, method, purity)) = input
        .get(..4)
        .and_then(|s| table.iter().find(|m| m.0[..] == *s))
    else {
        return all_gas();
    };
    if purity != Pure && inputs.target_address != at {
        return all_gas();
    }
    if purity == Payable && is_static {
        return all_gas();
    }
    if purity != Payable && !inputs.call_value.is_zero() {
        return all_gas();
    }
    let mut gas = Gas::new(gas_limit);
    if purity != Pure && !gas.record_cost(800) {
        return Ok(ended(InstructionResult::PrecompileOOG, gas_limit));
    }
    if !gas.record_cost(word_gas(input.len() - 4)) {
        return all_gas();
    }
    let done = match answer(ctx, method, &input[4..], inputs, &mut gas) {
        Ok(done) => done,
        Err(e) => {
            fail(ctx, e);
            return Err(STOPPED.into());
        }
    };
    let (result, output) = match done {
        Done::Output(out) => (InstructionResult::Return, out),
        Done::SolError(data) => (InstructionResult::Revert, data),
        Done::Plain => (InstructionResult::Revert, Vec::new()),
        Done::AllGas => return all_gas(),
        Done::Refused(why) => {
            refuse(ctx, why.clone());
            return Err(why);
        }
    };
    if !gas.record_cost(word_gas(output.len())) {
        return all_gas();
    }
    Ok(InterpreterResult {
        result,
        output: output.into(),
        gas,
    })
}

fn words(ws: &[U256]) -> Done {
    Done::Output(ws.iter().flat_map(|w| w.to_be_bytes::<32>()).collect())
}


fn answer<DB: Database>(
    ctx: &mut NitroCtx<DB>,
    method: Method,
    args: &[u8],
    inputs: &InputsImpl,
    gas: &mut Gas,
) -> Result<Done, DB::Error> {
    let arg = |i: usize| args.get(32 * i..32 * i + 32).map(U256::from_be_slice);
    macro_rules! read {
        ($slot:ident) => {{
            if !gas.record_cost(800) {
                return Ok(Done::AllGas);
            }
            arbos_word(ctx, SLOTS.$slot)?
        }};
    }
    let p2 = ctx.chain.base_fee;
    Ok(match method {
        M::BlockNumber => words(&[ctx.chain.number]),
        M::BlockHash => {
            let Some(n) = arg(0) else {
                return Ok(Done::AllGas);
            };
            let number = ctx.chain.number;
            let now = low64(number);
            let out_of_range = match u64::try_from(n) {
                Ok(n) => n >= now || n.wrapping_add(256) < now,
                Err(_) => true,
            };
            if out_of_range {
                let mut data = INVALID_BLOCK_NUMBER.to_vec();
                data.extend_from_slice(&n.to_be_bytes::<32>());
                data.extend_from_slice(&number.to_be_bytes::<32>());
                return Ok(Done::SolError(data));
            }
            let n = n.to::<u64>();
            match n >= ctx.chain.header_number {
                true => words(&[U256::ZERO]),
                false => {
                    let hash = ctx.journaled_state.database.block_hash(n)?;
                    words(&[U256::from_be_bytes(hash.0)])
                }
            }
        }
        M::ChainId => words(&[U256::from(ctx.cfg.chain_id)]),
        M::OsVersion => words(&[U256::from(55u64) + U256::from(ctx.chain.arbos_version)]),
        M::StorageGasAvailable | M::WasAliased => words(&[U256::ZERO]),
        M::IsTopLevelCall => words(&[U256::from(u8::from(ctx.journaled_state.depth() <= 2))]),
        M::AliasOf => {
            let (Some(sender), Some(_)) = (arg(0), arg(1)) else {
                return Ok(Done::AllGas);
            };
            let mask = (U256::from(1u64) << 160) - U256::from(1u64);
            let offset = U256::from_be_slice(&hex!("1111000000000000000000000000000000001111"));
            words(&[((sender & mask) + offset) & mask])
        }
        M::CallerUnaliased => match ctx.journaled_state.depth() {
            0 | 1 => words(&[U256::ZERO]),
            _ => Done::Refused("it calls ArbSys.myCallersAddressWithoutAliasing".into()),
        },
        M::WithdrawEth => Done::Refused("it calls ArbSys.withdrawEth".into()),
        M::SendTxToL1 => Done::Refused("it calls ArbSys.sendTxToL1".into()),
        M::SendMerkleTreeState => match inputs.caller_address.is_zero() {
            true => Done::Refused("it calls ArbSys.sendMerkleTreeState".into()),
            false => Done::Plain,
        },
        M::PricesInWei { aggregator } | M::PricesInArbGas { aggregator } => {
            if aggregator && arg(0).is_none() {
                return Ok(Done::AllGas);
            }
            let l1 = read!(l1_price_per_unit);
            let Some(w1) = l1.checked_mul(U256::from(16u64)) else {
                return Ok(Done::Plain);
            };
            let Some(w0) = w1.checked_mul(U256::from(140u64)) else {
                return Ok(Done::Plain);
            };
            match method {
                M::PricesInWei { .. } => {
                    let min = read!(l2_min_base_fee);
                    let w3 = min.min(p2);
                    words(&[w0, w1, p2 * U256::from(20_000u64), w3, p2 - w3, p2])
                }
                _ => match p2.is_zero() {
                    true => words(&[U256::ZERO, U256::ZERO, U256::from(20_000u64)]),
                    false => words(&[w0 / p2, w1 / p2, U256::from(20_000u64)]),
                },
            }
        }
        M::GasAccountingParams => {
            let speed = read!(l2_speed_limit);
            let block = read!(l2_per_block_gas_limit);
            let (s, x) = (U256::from(low64(speed)), U256::from(low64(block)));
            words(&[s, x, x])
        }
        M::MaxTxGasLimit => words(&[U256::from(low64(read!(l2_per_tx_gas_limit)))]),
        M::MinimumGasPrice => words(&[read!(l2_min_base_fee)]),
        M::L1BaseFeeEstimate | M::L1GasPriceEstimate => words(&[read!(l1_price_per_unit)]),
        M::L1BaseFeeEstimateInertia => words(&[U256::from(low64(read!(l1_inertia)))]),
        M::L1RewardRate => words(&[U256::from(low64(read!(l1_per_unit_reward)))]),
        M::L1RewardRecipient => {
            let to = low160(read!(l1_pay_rewards_to));
            words(&[U256::from_be_slice(to.as_slice())])
        }
        M::CurrentTxL1GasFees => words(&[ctx.chain.poster_fee]),
        M::GasBacklog => words(&[U256::from(low64(read!(l2_gas_backlog)))]),
        M::PricingInertia => words(&[U256::from(low64(read!(l2_pricing_inertia)))]),
        M::GasBacklogTolerance => words(&[U256::from(low64(read!(l2_backlog_tolerance)))]),
        M::L1PricingSurplus => {
            let total_due = read!(l1_total_funds_due);
            let rewards_due = read!(l1_funds_due_for_rewards);
            let available = read!(l1_fees_available);
            words(&[available.wrapping_sub(total_due).wrapping_sub(rewards_due)])
        }
        M::PerBatchGasCharge => {
            let v = low64(read!(l1_per_batch_gas_cost)) as i64;
            let word = match v < 0 {
                true => U256::ZERO.wrapping_sub(U256::from(v.unsigned_abs())),
                false => U256::from(v as u64),
            };
            words(&[word])
        }
        M::AmortizedCostCapBips => words(&[U256::from(low64(read!(l1_amortized_cost_cap_bips)))]),
        M::L1FeesAvailable => words(&[read!(l1_fees_available)]),
        M::L1PricingEquilibrationUnits => words(&[read!(l1_equilibration_units)]),
        M::LastL1PricingUpdateTime => words(&[U256::from(low64(read!(l1_last_update_time)))]),
        M::L1PricingFundsDueForRewards => {
            let due = read!(l1_funds_due_for_rewards);
            match signed(due).0 {
                true => Done::Plain,
                false => words(&[due]),
            }
        }
        M::L1PricingUnitsSinceUpdate => {
            let units = low64(read!(l1_units_since_update)).wrapping_add(ctx.chain.units_added);
            words(&[U256::from(units)])
        }
        M::LastL1PricingSurplus => words(&[read!(l1_last_surplus)]),
        M::MaxBlockGasLimit => words(&[U256::from(low64(read!(l2_per_block_gas_limit)))]),
        M::NotRebuilt(name) => Done::Refused(format!("it calls ArbGasInfo.{name}")),
    })
}


#[derive(Clone, Copy)]
enum Arg {
    Uint256,
    Uint64,
    Uint16,
    Bool,
    Str,
}



pub(crate) fn render_error(data: &[u8]) -> Option<String> {
    use Arg::*;
    let selector: [u8; 4] = data.get(..4)?.try_into().ok()?;
    let (name, kinds): (&str, &[Arg]) = match selector {
        s if s == hex!("f812e656") => ("CallerNotArbOS", &[]),
        s if s == hex!("1753067f") => ("Custom", &[Uint64, Str, Bool]),
        s if s == hex!("3a66de7d") => ("Unused", &[]),
        s if s == hex!("80698456") => ("NoTicketWithID", &[]),
        s if s == hex!("902824c1") => ("NotCallable", &[]),
        s if s == INVALID_BLOCK_NUMBER => ("InvalidBlockNumber", &[Uint256, Uint256]),
        s if s == hex!("27f38212") => ("ProgramNotWasm", &[]),
        s if s == hex!("6f809c4e") => ("ProgramNotActivated", &[]),
        s if s == hex!("637d968f") => ("ProgramNeedsUpgrade", &[Uint16, Uint16]),
        s if s == hex!("c9b12e52") => ("ProgramExpired", &[Uint64]),
        s if s == hex!("cc944bf2") => ("ProgramUpToDate", &[]),
        s if s == hex!("16bd0cf8") => ("ProgramKeepaliveTooSoon", &[Uint64]),
        s if s == hex!("09781ab7") => ("ProgramInsufficientValue", &[Uint256, Uint256]),
        _ => return None,
    };
    let args = &data[4..];
    let word =
        |at: usize| -> Option<U256> { args.get(at..at.checked_add(32)?).map(U256::from_be_slice) };
    let mut shown = Vec::with_capacity(kinds.len());
    for (i, kind) in kinds.iter().enumerate() {
        let w = word(32 * i)?;
        shown.push(match kind {
            Uint256 => w.to_string(),
            Uint64 => u64::try_from(w).ok()?.to_string(),
            Uint16 => u16::try_from(w).ok()?.to_string(),
            Bool => match u8::try_from(w).ok()? {
                0 => "false".to_string(),
                1 => "true".to_string(),
                _ => return None,
            },
            Str => {
                let at = usize::try_from(w).ok()?;
                let len = usize::try_from(word(at)?).ok()?;
                let start = at.checked_add(32)?;
                let bytes = args.get(start..start.checked_add(len)?)?;
                String::from_utf8_lossy(bytes).into_owned()
            }
        });
    }
    Some(format!("error {name}({})", shown.join(", ")))
}
