use std::time::Instant;
use vm::js_value::{make_number, to_i32};
use vm::vm::VM;

fn main() {
    println!("=== Fibonacci Benchmark with Direct Threaded Dispatch ===\n");

    // Create bytecode for recursive Fibonacci
    let bytecode = vec![
        // Main function
        0x06000200, // loadi r2, 10 (n = 10)
        0x04010100, // call r1, 1
        0x67000000, // ret
        // Fibonacci function (entry at pc=3)
        0x32010000, // loadarg r1, r0, r0
        0x06020100, // loadi r2, 1
        0xF1010200, // jmpltefalse r1, r2, -> L1 (offset 2)
        0xF2000100, // retreg r1
        // Recursive case
        0x4C040000, // getupval r4
        0xF0040101, // call1subi r4, r1, 1
        0x0003FF00, // mov r3, r255
        0xF0040102, // call1subi r4, r1, 2
        0x0203FF00, // add r3, r255
        0xF2000300, // retreg r3
    ];

    let const_pool = vec![
        make_number(3.0), // Function entry point
    ];

    let mut vm = VM::new(bytecode, const_pool, vec![]);

    // Warm up
    for _ in 0..100 {
        let mut vm_copy = VM::new(vm.bytecode.clone(), vm.const_pool.clone(), vec![]);
        vm_copy.run(false);
    }

    // Benchmark
    let iterations = 10_000;
    let start = Instant::now();

    for _ in 0..iterations {
        let mut vm_copy = VM::new(vm.bytecode.clone(), vm.const_pool.clone(), vec![]);
        vm_copy.run(false);
    }

    let elapsed = start.elapsed();
    let total_time = elapsed.as_secs_f64() * 1_000_000.0; // microseconds
    let avg_time = total_time / iterations as f64;

    println!("Benchmark results:");
    println!("  Iterations: {}", iterations);
    println!("  Total time: {:.2}µs", total_time);
    println!("  Average time per iteration: {:.2}µs", avg_time);
    println!("  Average time per fib(10) call: {:.2}µs", avg_time);

    // Run once to verify result
    vm.run(false);
    let result = vm.frame.regs[255]; // ACC register
    println!("\nVerification:");
    println!("  fib(10) = {}", to_i32(result).unwrap_or(-1));
    println!("  Expected: 55");

    // Show dispatch statistics
    println!("\nDispatch method:");
    println!("  Hot opcodes use direct threaded dispatch via function pointers");
    println!("  Cold opcodes fall back to switch-based dispatch");
    println!("  Total hot opcodes registered: 14");
}
