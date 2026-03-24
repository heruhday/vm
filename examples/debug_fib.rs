use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("=== Debug Fibonacci Computation ===\n");

    let mut builder = BytecodeBuilder::new();

    // main: load argument n=25 into r1
    builder.emit_load_i(1, 25); // r1 = n

    // --- Base case: if n <= 1, return n ---
    builder.emit_load_i(2, 1); // r2 = 1
    builder.emit_lte(1, 2); // ACC = r1 <= r2 -> boolean
    builder.emit_jmp_false(255, 4); // if n > 1, skip 4 instructions (to init a,b)
    builder.emit_mov(255, 1); // ACC = r1 (return n)
    builder.emit_ret(); // ret

    // Initialize a = 0, b = 1
    builder.emit_load_0(); // ACC = 0
    builder.emit_mov(3, 255); // r3 = a = 0
    builder.emit_load_1(); // ACC = 1
    builder.emit_mov(4, 255); // r4 = b = 1

    // i = 2
    builder.emit_load_i(5, 2); // r5 = i = 2

    // --- Loop start ---
    let loop_start = builder.len();
    println!("Loop start at position: {}", loop_start);

    builder.emit_lte(5, 1); // ACC = i <= n
    let jmpfalse_pos = builder.len();
    println!("jmpfalse at position: {}", jmpfalse_pos);
    builder.emit_jmp_false(255, 8); // if false, jump to loop end

    builder.emit_add(3, 4); // ACC = a + b
    builder.emit_mov(6, 255); // r6 = c

    builder.emit_mov(3, 4); // a = b
    builder.emit_mov(4, 6); // b = c

    builder.emit_mov(255, 5); // ACC = i
    builder.emit_inc_acc(); // ACC = i + 1
    builder.emit_mov(5, 255); // i = ACC

    let jump_pos = builder.len();
    let offset = -(jump_pos as i16 - loop_start as i16);
    println!("Jump at position: {}, offset: {}", jump_pos, offset);
    builder.emit_jmp(offset); // jump back

    // --- Loop end: return b ---
    let loop_end = builder.len();
    println!("Loop end at position: {}", loop_end);
    builder.emit_mov(255, 4); // ACC = b
    builder.emit_ret();

    let (bytecode, constants) = builder.build();

    println!("\n=== Generated Assembly ===");
    use vm::asm::disassemble_clean;
    let assembly = disassemble_clean(&bytecode, &constants);
    for (i, line) in assembly.iter().enumerate() {
        println!("  {:3}: {}", i, line);
    }

    // Calculate what the jmpfalse offset should be
    println!("\n=== Offset Calculations ===");
    println!("jmpfalse position: {}", jmpfalse_pos);
    println!("Target (loop end): {}", loop_end);
    println!("When jmpfalse executes, PC = {}", jmpfalse_pos + 1);
    println!(
        "Offset needed: {} - {} = {}",
        loop_end,
        jmpfalse_pos + 1,
        loop_end as i16 - (jmpfalse_pos as i16 + 1)
    );
    println!("Current offset: 8");

    // Calculate what the jmp offset should be
    println!("\nJump position: {}", jump_pos);
    println!("Loop start: {}", loop_start);
    println!("When jump executes, PC = {}", jump_pos + 1);
    println!(
        "Offset needed: {} - {} = {}",
        loop_start,
        jump_pos + 1,
        loop_start as i16 - (jump_pos as i16 + 1)
    );
    println!("Current offset: {}", offset);
}
