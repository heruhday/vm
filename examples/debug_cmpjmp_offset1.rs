use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging CmpJmp with offset=1...");

    // Test with offset=1 (jump over next instruction)
    let bytecode = vec![
        0x00050106, // LoadI a=1, value=5
        0x000A0206, // LoadI a=2, value=10
        0x010201FF, // CmpJmp a=1, b=2, offset=1 (c=0x01)
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        0x0001FF06, // LoadI a=ACC, value=1 (should execute)
    ];

    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    println!("  ACC: {:?} (should be 1)", vm.frame.regs[ACC]);

    // Test with offset=2 (jump to the final instruction)
    println!("\nTest with offset=2 (jump to the final instruction)");
    let bytecode = vec![
        0x00050106, // LoadI a=1, value=5
        0x000A0206, // LoadI a=2, value=10
        0x020201FF, // CmpJmp a=1, b=2, offset=2 (c=0x02)
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        0x0001FF06, // LoadI a=ACC, value=1 (should be skipped)
        0x0002FF06, // LoadI a=ACC, value=2 (should execute)
    ];

    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    println!("  ACC: {:?} (should be 2)", vm.frame.regs[ACC]);

    println!("\nSkipping negative offset case.");
    println!("  A taken negative offset with unchanged operands would loop forever here.");
}
