use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("Debugging LoadInc...");

    // Simple test: LoadI then LoadInc
    let bytecode = vec![
        0x000A0106, // loadi r1, 10
        0x010000B3, // loadinc r0, r1 (b=1, a=0, opcode=0xB3)
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("Before execution:");
    println!("  reg[0]: {:?}", vm.frame.regs[0]);
    println!("  reg[1]: {:?}", vm.frame.regs[1]);

    vm.run(false);

    println!("After execution:");
    println!("  reg[0]: {:?} (expected: Int(11))", vm.frame.regs[0]);
    println!("  reg[1]: {:?} (should be: Int(10))", vm.frame.regs[1]);

    // Check what make_int32(11) gives us
    println!("\nExpected value:");
    let expected = make_int32(11);
    println!(
        "  make_int32(11): {:?} (bits: {})",
        expected,
        expected.bits()
    );
}
