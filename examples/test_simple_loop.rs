use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Test Simple Loop ===\n");

    // Test: count from 5 down to 0
    let mut builder = BytecodeBuilder::new();

    // Initialize counter = 5
    builder.emit_load_i(1, 5); // r1 = counter = 5

    // Loop start
    let loop_start = builder.len();
    println!("Loop start: {}", loop_start);

    // Check if counter == 0
    builder.emit_mov(255, 1); // ACC = counter
    builder.emit_jmp_false(255, 4); // if ACC == 0, jump to end

    // Decrement counter
    builder.emit_mov(255, 1); // ACC = counter
    builder.emit_sub_acc_imm8(1); // ACC = counter - 1
    builder.emit_mov(1, 255); // counter = ACC

    // Jump back
    let jump_pos = builder.len();
    let offset = -(jump_pos as i16 - loop_start as i16 + 1);
    println!("Jump position: {}, offset: {}", jump_pos, offset);
    builder.emit_jmp(offset);

    // End: return counter (should be 0)
    let end_pos = builder.len();
    println!("End position: {}", end_pos);
    builder.emit_mov(255, 1); // ACC = counter
    builder.emit_ret();

    let (bytecode, constants) = builder.build();

    println!("\n=== Assembly ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&bytecode, &constants);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }

    // Calculate jmpfalse offset
    println!("\n=== Offset Calculations ===");
    let jmpfalse_pos = loop_start + 1;
    println!("jmpfalse position: {}", jmpfalse_pos);
    println!("Target (end): {}", end_pos);
    println!("When jmpfalse executes, PC = {}", jmpfalse_pos + 1);
    println!(
        "Offset needed: {} - {} = {}",
        end_pos,
        jmpfalse_pos + 1,
        end_pos as i16 - (jmpfalse_pos as i16 + 1)
    );
    println!("Current offset: 4");

    // Run VM
    println!("\n=== Running VM ===");
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = vm.frame.regs[255];
    println!("Raw result: {:?}", result);
    if let Some(number) = to_f64(result) {
        println!("Result: {} (expected 0)", number);
    } else {
        println!("Result is not a number!");
    }
}
