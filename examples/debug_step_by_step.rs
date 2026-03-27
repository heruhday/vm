use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging step by step...");

    // Simple test: LoadI then RetIfLteI
    let bytecode = vec![
        // Load values - little-endian format
        0x000A0106, // LoadI a=1, value=10 (bytes: [0x06, 0x01, 0x0A, 0x00])
        0x000A0206, // LoadI a=2, value=10 (bytes: [0x06, 0x02, 0x0A, 0x00])
        0x00140306, // LoadI a=3, value=20 (bytes: [0x06, 0x03, 0x14, 0x00])
        // Test RetIfLteI: if reg[1] <= reg[2], return reg[3]
        0x030201F9, // RetIfLteI a=1, b=2, c=3 (bytes: [0xF9, 0x01, 0x02, 0x03])
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("Initial state:");
    println!("  PC: {}", vm.pc);

    // Manually execute instructions
    for i in 0..4 {
        println!("\n--- Instruction {} ---", i);
        println!("  PC before: {}", vm.pc);
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
        let opcode = vm::vm::Opcode::from(opcode_byte);
        println!("  Opcode: {:?}", opcode);

        match opcode {
            vm::vm::Opcode::LoadI => {
                let sbx = ((insn >> 16) & 0xFFFF) as u16 as i16;
                println!("  LoadI: a={}, value={} (sbx={})", a, sbx, sbx);
                vm.frame.regs[a] = make_int32(sbx as i32);
                println!(
                    "  Set reg[{}] = {:?} (bits: {})",
                    a,
                    vm.frame.regs[a],
                    vm.frame.regs[a].bits()
                );
            }
            vm::vm::Opcode::RetIfLteI => {
                println!("  RetIfLteI: a={}, b={}, c={}", a, b, c);
                println!("  reg[{}] = {:?}", a, vm.frame.regs[a]);
                println!("  reg[{}] = {:?}", b, vm.frame.regs[b]);
                println!("  reg[{}] = {:?}", c, vm.frame.regs[c]);
                // Simplified check
                if vm.frame.regs[a].bits() <= vm.frame.regs[b].bits() {
                    println!("  Condition true, would return reg[{}]", c);
                } else {
                    println!("  Condition false, would continue");
                }
            }
            _ => {
                println!("  Unknown opcode");
            }
        }

        println!("  PC after: {}", vm.pc);
        println!("  reg[1]: {:?}", vm.frame.regs[1]);
        println!("  reg[2]: {:?}", vm.frame.regs[2]);
        println!("  reg[3]: {:?}", vm.frame.regs[3]);
        println!("  ACC: {:?}", vm.frame.regs[ACC]);
    }
}
