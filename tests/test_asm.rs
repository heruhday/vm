#[cfg(test)]
mod tests {
    use vm::asm::{AsmInstruction, Format, disassemble};
    use vm::js_value::make_number;
    use vm::vm::Opcode;

    fn make_instr(op: u8, a: u8, b: u8, c: u8) -> u32 {
        ((c as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | op as u32
    }

    fn make_abx_instr(op: u8, a: u8, bx: u16) -> u32 {
        ((bx as u32) << 16) | ((a as u32) << 8) | op as u32
    }

    fn make_asbx_instr(op: u8, a: u8, sbx: i16) -> u32 {
        (((sbx as u16) as u32) << 16) | ((a as u32) << 8) | op as u32
    }

    #[test]
    fn test_decode_abc_format() {
        let raw = make_instr(0, 1, 2, 3); // mov r1, r2, r3
        let instr = AsmInstruction::decode(0, raw);

        assert_eq!(instr.pc, 0);
        assert_eq!(instr.raw, raw);
        assert!(matches!(instr.opcode, Opcode::Mov));
        assert_eq!(instr.format, Format::ABC);
        assert_eq!(instr.a, 1);
        assert_eq!(instr.b, 2);
        assert_eq!(instr.c, 3);
        // bx is bits 16-31: b=2 (0x02), c=3 (0x03) -> 0x0203 = 515
        // But raw >> 16 gives 0x0302 = 770 because c is in higher bits
        // The test should match the actual implementation
        assert_eq!(instr.bx, 0x0302); // 0x03 << 8 | 0x02 = 770
        assert_eq!(instr.sbx, 0x0302);
    }

    #[test]
    fn test_decode_abx_format() {
        let raw = make_abx_instr(1, 5, 0x1234); // load_k r5, const[0x1234]
        let instr = AsmInstruction::decode(10, raw);

        assert_eq!(instr.pc, 10);
        assert_eq!(instr.raw, raw);
        assert!(matches!(instr.opcode, Opcode::LoadK));
        assert_eq!(instr.format, Format::ABx);
        assert_eq!(instr.a, 5);
        assert_eq!(instr.bx, 0x1234);
        assert_eq!(instr.sbx, 0x1234);
    }

    #[test]
    fn test_decode_asbx_format() {
        let raw = make_asbx_instr(5, 0, -10); // jmp -> -10
        let instr = AsmInstruction::decode(20, raw);

        assert_eq!(instr.pc, 20);
        assert_eq!(instr.raw, raw);
        assert!(matches!(instr.opcode, Opcode::Jmp));
        assert_eq!(instr.format, Format::AsBx);
        assert_eq!(instr.a, 0);
        assert_eq!(instr.sbx, -10);
    }

    #[test]
    fn test_to_asm_mov() {
        let raw = make_instr(0, 1, 2, 0); // mov r1, r2
        let instr = AsmInstruction::decode(0, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0000: mov r1, r2, r0");
    }

    #[test]
    fn test_to_asm_load_k() {
        let raw = make_abx_instr(1, 3, 0); // load_k r3, const[0]
        let instr = AsmInstruction::decode(1, raw);
        let constants = vec![make_number(42.0)];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0004: load_k r3, const[0]");
    }

    #[test]
    fn test_to_asm_add() {
        let raw = make_instr(2, 0, 1, 2); // add r1, r2
        let instr = AsmInstruction::decode(2, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        // Add is BC format, so it should be "add r1, r2"
        assert_eq!(asm, "0008: add r1, r2");
    }

    #[test]
    fn test_to_asm_jmp() {
        let raw = make_asbx_instr(5, 0, 5); // jmp -> +5
        let instr = AsmInstruction::decode(3, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        // Target PC = 3 + 5 + 1 = 9, byte offset = 9 * 4 = 0x24
        assert_eq!(asm, "000C: jmp -> 0024");
    }

    #[test]
    fn test_to_asm_load_i() {
        let raw = make_asbx_instr(6, 4, -100); // load_i r4, -100
        let instr = AsmInstruction::decode(4, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0010: load_i r4, -100");
    }

    #[test]
    fn test_to_asm_call() {
        let raw = make_instr(4, 1, 2, 0); // call r1, 2
        let instr = AsmInstruction::decode(5, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0014: call r1, 2");
    }

    #[test]
    fn test_to_asm_load_global_ic() {
        let raw = make_abx_instr(25, 2, 0x42); // load_global_ic r2, global[0x42]
        let instr = AsmInstruction::decode(6, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0018: load_global_ic r2, global[66]");
    }

    #[test]
    fn test_to_asm_inc_acc() {
        let raw = make_instr(11, 0, 0, 0); // inc_acc
        let instr = AsmInstruction::decode(7, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "001C: inc_acc");
    }

    #[test]
    fn test_to_asm_load_0() {
        let raw = make_instr(13, 0, 0, 0); // load_0
        let instr = AsmInstruction::decode(8, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0020: load_0");
    }

    #[test]
    fn test_to_asm_load_null() {
        let raw = make_instr(22, 0, 0, 0); // load_null
        let instr = AsmInstruction::decode(9, raw);
        let constants = vec![];

        let asm = instr.to_asm(&constants);
        assert_eq!(asm, "0024: load_null");
    }

    #[test]
    fn test_disassemble_function() {
        let bytecode = vec![
            make_instr(0, 1, 2, 0),   // mov r1, r2
            make_abx_instr(1, 3, 0),  // load_k r3, const[0]
            make_instr(2, 0, 1, 2),   // add r1, r2
            make_instr(103, 0, 0, 0), // ret
        ];

        let constants = vec![make_number(42.0)];

        let asm = disassemble(&bytecode, &constants);

        assert_eq!(asm.len(), 4);
        assert_eq!(asm[0], "0000: mov r1, r2, r0");
        assert_eq!(asm[1], "0004: load_k r3, const[0]");
        assert_eq!(asm[2], "0008: add r1, r2");
        assert_eq!(asm[3], "000C: ret");
    }

    #[test]
    fn test_opcode_mnemonics() {
        // Test a few key opcodes to ensure mnemonics are correct
        let test_cases = vec![
            (0, "mov"),
            (1, "load_k"),
            (2, "add"),
            (3, "get_prop_ic"),
            (4, "call"),
            (5, "jmp"),
            (6, "load_i"),
            (7, "jmp_true"),
            (8, "jmp_false"),
            (9, "set_prop_ic"),
            (10, "add_acc_imm8"),
            (11, "inc_acc"),
            (12, "load_this"),
            (13, "load_0"),
            (14, "load_1"),
            (15, "eq"),
            (16, "lt"),
            (17, "lte"),
            (18, "add_acc"),
            (19, "sub_acc"),
            (20, "mul_acc"),
            (21, "div_acc"),
            (22, "load_null"),
            (23, "load_true"),
            (24, "load_false"),
            (25, "load_global_ic"),
            (26, "set_global_ic"),
            (27, "typeof"),
            (28, "to_num"),
            (29, "to_str"),
            (30, "is_undef"),
            (31, "is_null"),
            (54, "bit_and"),
            (55, "bit_or"),
            (56, "bit_xor"),
            (57, "bit_not"),
            (58, "shl"),
            (59, "shr"),
            (60, "ushr"),
            (117, "pow"),
            (118, "logical_and"),
            (119, "logical_or"),
            (120, "nullish_coalesce"),
            (121, "in"),
            (122, "instanceof"),
            (103, "ret"),
            (200, "get_prop_ic_call"),
            (201, "inc_jmp_false_loop"),
            (202, "load_k_add_acc"),
            (203, "add_mov"),
            (204, "eq_jmp_true"),
            (205, "get_prop_acc_call"),
            (206, "load_k_mul_acc"),
            (207, "lt_jmp"),
            (208, "get_prop_ic_mov"),
            (209, "get_prop_add_imm_set_prop_ic"),
            (210, "add_acc_imm8_mov"),
            (211, "call_ic_super"),
            (212, "load_this_call"),
            (213, "eq_jmp_false"),
            (214, "load_k_sub_acc"),
            (215, "get_length_ic_call"),
            (216, "add_str_acc_mov"),
            (217, "inc_acc_jmp"),
            (218, "get_prop_chain_acc"),
            (219, "test_jmp_true"),
            (220, "load_arg_call"),
            (221, "mul_acc_mov"),
            (222, "lte_jmp_loop"),
            (223, "new_obj_init_prop"),
            (224, "profile_hot_call"),
        ];

        for (opcode_num, expected_mnemonic) in test_cases {
            let raw = make_instr(opcode_num as u8, 0, 0, 0);
            let instr = AsmInstruction::decode(0, raw);
            let mnemonic = instr.opcode_to_mnemonic();
            assert_eq!(
                mnemonic, expected_mnemonic,
                "Opcode {} should be {}",
                opcode_num, expected_mnemonic
            );
        }
    }
}
