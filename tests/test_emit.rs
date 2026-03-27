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

    #[test]
    fn test_late_instruction_builders_encode_expected_opcodes() {
        let mut builder = BytecodeBuilder::new();
        let k0 = builder.add_constant(make_number(10.0));
        let k1 = builder.add_constant(make_number(20.0));

        builder.emit_add_i32(1, 2, 3);
        builder.emit_add_f64(4, 5, 6);
        builder.emit_sub_i32(7, 8, 9);
        builder.emit_sub_f64(10, 11, 12);
        builder.emit_mul_i32(13, 14, 15);
        builder.emit_mul_f64(16, 17, 18);
        builder.emit_ret_if_lte_i(19, 20, 21);
        builder.emit_add_acc_reg(22, 23);
        builder.emit_call1_add(24, 25);
        builder.emit_call2_add(26, 27, 28);
        builder.emit_load_k_add(29, k0);
        builder.emit_load_k_cmp(30, k1);
        builder.emit_cmp_jmp(31, 32, -3);
        builder.emit_get_prop_call(33, 34, 35);
        builder.emit_call_ret(36, 2);
        builder.emit_add_i32_fast(37, 38, 39);
        builder.emit_add_f64_fast(40, 41, 42);
        builder.emit_sub_i32_fast(43, 44, 45);
        builder.emit_mul_i32_fast(46, 47, 48);
        builder.emit_eq_i32_fast(49, 50);
        builder.emit_lt_i32_fast(51, 52);
        builder.emit_jmp_i32_fast(53, 54, 5);
        builder.emit_get_prop_mono(55, 56, 57);
        builder.emit_call_mono(58, 1);
        builder.emit_call0(59);
        builder.emit_call1(60, 61);
        builder.emit_call2(62, 63, 64);
        builder.emit_call3(65, 66, 67, 65);
        builder.emit_call_method1(68, 69);
        builder.emit_call_method2(70, 71);
        builder.emit_load_add(73, 74, 75);
        builder.emit_load_sub(76, 77, 78);
        builder.emit_load_mul(79, 80, 81);
        builder.emit_load_inc(82, 83);
        builder.emit_load_dec(84, 85);
        builder.emit_load_cmp_eq(86, 87, 88);
        builder.emit_load_cmp_lt(89, 90, 91);
        builder.emit_load_jfalse(92, -4);
        builder.emit_load_cmp_eq_jfalse(93, 94, 6);
        builder.emit_load_cmp_lt_jfalse(95, 96, -7);
        builder.emit_load_get_prop(97, 98);
        builder.emit_load_get_prop_cmp_eq(99, 100, 101);
        builder.emit_get_prop2_ic(102, 103, 104, 102);
        builder.emit_get_prop3_ic(105, 106, 107, 105, 106);
        builder.emit_get_elem(108, 109, 110);
        builder.emit_set_elem(111, 112, 113);
        builder.emit_get_prop_elem(114, 115, 116, 114);
        builder.emit_call_method_ic(117, 118);
        builder.emit_call_method2_ic(119, 120, 121);

        let (bytecode, constants) = builder.build();
        assert_eq!(constants.len(), 2);

        let expected = [
            Opcode::AddI32,
            Opcode::AddF64,
            Opcode::SubI32,
            Opcode::SubF64,
            Opcode::MulI32,
            Opcode::MulF64,
            Opcode::RetIfLteI,
            Opcode::AddAccReg,
            Opcode::Call1Add,
            Opcode::Call2Add,
            Opcode::LoadKAdd,
            Opcode::LoadKCmp,
            Opcode::CmpJmp,
            Opcode::GetPropCall,
            Opcode::CallRet,
            Opcode::AddI32Fast,
            Opcode::AddF64Fast,
            Opcode::SubI32Fast,
            Opcode::MulI32Fast,
            Opcode::EqI32Fast,
            Opcode::LtI32Fast,
            Opcode::JmpI32Fast,
            Opcode::GetPropMono,
            Opcode::CallMono,
            Opcode::Call0,
            Opcode::Call1,
            Opcode::Call2,
            Opcode::Call3,
            Opcode::CallMethod1,
            Opcode::CallMethod2,
            Opcode::LoadAdd,
            Opcode::LoadSub,
            Opcode::LoadMul,
            Opcode::LoadInc,
            Opcode::LoadDec,
            Opcode::LoadCmpEq,
            Opcode::LoadCmpLt,
            Opcode::LoadJfalse,
            Opcode::LoadCmpEqJfalse,
            Opcode::LoadCmpLtJfalse,
            Opcode::LoadGetProp,
            Opcode::LoadGetPropCmpEq,
            Opcode::GetProp2Ic,
            Opcode::GetProp3Ic,
            Opcode::GetElem,
            Opcode::SetElem,
            Opcode::GetPropElem,
            Opcode::CallMethodIc,
            Opcode::CallMethod2Ic,
        ];

        assert_eq!(bytecode.len(), expected.len());
        for (index, expected_opcode) in expected.iter().enumerate() {
            assert_eq!(bytecode[index] & 0xFF, expected_opcode.as_u8() as u32);
        }

        assert_eq!((bytecode[10] >> 8) & 0xFF, 29);
        assert_eq!((bytecode[10] >> 16) & 0xFFFF, k0 as u32);
        assert_eq!((bytecode[11] >> 8) & 0xFF, 30);
        assert_eq!((bytecode[11] >> 16) & 0xFFFF, k1 as u32);
        assert_eq!((bytecode[27] >> 8) & 0xFF, 65);
        assert_eq!((bytecode[42] >> 8) & 0xFF, 102);
        assert_eq!((bytecode[43] >> 8) & 0xFF, 105);
        assert_eq!((bytecode[46] >> 8) & 0xFF, 114);
    }
}
