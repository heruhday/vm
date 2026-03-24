use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Test Add with Register Values ===\n");

    // Test: add values that come from load0 and load1
    let mut builder = BytecodeBuilder::new();

    // Load 0 into r3 via ACC
    builder.emit_load_0();
    builder.emit_mov(3, 255);

    // Load 1 into r4 via ACC
    builder.emit_load_1();
    builder.emit_mov(4, 255);

    // Add r3 and r4
    builder.emit_add(3, 4);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);

    println!("Before execution:");
    println!("  r3: {:?}", vm.frame.regs[3]);
    println!("  r4: {:?}", vm.frame.regs[4]);
    println!("  ACC: {:?}", vm.frame.regs[255]);

    vm.run(false);

    println!("\nAfter execution:");
    println!("  r3: {:?}", vm.frame.regs[3]);
    println!("  r4: {:?}", vm.frame.regs[4]);
    println!("  ACC: {:?}", vm.frame.regs[255]);

    let result = vm.frame.regs[255];
    if let Some(number) = to_f64(result) {
        println!("\nResult: {} (expected 1)", number);
    } else {
        println!("\nResult is not a number: {:?}", result);
    }

    // Test 2: add with lte in between
    println!("\n\n=== Test 2: Add with lte in between ===");
    let mut builder = BytecodeBuilder::new();

    // Load values
    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 1);

    // lte writes boolean to ACC
    builder.emit_lte(1, 2);

    // Load 0 into r3 via ACC (overwrites the boolean)
    builder.emit_load_0();
    builder.emit_mov(3, 255);

    // Load 1 into r4 via ACC
    builder.emit_load_1();
    builder.emit_mov(4, 255);

    // Add r3 and r4
    builder.emit_add(3, 4);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 1)", number);
    } else {
        println!("Result is not a number: {:?}", result);
    }
}
