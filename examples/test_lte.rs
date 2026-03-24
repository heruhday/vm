use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Test LTE Instruction ===\n");

    // Test 1: 5 <= 1 should be false
    println!("Test 1: 5 <= 1 (should be false)");
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 1);
    builder.emit_lte(1, 2); // ACC = 5 <= 1
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(boolean) = bool_from_value(result) {
        println!("Boolean result: {} (expected false)", boolean);
    } else if let Some(number) = to_f64(result) {
        println!("Numeric result: {} (should be boolean)", number);
    } else {
        println!("Result is not boolean or number: {:?}", result);
    }

    // Test 2: 1 <= 5 should be true
    println!("\n\nTest 2: 1 <= 5 (should be true)");
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_i(1, 1);
    builder.emit_load_i(2, 5);
    builder.emit_lte(1, 2); // ACC = 1 <= 5
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(boolean) = bool_from_value(result) {
        println!("Boolean result: {} (expected true)", boolean);
    } else if let Some(number) = to_f64(result) {
        println!("Numeric result: {} (should be boolean)", number);
    } else {
        println!("Result is not boolean or number: {:?}", result);
    }

    // Test 3: lte followed by jmpfalse
    println!("\n\nTest 3: lte followed by jmpfalse");
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 1);
    builder.emit_lte(1, 2); // ACC = 5 <= 1 (false)
    builder.emit_jmp_false(255, 2); // if false, jump over next instruction
    builder.emit_load_0(); // This should be skipped
    builder.emit_load_1(); // This should be executed
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 1)", number);
    } else {
        println!("Result is not a number: {:?}", result);
    }
}
