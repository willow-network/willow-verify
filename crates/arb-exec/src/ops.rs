







use crate::exec::{arbos_word, fail, refuse, NitroCtx};
use crate::precompiles::ARBOS_PRECOMPILES;
use crate::state::{low64, SLOTS};
use revm::bytecode::opcode;
use revm::bytecode::Bytecode;
use revm::context_interface::JournalTr;
use revm::handler::instructions::EthInstructions;
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::interpreter_types::LegacyBytecode;
use revm::interpreter::{instruction_table, Instruction, InstructionContext, InstructionResult};
use revm::primitives::U256;
use revm::Database;

type Ctx<'a, DB> = InstructionContext<'a, NitroCtx<DB>, EthInterpreter>;


const STYLUS_MARKER: u8 = 0xEF;


pub(crate) fn install<DB: Database>(table: &mut EthInstructions<EthInterpreter, NitroCtx<DB>>) {
    let base = instruction_table::<EthInterpreter, NitroCtx<DB>>();
    table.insert_instruction(opcode::NUMBER, Instruction::new(number::<DB>, 2));
    table.insert_instruction(opcode::BLOCKHASH, Instruction::new(blockhash::<DB>, 20));
    table.insert_instruction(opcode::GASPRICE, Instruction::new(exec_base_fee::<DB>, 2));
    table.insert_instruction(opcode::BASEFEE, Instruction::new(exec_base_fee::<DB>, 2));
    table.insert_instruction(
        opcode::BLOBBASEFEE,
        Instruction::new(blob_base_fee::<DB>, 2),
    );
    table.insert_instruction(STYLUS_MARKER, Instruction::new(stylus_marker::<DB>, 0));
    table.insert_instruction(opcode::INVALID, Instruction::new(invalid::<DB>, 0));
    macro_rules! guard {
        ($($op:ident),*) => {$(
            table.insert_instruction(
                opcode::$op,
                Instruction::new(guarded::<{ opcode::$op }, DB>, base[opcode::$op as usize].static_gas()),
            );
        )*};
    }
    guard!(
        MLOAD,
        MSTORE,
        MSTORE8,
        MCOPY,
        KECCAK256,
        CALLDATACOPY,
        CODECOPY,
        EXTCODECOPY,
        RETURNDATACOPY,
        LOG0,
        LOG1,
        LOG2,
        LOG3,
        LOG4,
        CREATE,
        CREATE2,
        CALL,
        CALLCODE,
        DELEGATECALL,
        STATICCALL,
        RETURN,
        REVERT
    );
}

fn push<DB: Database>(c: Ctx<'_, DB>, v: U256) {
    if !c.interpreter.stack.push(v) {
        c.interpreter.halt(InstructionResult::StackOverflow);
    }
}


fn state_failed<DB: Database>(c: Ctx<'_, DB>, e: DB::Error) {
    fail(c.host, e);
    c.interpreter.halt(InstructionResult::FatalExternalError);
}


fn refused<DB: Database>(c: Ctx<'_, DB>, why: String) {
    refuse(c.host, why);
    c.interpreter.halt(InstructionResult::FatalExternalError);
}


fn l1_block<DB: Database>(host: &mut NitroCtx<DB>) -> Result<u64, DB::Error> {
    if let Some(n) = host.chain.l1_block {
        return Ok(n);
    }
    let n = low64(arbos_word(host, SLOTS.l1_block_number)?);
    host.chain.l1_block = Some(n);
    Ok(n)
}


fn number<DB: Database>(c: Ctx<'_, DB>) {
    match l1_block(c.host) {
        Ok(n) => push(c, U256::from(n)),
        Err(e) => state_failed(c, e),
    }
}



fn blockhash<DB: Database>(c: Ctx<'_, DB>) {
    let n = match c.interpreter.stack.pop() {
        Ok(n) => n,
        Err(result) => return c.interpreter.halt(result),
    };
    let Ok(n) = u64::try_from(n) else {
        return push(c, U256::ZERO);
    };
    let now = match l1_block(c.host) {
        Ok(now) => now,
        Err(e) => return state_failed(c, e),
    };
    if n < now.saturating_sub(256) || n >= now {
        return push(c, U256::ZERO);
    }
    match arbos_word(c.host, SLOTS.l1_block_hash(n)) {
        Ok(hash) => push(c, hash),
        Err(e) => state_failed(c, e),
    }
}


fn exec_base_fee<DB: Database>(c: Ctx<'_, DB>) {
    let fee = c.host.chain.exec_base_fee;
    push(c, fee);
}



fn blob_base_fee<DB: Database>(c: Ctx<'_, DB>) {
    if c.host.journaled_state.depth() <= 1 {
        c.host.chain.blob_outermost = true;
    }
    c.interpreter.halt(InstructionResult::OpcodeNotFound);
}



fn code_account<DB: Database>(c: &Ctx<'_, DB>) -> Option<revm::primitives::Address> {
    c.interpreter.input.bytecode_address
}


fn stylus_marker<DB: Database>(c: Ctx<'_, DB>) {
    let code = c.interpreter.bytecode.bytecode_slice();
    let stylus =
        code.len() >= 4 && code[0] == 0xEF && code[1] == 0xF0 && matches!(code[2], 0x00 | 0x02);
    if stylus {
        let at = code_account(&c).unwrap_or(c.interpreter.input.target_address);
        return refused(
            c,
            format!("it runs the Stylus program at {}", at.to_checksum(None)),
        );
    }
    c.interpreter.halt(InstructionResult::OpcodeNotFound);
}


fn invalid<DB: Database>(c: Ctx<'_, DB>) {
    if let Some(at) = code_account(&c) {
        let delegate = match c.host.journaled_state.load_account_code(at) {
            Ok(account) => match &account.data.info.code {
                Some(Bytecode::Eip7702(d)) => Some(d.delegated_address),
                _ => None,
            },
            Err(e) => return state_failed(c, e),
        };
        if let Some(delegate) = delegate.filter(|d| ARBOS_PRECOMPILES.contains(d)) {
            return refused(
                c,
                format!(
                    "it runs an account delegated to the ArbOS precompile {}",
                    delegate.to_checksum(None)
                ),
            );
        }
    }
    c.interpreter.halt(InstructionResult::InvalidFEOpcode);
}



fn deploys_stylus<DB: Database>(c: &Ctx<'_, DB>) -> bool {
    if code_account(c).is_some() {
        return false;
    }
    let (Ok(offset), Ok(len)) = (c.interpreter.stack.peek(0), c.interpreter.stack.peek(1)) else {
        return false;
    };
    if len < U256::from(4u64) {
        return false;
    }
    let size = c.interpreter.memory.len();
    let byte = |i: u64| match offset.checked_add(U256::from(i)) {
        Some(at) if at < U256::from(size) => c.interpreter.memory.get_byte(at.to::<usize>()),
        _ => 0,
    };
    byte(0) == 0xEF && byte(1) == 0xF0 && byte(2) <= 0x02
}


type Ranges = [Option<(usize, Len)>; 2];


fn operands(op: u8) -> Option<(usize, Ranges)> {
    use Len::{Fixed, Item};
    let one = |offset: usize, len: Len| [Some((offset, len)), None];
    let two = |a: (usize, Len), b: (usize, Len)| [Some(a), Some(b)];
    Some(match op {
        opcode::MLOAD => (1, one(0, Fixed(32))),
        opcode::MSTORE => (2, one(0, Fixed(32))),
        opcode::MSTORE8 => (2, one(0, Fixed(1))),
        opcode::KECCAK256 | opcode::RETURN | opcode::REVERT => (2, one(0, Item(1))),
        opcode::LOG0..=opcode::LOG4 => (2 + usize::from(op - opcode::LOG0), one(0, Item(1))),
        opcode::CALLDATACOPY | opcode::CODECOPY | opcode::RETURNDATACOPY => (3, one(0, Item(2))),
        opcode::EXTCODECOPY => (4, one(1, Item(3))),
        opcode::MCOPY => (3, two((0, Item(2)), (1, Item(2)))),
        opcode::CREATE => (3, one(1, Item(2))),
        opcode::CREATE2 => (4, one(1, Item(2))),
        opcode::CALL | opcode::CALLCODE => (7, two((3, Item(4)), (5, Item(6)))),
        opcode::DELEGATECALL | opcode::STATICCALL => (6, two((2, Item(3)), (4, Item(5)))),
        _ => return None,
    })
}


#[derive(Clone, Copy)]
enum Len {
    Fixed(u64),
    Item(usize),
}


fn memory_verdict<DB: Database>(op: u8, c: &Ctx<'_, DB>) -> Option<InstructionResult> {
    let (takes, ranges) = operands(op)?;
    let stack = &c.interpreter.stack;
    if stack.len() < takes {
        return None;
    }
    let item = |i: usize| stack.peek(i).unwrap_or_default();
    let two_64 = U256::from(1u64) << 64;
    let two_32 = U256::from(1u64) << 32;
    let mut reaches_2_32 = false;
    let mut first_len = U256::ZERO;
    for (n, &(at, len)) in ranges.iter().flatten().enumerate() {
        let offset = item(at);
        let len = match len {
            Len::Fixed(l) => U256::from(l),
            Len::Item(i) => item(i),
        };
        if n == 0 {
            first_len = len;
        }
        if len.is_zero() {
            continue;
        }
        if offset >= two_64 || len >= two_64 || offset + len > two_64 - U256::from(32u64) {
            return Some(InstructionResult::InvalidOperandOOG);
        }
        reaches_2_32 |= offset + len >= two_32;
    }
    if reaches_2_32 {
        return Some(InstructionResult::MemoryLimitOOG);
    }
    let creates = op == opcode::CREATE || op == opcode::CREATE2;
    let limit = U256::from(c.host.chain.chain.max_initcode_size);
    (creates && first_len > limit).then_some(InstructionResult::OutOfGas)
}


fn guarded<const OP: u8, DB: Database>(c: Ctx<'_, DB>) {
    if OP == opcode::RETURN && deploys_stylus(&c) {
        return refused(c, "it deploys a Stylus program".into());
    }
    if let Some(result) = memory_verdict(OP, &c) {
        return c.interpreter.halt(result);
    }
    let original = const { instruction_table::<EthInterpreter, NitroCtx<DB>>()[OP as usize] };
    original.execute(c);
}
