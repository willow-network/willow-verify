













use revm::bytecode::opcode;
use revm::handler::instructions::EthInstructions;
use revm::interpreter::interpreter::EthInterpreter;
use revm::interpreter::interpreter_types::RuntimeFlag;
use revm::interpreter::{
    instruction_table, Host, Instruction, InstructionContext, InstructionResult,
};
use revm::primitives::hardfork::SpecId;
use revm::primitives::U256;


pub fn guard<H: Host>(instructions: &mut EthInstructions<EthInterpreter, H>) {
    macro_rules! wrap {
        ($($op:ident),*) => {$({
            let old = instructions.instruction_table[opcode::$op as usize];
            instructions.insert_instruction(
                opcode::$op,
                Instruction::new(guarded::<{ opcode::$op }, H>, old.static_gas()),
            );
        })*};
    }
    wrap!(
        KECCAK256,
        CALLDATACOPY,
        CODECOPY,
        EXTCODECOPY,
        RETURNDATACOPY,
        MLOAD,
        MSTORE,
        MSTORE8,
        MCOPY,
        LOG0,
        LOG1,
        LOG2,
        LOG3,
        LOG4,
        CREATE,
        CALL,
        CALLCODE,
        RETURN,
        DELEGATECALL,
        CREATE2,
        STATICCALL,
        REVERT
    );
}


fn guarded<const OP: u8, H: Host>(ctx: InstructionContext<'_, H, EthInterpreter>) {
    let verdict = {
        let stack = &ctx.interpreter.stack;
        classify(
            OP,
            ctx.interpreter.runtime_flag.spec_id(),
            |i| stack.peek(i).ok(),
            ctx.host.max_initcode_size(),
        )
    };
    match verdict {
        Some(result) => ctx.interpreter.halt(result),
        None => {
            let original = const { instruction_table::<EthInterpreter, H>()[OP as usize] };
            original.execute(ctx)
        }
    }
}




pub fn classify(
    op: u8,
    spec: SpecId,
    operand: impl Fn(usize) -> Option<U256>,
    max_initcode: usize,
) -> Option<InstructionResult> {
    let active = match op {
        opcode::MCOPY => spec.is_enabled_in(SpecId::CANCUN),
        opcode::RETURNDATACOPY | opcode::STATICCALL | opcode::REVERT => {
            spec.is_enabled_in(SpecId::BYZANTIUM)
        }
        opcode::DELEGATECALL => spec.is_enabled_in(SpecId::HOMESTEAD),
        opcode::CREATE2 => spec.is_enabled_in(SpecId::PETERSBURG),
        _ => true,
    };
    if !active {
        return None;
    }
    let pair = |o: usize, l: usize| Some((operand(o)?, operand(l)?));
    let fixed = |o: usize, l: u64| Some((operand(o)?, U256::from(l)));
    let (first, second) = match op {
        opcode::MLOAD | opcode::MSTORE => (fixed(0, 32), None),
        opcode::MSTORE8 => (fixed(0, 1), None),
        opcode::KECCAK256 | opcode::RETURN | opcode::REVERT => (pair(0, 1), None),
        opcode::LOG0..=opcode::LOG4 => (pair(0, 1), None),
        opcode::CALLDATACOPY | opcode::CODECOPY | opcode::RETURNDATACOPY => (pair(0, 2), None),
        opcode::EXTCODECOPY => (pair(1, 3), None),
        opcode::MCOPY => (pair(0, 2), Some(pair(1, 2))),
        opcode::CREATE | opcode::CREATE2 => (pair(1, 2), None),
        opcode::CALL | opcode::CALLCODE => (pair(3, 4), Some(pair(5, 6))),
        opcode::DELEGATECALL | opcode::STATICCALL => (pair(2, 3), Some(pair(4, 5))),
        _ => return None,
    };
    let first = first?;
    let second = match second {
        Some(range) => Some(range?),
        None => None,
    };
    let two_64 = U256::from(1u128 << 64);
    let two_32 = U256::from(1u64 << 32);
    let mut reaches_2_32 = false;
    for (offset, len) in std::iter::once(first).chain(second) {
        if len.is_zero() {
            continue;
        }
        if offset >= two_64 || len >= two_64 || offset + len > two_64 - U256::from(32) {
            return Some(InstructionResult::InvalidOperandOOG);
        }
        reaches_2_32 |= offset + len >= two_32;
    }
    if reaches_2_32 {
        return Some(InstructionResult::MemoryLimitOOG);
    }
    let over_initcode = matches!(op, opcode::CREATE | opcode::CREATE2)
        && spec.is_enabled_in(SpecId::SHANGHAI)
        && first.1 > U256::from(max_initcode);
    over_initcode.then_some(InstructionResult::OutOfGas)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LIMIT: usize = 49_152;


    fn verdict(op: u8, stack: &[U256]) -> Option<InstructionResult> {
        classify(op, SpecId::OSAKA, |i| stack.get(i).copied(), LIMIT)
    }

    fn u(n: u128) -> U256 {
        U256::from(n)
    }

    #[test]
    fn mload_below_reaching_and_past_the_limits() {
        assert_eq!(verdict(opcode::MLOAD, &[u(0)]), None);
        assert_eq!(verdict(opcode::MLOAD, &[u((1 << 32) - 33)]), None);
        assert_eq!(
            verdict(opcode::MLOAD, &[u((1 << 32) - 32)]),
            Some(InstructionResult::MemoryLimitOOG)
        );
        assert_eq!(
            verdict(opcode::MLOAD, &[u(1 << 32)]),
            Some(InstructionResult::MemoryLimitOOG)
        );
        assert_eq!(
            verdict(opcode::MLOAD, &[u((1 << 64) - 64)]),
            Some(InstructionResult::MemoryLimitOOG)
        );
        assert_eq!(
            verdict(opcode::MLOAD, &[u((1 << 64) - 63)]),
            Some(InstructionResult::InvalidOperandOOG)
        );
        assert_eq!(
            verdict(opcode::MLOAD, &[u(1 << 64)]),
            Some(InstructionResult::InvalidOperandOOG)
        );
        assert_eq!(
            verdict(opcode::MLOAD, &[U256::MAX]),
            Some(InstructionResult::InvalidOperandOOG)
        );
    }

    #[test]
    fn a_zero_length_touches_no_memory() {
        assert_eq!(verdict(opcode::RETURN, &[U256::MAX, u(0)]), None);
        assert_eq!(
            verdict(opcode::RETURNDATACOPY, &[u(1 << 40), u(0), u(0)]),
            None
        );
        assert_eq!(
            verdict(
                opcode::CALL,
                &[u(0), u(1), u(0), U256::MAX, u(0), u(0), u(0)]
            ),
            None
        );
    }

    #[test]
    fn copies_and_creates_reaching_2_32_are_out_of_gas_before_their_own_checks() {
        let oog = Some(InstructionResult::MemoryLimitOOG);
        assert_eq!(
            verdict(opcode::RETURNDATACOPY, &[u(0), u(0), u(1 << 32)]),
            oog
        );
        assert_eq!(verdict(opcode::CREATE, &[u(0), u(0), u(1 << 32)]), oog);
        assert_eq!(
            verdict(opcode::CREATE2, &[u(0), u(0), u(1 << 32), u(0)]),
            oog
        );
        assert_eq!(
            verdict(opcode::CALLDATACOPY, &[u(1 << 32), u(0), u(1)]),
            oog
        );
        assert_eq!(
            verdict(opcode::EXTCODECOPY, &[u(7), u(0), u(0), u(1 << 32)]),
            oog
        );
        assert_eq!(verdict(opcode::KECCAK256, &[u(0), u(1 << 32)]), oog);
        assert_eq!(verdict(opcode::LOG2, &[u(0), u(1 << 32), u(1), u(2)]), oog);
    }

    #[test]
    fn either_range_of_a_call_or_mcopy_counts_and_an_invalid_one_wins() {
        let call = |args: (u128, u128), ret: (u128, u128)| {
            verdict(
                opcode::CALL,
                &[u(0), u(1), u(0), u(args.0), u(args.1), u(ret.0), u(ret.1)],
            )
        };
        assert_eq!(call((0, 4), (0, 32)), None);
        assert_eq!(
            call((0, 4), (1 << 32, 1)),
            Some(InstructionResult::MemoryLimitOOG)
        );
        assert_eq!(
            call((1 << 32, 1), (1 << 64, 1)),
            Some(InstructionResult::InvalidOperandOOG)
        );
        assert_eq!(
            verdict(
                opcode::STATICCALL,
                &[u(0), u(1), u(0), u(0), u(1 << 32), u(1)]
            ),
            Some(InstructionResult::MemoryLimitOOG)
        );
        assert_eq!(
            verdict(opcode::MCOPY, &[u(0), u(1 << 32), u(1)]),
            Some(InstructionResult::MemoryLimitOOG)
        );
    }

    #[test]
    fn initcode_over_the_limit_is_out_of_gas_from_shanghai() {
        assert_eq!(
            verdict(opcode::CREATE, &[u(0), u(0), u(LIMIT as u128)]),
            None
        );
        assert_eq!(
            verdict(opcode::CREATE, &[u(0), u(0), u(LIMIT as u128 + 1)]),
            Some(InstructionResult::OutOfGas)
        );
        assert_eq!(
            verdict(opcode::CREATE2, &[u(0), u(0), u(LIMIT as u128 + 1), u(0)]),
            Some(InstructionResult::OutOfGas)
        );
        let at = |spec| {
            classify(
                opcode::CREATE,
                spec,
                |i| [u(0), u(0), u(LIMIT as u128 + 1)].get(i).copied(),
                LIMIT,
            )
        };
        assert_eq!(at(SpecId::MERGE), None);
        assert_eq!(at(SpecId::SHANGHAI), Some(InstructionResult::OutOfGas));
    }

    #[test]
    fn missing_operands_and_inactive_opcodes_run_as_they_are() {
        assert_eq!(verdict(opcode::RETURNDATACOPY, &[u(0), u(0)]), None);
        assert_eq!(
            classify(
                opcode::MCOPY,
                SpecId::SHANGHAI,
                |i| [u(0), u(1 << 32), u(1)].get(i).copied(),
                LIMIT
            ),
            None
        );
        assert_eq!(
            classify(
                opcode::CREATE2,
                SpecId::BYZANTIUM,
                |i| [u(0), u(0), u(1 << 32), u(0)].get(i).copied(),
                LIMIT
            ),
            None
        );
        assert_eq!(verdict(opcode::ADD, &[U256::MAX, U256::MAX]), None);
    }
}
