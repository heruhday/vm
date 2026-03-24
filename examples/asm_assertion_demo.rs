//! Demonstration of assembly code generation for assertion opcodes

use vm::asm::{disassemble, disassemble_clean};
use vm::js_value::make_number;

fn main() {
    println!("=== Assembly Code Generation for Assertion Opcodes ===\n");

    // Create some bytecode with assertion opcodes
    let bytecode = vec![
        // mov r1, r2, r0
        ((0 as u32) << 24) | ((2 as u32) << 16) | ((1 as u32) << 8) | 0,
        // load_k r3, const[0] (42.0)
        ((0 as u32) << 16) | ((3 as u32) << 8) | 1,
        // add r1, r2
        ((2 as u32) << 16) | ((1 as u32) << 8) | 2,
        // assert_equal r1, r2
        ((2 as u32) << 16) | ((1 as u32) << 8) | 227, // 227 = AssertEqual
        // assert_ok
        ((0 as u32) << 8) | 226, // 226 = AssertOk
        // assert_fail
        ((0 as u32) << 8) | 239, // 239 = AssertFail
        // ret
        ((0 as u32) << 8) | 103,
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
        (225, "assert_value"),
        (226, "assert_ok"),
        (227, "assert_equal"),
        (228, "assert_not_equal"),
        (229, "assert_deep_equal"),
        (230, "assert_not_deep_equal"),
        (231, "assert_strict_equal"),
        (232, "assert_not_strict_equal"),
        (233, "assert_deep_strict_equal"),
        (234, "assert_not_deep_strict_equal"),
        (235, "assert_throws"),
        (236, "assert_does_not_throw"),
        (237, "assert_rejects"),
        (238, "assert_does_not_reject"),
        (239, "assert_fail"),
    ];

    for (opcode_num, expected_name) in assertion_opcodes {
        let instr = ((0 as u32) << 8) | opcode_num as u32;
        let bytecode_single = vec![instr];
        let asm_single = disassemble(&bytecode_single, &constants);
        println!(
            "  Opcode {} (0x{:02X}): {}",
            opcode_num, opcode_num, asm_single[0]
        );
    }
}
