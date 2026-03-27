use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging AddAccReg...");

    let bytecode = vec![
        // Load values - little-endian
        0x000A0106, // LoadI a=1, value=10
        0x00140206, // LoadI a=2, value=20
        // Test AddAccReg: ACC = reg[1] + reg[2]
        0xFF0201FA, // AddAccReg a=1, b=2, c=ACC (0xFF)
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("Before execution:");
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
        "  ACC: {:?} (bits: {})",
        vm.frame.regs[ACC],
        vm.frame.regs[ACC].bits()
    );

    vm.run(false);

    println!("\nAfter execution:");
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
        "  ACC: {:?} (bits: {})",
        vm.frame.regs[ACC],
        vm.frame.regs[ACC].bits()
    );

    // Check what values we expect
    println!("\nExpected values:");
    let val10 = make_int32(10);
    let val20 = make_int32(20);
    let val30 = make_int32(30);
    println!("  make_int32(10): {:?} (bits: {})", val10, val10.bits());
    println!("  make_int32(20): {:?} (bits: {})", val20, val20.bits());
    println!("  make_int32(30): {:?} (bits: {})", val30, val30.bits());

    // Also check what 24.0 looks like
    let val24_float = make_number(24.0);
    println!(
        "  make_number(24.0): {:?} (bits: {})",
        val24_float,
        val24_float.bits()
    );
}
