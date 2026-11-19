






use crate::poster::{self, Message};
use crate::precompiles::{render_error, ArbPrecompiles};
use crate::state::{low64, SLOTS};
use crate::{
    chain, header_arbos_version, rules, ArbChain, Block, BlockOverrides, Call, Measured, Outcome,
    ARBOS_STATE, GAS_CAP, NODE_INTERFACES,
};
use alloy_primitives::{Address, Bytes, B256, U256};
use revm::context::result::HaltReason;
use revm::context::{BlockEnv, CfgEnv, ContextError, Evm, LocalContext, LocalContextTr, TxEnv};
use revm::context_interface::transaction::{AccessList, AccessListItem};
use revm::context_interface::JournalTr;
use revm::handler::instructions::EthInstructions;
use revm::handler::{execution, pre_execution, EthFrame, EvmTr, FrameResult, ItemOrResult};
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::interpreter_action::FrameInit;
use revm::interpreter::{InstructionResult, SharedMemory, SuccessOrHalt};
use revm::primitives::hardfork::SpecId;
use revm::primitives::TxKind;
use revm::{Context, Database, Journal};
use std::collections::HashSet;



pub(crate) struct Arb {

    pub chain: ArbChain,

    pub header_number: u64,

    pub number: U256,

    pub arbos_version: u64,

    pub base_fee: U256,

    pub exec_base_fee: U256,

    pub poster_fee: U256,

    pub units_added: u64,

    pub l1_block: Option<u64>,

    pub blob_outermost: bool,

    pub refused: bool,
}

pub(crate) type NitroCtx<DB> = Context<BlockEnv, TxEnv, CfgEnv, DB, Journal<DB>, Arb>;

type NitroEvm<DB> = Evm<
    NitroCtx<DB>,
    (),
    EthInstructions<EthInterpreter, NitroCtx<DB>>,
    ArbPrecompiles,
    EthFrame<EthInterpreter>,
>;


pub(crate) fn refuse<DB: Database>(ctx: &mut NitroCtx<DB>, why: String) {
    ctx.chain.refused = true;
    ctx.error = Err(ContextError::Custom(why));
}


pub(crate) fn fail<DB: Database>(ctx: &mut NitroCtx<DB>, e: DB::Error) {
    ctx.error = Err(ContextError::Db(e));
}


pub(crate) fn arbos_word<DB: Database>(
    ctx: &mut NitroCtx<DB>,
    slot: B256,
) -> Result<U256, DB::Error> {
    read(&mut ctx.journaled_state.database, slot)
}

fn read<DB: Database>(db: &mut DB, slot: B256) -> Result<U256, DB::Error> {
    db.storage(ARBOS_STATE, U256::from_be_bytes(slot.0))
}


fn level(word: U256) -> u32 {
    word.saturating_to()
}

const BASE_FEE_PAST: &str = "a base fee override past 2^64";


fn base_fee(block: &Block, ov: &BlockOverrides) -> Option<u64> {
    match ov.base_fee {
        Some(b) => u64::try_from(b).ok(),
        None => Some(block.base_fee),
    }
}


struct Fees {
    price: U256,
    cap: U256,
    tip: U256,
}

impl Fees {
    fn of(call: &Call, base: u64) -> Fees {
        if let Some(p) = call.gas_price {
            let p = U256::from(p);
            return Fees {
                price: p,
                cap: p,
                tip: p,
            };
        }
        let cap = U256::from(call.max_fee_per_gas.unwrap_or(0));
        let tip = U256::from(call.max_priority_fee_per_gas.unwrap_or(0));
        let price = match cap.is_zero() && tip.is_zero() {
            true => U256::ZERO,
            false => tip.saturating_add(U256::from(base)).min(cap),
        };
        Fees { price, cap, tip }
    }
}



fn message(call: &Call, tip: U256, fee_cap: U256, gas: u64) -> Message<'_> {
    Message {
        nonce: call.nonce.unwrap_or(0),
        tip,
        fee_cap,
        gas,
        to: call.to,
        value: call.value,
        data: &call.data,
        access_list: &call.access_list,
    }
}


fn intrinsic_gas(call: &Call) -> u64 {
    let zeros = call.data.iter().filter(|b| **b == 0).count() as u64;
    let others = call.data.len() as u64 - zeros;
    let keys: u64 = call.access_list.iter().map(|(_, k)| k.len() as u64).sum();
    let mut gas = match call.to {
        Some(_) => 21_000u64,
        None => 53_000 + 2 * (call.data.len() as u64).div_ceil(32),
    };
    gas = gas
        .saturating_add(zeros.saturating_mul(4))
        .saturating_add(others.saturating_mul(16))
        .saturating_add((call.access_list.len() as u64).saturating_mul(2_400))
        .saturating_add(keys.saturating_mul(1_900));
    gas
}


fn floor_gas(data: &Bytes) -> u64 {
    let zeros = data.iter().filter(|b| **b == 0).count() as u64;
    let others = data.len() as u64 - zeros;
    21_000u64.saturating_add(
        zeros
            .saturating_add(others.saturating_mul(4))
            .saturating_mul(10),
    )
}

fn not_run(outcome: Outcome, intrinsic: bool) -> Measured {
    Measured {
        outcome,
        used: 0,
        peak: 0,
        intrinsic,
    }
}

fn refused<E>(why: impl Into<String>) -> Result<Measured, E> {
    Ok(not_run(Outcome::Refused(why.into()), false))
}


pub(crate) fn posting_gas<DB: Database>(
    db: &mut DB,
    block: &Block,
    call: &Call,
    ov: &BlockOverrides,
) -> Result<Result<u64, String>, DB::Error> {
    let Some(base) = base_fee(block, ov) else {
        return Ok(Err(BASE_FEE_PAST.into()));
    };
    if base == 0 {
        return Ok(Ok(0));
    }
    let level = level(read(db, SLOTS.brotli_level)?);
    let unit_price = read(db, SLOTS.l1_price_per_unit)?;
    let min = read(db, SLOTS.l2_min_base_fee)?;
    let fees = Fees::of(call, base);
    let msg = message(call, fees.tip, fees.cap, call.gas.unwrap_or(0).min(GAS_CAP));
    Ok(poster::data_cost(&msg, level, unit_price, true)
        .map(|(cost, _)| poster::poster_gas(cost, U256::from(base), Some(min))))
}


pub(crate) fn execute<DB: Database>(
    mut db: DB,
    block: &Block,
    call: &Call,
    ov: &BlockOverrides,
    overridden: &HashSet<Address>,
    estimate: Option<u64>,
) -> Result<Measured, DB::Error> {

    let Some(chain) = chain(block.chain_id) else {
        return refused(format!(
            "chain {}'s calls are not run on Nitro here",
            block.chain_id
        ));
    };
    let version = header_arbos_version(&block.mix_hash);
    if rules(version).is_none() || block.difficulty != U256::from(1u64) {
        return refused(format!("ArbOS {version}'s rules are not rebuilt here"));
    }
    let state_version = read(&mut db, SLOTS.version)?;
    if state_version != U256::from(version) {
        return refused(format!(
            "ArbOS's state is at version {state_version}, its header at {version}"
        ));
    }
    let collects_tips = !read(&mut db, SLOTS.collect_tips)?.is_zero();
    if estimate.is_some() && read(&mut db, SLOTS.features)?.bit(0) {
        return refused("ArbOS's increased calldata price (EIP-7623's floor) is not rebuilt here");
    }
    if let Some(to) = call.to.filter(|to| NODE_INTERFACES.contains(to)) {
        return refused(format!(
            "{} is answered by a Nitro node's RPC (NodeInterface), not by its EVM",
            to.to_checksum(None)
        ));
    }
    let Some(base) = base_fee(block, ov) else {
        return refused(BASE_FEE_PAST);
    };
    let fees = Fees::of(call, base);
    if collects_tips && fees.price > U256::from(base) {
        return refused("a call paying a tip where ArbOS collects tips is not rebuilt here");
    }


    let gas = match estimate {
        Some(g) => g,
        None if call.gas.is_none() && !call.skip_l1_charging => {
            let raise = match base {
                0 => 0,
                _ => {
                    let level = level(read(&mut db, SLOTS.brotli_level)?);
                    let unit_price = read(&mut db, SLOTS.l1_price_per_unit)?;
                    let min = read(&mut db, SLOTS.l2_min_base_fee)?;
                    let msg = message(call, fees.tip, fees.cap, GAS_CAP);
                    match poster::data_cost(&msg, level, unit_price, false) {
                        Ok((cost, _)) => poster::poster_gas(cost, U256::from(base), Some(min)),
                        Err(why) => return refused(why),
                    }
                }
            };
            match raise >= u64::MAX - GAS_CAP {
                true => GAS_CAP,
                false => GAS_CAP + raise,
            }
        }
        None => call.gas.map_or(GAS_CAP, |g| g.min(GAS_CAP)),
    };


    let invalid = |text: String, intrinsic: bool| -> Result<Measured, DB::Error> {
        let text = match estimate {
            None => format!("err: {text} (supplied gas {gas})"),
            Some(_) => text,
        };
        Ok(not_run(Outcome::Invalid(text), intrinsic))
    };
    let exec_base = match fees.price.is_zero() {
        true => 0,
        false => base,
    };
    let b = U256::from(exec_base);
    let (price, tip) = match fees.price > b {
        true => (b, U256::ZERO),
        false => (fees.price, fees.tip),
    };
    let from = call.from.to_checksum(None);
    if !(fees.cap.is_zero() && tip.is_zero()) {
        if fees.cap < tip {
            return invalid(
                format!(
                    "max priority fee per gas higher than max fee per gas: address {from}, \
                     maxPriorityFeePerGas: {tip}, maxFeePerGas: {}",
                    fees.cap
                ),
                false,
            );
        }
        if fees.cap < b {
            return invalid(
                format!(
                    "max fee per gas less than block base fee: address {from}, maxFeePerGas: {}, \
                     baseFee: {exec_base}",
                    fees.cap
                ),
                false,
            );
        }
    }
    let gas_word = U256::from(gas);
    let cost = gas_word.checked_mul(price);
    let required = gas_word
        .checked_mul(fees.cap)
        .and_then(|r| r.checked_add(call.value));
    let (Some(cost), Some(required)) = (cost, required) else {
        return invalid(
            format!(
                "insufficient funds for gas * price + value: address {from} required balance \
                 exceeds 256 bits"
            ),
            false,
        );
    };
    let balance = db.basic(call.from)?.map_or(U256::ZERO, |a| a.balance);
    if balance < required {
        return invalid(
            format!(
                "insufficient funds for gas * price + value: address {from} have {balance} want \
                 {required}"
            ),
            false,
        );
    }
    let intrinsic = intrinsic_gas(call);
    if gas < intrinsic {
        return invalid(
            format!("intrinsic gas too low: have {gas}, want {intrinsic}"),
            true,
        );
    }
    if estimate.is_none() {
        let floor = floor_gas(&call.data);
        if gas < floor && read(&mut db, SLOTS.features)?.bit(0) {
            return invalid(
                format!("insufficient gas for floor data gas cost: have {gas}, want {floor}"),
                false,
            );
        }
    }
    let (poster_gas, units_added, poster_fee) = match base != 0 && !call.skip_l1_charging {
        false => (0, 0, U256::ZERO),
        true => {
            let level = level(read(&mut db, SLOTS.brotli_level)?);
            let unit_price = read(&mut db, SLOTS.l1_price_per_unit)?;
            let msg = message(call, tip, fees.cap, gas);
            let (cost, units) = match poster::data_cost(&msg, level, unit_price, estimate.is_some())
            {
                Ok(priced) => priced,
                Err(why) => return refused(why),
            };
            let poster_gas = match estimate {
                None => poster::poster_gas(cost, U256::from(base), None),
                Some(_) => {
                    let min = read(&mut db, SLOTS.l2_min_base_fee)?;
                    poster::poster_gas(cost, U256::from(base), Some(min))
                }
            };
            let fee = U256::from(base).saturating_mul(U256::from(poster_gas));
            (poster_gas, units, fee)
        }
    };
    if gas - intrinsic < poster_gas {
        return invalid("intrinsic gas too low".into(), true);
    }
    let held = match estimate {
        None => 0,
        Some(_) => {
            let limit = low64(read(&mut db, SLOTS.l2_per_tx_gas_limit)?);
            let allowed = limit.saturating_sub(intrinsic);
            (gas - intrinsic - poster_gas).saturating_sub(allowed)
        }
    };
    if !call.value.is_zero() && balance.saturating_sub(cost) < call.value {
        return invalid(
            format!("insufficient funds for transfer: address {from}"),
            false,
        );
    }
    if call.to.is_none() && call.data.len() > chain.max_initcode_size {
        return invalid(
            format!(
                "max initcode size exceeded: code size {} limit {}",
                call.data.len(),
                chain.max_initcode_size
            ),
            false,
        );
    }


    let number = ov.number.unwrap_or(U256::from(block.number));
    let difficulty = ov.difficulty.unwrap_or(U256::from(1u64));
    let block_env = BlockEnv {
        number,
        beneficiary: ov.coinbase.unwrap_or(block.beneficiary),
        timestamp: U256::from(ov.time.unwrap_or(block.timestamp)),
        gas_limit: ov.gas_limit.unwrap_or(block.gas_limit),
        basefee: exec_base,
        difficulty,
        prevrandao: Some(B256::from(difficulty)),
        blob_excess_gas_and_price: None,
    };
    let tx = TxEnv {
        tx_type: 2,
        caller: call.from,
        gas_limit: gas,
        gas_price: price.saturating_to(),
        kind: call.to.map_or(TxKind::Create, TxKind::Call),
        value: call.value,
        data: call.data.clone(),
        chain_id: Some(block.chain_id),
        access_list: AccessList(
            call.access_list
                .iter()
                .map(|(address, keys)| AccessListItem {
                    address: *address,
                    storage_keys: keys.clone(),
                })
                .collect(),
        ),
        gas_priority_fee: Some(tip.saturating_to()),
        ..Default::default()
    };
    let mut cfg = CfgEnv::new_with_spec(SpecId::OSAKA);
    cfg.chain_id = block.chain_id;
    cfg.limit_contract_code_size = Some(chain.max_code_size);
    cfg.limit_contract_initcode_size = Some(chain.max_initcode_size);
    let mut journal = Journal::new(db);
    journal.set_spec_id(SpecId::OSAKA);
    let ctx = Context {
        block: block_env,
        tx,
        cfg,
        journaled_state: journal,
        chain: Arb {
            chain: *chain,
            header_number: block.number,
            number,
            arbos_version: version,
            base_fee: U256::from(base),
            exec_base_fee: b,
            poster_fee,
            units_added,
            l1_block: None,
            blob_outermost: false,
            refused: false,
        },
        local: LocalContext::default(),
        error: Ok(()),
    };
    let mut instructions = EthInstructions::new_mainnet();
    crate::ops::install(&mut instructions);
    let mut evm: NitroEvm<DB> =
        Evm::new(ctx, instructions, ArbPrecompiles::new(overridden.clone()));


    {
        let journal = &mut evm.ctx.journaled_state;
        let is_call = call.to.is_some();
        let account = journal.load_account_code(call.from)?.data;
        let before = account.info.balance;
        account.info.balance = before.saturating_sub(cost);
        if is_call {
            account.info.nonce = account.info.nonce.saturating_add(1);
        }
        account.mark_touch();
        journal.caller_accounting_journal_entry(call.from, before, is_call);
    }

    pre_execution::load_accounts::<_, DB::Error>(&mut evm)?;

    let frame_gas = gas - intrinsic - poster_gas - held;
    let first = FrameInit {
        depth: 0,
        memory: SharedMemory::new_with_buffer(evm.ctx.local.shared_memory_buffer().clone()),
        frame_input: execution::create_init_frame(&evm.ctx.tx, frame_gas),
    };
    let result = match drive(&mut evm, first) {
        Ok(result) => result,
        Err(Stop::State(e)) => return Err(e),
        Err(Stop::Refused(why)) => return refused(why),
        Err(Stop::Internal(why)) => return Ok(not_run(Outcome::Internal(why), false)),
    };


    let ended = result.interpreter_result();
    let output = ended.output.clone();
    let spent = frame_gas.saturating_sub(ended.gas.remaining());
    let refund = u64::try_from(ended.gas.refunded()).unwrap_or(0);
    let ran = intrinsic + poster_gas + spent;
    let halted = (gas - held, gas - held);
    let (outcome, (used, peak)) = match SuccessOrHalt::<HaltReason>::from(ended.result) {
        SuccessOrHalt::Success(_) => {
            let refund = refund.min((intrinsic + spent) / 5);
            (Outcome::Returned(output), (ran - refund, ran))
        }
        SuccessOrHalt::Revert => {
            let reason = render_error(&output);
            (Outcome::Reverted(output, reason), (ran, ran))
        }
        SuccessOrHalt::Halt(_)
            if evm.ctx.chain.blob_outermost
                && ended.result == InstructionResult::OpcodeNotFound =>
        {
            (
                Outcome::HaltedSaying("BLOBBASEFEE is not supported on Arbitrum".into()),
                halted,
            )
        }
        SuccessOrHalt::Halt(reason) => (Outcome::Halted(reason), halted),
        SuccessOrHalt::FatalExternalError => {
            (Outcome::Internal("a fatal external error".into()), (0, 0))
        }
        SuccessOrHalt::Internal(why) => (Outcome::Internal(format!("{why:?}")), (0, 0)),
    };
    Ok(Measured {
        outcome,
        used,
        peak,
        intrinsic: false,
    })
}


enum Stop<E> {
    State(E),
    Refused(String),
    Internal(String),
}



fn stopped<DB: Database>(
    evm: &mut NitroEvm<DB>,
    given: Option<ContextError<DB::Error>>,
) -> Stop<DB::Error> {
    let held = std::mem::replace(&mut evm.ctx.error, Ok(())).err();
    match held.or(given) {
        Some(ContextError::Db(e)) => Stop::State(e),
        Some(ContextError::Custom(why)) if evm.ctx.chain.refused => Stop::Refused(why),
        Some(ContextError::Custom(why)) => Stop::Internal(why),
        None => Stop::Internal("the run stopped without a cause".into()),
    }
}



fn drive<DB: Database>(
    evm: &mut NitroEvm<DB>,
    first: FrameInit,
) -> Result<FrameResult, Stop<DB::Error>> {
    let began = match evm.frame_init(first) {
        Ok(ItemOrResult::Item(_)) => None,
        Ok(ItemOrResult::Result(result)) => Some(result),
        Err(e) => return Err(stopped(evm, Some(e))),
    };
    if evm.ctx.error.is_err() {
        return Err(stopped(evm, None));
    }
    if let Some(result) = began {
        return Ok(result);
    }
    loop {
        let next = match evm.frame_run() {
            Ok(next) => next,
            Err(e) => return Err(stopped(evm, Some(e))),
        };
        if evm.ctx.error.is_err() {
            return Err(stopped(evm, None));
        }
        let result = match next {
            ItemOrResult::Item(init) => {
                let made = match evm.frame_init(init) {
                    Ok(ItemOrResult::Item(_)) => None,
                    Ok(ItemOrResult::Result(result)) => Some(result),
                    Err(e) => return Err(stopped(evm, Some(e))),
                };
                if evm.ctx.error.is_err() {
                    return Err(stopped(evm, None));
                }
                match made {
                    None => continue,
                    Some(result) => result,
                }
            }
            ItemOrResult::Result(result) => result,
        };
        match evm.frame_return_result(result) {
            Ok(Some(result)) => return Ok(result),
            Ok(None) => {}
            Err(e) => return Err(stopped(evm, Some(e))),
        }
    }
}
