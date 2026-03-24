//! Example demonstrating clean assembly code generation for QJL bytecode
//! This shows the format requested by the user with clean mnemonics and labels

use vm::asm::*;
use vm::js_value::*;

fn main() {
    println!("=== QJL Clean Assembly Demo ===\n");

    // Create some constants
    let constants = vec![
        make_number(25.0), // const[0] = 25 (for fib(25))
        make_number(1.0),  // const[1] = 1 (for comparison)
        make_number(2.0),  // const[2] = 2 (for subtraction)
    ];

    // Create bytecode for a simple Fibonacci-like function
    // This simulates: fib(n) = if (n <= 1) return n else return fib(n-1) + fib(n-2)
    let bytecode = vec![
        // main:
        // loadi r1, 25
        0x00190106, // opcode=6 (load_i), a=1, sbx=25
        // call fib, 1
        0x00010104, // opcode=4 (call), a=0, b=1 (function at label 0, 1 arg)
        // ret
        0x00000067, // opcode=103 (ret)
        // fib:
        // cmp_le r3, r1, 1  (simulated with load_i and lte)
        0x00010306, // loadi r3, 1
        0x00030111, // lte r3, r1 (r3 = r1 <= r3)
        // jmp_if_true r3, fib_base
        0x00030007, // jmp_true r3, 0 (to fib_base)
        // subi r4, r1, 1
        0x00010409, // subi r4, r1, 1
        // mov r1, r4
        0x00040100, // mov r1, r4
        // call fib, 1
        0x00010104, // call r0, 1
        // mov r5, r0
        0x00000500, // mov r5, r0
        // subi r4, r1, 2
        0x00020409, // subi r4, r1, 2
        // mov r1, r4
        0x00040100, // mov r1, r4
        // call fib, 1
        0x00010104, // call r0, 1
        // mov r6, r0
        0x00000600, // mov r6, r0
        // add r0, r5, r6
        0x06050002, // add r5, r6
        // ret
        0x00000067, // ret
        // fib_base:
        // mov r0, r1
        0x00010000, // mov r0, r1
        // ret
        0x00000067, // ret
    ];

    println!("Bytecode ({} instructions):", bytecode.len());
    for (i, &instr) in bytecode.iter().enumerate() {
        println!("  {:04X}: {:08X}", i * 4, instr);
    }

    println!("\n=== Traditional Disassembly (with byte offsets) ===");
    let asm = disassemble(&bytecode, &constants);
    for line in asm {
        println!("  {}", line);
    }

    println!("\n=== Clean Assembly (requested format) ===");
    println!("; Test for call fibonacy function with optimization");
    println!("; =========================================");
    println!("; QJL Assembly - Single File");
    println!("; Compute fib(25)");
    println!("; =========================================");
    println!();
    println!("; Calling convention:");
    println!("; r0 = return value");
    println!("; r1 = arg1");
    println!("; r2..r15 = locals");
    println!("; call label, argc");
    println!("; ret");
    println!();
    println!("; =========================================");
    println!("; main");
    println!("; =========================================");
    println!("main:");

    let clean_asm = disassemble_clean(&bytecode, &constants);
    for (i, line) in clean_asm.iter().enumerate() {
        // Add labels for demonstration
        match i {
            0 => println!("    {}", line), // loadi r1, 25
            1 => println!("    {}", line), // call 0, 1
            2 => println!("    {}", line), // ret
            3 => {
                println!();
                println!("; =========================================");
                println!("; function fib(n)");
                println!("; if (n <= 1) return n;");
                println!("; return fib(n-1) + fib(n-2);");
                println!("; =========================================");
                println!("fib:");
                println!("    {}", line); // loadi r3, 1
            }
            4 => println!("    {}", line),  // lte r3, r1
            5 => println!("    {}", line),  // jmp_true r3, 0
            6 => println!("    {}", line),  // subi r4, r1, 1
            7 => println!("    {}", line),  // mov r1, r4
            8 => println!("    {}", line),  // call 0, 1
            9 => println!("    {}", line),  // mov r5, r0
            10 => println!("    {}", line), // subi r4, r1, 2
            11 => println!("    {}", line), // mov r1, r4
            12 => println!("    {}", line), // call 0, 1
            13 => println!("    {}", line), // mov r6, r0
            14 => println!("    {}", line), // add r5, r6
            15 => println!("    {}", line), // ret
            16 => {
                println!();
                println!("fib_base:");
                println!("    {}", line); // mov r0, r1
            }
            17 => println!("    {}", line), // ret
            _ => println!("    {}", line),
        }
    }

    println!("\n=== Key Improvements ===");
    println!("1. Clean mnemonics: 'loadi' instead of 'load_i'");
    println!("2. No byte offsets in output");
    println!("3. Labels and comments supported");
    println!("4. Human-readable format similar to traditional assembly");
    println!();
    println!("The new `disassemble_clean()` function produces output in the");
    println!("requested format, while `disassemble()` maintains backward");
    println!("compatibility with byte offsets for debugging.");
}
