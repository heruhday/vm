//! Optimized iterative Fibonacci bytecode example.
//! Computes fib(25) = 75025 without changing the VM.

use vm::asm::disassemble_clean;
use vm::emit::BytecodeBuilder;
use vm::js_value::{bool_from_value, is_null, is_string, is_undefined, to_f64};
use vm::vm::VM;

const ACC: u8 = 255;
const N: i16 = 25;
const EXPECTED: f64 = 75_025.0;

fn main() {
    println!("=== Fibonacci Computation Example ===\n");
    println!("Computing fib({N})...");
    println!("Expected result: {EXPECTED}\n");

    let mut builder = BytecodeBuilder::new();

    // r1 = n
    builder.emit_load_i(1, N);

    // if (n <= 1) return n;
    builder.emit_load_i(2, 1);
    builder.emit_lte(1, 2);
    builder.emit_jmp_false(ACC, 2);
    builder.emit_mov(ACC, 1);
    builder.emit_ret();

    // a = 0, b = 1, i = 2
    builder.emit_load_0();
    builder.emit_mov(3, ACC);
    builder.emit_load_1();
    builder.emit_mov(4, ACC);
    builder.emit_load_i(5, 2);

    // Since n > 1 here, the loop always executes at least once.
    // Optimized loop body:
    //   ACC = a + b
    //   a = b
    //   b = ACC
    //   i = i + 1   (single add_i instead of mov/incacc/mov)
    //   if (i <= n) jump back
    let loop_start = builder.len();
    builder.emit_add(3, 4);
    builder.emit_mov(3, 4);
    builder.emit_mov(4, ACC);
    builder.emit_add_i(5, 5, 1);
    builder.emit_lte(5, 1);
    let loop_back = loop_start as i16 - (builder.len() as i16 + 1);
    builder.emit_jmp_true(ACC, loop_back);

    builder.emit_mov(ACC, 4);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    println!("Bytecode generated ({} instructions)", bytecode.len());
    println!("Optimizations applied:");
    println!("  - Removed the temporary c register");
    println!("  - Reused ACC for a + b");
    println!("  - Replaced mov/incacc/mov with add_i");
    println!("  - Switched to a bottom-tested loop");

    let mut vm = VM::new(bytecode, constants, vec![]);
    let start = std::time::Instant::now();
    vm.run(false);
    let duration = start.elapsed();
    println!("Execution time: {:.2?}", duration);

    let result = vm.frame.regs[ACC as usize];

    println!("\n=== Result ===");
    if let Some(number) = to_f64(result) {
        println!("fib({N}) = {number}");
        if (number - EXPECTED).abs() < 0.0001 {
            println!("Result matches expected value.");
        } else {
            println!("Result does not match expected value.");
            println!("Difference: {}", number - EXPECTED);
        }
    } else if let Some(boolean) = bool_from_value(result) {
        println!("Result is boolean: {boolean}");
    } else if is_undefined(result) {
        println!("Result is undefined");
    } else if is_null(result) {
        println!("Result is null");
    } else if is_string(result) {
        println!("Result is a string");
    } else {
        println!("Result type unknown: {:?}", result);
    }

    println!("\n=== Generated Assembly ===");
    for (i, line) in disassemble_clean(&vm.bytecode, &vm.const_pool)
        .iter()
        .enumerate()
    {
        println!("  {:3}: {}", i, line);
    }
}
