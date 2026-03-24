#[cfg(test)]
mod tests {
    use vm::emit::BytecodeBuilder;
    use vm::js_value::{make_number, to_f64};
    use vm::vm::Opcode;

    #[test]
    fn test_basic_instructions() {
        let mut builder = BytecodeBuilder::new();

        // Test basic arithmetic
        builder.emit_load_i(1, 5);
        builder.emit_load_i(2, 3);
        builder.emit_add(1, 2);
        builder.emit_ret();

        let (bytecode, constants) = builder.build();
        assert_eq!(bytecode.len(), 4);
        assert!(constants.is_empty());

        // Verify the instructions
        // load_i r1, 5
        assert_eq!(bytecode[0] & 0xFF, Opcode::LoadI.as_u8() as u32);
        assert_eq!((bytecode[0] >> 8) & 0xFF, 1);

        // load_i r2, 3
        assert_eq!(bytecode[1] & 0xFF, Opcode::LoadI.as_u8() as u32);
        assert_eq!((bytecode[1] >> 8) & 0xFF, 2);

        // add r1, r2
        assert_eq!(bytecode[2] & 0xFF, Opcode::Add.as_u8() as u32);
        assert_eq!((bytecode[2] >> 16) & 0xFF, 1);
        assert_eq!((bytecode[2] >> 24) & 0xFF, 2);

        // ret
        assert_eq!(bytecode[3] & 0xFF, Opcode::Ret.as_u8() as u32);
    }

    #[test]
    fn test_assertion_instructions() {
        let mut builder = BytecodeBuilder::new();

        // Test assertion instructions
        builder.emit_load_i(1, 42);
        builder.emit_load_i(2, 42);
        builder.emit_assert_equal(1, 2);
        builder.emit_assert_ok(1);
        builder.emit_assert_fail();

        let (bytecode, _) = builder.build();
        assert_eq!(bytecode.len(), 5);

        // assert_equal r1, r2
        assert_eq!(bytecode[2] & 0xFF, Opcode::AssertEqual.as_u8() as u32);
        assert_eq!((bytecode[2] >> 16) & 0xFF, 1);
        assert_eq!((bytecode[2] >> 24) & 0xFF, 2);

        // assert_ok r1
        assert_eq!(bytecode[3] & 0xFF, Opcode::AssertOk.as_u8() as u32);
        assert_eq!((bytecode[3] >> 8) & 0xFF, 1);

        // assert_fail
        assert_eq!(bytecode[4] & 0xFF, Opcode::AssertFail.as_u8() as u32);
    }

    #[test]
    fn test_constants() {
        let mut builder = BytecodeBuilder::new();

        let const_idx = builder.add_constant(make_number(42.0));
        builder.emit_load_k(1, const_idx);
        builder.emit_ret();

        let (bytecode, constants) = builder.build();
        assert_eq!(bytecode.len(), 2);
        assert_eq!(constants.len(), 1);
        assert_eq!(to_f64(constants[0]), Some(42.0));

        // load_k r1, const[0]
        assert_eq!(bytecode[0] & 0xFF, Opcode::LoadK.as_u8() as u32);
        assert_eq!((bytecode[0] >> 8) & 0xFF, 1);
        assert_eq!((bytecode[0] >> 16) & 0xFFFF, 0);
    }

    #[test]
    fn test_complex_expression() {
        let mut builder = BytecodeBuilder::new();

        // Compute (5 + 3) * 2 - 1 using the clean API
        builder.emit_load_i(1, 5);
        builder.emit_load_i(2, 3);
        builder.emit_add(1, 2); // 5 + 3 = 8 -> accumulator
        builder.emit_mov(3, 255); // r3 = accumulator (save 8)
        builder.emit_load_i(4, 2); // r4 = 2
        builder.emit_mov(255, 3); // accumulator = r3 (load 8)
        builder.emit_mul_acc(4); // 8 * 2 = 16 -> accumulator
        builder.emit_mov(5, 255); // r5 = accumulator (save 16)
        builder.emit_load_i(6, 1); // r6 = 1
        builder.emit_mov(255, 5); // accumulator = r5 (load 16)
        builder.emit_sub_acc(6); // 16 - 1 = 15 -> accumulator
        builder.emit_ret();

        let (bytecode, _) = builder.build();
        assert_eq!(bytecode.len(), 12);

        // Verify the sequence
        let expected_opcodes = vec![
            Opcode::LoadI,
            Opcode::LoadI,
            Opcode::Add,
            Opcode::Mov,
            Opcode::LoadI,
            Opcode::Mov,
            Opcode::MulAcc,
            Opcode::Mov,
            Opcode::LoadI,
            Opcode::Mov,
            Opcode::SubAcc,
            Opcode::Ret,
        ];

        for (i, &expected) in expected_opcodes.iter().enumerate() {
            assert_eq!(
                bytecode[i] & 0xFF,
                expected.as_u8() as u32,
                "Instruction {} should be {:?}",
                i,
                expected
            );
        }
    }

    #[test]
    fn test_bitwise_instructions() {
        let mut builder = BytecodeBuilder::new();

        builder.emit_bit_and(1, 2);
        builder.emit_bit_or(3, 4);
        builder.emit_bit_xor(5, 6);
        builder.emit_bit_not(7);
        builder.emit_shl(8, 9);
        builder.emit_shr(10, 11);
        builder.emit_ushr(12, 13);
        builder.emit_ret();

        let (bytecode, _) = builder.build();
        let expected_opcodes = [
            Opcode::BitAnd,
            Opcode::BitOr,
            Opcode::BitXor,
            Opcode::BitNot,
            Opcode::Shl,
            Opcode::Shr,
            Opcode::Ushr,
            Opcode::Ret,
        ];

        for (i, expected) in expected_opcodes.iter().enumerate() {
            assert_eq!(bytecode[i] & 0xFF, expected.as_u8() as u32);
        }

        assert_eq!((bytecode[0] >> 16) & 0xFF, 1);
        assert_eq!((bytecode[0] >> 24) & 0xFF, 2);
        assert_eq!((bytecode[3] >> 16) & 0xFF, 7);
    }

    #[test]
    fn test_extended_operator_instructions() {
        let mut builder = BytecodeBuilder::new();

        builder.emit_pow(1, 2);
        builder.emit_logical_and(3, 4);
        builder.emit_logical_or(5, 6);
        builder.emit_nullish_coalesce(7, 8);
        builder.emit_in(9, 10);
        builder.emit_instanceof(11, 12);
        builder.emit_ret();

        let (bytecode, _) = builder.build();
        let expected_opcodes = [
            Opcode::Pow,
            Opcode::LogicalAnd,
            Opcode::LogicalOr,
            Opcode::NullishCoalesce,
            Opcode::In,
            Opcode::Instanceof,
            Opcode::Ret,
        ];

        for (i, expected) in expected_opcodes.iter().enumerate() {
            assert_eq!(bytecode[i] & 0xFF, expected.as_u8() as u32);
        }

        assert_eq!((bytecode[0] >> 16) & 0xFF, 1);
        assert_eq!((bytecode[0] >> 24) & 0xFF, 2);
        assert_eq!((bytecode[5] >> 16) & 0xFF, 11);
        assert_eq!((bytecode[5] >> 24) & 0xFF, 12);
    }
}
