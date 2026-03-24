//! Fibonacci computation example for QJL bytecode
//! Computes fib(5) and expects result 5

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Fibonacci Computation Example ===\n");
    println!("Computing fib(5)...");
    println!("Expected result: 5\n");

    let mut builder = BytecodeBuilder::new();

    // main: load argument n=5 into r1
    builder.emit_load_i(1, 5); // r1 = n

    // --- Base case: if n <= 1, return n ---
    builder.emit_load_i(2, 1); // r2 = 1
    builder.emit_lte(1, 2); // ACC = r1 <= r2 -> boolean
    // Jump to the initializer block. The VM advances PC before applying the offset.
    builder.emit_jmp_false(255, 2); // if n > 1, skip the base-case return
    builder.emit_mov(255, 1); // ACC = r1 (return n)
    builder.emit_ret(); // ret

    // Initialize a = 0, b = 1
    builder.emit_load_0(); // ACC = 0
    builder.emit_mov(3, 255); // r3 = a = 0
    builder.emit_load_1(); // ACC = 1
    builder.emit_mov(4, 255); // r4 = b = 1

    // i = 2
    builder.emit_load_i(5, 2); // r5 = i = 2

    // --- Loop start ---
    let loop_start = builder.len();

    builder.emit_lte(5, 1); // ACC = i <= n
    builder.emit_jmp_false(255, 8); // if false, jump to loop end

    builder.emit_add(3, 4); // ACC = a + b
    builder.emit_mov(6, 255); // r6 = c

    builder.emit_mov(3, 4); // a = b
    builder.emit_mov(4, 6); // b = c

    builder.emit_mov(255, 5); // ACC = i
    builder.emit_inc_acc(); // ACC = i + 1
    builder.emit_mov(5, 255); // i = ACC

    builder.emit_jmp(-(builder.len() as i16 - loop_start as i16 + 1)); // jump back

    // --- Loop end: return b ---
    builder.emit_mov(255, 4); // ACC = b
    builder.emit_ret();
    let (bytecode, constants) = builder.build();

    println!("Bytecode generated ({} instructions)", bytecode.len());

    // Create and run VM
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    // Get result from accumulator
    let result = vm.frame.regs[255]; // ACC is 255

    // Display result
    println!("\n=== Result ===");
    println!("Raw result value: {:?}", result);

    if let Some(number) = to_f64(result) {
        println!("fib(5) = {}", number);

        // Check if result matches expected
        let expected = 5.0;
        if (number - expected).abs() < 0.0001 {
            println!("✓ Result matches expected value: {}", expected);
        } else {
            println!("✗ Result does not match expected value: {}", expected);
            println!("  Difference: {}", number - expected);
        }
    } else if let Some(boolean) = bool_from_value(result) {
        println!("Result is boolean: {}", boolean);
    } else if is_undefined(result) {
        println!("Result is undefined");
    } else if is_null(result) {
        println!("Result is null");
    } else if is_string(result) {
        // For string display, we can use the display_string method
        println!("Result is string (cannot display private content)");
    } else {
        println!("Result type unknown: {:?}", result);
    }

    // Also show the assembly for debugging
    println!("\n=== Generated Assembly (for debugging) ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&vm.bytecode, &vm.const_pool);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }
}
