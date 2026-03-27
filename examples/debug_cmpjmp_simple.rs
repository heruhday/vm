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

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("After VM execution:");
    println!("  PC: {}", vm.pc);
    println!("  reg[1]: {:?}", vm.frame.regs[1]);
    println!("  reg[2]: {:?}", vm.frame.regs[2]);
    println!(
        "  ACC: {:?} (bits: {})",
        vm.frame.regs[ACC],
        vm.frame.regs[ACC].bits()
    );

    // Check what make_int32(1) gives
    println!("\nExpected:");
    println!(
        "  make_int32(1): {:?} (bits: {})",
        make_int32(1),
        make_int32(1).bits()
    );

    // Also check what happens if we don't jump
    println!("\n--- Testing without jump ---");
    let bytecode2 = vec![
        0x00050106, // LoadI a=1, value=5
        0x000A0206, // LoadI a=2, value=10
        0x0001FF06, // LoadI a=ACC, value=1 (direct, no jump)
    ];

    let mut vm2 = VM::new(bytecode2, vec![], vec![]);
    vm2.run(false);
    println!(
        "  ACC: {:?} (bits: {})",
        vm2.frame.regs[ACC],
        vm2.frame.regs[ACC].bits()
    );
}
