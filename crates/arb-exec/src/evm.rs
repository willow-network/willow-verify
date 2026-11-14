//! Nitro's `eth_call` on revm: the message as Nitro's RPC builds it (`ToMessage`, its L1 posting
//! gas), the state transition's checks and charges as Nitro's geth runs them (the tip dropped,
//! the base fee checked, gas bought, intrinsic gas, then L1 data gas), and Nitro's opcodes. The
//! same run in Nitro's gas estimation mode ([`Mode::Estimate`]) is one of `eth_estimateGas`'s:
//! the L1 data priced as an estimate prices it, the computation held to ArbOS's per-transaction
//! gas limit, and the gas used counted as Nitro counts it (the L1 data's gas never refunded).

use crate::precompiles::Precompiles;
use crate::state::{ARBOS_STATE, SLOTS};
use crate::{
    poster, rules, Block, BlockOverrides, Call, Outcome, GAS_CAP, MAX_CODE_SIZE, MAX_INITCODE_SIZE,
    ROBINHOOD,
};
use alloy_primitives::{Address, Bytes, B256, U256, U512};
use revm::bytecode::opcode;
use revm::context::result::{EVMError, ExecutionResult, HaltReason, Output};
use revm::context::{BlockEnv, CfgEnv, Context, Evm, Journal, LocalContext, TxEnv};
use revm::context_interface::context::ContextError;
use revm::context_interface::transaction::{AccessList, AccessListItem};
use revm::context_interface::JournalTr;
use revm::handler::instructions::EthInstructions;
use revm::handler::{EthFrame, EvmTr, FrameResult, Handler};
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::interpreter_types::{InputsTr, LegacyBytecode, MemoryTr};
use revm::interpreter::{InitialAndFloorGas, Instruction, InstructionContext, InstructionResult};
use revm::primitives::TxKind;
use revm::Database;
use std::collections::HashSet;

pub(crate) type Ctx<DB> = Context<BlockEnv, TxEnv, CfgEnv, DB, Journal<DB>, Chain>;
type Insts<DB> = EthInstructions<EthInterpreter, Ctx<DB>>;
type ArbEvm<DB> = Evm<Ctx<DB>, (), Insts<DB>, Precompiles, EthFrame<EthInterpreter>>;

/// Why a run stopped before the EVM could answer.
#[derive(Clone, Debug)]
pub(crate) enum Stop {
    Refused(String),
    Invalid(String),
}

/// How a message is run: as `eth_call` runs it, or as one of `eth_estimateGas`'s runs at a gas
/// limit (Nitro's `MessageGasEstimationMode`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Call,
    Estimate(u64),
}

/// The message as Nitro's RPC makes it of the call (`TransactionArgs.ToMessage`).
#[derive(Clone, Debug, Default)]
pub(crate) struct Msg {
    from: Address,
    to: Option<Address>,
    gas: u64,
    price: U256,
    fee_cap: U256,
    tip: U256,
    value: U256,
    data: Bytes,
    access_list: Vec<(Address, Vec<B256>)>,
    nonce: u64,
    skip_l1: bool,
}

/// What a call's run keeps beside the EVM: Nitro's `TxProcessor` and block context fields.
#[derive(Default)]
pub(crate) struct Chain {
    pub version: u64,
    /// The block's own number (block hashes above it read as zero) and the number calls see.
    pub header_number: u64,
    pub block_number: U256,
    /// `BaseFeeInBlock`: the block's base fee, where a call with no price runs at zero.
    pub base_fee_in_block: Option<U256>,
    /// The L1 fee this call was charged (`PosterFee`) and the L1 units it added.
    pub poster_fee: U256,
    pub units_added: u64,
    l1_block_number: Option<u64>,
    msg: Msg,
    deduct: U256,
    pub stop: Option<Stop>,
    blob_halt_top: bool,
    /// An estimate's run: the L1 data's gas (`posterGas`, never refunded), the gas held back from
    /// the computation (`computeHoldGas`, returned unused), and whether the run failed for its
    /// intrinsic gas (`ErrIntrinsicGas`: the search raises the gas).
    estimating: bool,
    poster_gas: u64,
    held: u64,
    intrinsic_failed: bool,
}

/// `slot` of ArbOS's state; a failed read is the context's error.
pub(crate) fn arbos_slot<DB: Database>(ctx: &mut Ctx<DB>, slot: B256) -> Option<U256> {
    match ctx
        .journaled_state
        .database
        .storage(ARBOS_STATE, slot.into())
    {
        Ok(v) => Some(v),
        Err(e) => {
            ctx.error = Err(ContextError::Db(e));
            None
        }
    }
}

/// Stops the run: not answered here.
pub(crate) fn refuse<DB: Database>(ctx: &mut Ctx<DB>, why: String) {
    ctx.error = Err(ContextError::Custom(why.clone()));
    ctx.chain.stop = Some(Stop::Refused(why));
}

/// The L1 block number ArbOS keeps (what NUMBER reads), read once per call.
fn l1_block_number<DB: Database>(ctx: &mut Ctx<DB>) -> Option<u64> {
    if let Some(n) = ctx.chain.l1_block_number {
        return Some(n);
    }
    let n = arbos_slot(ctx, SLOTS.l1_block_number)?.as_limbs()[0];
    ctx.chain.l1_block_number = Some(n);
    Some(n)
}

/// A Stylus program Nitro would run as WASM (from ArbOS 60, a root of fragments too).
fn stylus_program(code: &[u8]) -> bool {
    code.len() > 3 && code[..2] == [0xEF, 0xF0] && matches!(code[2], 0x00 | 0x02)
}

/// Code Nitro keeps as a Stylus component where an EVM refuses code starting 0xEF.
fn stylus_component(code: &[u8]) -> bool {
    code.len() > 3 && code[..2] == [0xEF, 0xF0] && matches!(code[2], 0x00..=0x02)
}

type Ic<'a, DB> = InstructionContext<'a, Ctx<DB>, EthInterpreter>;

fn push<DB: Database>(c: Ic<'_, DB>, v: U256) {
    if !c.interpreter.stack.push(v) {
        c.interpreter.halt(InstructionResult::StackOverflow);
    }
}

/// NUMBER: the L1 block number ArbOS keeps, not the L2 block's.
fn number<DB: Database>(c: Ic<'_, DB>) {
    match l1_block_number(c.host) {
        Some(n) => push(c, U256::from(n)),
        None => c.interpreter.halt(InstructionResult::FatalExternalError),
    }
}

/// BLOCKHASH: one of the 256 L1 block hashes ArbOS keeps, below the L1 block number it keeps.
fn blockhash<DB: Database>(c: Ic<'_, DB>) {
    let Ok(n) = c.interpreter.stack.pop() else {
        return c.interpreter.halt(InstructionResult::StackUnderflow);
    };
    let hash = match u64::try_from(n) {
        Err(_) => Some(U256::ZERO),
        Ok(n) => l1_block_number(c.host).and_then(|upper| {
            let lower = upper.saturating_sub(256);
            match n >= lower && n < upper {
                true => arbos_slot(c.host, SLOTS.l1_block_hash(n)),
                false => Some(U256::ZERO),
            }
        }),
    };
    match hash {
        Some(h) => push(c, h),
        None => c.interpreter.halt(InstructionResult::FatalExternalError),
    }
}

/// GASPRICE: the price paid, which with tips dropped is the base fee (`GasPriceOp`).
fn gasprice<DB: Database>(c: Ic<'_, DB>) {
    let fee = U256::from(c.host.block.basefee);
    push(c, fee)
}

/// BLOBBASEFEE: Nitro's EVM fails it.
fn blobbasefee<DB: Database>(c: Ic<'_, DB>) {
    if c.host.journaled_state.depth() == 1 {
        c.host.chain.blob_halt_top = true;
    }
    c.interpreter.halt(InstructionResult::OpcodeNotFound);
}

/// 0xEF: a Stylus program starts with it, and Nitro runs it as WASM, which isn't rebuilt here.
fn ef<DB: Database>(c: Ic<'_, DB>) {
    if stylus_program(c.interpreter.bytecode.bytecode_slice()) {
        let at = c
            .interpreter
            .input
            .bytecode_address()
            .copied()
            .unwrap_or_default();
        refuse(c.host, format!("it runs the Stylus program at {at}"));
        return c.interpreter.halt(InstructionResult::FatalExternalError);
    }
    c.interpreter.halt(InstructionResult::OpcodeNotFound);
}

/// 0xFE: what an ArbOS precompile's account holds. Nitro runs an account delegated to a
/// precompile as empty code (from ArbOS 50); here it would run this, so that is refused.
fn invalid_fe<DB: Database>(c: Ic<'_, DB>) {
    let delegated = c.interpreter.input.bytecode_address().and_then(|at| {
        match &c.host.journaled_state.state.get(at)?.info.code {
            Some(revm::bytecode::Bytecode::Eip7702(d)) => Some(d.delegated_address),
            _ => None,
        }
    });
    if let Some(to) = delegated.filter(|d| crate::precompiles::ARBOS.contains(d)) {
        refuse(
            c.host,
            format!("it runs an account delegated to the ArbOS precompile {to}"),
        );
        return c.interpreter.halt(InstructionResult::FatalExternalError);
    }
    c.interpreter.halt(InstructionResult::InvalidFEOpcode);
}

/// RETURN: a creation returning a Stylus component deploys it on Nitro (an EVM refuses it).
fn ret<DB: Database>(c: Ic<'_, DB>) {
    if c.interpreter.input.bytecode_address().is_none() {
        let s = &c.interpreter.stack;
        if let (Ok(offset), Ok(len)) = (s.peek(0), s.peek(1)) {
            if len > U256::from(3) {
                let size = c.interpreter.memory.size();
                let head: Vec<u8> = (0..4u64)
                    .map(
                        |i| match usize::try_from(offset.saturating_add(U256::from(i))) {
                            Ok(at) if at < size => c.interpreter.memory.slice_len(at, 1)[0],
                            _ => 0,
                        },
                    )
                    .collect();
                if stylus_component(&head) {
                    refuse(c.host, "it deploys a Stylus program".into());
                    return c.interpreter.halt(InstructionResult::FatalExternalError);
                }
            }
        }
    }
    revm::interpreter::instructions::control::ret(c)
}

fn instructions<DB: Database>() -> Insts<DB> {
    let mut i = EthInstructions::new_mainnet();
    i.insert_instruction(opcode::NUMBER, Instruction::new(number::<DB>, 2));
    i.insert_instruction(opcode::BLOCKHASH, Instruction::new(blockhash::<DB>, 20));
    i.insert_instruction(opcode::GASPRICE, Instruction::new(gasprice::<DB>, 2));
    i.insert_instruction(opcode::BLOBBASEFEE, Instruction::new(blobbasefee::<DB>, 2));
    i.insert_instruction(0xEF, Instruction::new(ef::<DB>, 0));
    i.insert_instruction(opcode::INVALID, Instruction::new(invalid_fe::<DB>, 0));
    i.insert_instruction(opcode::RETURN, Instruction::new(ret::<DB>, 0));
    i
}

/// Nitro's run of the message: checks and charges before the EVM, none after (a call's answer
/// doesn't see its refund or fees).
struct ArbHandler<DB>(std::marker::PhantomData<DB>);

type Fail<DB> = EVMError<<DB as Database>::Error>;

/// Stops the run: the message is invalid, as Nitro words it (`applyMessageWithEVM`); an
/// estimate's run keeps the state transition's own words (its search wraps them).
fn invalid<DB: Database>(ctx: &mut Ctx<DB>, why: String) -> Fail<DB> {
    let m = match ctx.chain.estimating {
        true => why,
        false => format!("err: {why} (supplied gas {})", ctx.chain.msg.gas),
    };
    ctx.chain.stop = Some(Stop::Invalid(m.clone()));
    EVMError::Custom(m)
}

/// [`invalid`] for too little gas to start (`ErrIntrinsicGas`).
fn intrinsic_too_low<DB: Database>(ctx: &mut Ctx<DB>, why: String) -> Fail<DB> {
    ctx.chain.intrinsic_failed = true;
    invalid(ctx, why)
}

impl<DB: Database> Handler for ArbHandler<DB> {
    type Evm = ArbEvm<DB>;
    type Error = Fail<DB>;
    type HaltReason = HaltReason;

    /// The state transition's checks and charges before the EVM runs, in Nitro's order.
    fn validate(&self, evm: &mut Self::Evm) -> Result<InitialAndFloorGas, Self::Error> {
        let ctx = evm.ctx();
        let m = ctx.chain.msg.clone();
        let from = m.from.to_checksum(None);
        let base_fee = U256::from(ctx.block.basefee);
        // Tips are dropped (ArbOS collects none here): a price above the base fee is the base fee.
        let (price, tip) = match m.price > base_fee {
            true => (base_fee, U256::ZERO),
            false => (m.price, m.tip),
        };
        if !(m.fee_cap.is_zero() && tip.is_zero()) {
            if m.fee_cap < tip {
                return Err(invalid(
                    ctx,
                    format!(
                        "max priority fee per gas higher than max fee per gas: address {from}, \
                         maxPriorityFeePerGas: {tip}, maxFeePerGas: {}",
                        m.fee_cap
                    ),
                ));
            }
            if m.fee_cap < base_fee {
                return Err(invalid(
                    ctx,
                    format!(
                        "max fee per gas less than block base fee: address {from}, maxFeePerGas: \
                         {}, baseFee: {base_fee}",
                        m.fee_cap
                    ),
                ));
            }
        }
        // Gas bought at the price, the balance checked against the fee cap and the value.
        let gas = U512::from(m.gas);
        let (cost, check) = (
            gas * U512::from(price),
            gas * U512::from(m.fee_cap) + U512::from(m.value),
        );
        let (true, true) = (cost.bit_len() <= 256, check.bit_len() <= 256) else {
            return Err(invalid(
                ctx,
                format!(
                    "insufficient funds for gas * price + value: address {from} required balance \
                     exceeds 256 bits"
                ),
            ));
        };
        let (cost, check) = (poster::narrow(cost), poster::narrow(check));
        let balance = ctx.journaled_state.load_account(m.from)?.data.info.balance;
        if balance < check {
            return Err(invalid(
                ctx,
                format!(
                    "insufficient funds for gas * price + value: address {from} have {balance} \
                     want {check}"
                ),
            ));
        }
        ctx.chain.deduct = cost;
        let intrinsic =
            revm::interpreter::gas::calculate_initial_tx_gas_for_tx(&ctx.tx, ctx.cfg.spec)
                .initial_gas;
        if m.gas < intrinsic {
            return Err(intrinsic_too_low(
                ctx,
                format!("intrinsic gas too low: have {}, want {intrinsic}", m.gas),
            ));
        }
        // L1 data, as gas at the price paid (the base fee, or the block's where that is zero).
        let in_block = ctx.chain.base_fee_in_block.unwrap_or(base_fee);
        let mut poster_gas = 0;
        if !in_block.is_zero() && !m.skip_l1 {
            let paid = match base_fee.is_zero() {
                true => in_block,
                false => base_fee,
            };
            let (Some(level), Some(per_unit)) = (
                arbos_slot(ctx, SLOTS.brotli_level),
                arbos_slot(ctx, SLOTS.l1_price_per_unit),
            ) else {
                return Err(ctx_err(ctx));
            };
            let message = poster::Message {
                nonce: m.nonce,
                tip,
                fee_cap: m.fee_cap,
                gas: m.gas,
                to: m.to,
                value: m.value,
                data: &m.data,
                access_list: &m.access_list,
            };
            let level = u32::try_from(level).unwrap_or(u32::MAX);
            let estimating = ctx.chain.estimating;
            let (cost, units) = match poster::data_cost(&message, level, per_unit, estimating) {
                Ok(c) => c,
                Err(why) => {
                    refuse(ctx, why.clone());
                    return Err(EVMError::Custom(why));
                }
            };
            // An estimate prices it at 7/8 of the price (at least the minimum base fee), padded.
            let floor = match estimating {
                true => match arbos_slot(ctx, SLOTS.l2_min_base_fee) {
                    Some(f) => Some(f),
                    None => return Err(ctx_err(ctx)),
                },
                false => None,
            };
            ctx.chain.units_added = units;
            poster_gas = poster::poster_gas(cost, paid, floor);
            ctx.chain.poster_fee = paid.saturating_mul(U256::from(poster_gas));
        }
        if m.gas - intrinsic < poster_gas {
            return Err(intrinsic_too_low(ctx, "intrinsic gas too low".into()));
        }
        ctx.chain.poster_gas = poster_gas;
        // Outside eth_call, the computation gets at most ArbOS's per-transaction gas limit less
        // the intrinsic gas (from ArbOS 50; only 61 is run here); the rest is held, and returned.
        let mut held = 0;
        if ctx.chain.estimating {
            let Some(max) = arbos_slot(ctx, SLOTS.l2_per_tx_gas_limit) else {
                return Err(ctx_err(ctx));
            };
            let max = max.as_limbs()[0].saturating_sub(intrinsic);
            held = (m.gas - intrinsic - poster_gas).saturating_sub(max);
        }
        ctx.chain.held = held;
        if !m.value.is_zero() && balance - cost < m.value {
            return Err(invalid(
                ctx,
                format!("insufficient funds for transfer: address {from}"),
            ));
        }
        if m.to.is_none() && m.data.len() > MAX_INITCODE_SIZE {
            return Err(invalid(
                ctx,
                format!(
                    "max initcode size exceeded: code size {} limit {MAX_INITCODE_SIZE}",
                    m.data.len()
                ),
            ));
        }
        Ok(InitialAndFloorGas::new(intrinsic + poster_gas + held, 0))
    }

    /// An estimate's refund as Nitro counts it (`calcRefund`): at most a fifth of the gas used
    /// before refunds less the L1 data's gas (`NonrefundableGas`), the held gas not used. A call's
    /// as revm's (what it returns doesn't depend on it).
    fn refund(&self, evm: &mut Self::Evm, exec_result: &mut FrameResult, eip7702_refund: i64) {
        let chain = &evm.ctx().chain;
        let (estimating, poster_gas, held) = (chain.estimating, chain.poster_gas, chain.held);
        let spec = evm.ctx().cfg.spec;
        let gas = exec_result.gas_mut();
        if !estimating {
            return revm::handler::post_execution::refund(spec, gas, eip7702_refund);
        }
        gas.record_refund(eip7702_refund);
        let before = gas.spent().saturating_sub(held);
        let cap = before.saturating_sub(poster_gas) / 5;
        let counter = u64::try_from(gas.refunded()).unwrap_or(0);
        gas.set_refund(i64::try_from(counter.min(cap)).unwrap_or(i64::MAX));
    }

    /// Gas bought (balance less gas times price), and the sender's nonce bumped for a call.
    fn validate_against_state_and_deduct_caller(
        &self,
        evm: &mut Self::Evm,
    ) -> Result<(), Self::Error> {
        let ctx = evm.ctx();
        let (caller, is_call, deduct) = (ctx.tx.caller, ctx.tx.kind.is_call(), ctx.chain.deduct);
        let journal = &mut ctx.journaled_state;
        let account = journal.load_account_code(caller)?.data;
        let balance = account.info.balance;
        account.info.balance = balance.saturating_sub(deduct);
        account.mark_touch();
        if is_call {
            account.info.nonce = account.info.nonce.saturating_add(1);
        }
        journal.caller_accounting_journal_entry(caller, balance, is_call);
        Ok(())
    }

    fn reimburse_caller(&self, _: &mut Self::Evm, _: &mut FrameResult) -> Result<(), Self::Error> {
        Ok(())
    }

    fn reward_beneficiary(
        &self,
        _: &mut Self::Evm,
        _: &mut FrameResult,
    ) -> Result<(), Self::Error> {
        Ok(())
    }
}

/// The context's error, taken.
fn ctx_err<DB: Database>(ctx: &mut Ctx<DB>) -> Fail<DB> {
    match std::mem::replace(&mut ctx.error, Ok(())) {
        Err(ContextError::Db(e)) => EVMError::Database(e),
        Err(ContextError::Custom(m)) => EVMError::Custom(m),
        Ok(()) => EVMError::Custom("a read failed".into()),
    }
}

/// `RPCPostingGasHook`: the gas a call that names none gets for its L1 data on top of the cap,
/// at the header's base fee as a gas estimate prices it (`Err` within: refused, and why). The
/// stand-in transaction is the message's own (`estimating`: an estimate's, whose gas is filled).
fn posting_gas<DB: Database>(
    db: &mut DB,
    m: &Msg,
    header_base_fee: U256,
    estimating: bool,
) -> Result<Result<u64, String>, DB::Error> {
    if header_base_fee.is_zero() {
        return Ok(Ok(0));
    }
    let s = &*SLOTS;
    let level = db.storage(ARBOS_STATE, s.brotli_level.into())?;
    let per_unit = db.storage(ARBOS_STATE, s.l1_price_per_unit.into())?;
    let min_base_fee = db.storage(ARBOS_STATE, s.l2_min_base_fee.into())?;
    let message = poster::Message {
        nonce: m.nonce,
        tip: m.tip,
        fee_cap: m.fee_cap,
        gas: m.gas,
        to: m.to,
        value: m.value,
        data: &m.data,
        access_list: &m.access_list,
    };
    let level = u32::try_from(level).unwrap_or(u32::MAX);
    Ok(poster::data_cost(&message, level, per_unit, estimating)
        .map(|(cost, _)| poster::poster_gas(cost, header_base_fee, Some(min_base_fee))))
}

pub(crate) fn run<DB: Database>(
    db: DB,
    block: &Block,
    call: &Call,
    o: &BlockOverrides,
    overridden: &HashSet<Address>,
) -> Result<Outcome, DB::Error> {
    run_as(db, block, call, o, overridden, Mode::Call).map(|m| m.outcome)
}

/// What a run came to, with the gas it took as Nitro counts it.
pub(crate) fn run_as<DB: Database>(
    mut db: DB,
    block: &Block,
    call: &Call,
    o: &BlockOverrides,
    overridden: &HashSet<Address>,
    mode: Mode,
) -> Result<crate::Measured, DB::Error> {
    let not_run = |outcome| crate::Measured::not_run(outcome, false);
    let version = crate::header_arbos_version(&block.mix_hash);
    let Some(spec) = rules(version).filter(|_| block.difficulty == U256::from(1)) else {
        return Ok(not_run(Outcome::Refused(format!(
            "ArbOS {version}'s rules are not rebuilt here"
        ))));
    };
    let s = &*SLOTS;
    let in_state = db.storage(ARBOS_STATE, s.version.into())?;
    if in_state != U256::from(version) {
        return Ok(not_run(Outcome::Refused(format!(
            "ArbOS's state is at version {in_state}, its header at {version}"
        ))));
    }
    if !db.storage(ARBOS_STATE, s.collect_tips.into())?.is_zero() {
        return Ok(not_run(Outcome::Refused(
            "ArbOS collects tips at this block".into(),
        )));
    }
    // EIP-7623's floor applies where ArbOS's increased calldata price is on (it is off on
    // Robinhood Chain): not rebuilt for an estimate, whose gas used it would change.
    if matches!(mode, Mode::Estimate(_)) && db.storage(ARBOS_STATE, s.features.into())?.bit(0) {
        return Ok(not_run(Outcome::Refused(
            "ArbOS's increased calldata price (EIP-7623's floor) is not rebuilt here".into(),
        )));
    }
    if let Some(a) = call.to.filter(|a| crate::NODE_INTERFACES.contains(a)) {
        return Ok(not_run(Outcome::Refused(format!(
            "{a} is answered by a Nitro node's RPC (NodeInterface), not by its EVM"
        ))));
    }
    // The block as the call sees it: the header's fields under the caller's overrides.
    let base_fee = match o.base_fee.map(u64::try_from) {
        None => U256::from(block.base_fee),
        Some(Ok(b)) => U256::from(b),
        Some(Err(_)) => {
            return Ok(not_run(Outcome::Refused(
                "a base fee override past 2^64".into(),
            )))
        }
    };
    let difficulty = o.difficulty.unwrap_or(U256::from(1));
    let number = o.number.unwrap_or(U256::from(block.number));
    let (price, fee_cap, tip) = message_price(call, base_fee);
    let mut msg = Msg {
        from: call.from,
        to: call.to,
        gas: match mode {
            Mode::Call => call.gas.unwrap_or(GAS_CAP).min(GAS_CAP),
            Mode::Estimate(gas) => gas,
        },
        price,
        fee_cap,
        tip,
        value: call.value,
        data: call.data.clone(),
        access_list: call.access_list.clone(),
        nonce: call.nonce.unwrap_or(0),
        skip_l1: call.skip_l1_charging,
    };
    // A call that names no gas gets its L1 data's on top of the cap (an estimate's runs: the gas
    // the search tries).
    if mode == Mode::Call && call.gas.is_none() && !msg.skip_l1 {
        let posting = match posting_gas(&mut db, &msg, base_fee, false)? {
            Ok(p) => p,
            Err(why) => return Ok(not_run(Outcome::Refused(why))),
        };
        if GAS_CAP < u64::MAX - posting {
            msg.gas = GAS_CAP + posting;
        }
    }
    // A call with no price runs at a zero base fee; the block's is kept for what reads it.
    let (block_base_fee, base_fee_in_block) = match msg.price.is_zero() {
        true => (0, Some(base_fee)),
        false => (base_fee.to::<u64>(), None),
    };
    let block_env = BlockEnv {
        number,
        beneficiary: o.coinbase.unwrap_or(block.beneficiary),
        timestamp: U256::from(o.time.unwrap_or(block.timestamp)),
        gas_limit: o.gas_limit.unwrap_or(block.gas_limit),
        basefee: block_base_fee,
        difficulty,
        prevrandao: Some(B256::from(difficulty)),
        blob_excess_gas_and_price: None,
    };
    let mut cfg = CfgEnv::new_with_spec(spec);
    cfg.chain_id = ROBINHOOD;
    cfg.limit_contract_code_size = Some(MAX_CODE_SIZE);
    cfg.limit_contract_initcode_size = Some(MAX_INITCODE_SIZE);
    cfg.disable_nonce_check = true;
    cfg.disable_eip3607 = true;
    cfg.disable_block_gas_limit = true;
    cfg.disable_base_fee = true;
    cfg.tx_gas_limit_cap = Some(u64::MAX);
    let tx = TxEnv {
        tx_type: 2,
        caller: msg.from,
        gas_limit: msg.gas,
        gas_price: u128::try_from(msg.price).unwrap_or(u128::MAX),
        gas_priority_fee: Some(u128::try_from(msg.tip).unwrap_or(u128::MAX)),
        kind: msg.to.map_or(TxKind::Create, TxKind::Call),
        value: msg.value,
        data: msg.data.clone(),
        chain_id: Some(ROBINHOOD),
        access_list: AccessList(
            msg.access_list
                .iter()
                .map(|(address, keys)| AccessListItem {
                    address: *address,
                    storage_keys: keys.clone(),
                })
                .collect(),
        ),
        ..Default::default()
    };
    let chain = Chain {
        version,
        header_number: block.number,
        block_number: number,
        base_fee_in_block,
        msg,
        estimating: matches!(mode, Mode::Estimate(_)),
        ..Default::default()
    };
    let mut journaled_state = Journal::new(db);
    journaled_state.set_spec_id(spec);
    let ctx: Ctx<DB> = Context {
        block: block_env,
        tx,
        cfg,
        journaled_state,
        chain,
        local: LocalContext::default(),
        error: Ok(()),
    };
    let mut evm = Evm::new(
        ctx,
        instructions::<DB>(),
        Precompiles::new(spec, overridden),
    );
    let out = ArbHandler(std::marker::PhantomData).run(&mut evm);
    let chain = &mut evm.ctx.chain;
    // The gas used as Nitro counts it (after refunds, the held gas returned) and before refunds
    // (`MaxUsedGas`).
    let (used, spent) = match &out {
        Ok(ExecutionResult::Success {
            gas_used,
            gas_refunded,
            ..
        }) => (*gas_used, gas_used.saturating_add(*gas_refunded)),
        Ok(ExecutionResult::Revert { gas_used, .. } | ExecutionResult::Halt { gas_used, .. }) => {
            (*gas_used, *gas_used)
        }
        Err(_) => (0, 0),
    };
    let (held, intrinsic) = (chain.held, chain.intrinsic_failed);
    let outcome = match out {
        Ok(ExecutionResult::Success { output, .. }) => Outcome::Returned(match output {
            Output::Call(b) | Output::Create(b, _) => b,
        }),
        Ok(ExecutionResult::Revert { output, .. }) => {
            let said = crate::precompiles::render_error(&output);
            Outcome::Reverted(output, said)
        }
        Ok(ExecutionResult::Halt { .. }) if chain.blob_halt_top => {
            Outcome::HaltedSaying("BLOBBASEFEE is not supported on Arbitrum".into())
        }
        Ok(ExecutionResult::Halt { reason, .. }) => Outcome::Halted(reason),
        Err(EVMError::Database(e)) => return Err(e),
        Err(e) => match chain.stop.take() {
            Some(Stop::Refused(m)) => Outcome::Refused(m),
            Some(Stop::Invalid(m)) => Outcome::Invalid(m),
            None => Outcome::Internal(e.to_string()),
        },
    };
    Ok(crate::Measured {
        outcome,
        used: used.saturating_sub(held),
        peak: spent.saturating_sub(held),
        intrinsic,
    })
}

/// `RPCPostingGasHook` as `eth_estimateGas` calls it (`DoEstimateGas`): the gas the RPC's gas cap
/// is raised by, the call's L1 data priced as an estimate prices it at the header's base fee (the
/// caller's override of it included), whether or not the call skips L1 charging. `Err` within:
/// refused, and why.
pub(crate) fn estimate_posting_gas<DB: Database>(
    db: &mut DB,
    block: &Block,
    call: &Call,
    o: &BlockOverrides,
) -> Result<Result<u64, String>, DB::Error> {
    let base_fee = match o.base_fee.map(u64::try_from) {
        None => U256::from(block.base_fee),
        Some(Ok(b)) => U256::from(b),
        Some(Err(_)) => return Ok(Err("a base fee override past 2^64".into())),
    };
    let (_, fee_cap, tip) = message_price(call, base_fee);
    let msg = Msg {
        from: call.from,
        to: call.to,
        gas: call.gas.unwrap_or(0).min(GAS_CAP),
        fee_cap,
        tip,
        value: call.value,
        data: call.data.clone(),
        access_list: call.access_list.clone(),
        nonce: call.nonce.unwrap_or(0),
        ..Default::default()
    };
    posting_gas(db, &msg, base_fee, true)
}

/// The message's price, fee cap and tip (`ToMessage`): a legacy price is all three; else the tip
/// over the base fee up to the cap, or zero when neither is named.
fn message_price(call: &Call, base_fee: U256) -> (U256, U256, U256) {
    match call.gas_price {
        Some(p) => (U256::from(p), U256::from(p), U256::from(p)),
        None => {
            let cap = U256::from(call.max_fee_per_gas.unwrap_or(0));
            let tip = U256::from(call.max_priority_fee_per_gas.unwrap_or(0));
            let price = match cap.is_zero() && tip.is_zero() {
                true => U256::ZERO,
                false => tip.saturating_add(base_fee).min(cap),
            };
            (price, cap, tip)
        }
    }
}
