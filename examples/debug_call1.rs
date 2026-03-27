use vm::js_value::*;
use vm::vm::VM;

fn main() {
    println!("Debugging Call1...");

    // Create a simple function that returns its argument + 1
    let bytecode = vec![
        // Function entry point (PC 0)
        0x00010106, // loadi r1, 1 (0x00010106 = sbx=1, a=1, opcode=6)
        0x02000100, // add r1, r0, r1 (ACC = arg0 + 1)
        0xF2000100, // retreg r1 (return ACC which is in r1)
        // Main code (PC 3)
        0x000A0106, // loadi r1, 10 (argument) (0x000A0106 = sbx=10, a=1, opcode=6)
        0x8C000100, // call1 r0, r1 (call function at r0 with 1 arg)
    ];
    let const_pool = vec![];
    let args = vec![];
    let mut vm = VM::new(bytecode, const_pool, args);

    // Set up function pointer - WRONG: this is a number, not a function object
    vm.frame.regs[0] = make_number(0.0); // Function entry point at PC 0

    println!("Before call: reg[0] = {:?}", vm.frame.regs[0]);
    println!("Before call: reg[1] = {:?}", vm.frame.regs[1]);

    vm.run(false);

    println!("After call: reg[0] = {:?}", vm.frame.regs[0]);
    println!("After call: ACC = {:?}", vm.frame.regs[255]);

    // Now test with a proper function object
    println!("\nTesting with proper function object...");

    let bytecode2 = vec![
        // Function entry point (PC 0)
        0x00010106, // loadi r1, 1
        0x02000100, // add r1, r0, r1
        0xF2000100, // retreg r1
        // Create function object (PC 3)
        0x43000000, // newfunc r3, 0 (create function with descriptor 0)
        // Main code (PC 4)
        0x000A0106, // loadi r1, 10
        0x8C030100, // call1 r3, r1 (call function at r3 with 1 arg)
    ];
    let const_pool2 = vec![make_number(0.0)]; // Descriptor at index 0
    let args2 = vec![];
    let mut vm2 = VM::new(bytecode2, const_pool2, args2);

    vm2.run(false);

    println!(
        "After call with function object: reg[0] = {:?}",
        vm2.frame.regs[0]
    );
    println!(
        "After call with function object: ACC = {:?}",
        vm2.frame.regs[255]
    );
}
