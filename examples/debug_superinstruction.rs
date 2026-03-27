use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging superinstruction execution...");

    // Simple test: LoadI then RetIfLteI
    let bytecode = vec![
        // Load values
        0x0601000A, // LoadI a=1, value=10 (opcode=0x06, a=0x01, value=0x000A)
        0x0602000A, // LoadI a=2, value=10
        0x06030014, // LoadI a=3, value=20
        // Test RetIfLteI: if reg[1] <= reg[2], return reg[3]
        0xF9010203, // RetIfLteI a=1, b=2, c=3
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Run one instruction at a time
    println!("Initial state:");
    println!("  PC: {}", vm.pc);
    println!(
        "  reg[1]: {:?} (bits: {})",
        vm.frame.regs[1],
        vm.frame.regs[1].bits()
    );
    println!(
        "  reg[2]: {:?} (bits: {})",
        vm.frame.regs[2],
        vm.frame.regs[2].bits()
    );
    println!(
        "  reg[3]: {:?} (bits: {})",
        vm.frame.regs[3],
        vm.frame.regs[3].bits()
    );

    // Execute first LoadI
    vm.run(false);

    println!("\nAfter execution:");
    println!("  PC: {}", vm.pc);
    println!(
        "  reg[1]: {:?} (bits: {})",
        vm.frame.regs[1],
        vm.frame.regs[1].bits()
    );
    println!(
        "  reg[2]: {:?} (bits: {})",
        vm.frame.regs[2],
        vm.frame.regs[2].bits()
    );
    println!(
        "  reg[3]: {:?} (bits: {})",
        vm.frame.regs[3],
        vm.frame.regs[3].bits()
    );
    println!(
        "  ACC: {:?} (bits: {})",
        vm.frame.regs[ACC],
        vm.frame.regs[ACC].bits()
    );

    // Check what make_int32(20) gives us
    println!("\nExpected value:");
    let expected = make_int32(20);
    println!(
        "  make_int32(20): {:?} (bits: {})",
        expected,
        expected.bits()
    );
}
