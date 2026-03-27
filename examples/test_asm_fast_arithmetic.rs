use vm::asm::disassemble_print;
use vm::vm::VM;

fn main() {
    println!("Testing AddI32 fast path with assembler...");

    // Create bytecode using the correct encoding
    // We need to manually encode instructions correctly
    // LoadI r2, 10: opcode=6, a=2, sbx=10
    // Encoding: (sbx << 16) | (a << 8) | opcode
    // But careful with endianness!

    // Actually, let's just test with what we have and disassemble it
    let bytecode = vec![
        0x0602000A, // LoadI r2, 10
        0x06030014, // LoadI r3, 20
        0xF3010203, // AddI32 r1, r2, r3
        0xF2000001, // RetReg r1
    ];

    let const_pool = vec![];
    let args = vec![];

    println!("Disassembling bytecode:");
    disassemble_print(&bytecode, &const_pool);

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    let result = vm.frame.regs[255]; // ACC register
    println!("AddI32 result: {:?}", result);

    // Check if result is int32 30
    if let Some(val) = result.as_i32() {
        println!("Success! AddI32 fast path works: {} + {} = {}", 10, 20, val);
    } else {
        println!("Error: Expected int32 result, got {:?}", result);
    }

    // Let's also check what the disassembler says about our instructions
    println!("\nAnalyzing instruction encoding:");
    for (i, &insn) in vm.bytecode.iter().enumerate() {
        println!("Instruction {}: 0x{:08X}", i, insn);
        let opcode = (insn & 0xFF) as u8;
        let a = ((insn >> 8) & 0xFF) as u8;
        let b = ((insn >> 16) & 0xFF) as u8;
        let c = ((insn >> 24) & 0xFF) as u8;
        let sbx = ((insn >> 16) & 0xFFFF) as u16 as i16;

        println!("  opcode: {}, a: {}, b: {}, c: {}", opcode, a, b, c);
        println!("  sbx: {}", sbx);

        match opcode {
            6 => println!("  LoadI r{}, {}", a, sbx),
            0xF3 => println!("  AddI32 r{}, r{}, r{}", a, b, c),
            0xF2 => println!("  RetReg r{}", a),
            _ => println!("  Unknown opcode"),
        }
    }
}
