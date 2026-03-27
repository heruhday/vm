//! Demonstration of assembly code generation for assertion opcodes

use vm::asm::{disassemble, disassemble_clean};
use vm::js_value::make_number;
use vm::vm::Opcode;

fn encode_abc(opcode: Opcode, a: u8, b: u8, c: u8) -> u32 {
    ((c as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_abx(opcode: Opcode, a: u8, bx: u16) -> u32 {
    ((bx as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn main() {
    println!("=== Assembly Code Generation for Assertion Opcodes ===\n");

    // Create some bytecode with assertion opcodes
    let bytecode = vec![
        // mov r1, r2, r0
        encode_abc(Opcode::Mov, 1, 2, 0),
        // load_k r3, const[0] (42.0)
        encode_abx(Opcode::LoadK, 3, 0),
        // add r1, r2
        encode_abc(Opcode::Add, 0, 1, 2),
        // assert_equal r1, r2
        encode_abc(Opcode::AssertEqual, 0, 1, 2),
        // assert_ok
        Opcode::AssertOk.as_u8() as u32,
        // assert_fail
        Opcode::AssertFail.as_u8() as u32,
        // ret
        Opcode::Ret.as_u8() as u32,
    ];

    let constants = vec![make_number(42.0)];

    println!("Bytecode with assertions:");
    println!("-------------------------");
    for (i, &instr) in bytecode.iter().enumerate() {
        println!("  [{:02}] 0x{:08X}", i, instr);
    }
    println!();

    println!("Disassembled assembly:");
    println!("----------------------");
    let asm = disassemble(&bytecode, &constants);
    for line in asm {
        println!("  {}", line);
    }
    println!();

    println!("Clean assembly (no byte offsets):");
    println!("---------------------------------");
    let clean_asm = disassemble_clean(&bytecode, &constants);
    for line in clean_asm {
        println!("  {}", line);
    }
    println!();

    // Test all assertion opcodes
    println!("All assertion opcodes:");
    println!("----------------------");

    let assertion_opcodes = vec![
        (Opcode::AssertValue, "assert_value"),
        (Opcode::AssertOk, "assert_ok"),
        (Opcode::AssertEqual, "assert_equal"),
        (Opcode::AssertNotEqual, "assert_not_equal"),
        (Opcode::AssertDeepEqual, "assert_deep_equal"),
        (Opcode::AssertNotDeepEqual, "assert_not_deep_equal"),
        (Opcode::AssertStrictEqual, "assert_strict_equal"),
        (Opcode::AssertNotStrictEqual, "assert_not_strict_equal"),
        (Opcode::AssertDeepStrictEqual, "assert_deep_strict_equal"),
        (
            Opcode::AssertNotDeepStrictEqual,
            "assert_not_deep_strict_equal",
        ),
        (Opcode::AssertThrows, "assert_throws"),
        (Opcode::AssertDoesNotThrow, "assert_does_not_throw"),
        (Opcode::AssertRejects, "assert_rejects"),
        (Opcode::AssertDoesNotReject, "assert_does_not_reject"),
        (Opcode::AssertFail, "assert_fail"),
    ];

    for (opcode, expected_name) in assertion_opcodes {
        let opcode_num = opcode.as_u8();
        let instr = opcode_num as u32;
        let bytecode_single = vec![instr];
        let asm_single = disassemble(&bytecode_single, &constants);
        println!(
            "  Opcode {} (0x{:02X}) {}: {}",
            opcode_num, opcode_num, expected_name, asm_single[0]
        );
    }
}
