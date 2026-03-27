//! Simple test to debug VM execution

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Simple VM Test ===\n");

    // Test 1: Simple addition
    println!("Test 1: Simple addition (3 + 4)");
    let mut builder1 = BytecodeBuilder::new();
    builder1.emit_load_i(1, 3); // load_i r1, 3
    builder1.emit_load_i(2, 4); // load_i r2, 4
    builder1.emit_add(1, 2); // add r1, r2 -> accumulator
    builder1.emit_ret(); // ret

    let (bytecode1, constants1) = builder1.build();
    let mut vm1 = VM::new(bytecode1, constants1, vec![]);
    vm1.run(false);
    let result1 = vm1.frame.regs[255];
    if let Some(number) = to_f64(result1) {
        println!("Result: {} (expected: 7.0)", number);
    } else {
        println!("Result: {:?}", result1);
    }

    // Test 2: Simple loop
    println!("\nTest 2: Simple loop (sum 1 to 5)");
    let mut builder2 = BytecodeBuilder::new();
    builder2.emit_load_i(1, 5); // load_i r1, 5 (n = 5)
    builder2.emit_load_0(); // load_0 -> accumulator
    builder2.emit_mov(2, 255); // mov r2, accumulator (sum = 0)
    builder2.emit_load_1(); // load_1 -> accumulator
    builder2.emit_mov(3, 255); // mov r3, accumulator (i = 1)

    // Loop start:
    let loop_start = builder2.len();
    builder2.emit_lte(3, 1); // lte r3, r1 (i <= n)
    builder2.emit_jmp_false(255, 6); // jmp_false accumulator, +6 (exit loop)

    // sum = sum + i
    builder2.emit_add(2, 3); // add r2, r3 -> accumulator (sum + i)
    builder2.emit_mov(2, 255); // mov r2, accumulator (sum = accumulator)

    // i = i + 1
    builder2.emit_mov(255, 3); // mov accumulator, r3 (accumulator = i)
    builder2.emit_inc_acc(); // accumulator = i + 1
    builder2.emit_mov(3, 255); // mov r3, accumulator (i = accumulator)

    let loop_back = -(builder2.len() as i16 - loop_start as i16 + 1);
    builder2.emit_jmp(loop_back); // jump back to the loop condition

    // Return sum
    builder2.emit_mov(255, 2); // mov accumulator, r2
    builder2.emit_ret(); // ret

    let (bytecode2, constants2) = builder2.build();
    let mut vm2 = VM::new(bytecode2, constants2, vec![]);
    vm2.run(false);
    let result2 = vm2.frame.regs[255];
    if let Some(number) = to_f64(result2) {
        println!("Result: {} (expected: 15.0, sum of 1 to 5)", number);
    } else {
        println!("Result: {:?}", result2);
    }

    // Show assembly for debugging
    println!("\n=== Assembly for Test 2 ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&vm2.bytecode, &vm2.const_pool);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }
}
