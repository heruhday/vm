use vm::vm::VM;
use vm::js_value::{make_int32, Value};

fn main() {
    println!("Debugging AddI32 fast path...");
    
    // Simple test: just load two ints and add them
    let bytecode = vec![
        // Load 10 into r2
        0x0602000A, // LoadI r2, 10
        // Load 20 into r3  
        0x06030014, // LoadI r3, 20
        // AddI32 r1, r2, r3
        0xF3010203, // AddI32 r1, r2, r3
        // Print result (we'll add a custom opcode or just check registers)
    ];
    
    let const_pool = vec![];
    let args = vec![];
    
    let mut vm = VM::new(bytecode, const_pool, args);
    
    // Run one instruction at a time
    println!("Initial PC: {}", vm.pc);
    
    // Execute LoadI r2, 10
    if vm.pc < vm.bytecode.len() {
        let insn = vm.bytecode[vm.pc];
        vm.pc += 1;
        println!("Instruction 1: 0x{:08X}", insn);
        // Manually execute LoadI
        let a = ((insn >> 8) & 0xFF) as usize;
        let value = ((insn >> 16) & 0xFFFF) as u16 as i16 as i32;
        vm.frame.regs[a] = make_int32(value);
        println!("  Loaded {} into r{}", value, a);
    }
    
    // Execute LoadI r3, 20
    if vm.pc < vm.bytecode.len() {
        let insn = vm.bytecode[vm.pc];
        vm.pc += 1;
        println!("Instruction 2: 0x{:08X}", insn);
        // Manually execute LoadI
        let a = ((insn >> 8) & 0xFF) as usize;
        let value = ((insn >> 16) & 0xFFFF) as u16 as i16 as i32;
        vm.frame.regs[a] = make_int32(value);
        println!("  Loaded {} into r{}", value, a);
    }
    
    // Check registers
    println!("r2: {:?}", vm.frame.regs[2]);
    println!("r3: {:?}", vm.frame.regs[3]);
    println!("Is r2 int? {}", vm.frame.regs[2].is_int());
    println!("Is r3 int? {}", vm.frame.regs[3].is_int());
    
    if vm.frame.regs[2].is_int() {
        println!("r2 value: {}", vm.frame.regs[2].int_payload_unchecked());
    }
    if vm.frame.regs[3].is_int() {
        println!("r3 value: {}", vm.frame.regs[3].int_payload_unchecked());
    }
    
    // Execute AddI32 r1, r2, r3
    if vm.pc < vm.bytecode.len() {
        let insn = vm.bytecode[vm.pc];
        vm.pc += 1;
        println!("Instruction 3: 0x{:08X}", insn);
        
        // Decode instruction
        let opcode = (insn & 0xFF) as u8;
        let a = ((insn >> 8) & 0xFF) as usize;
        let b = ((insn >> 16) & 0xFF) as usize;
        let c = ((insn >> 24) & 0xFF) as usize;
        
        println!("  Opcode: {}, a: {}, b: {}, c: {}", opcode, a, b, c);
        println!("  r{} = r{} + r{}", a, b, c);
        
        // Manually execute AddI32
        let lhs = vm.frame.regs[b];
        let rhs = vm.frame.regs[c];
        
        println!("  lhs: {:?}, is_int: {}", lhs, lhs.is_int());
        println!("  rhs: {:?}, is_int: {}", rhs, rhs.is_int());
        
        if lhs.is_int() && rhs.is_int() {
            let a_int = lhs.int_payload_unchecked();
            let b_int = rhs.int_payload_unchecked();
            println!("  a_int: {}, b_int: {}", a_int, b_int);
            if let Some(result) = a_int.checked_add(b_int) {
                println!("  Result: {}", result);
                vm.frame.regs[255] = make_int32(result); // ACC
                if a != 255 {
                    vm.frame.regs[a] = make_int32(result);
                }
                println!("  Stored result in ACC and r{}", a);
            } else {
                println!("  Overflow!");
            }
        } else {
            println!("  Not both ints, would fall back to slow path");
        }
    }
    
    println!("ACC (r255): {:?}", vm.frame.regs[255]);
    println!("r1: {:?}", vm.frame.regs[1]);
}