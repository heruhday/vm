//! Example demonstrating assembly code generation for QJL bytecode

use std::f64::consts::PI;

use vm::asm::*;
use vm::js_value::*;

fn main() {
    println!("=== QJL Bytecode Assembly Demo ===\n");

    // Create some constants (just numbers for simplicity)
    let constants = vec![make_number(42.0), make_number(PI), make_number(100.0)];

    // Create some bytecode instructions
    let bytecode = vec![
        // mov r1, r2
        0x00000201, // opcode=0 (mov), a=1, b=2, c=0
        // load_k r3, const[0] (42.0)
        0x00000301, // opcode=1 (load_k), a=3, bx=0
        // add r1, r2
        0x02000102, // opcode=2 (add), a=0, b=1, c=2
        // load_i r4, -100
        0xFF9C0406, // opcode=6 (load_i), a=4, sbx=-100
        // jmp -> +5
        0x00050005, // opcode=5 (jmp), a=0, sbx=5
        // load_global_ic r2, global[66]
        0x00420219, // opcode=25 (load_global_ic), a=2, bx=66
        // inc_acc
        0x0000000B, // opcode=11 (inc_acc)
        // load_null
        0x00000016, // opcode=22 (load_null)
        // ret
        0x00000067, // opcode=103 (ret)
    ];

    println!("Bytecode ({} instructions):", bytecode.len());
    for (i, &instr) in bytecode.iter().enumerate() {
        println!("  {:04X}: {:08X}", i * 4, instr);
    }

    println!("\nDisassembled Assembly:");
    let asm = disassemble(&bytecode, &constants);
    for line in asm {
        println!("  {}", line);
    }

    println!("\n=== Explanation ===");
    println!("1. mov r1, r2, r0      - Move value from r2 to r1");
    println!("2. load_k r3, const[0] - Load constant 0 (42.0) into r3");
    println!("3. add r1, r2          - Add r1 and r2, result in accumulator");
    println!("4. load_i r4, -100     - Load immediate -100 into r4");
    println!("5. jmp -> 0024         - Jump to byte offset 0x24 (instruction 9)");
    println!("6. load_global_ic r2, global[66] - Load global 66 into r2");
    println!("7. inc_acc             - Increment accumulator");
    println!("8. load_null           - Load null value");
    println!("9. ret                 - Return from function");

    println!("\nConstants:");
    for (i, constant) in constants.iter().enumerate() {
        println!("  const[{}] = {:?}", i, constant);
    }
}
