use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Test Arithmetic ===\n");

    // Test 1: Simple addition
    println!("Test 1: Simple addition (1 + 2)");
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_1(); // ACC = 1
    builder.emit_mov(1, 255); // r1 = 1
    builder.emit_load_i(2, 2); // r2 = 2
    builder.emit_add(1, 2); // ACC = r1 + r2 = 1 + 2 = 3
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 3)", number);
    } else {
        println!("Result is not a number!");
    }

    // Test 2: Addition in a loop
    println!("\n\nTest 2: Sum of 1 to 5");
    let mut builder = BytecodeBuilder::new();

    // Initialize sum = 0, i = 1
    builder.emit_load_0();
    builder.emit_mov(1, 255); // r1 = sum = 0
    builder.emit_load_1();
    builder.emit_mov(2, 255); // r2 = i = 1
    builder.emit_load_i(3, 5); // r3 = 5

    // Loop start
    let loop_start = builder.len();

    // Check if i <= 5
    builder.emit_lte(2, 3); // ACC = i <= 5
    builder.emit_jmp_false(255, 5); // if false, jump to end

    // sum = sum + i
    builder.emit_add(1, 2); // ACC = sum + i
    builder.emit_mov(1, 255); // sum = ACC

    // i = i + 1
    builder.emit_mov(255, 2); // ACC = i
    builder.emit_inc_acc(); // ACC = i + 1
    builder.emit_mov(2, 255); // i = ACC

    // Jump back
    builder.emit_jmp(-(builder.len() as i16 - loop_start as i16 + 1));

    // End: return sum
    builder.emit_mov(255, 1);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 15)", number);
    } else {
        println!("Result is not a number!");
    }

    // Print assembly
    println!("\n=== Assembly for Test 2 ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&vm.bytecode, &vm.const_pool);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }
}
