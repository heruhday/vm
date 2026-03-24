use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Test JMPFALSE Offset ===\n");

    // Test: jmpfalse with offset 1
    println!("Test: jmpfalse with offset 1");
    let mut builder = BytecodeBuilder::new();

    // Load false into ACC
    builder.emit_load_false(255); // ACC = false

    // jmpfalse with offset 1 (should skip load0)
    builder.emit_jmp_false(255, 1);

    builder.emit_load_0(); // This should be skipped
    builder.emit_load_1(); // This should be executed
    builder.emit_ret();

    let (bytecode, constants) = builder.build();

    println!("Bytecode length: {}", bytecode.len());

    // Print assembly
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&bytecode, &constants);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }

    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("\nRaw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 1)", number);
    } else {
        println!("Result is not a number: {:?}", result);
    }

    // Test 2: jmpfalse with offset 2
    println!("\n\nTest 2: jmpfalse with offset 2");
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_false(255); // ACC = false
    builder.emit_jmp_false(255, 2); // should skip load0 and load1
    builder.emit_load_0(); // skipped
    builder.emit_load_1(); // skipped  
    builder.emit_load_i(1, 42); // executed
    builder.emit_mov(255, 1); // ACC = 42
    builder.emit_ret();

    let (bytecode, constants) = builder.build();

    println!("Bytecode length: {}", bytecode.len());

    let assembly = disassemble_clean(&bytecode, &constants);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }

    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("\nRaw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 42)", number);
    } else {
        println!("Result is not a number: {:?}", result);
    }
}
