use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging CmpJmp...");

    let bytecode = vec![
        // Load values - little-endian
        0x00050106, // LoadI a=1, value=5 (bytes: [0x06, 0x01, 0x05, 0x00])
        0x000A0206, // LoadI a=2, value=10 (bytes: [0x06, 0x02, 0x0A, 0x00])
        // Test CmpJmp: if reg[1] < reg[2], jump +2 - little-endian: [0xFF, 0x01, 0x02, 0x02]
        0x020201FF, // CmpJmp a=1, b=2, offset=2
        // This should be skipped - little-endian: [0x06, 0xFF, 0x00, 0x00]
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        // This should be executed - little-endian: [0x06, 0xFF, 0x01, 0x00]
        0x0001FF06, // LoadI a=ACC, value=1
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode.clone(), const_pool.clone(), args.clone());

    println!("Before execution:");
    println!("  PC: {}", vm.pc);
    println!("  reg[1]: {:?}", vm.frame.regs[1]);
    println!("  reg[2]: {:?}", vm.frame.regs[2]);
    println!("  ACC: {:?}", vm.frame.regs[ACC]);

    // Execute step by step
    for i in 0..5 {
        println!("\n--- Step {} ---", i);
        println!("  PC before: {}", vm.pc);
        if vm.pc < vm.bytecode.len() {
            let insn = vm.bytecode[vm.pc];
            println!("  Instruction: 0x{:08X}", insn);

            let opcode_byte = (insn & 0xFF) as u8;
            let a = ((insn >> 8) & 0xFF) as usize;
            let b = ((insn >> 16) & 0xFF) as usize;
            let c = ((insn >> 24) & 0xFF) as usize;

            println!(
                "  Decoded: opcode={}, a={}, b={}, c={}",
                opcode_byte, a, b, c
            );

            // Execute one instruction
            vm.pc += 1;
            // In real VM, the instruction would be executed here
            // For now, just print what would happen
        } else {
            println!("  PC out of bounds!");
        }
        println!("  PC after: {}", vm.pc);
        println!("  reg[1]: {:?}", vm.frame.regs[1]);
        println!("  reg[2]: {:?}", vm.frame.regs[2]);
        println!("  ACC: {:?}", vm.frame.regs[ACC]);
    }

    // Now run the actual VM
    println!("\n--- Running actual VM ---");
    let bytecode2 = bytecode.clone();
    let const_pool2 = const_pool.clone();
    let args2 = args.clone();
    let mut vm2 = VM::new(bytecode2, const_pool2, args2);
    vm2.run(false);

    println!("After VM execution:");
    println!("  PC: {}", vm2.pc);
    println!("  reg[1]: {:?}", vm2.frame.regs[1]);
    println!("  reg[2]: {:?}", vm2.frame.regs[2]);
    println!(
        "  ACC: {:?} (bits: {})",
        vm2.frame.regs[ACC],
        vm2.frame.regs[ACC].bits()
    );

    // Check what make_int32(1) gives
    println!("\nExpected:");
    println!(
        "  make_int32(1): {:?} (bits: {})",
        make_int32(1),
        make_int32(1).bits()
    );
}
