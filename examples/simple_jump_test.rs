use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Simple Jump Test ===\n");

    // Test: jump forward 1 instruction
    println!("Test: jump forward 1 instruction");
    let mut builder = BytecodeBuilder::new();

    // Instruction 0: load 42 into r1
    builder.emit_load_i(1, 42);

    // Instruction 1: jump forward 1 (should skip instruction 2)
    builder.emit_jmp(1);

    // Instruction 2: load 99 into r2 (should be skipped)
    builder.emit_load_i(2, 99);

    // Instruction 3: return r1
    builder.emit_mov(255, 1);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    if let Some(number) = to_f64(vm.frame.regs[255]) {
        println!("  Result: {} (expected 42)", number);
        if number == 42.0 {
            println!("  ✓ PASS: Jump forward 1 works correctly");
        } else {
            println!("  ✗ FAIL: Expected 42, got {}", number);
        }
    }

    // Print assembly
    println!("\n=== Assembly ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&vm.bytecode, &vm.const_pool);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }

    // Test 2: jump backward 1 instruction (infinite loop test)
    println!("\n\nTest 2: jump backward 1 instruction");
    let mut builder = BytecodeBuilder::new();

    // Instruction 0: load 5 into r1 (loop counter)
    builder.emit_load_i(1, 5);

    // Instruction 1: label (loop start)
    let loop_start = builder.len();

    // Instruction 1: decrement counter
    builder.emit_mov(255, 1);
    builder.emit_sub_acc_imm8(1);
    builder.emit_mov(1, 255);

    // Instruction 4: if counter != 0, jump back
    builder.emit_mov(255, 1);
    builder.emit_jmp_false(255, 2); // jump forward 2 if counter == 0

    // Instruction 6: jump back to loop_start
    let jump_pos = builder.len();
    let offset = -(jump_pos as i16 - loop_start as i16);
    println!(
        "  Jump position: {}, Loop start: {}, Offset: {}",
        jump_pos, loop_start, offset
    );
    builder.emit_jmp(offset);

    // Instruction 7: return counter (should be 0)
    builder.emit_mov(255, 1);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);

    // Run with timeout
    println!("  Running (might timeout if infinite loop)...");
    vm.run(false);

    if let Some(number) = to_f64(vm.frame.regs[255]) {
        println!("  Result: {} (expected 0)", number);
        if number == 0.0 {
            println!("  ✓ PASS: Loop works correctly");
        } else {
            println!("  ✗ FAIL: Expected 0, got {}", number);
        }
    }
}
