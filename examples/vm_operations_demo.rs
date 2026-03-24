//! VM Operations Demo
//! Actually demonstrates runtime operations using the VM

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== VM Operations Demo ===\n");

    // Test 1: Arithmetic operations
    println!("1. Arithmetic Operations:");
    let mut builder1 = BytecodeBuilder::new();
    builder1.emit_load_i(1, 5); // load_i r1, 5
    builder1.emit_load_i(2, 3); // load_i r2, 3
    builder1.emit_add(1, 2); // add r1, r2 -> accumulator (5 + 3 = 8)
    builder1.emit_ret(); // ret

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
    builder2.emit_load_i(1, 5); // load_i r1, 5
    builder2.emit_load_i(2, 3); // load_i r2, 3
    builder2.emit_lt(1, 2); // lt r1, r2 -> accumulator (5 < 3 = false)
    builder2.emit_ret(); // ret

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
    builder3.emit_load_i(1, 42); // load_i r1, 42
    builder3.emit_to_str(255, 1); // to_str r1 -> accumulator (42 -> "42")
    builder3.emit_ret(); // ret

    let (bytecode3, constants3) = builder3.build();
    let mut vm3 = VM::new(bytecode3, constants3, vec![]);
    vm3.run(false);
    let result3 = vm3.frame.regs[255];
    println!("  to_string(42) = {:?} (should be string)", result3);

    // Test 4: Complex expression
    println!("\n4. Complex Expression:");
    // Compute (5 + 3) * 2 - 1
    let mut builder4 = BytecodeBuilder::new();
    builder4.emit_load_i(1, 5); // load_i r1, 5
    builder4.emit_load_i(2, 3); // load_i r2, 3
    builder4.emit_add(1, 2); // add r1, r2 -> accumulator (5 + 3 = 8)
    builder4.emit_mov(3, 255); // mov r3, accumulator (save 8)
    builder4.emit_load_i(4, 2); // load_i r4, 2
    builder4.emit_mov(255, 3); // mov accumulator, r3 (load 8)
    builder4.emit_mul_acc(4); // mul_acc r4 (8 * 2 = 16)
    builder4.emit_mov(5, 255); // mov r5, accumulator (save 16)
    builder4.emit_load_i(6, 1); // load_i r6, 1
    builder4.emit_mov(255, 5); // mov accumulator, r5 (load 16)
    builder4.emit_sub_acc(6); // sub_acc r6 (16 - 1 = 15)
    builder4.emit_ret(); // ret

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
    builder5.emit_load_true(1); // load_true -> r1
    builder5.emit_load_false(2); // load_false -> r2
    builder5.emit_eq(1, 2); // eq r1, r2 -> accumulator (true == false = false)
    builder5.emit_ret(); // ret

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
    builder6.emit_load_i(1, 100); // load_i r1, 100
    builder6.emit_typeof(255, 1); // typeof r1 -> accumulator
    builder6.emit_ret(); // ret

    let (bytecode6, constants6) = builder6.build();
    let mut vm6 = VM::new(bytecode6, constants6, vec![]);
    vm6.run(false);
    let result6 = vm6.frame.regs[255];
    println!("  typeof 100 = {:?} (should be 'number')", result6);

    println!("\n=== Summary ===");
    println!("These tests demonstrate actual VM execution of operations.");
    println!("Each test corresponds to trait methods in runtime_trait.rs:");
    println!("  - Test 1: ArithmeticOps::add");
    println!("  - Test 2: ComparisonOps::lt");
    println!("  - Test 3: CoercionOps::to_string");
    println!("  - Test 4: Multiple arithmetic operations");
    println!("  - Test 5: ComparisonOps::eq with booleans");
    println!("  - Test 6: TypeOps::typeof_");
    println!("\nThe VM executes bytecode that implements these operations.");
}
