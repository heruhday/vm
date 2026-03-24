//! Clean Bytecode Generation Demo
//! Shows a cleaner API for generating bytecode using the real BytecodeBuilder

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Clean Bytecode Generation Demo ===\n");

    // Test 1: Arithmetic operations
    println!("1. Arithmetic Operations:");
    let mut builder1 = BytecodeBuilder::new();
    builder1.emit_load_i(1, 5);
    builder1.emit_load_i(2, 3);
    builder1.emit_add(1, 2);
    builder1.emit_ret();

    let (bytecode1, constants1) = builder1.build();
    let mut vm1 = VM::new(bytecode1, constants1, vec![]);
    vm1.run(false);
    let result1 = vm1.frame.regs[255];
    if let Some(number) = to_f64(result1) {
        println!("  5 + 3 = {} (expected: 8)", number);
    }

    // Test 2: Comparison operations
    println!("\n2. Comparison Operations:");
    let mut builder2 = BytecodeBuilder::new();
    builder2.emit_load_i(1, 5);
    builder2.emit_load_i(2, 3);
    builder2.emit_lt(1, 2);
    builder2.emit_ret();

    let (bytecode2, constants2) = builder2.build();
    let mut vm2 = VM::new(bytecode2, constants2, vec![]);
    vm2.run(false);
    let result2 = vm2.frame.regs[255];
    if let Some(boolean) = bool_from_value(result2) {
        println!("  5 < 3 = {} (expected: false)", boolean);
    }

    // Test 3: Type conversion
    println!("\n3. Type Conversion Operations:");
    let mut builder3 = BytecodeBuilder::new();
    builder3.emit_load_i(1, 42);
    builder3.emit_to_str(255, 1);
    builder3.emit_ret();

    let (bytecode3, constants3) = builder3.build();
    let mut vm3 = VM::new(bytecode3, constants3, vec![]);
    vm3.run(false);
    let result3 = vm3.frame.regs[255];
    println!("  to_string(42) = {:?} (should be string)", result3);

    // Test 4: Complex expression
    println!("\n4. Complex Expression:");
    let mut builder4 = BytecodeBuilder::new();
    // Compute (5 + 3) * 2 - 1
    builder4.emit_load_i(1, 5);
    builder4.emit_load_i(2, 3);
    builder4.emit_add(1, 2); // 5 + 3 = 8 -> accumulator
    builder4.emit_mov(3, 255); // r3 = accumulator (save 8)
    builder4.emit_load_i(4, 2); // r4 = 2
    builder4.emit_mov(255, 3); // accumulator = r3 (load 8)
    builder4.emit_mul_acc(4); // 8 * 2 = 16 -> accumulator
    builder4.emit_mov(5, 255); // r5 = accumulator (save 16)
    builder4.emit_load_i(6, 1); // r6 = 1
    builder4.emit_mov(255, 5); // accumulator = r5 (load 16)
    builder4.emit_sub_acc(6); // 16 - 1 = 15 -> accumulator
    builder4.emit_ret();

    let (bytecode4, constants4) = builder4.build();
    let mut vm4 = VM::new(bytecode4, constants4, vec![]);
    vm4.run(false);
    let result4 = vm4.frame.regs[255];
    if let Some(number) = to_f64(result4) {
        println!("  (5 + 3) * 2 - 1 = {} (expected: 15)", number);
    }

    // Test 5: Boolean logic
    println!("\n5. Boolean Logic:");
    let mut builder5 = BytecodeBuilder::new();
    builder5.emit_load_true(1);
    builder5.emit_load_false(2);
    builder5.emit_eq(1, 2);
    builder5.emit_ret();

    let (bytecode5, constants5) = builder5.build();
    let mut vm5 = VM::new(bytecode5, constants5, vec![]);
    vm5.run(false);
    let result5 = vm5.frame.regs[255];
    if let Some(boolean) = bool_from_value(result5) {
        println!("  true == false = {} (expected: false)", boolean);
    }

    // Test 6: Type checking
    println!("\n6. Type Checking:");
    let mut builder6 = BytecodeBuilder::new();
    builder6.emit_load_i(1, 100);
    builder6.emit_typeof(255, 1);
    builder6.emit_ret();

    let (bytecode6, constants6) = builder6.build();
    let mut vm6 = VM::new(bytecode6, constants6, vec![]);
    vm6.run(false);
    let result6 = vm6.frame.regs[255];
    println!("  typeof 100 = {:?} (should be 'number')", result6);

    println!("\n=== API Comparison ===");
    println!("Old way (manual instruction encoding):");
    println!("  make_asbx_instr(6, 1, 5)");
    println!("  make_asbx_instr(6, 2, 3)");
    println!("  make_instr(2, 0, 1, 2)");
    println!("  make_instr(103, 0, 0, 0)");
    println!("\nNew way (using BytecodeBuilder):");
    println!("  emit_load_i(1, 5)");
    println!("  emit_load_i(2, 3)");
    println!("  emit_add(1, 2)");
    println!("  emit_ret()");
    println!("\nThe BytecodeBuilder API is cleaner and more readable!");
    println!("It also handles constant pool management automatically.");
}
