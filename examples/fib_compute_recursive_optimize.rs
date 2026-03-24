//! Recursive Fibonacci bytecode example using new_func + call.

use vm::asm::disassemble_clean;
use vm::emit::BytecodeBuilder;
use vm::js_value::{bool_from_value, is_null, is_string, is_undefined, make_number, to_f64};
use vm::vm::{Opcode, VM};

const ACC: u8 = 255;
const N: i16 = 25;
const EXPECTED: f64 = 75025.0;
const WARMUP_RUNS: usize = 2;
const MEASURED_RUNS: usize = 10;

fn patch_abc_offset(bytecode: &mut [u32], index: usize, opcode: u8, a: u8, b: u8, target: usize) {
    let offset = target as i16 - (index as i16 + 1);
    let offset = i8::try_from(offset).expect("jump offset must fit in i8");
    bytecode[index] =
        ((offset as u8 as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode as u32;
}

fn main() {
    println!("=== Recursive Fibonacci Bytecode Example ===\n");
    if cfg!(debug_assertions) {
        println!("Note: debug build timings are misleading; use --release for benchmarking.\n");
    }
    println!("Computing fib({N}) recursively...");
    println!("Expected result: {EXPECTED}\n");

    let mut builder = BytecodeBuilder::new();
    let fib_entry_const = builder.add_constant(make_number(0.0));

    // main:
    //   const fib = function(n) { ... }
    //   remember fib in an upvalue slot for self-recursion
    //   return fib(N)
    builder.emit_new_func(1, fib_entry_const);
    builder.emit_set_upval(1, 0);
    builder.emit_load_i(2, N);
    builder.emit_call(1, 1);
    builder.emit_ret();

    // fib(n):
    //   if (n <= 1) return n;
    //   return fib(n - 1) + fib(n - 2);
    let fib_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_i(2, 1);
    let recurse_jump = builder.len();
    builder.emit_jmp_lte_false(1, 2, 0);
    builder.emit_ret_reg(1);

    let recurse_label = builder.len();
    builder.emit_get_upval(4, 0);

    // Fused recursive call: fib(n - 1)
    builder.emit_call1_sub_i(4, 1, 1); // ACC = fib(n-1)

    // save result
    builder.emit_mov(3, ACC); // r3 = fib(n-1)

    // Fused recursive call: fib(n - 2)
    builder.emit_call1_sub_i(4, 1, 2); // ACC = fib(n-2)

    // add - optimized: add r3 to ACC (using Add instead of AddAcc)
    builder.emit_add(3, ACC); // ACC = fib(n-1) + fib(n-2)
    builder.emit_ret();

    let (mut bytecode, mut constants) = builder.build();
    constants[fib_entry_const as usize] = make_number(fib_entry as f64);
    patch_abc_offset(
        &mut bytecode,
        recurse_jump,
        Opcode::JmpLteFalse.as_u8(),
        1,
        2,
        recurse_label,
    );

    println!("Bytecode generated ({} instructions)", bytecode.len());

    for _ in 0..WARMUP_RUNS {
        let mut vm = VM::new(bytecode.clone(), constants.clone(), vec![]);
        vm.run(false);
    }

    let mut timings = Vec::with_capacity(MEASURED_RUNS);
    let mut result = make_number(0.0);
    for _ in 0..MEASURED_RUNS {
        let mut vm = VM::new(bytecode.clone(), constants.clone(), vec![]);
        vm.optimize();
        let start = std::time::Instant::now();
        vm.run(false);
        timings.push(start.elapsed());
        result = vm.frame.regs[ACC as usize];
    }

    let total = timings.iter().copied().sum::<std::time::Duration>();
    let min = timings.iter().copied().min().unwrap_or_default();
    let max = timings.iter().copied().max().unwrap_or_default();
    let avg = total / MEASURED_RUNS as u32;
    println!(
        "Execution time over {MEASURED_RUNS} runs: avg {:.2?}, min {:.2?}, max {:.2?}",
        avg, min, max
    );

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
    for (i, line) in disassemble_clean(&bytecode, &constants).iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }
}
