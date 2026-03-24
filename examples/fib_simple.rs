//! Simple Fibonacci computation without base case check
//! Computes fib(5) = 5

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Simple Fibonacci Computation ===\n");
    println!("Computing fib(5) without base case check...");
    println!("Expected result: 5\n");

    let mut builder = BytecodeBuilder::new();

    // Initialize a = 0, b = 1
    builder.emit_load_0(); // ACC = 0
    builder.emit_mov(3, 255); // r3 = a = 0
    builder.emit_load_1(); // ACC = 1
    builder.emit_mov(4, 255); // r4 = b = 1

    // Manually compute fib(5)
    // i = 2
    builder.emit_add(3, 4); // ACC = a + b = 0 + 1 = 1
    builder.emit_mov(6, 255); // r6 = c = 1
    builder.emit_mov(3, 4); // a = b = 1
    builder.emit_mov(4, 6); // b = c = 1

    // i = 3
    builder.emit_add(3, 4); // ACC = 1 + 1 = 2
    builder.emit_mov(6, 255); // r6 = c = 2
    builder.emit_mov(3, 4); // a = 1
    builder.emit_mov(4, 6); // b = 2

    // i = 4
    builder.emit_add(3, 4); // ACC = 1 + 2 = 3
    builder.emit_mov(6, 255); // r6 = c = 3
    builder.emit_mov(3, 4); // a = 2
    builder.emit_mov(4, 6); // b = 3

    // i = 5
    builder.emit_add(3, 4); // ACC = 2 + 3 = 5
    builder.emit_mov(6, 255); // r6 = c = 5
    builder.emit_mov(3, 4); // a = 3
    builder.emit_mov(4, 6); // b = 5

    // Return b
    builder.emit_mov(255, 4); // ACC = b
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    println!("Bytecode generated ({} instructions)", bytecode.len());

    // Create and run VM
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    // Get result from accumulator
    let result = vm.frame.regs[255];

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
    } else {
        println!("Result type unknown: {:?}", result);
    }
}
