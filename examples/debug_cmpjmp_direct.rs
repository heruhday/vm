use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Debugging CmpJmp with direct jump...");

    // Test 1: Simple jump (unconditional)
    println!("Test 1: Simple Jmp instruction");
    let bytecode = vec![
        0x00050106, // LoadI a=1, value=5
        0x000A0206, // LoadI a=2, value=10
        0x0200000C, // Jmp offset=2 (opcode 0x0C, offset in c field = 0x02)
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        0x0001FF06, // LoadI a=ACC, value=1 (should be executed)
    ];

    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    println!("  ACC: {:?}", vm.frame.regs[ACC]);

    // Test 2: CmpJmp with false condition
    println!("\nTest 2: CmpJmp with false condition (10 < 5)");
    let bytecode = vec![
        0x000A0106, // LoadI a=1, value=10
        0x00050206, // LoadI a=2, value=5
        0x020201FF, // CmpJmp a=1, b=2, offset=2 (10 < 5 is false, shouldn't jump)
        0x0000FF06, // LoadI a=ACC, value=0 (should execute)
        0x0001FF06, // LoadI a=ACC, value=1 (should execute after)
    ];

    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    println!("  ACC: {:?} (should be 1)", vm.frame.regs[ACC]);

    // Test 3: CmpJmp with true condition
    println!("\nTest 3: CmpJmp with true condition (5 < 10)");
    let bytecode = vec![
        0x00050106, // LoadI a=1, value=5
        0x000A0206, // LoadI a=2, value=10
        0x020201FF, // CmpJmp a=1, b=2, offset=2 (5 < 10 is true, should jump)
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        0x0001FF06, // LoadI a=ACC, value=1 (should execute)
    ];

    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    println!("  ACC: {:?} (should be 1)", vm.frame.regs[ACC]);
}
