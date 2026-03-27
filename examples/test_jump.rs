//! Test to understand jump offset behavior

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Testing Jump Offset Behavior ===\n");

    // Test 1: Simple forward jump
    println!("Test 1: Simple forward jump");
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 42); // instruction 0
    builder.emit_jmp(2); // instruction 1: jump forward 2
    builder.emit_load_i(2, 99); // instruction 2: should be skipped
    builder.emit_load_i(3, 100); // instruction 3: target
    builder.emit_mov(255, 1); // instruction 4: return r1 (42)
    builder.emit_ret(); // instruction 5

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    if let Some(number) = to_f64(vm.frame.regs[255]) {
        println!("  Result: {} (expected 42)", number);
    }

    // Test 2: Simple backward jump (loop)
    println!("\nTest 2: Simple backward jump (loop)");
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 3); // instruction 0: loop counter
    builder.emit_load_i(2, 0); // instruction 1: accumulator

    let loop_start = builder.len(); // Should be 2
    println!("  Loop start at instruction {}", loop_start);

    builder.emit_mov(255, 1); // instruction 2: load counter to ACC
    builder.emit_jmp_false(255, 7); // instruction 3: if counter == 0, jump to the return path

    builder.emit_mov(255, 2); // instruction 4: load accumulator
    builder.emit_inc_acc(); // instruction 5: increment
    builder.emit_mov(2, 255); // instruction 6: store back

    builder.emit_mov(255, 1); // instruction 7: load counter
    builder.emit_sub_acc_imm8(1); // instruction 8: decrement by 1
    builder.emit_mov(1, 255); // instruction 9: store back

    let jump_pos = builder.len(); // Should be 10
    println!("  Jump at instruction {}", jump_pos);

    // Try different jump offsets
    let offset = -(jump_pos as i16 - loop_start as i16 + 1);
    println!(
        "  Offset calculation: -({} - {} + 1) = {}",
        jump_pos, loop_start, offset
    );

    builder.emit_jmp(offset); // instruction 10: jump back

    builder.emit_mov(255, 2); // instruction 11: return accumulator
    builder.emit_ret(); // instruction 12

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    if let Some(number) = to_f64(vm.frame.regs[255]) {
        println!("  Result: {} (expected 3)", number);
    }

    // Print assembly for debugging
    println!("\n=== Assembly for Test 2 ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&vm.bytecode, &vm.const_pool);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }
}
