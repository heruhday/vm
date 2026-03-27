use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    // Test superinstructions
    println!("Testing superinstructions...");

    // Test 1: RetIfLteI - if a <= b, return c
    let bytecode = vec![
        // Load values - little-endian format
        0x000A0106, // LoadI a=1, value=10 (bytes: [0x06, 0x01, 0x0A, 0x00])
        0x000A0206, // LoadI a=2, value=10
        0x00140306, // LoadI a=3, value=20
        // Test RetIfLteI: if reg[1] <= reg[2], return reg[3]
        0x030201F9, // RetIfLteI a=1, b=2, c=3 (bytes: [0xF9, 0x01, 0x02, 0x03])
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("Test 1 - RetIfLteI:");
    println!("  Result: {}", vm.frame.regs[ACC].bits());
    println!("  Expected: {}", make_int32(20).bits());

    // Test 2: AddAccReg - ACC = reg[a] + reg[b]
    let bytecode = vec![
        // Load values - little-endian
        0x000A0106, // LoadI a=1, value=10
        0x00140206, // LoadI a=2, value=20
        // Test AddAccReg: ACC = reg[1] + reg[2]
        0xFF0201FA, // AddAccReg a=1, b=2, c=ACC (0xFF) (bytes: [0xFA, 0x01, 0x02, 0xFF])
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("\nTest 2 - AddAccReg:");
    println!("  Result: {}", vm.frame.regs[ACC].bits());
    println!("  Expected: {}", make_number(30.0).bits());

    // Test 3: LoadKAdd - reg[a] = const_pool[index] + ACC
    let bytecode = vec![
        // Load ACC with 5 - little-endian: [0x06, 0xFF, 0x05, 0x00]
        0x0005FF06, // LoadI a=ACC, value=5
        // Test LoadKAdd: reg[1] = const_pool[0] + ACC - little-endian: [0xFD, 0x01, 0x00, 0x00]
        0x000001FD, // LoadKAdd a=1, index=0
    ];

    let const_pool = vec![make_int32(10)];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("\nTest 3 - LoadKAdd:");
    println!("  Result: {}", vm.frame.regs[1].bits());
    println!("  Expected: {}", make_number(15.0).bits());

    // Test 4: LoadKCmp - ACC = const_pool[index] < reg[a]
    let bytecode = vec![
        // Load reg[1] with 20 - little-endian: [0x06, 0x01, 0x14, 0x00]
        0x00140106, // LoadI a=1, value=20
        // Test LoadKCmp: ACC = const_pool[0] < reg[1] - little-endian: [0xFE, 0x01, 0x00, 0x00]
        0x000001FE, // LoadKCmp a=1, index=0
    ];

    let const_pool = vec![make_int32(10)];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("\nTest 4 - LoadKCmp:");
    println!("  Result: {}", vm.frame.regs[ACC].bits());
    println!("  Expected: {}", make_true().bits());

    // Test 5: CmpJmp - if reg[a] < reg[b], jump by offset
    let bytecode = vec![
        // Load values - little-endian
        0x00050106, // LoadI a=1, value=5 (bytes: [0x06, 0x01, 0x05, 0x00])
        0x000A0206, // LoadI a=2, value=10 (bytes: [0x06, 0x02, 0x0A, 0x00])
        // Test CmpJmp: if reg[1] < reg[2], jump +1 - little-endian: [0xFF, 0x01, 0x02, 0x01]
        0x010201FF, // CmpJmp a=1, b=2, offset=1
        // This should be skipped - little-endian: [0x06, 0xFF, 0x00, 0x00]
        0x0000FF06, // LoadI a=ACC, value=0 (should be skipped)
        // This should be executed - little-endian: [0x06, 0xFF, 0x01, 0x00]
        0x0001FF06, // LoadI a=ACC, value=1
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("\nTest 5 - CmpJmp:");
    println!("  Result: {}", vm.frame.regs[ACC].bits());
    println!("  Expected: {}", make_int32(1).bits());

    // Test 6: Specialized opcodes - AddI32Fast
    let bytecode = vec![
        // Load int32 values - little-endian
        0x000A0106, // LoadI a=1, value=10 (bytes: [0x06, 0x01, 0x0A, 0x00])
        0x00140206, // LoadI a=2, value=20 (bytes: [0x06, 0x02, 0x14, 0x00])
        // Test AddI32Fast: ACC = reg[1] + reg[2] - little-endian: [0x82, 0xFF, 0x01, 0x02]
        0x0201FF82, // AddI32Fast a=ACC, b=1, c=2 (opcode 130 = 0x82)
    ];

    let const_pool = vec![];
    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    println!("\nTest 6 - AddI32Fast:");
    println!("  Result: {}", vm.frame.regs[ACC].bits());
    println!("  Expected: {}", make_int32(30).bits());

    // Test 7: Call opcodes - Call0 (skipped - requires proper function implementation)
    // Note: Call0 is implemented but requires actual function objects, not simple numbers
    println!("\nTest 7 - Call0:");
    println!("  Skipped - requires proper function implementation");

    println!("\nAll superinstruction tests completed!");
}
