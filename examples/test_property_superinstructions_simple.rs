//! Simple test for Pareto 80% property access superinstructions with IC
//!
//! This test demonstrates the new property access superinstructions using the public API.

use vm::js_value::make_undefined;
use vm::vm::{Opcode, VM};

fn encode_abc(opcode: Opcode, a: u8, b: u8, c: u8) -> u32 {
    ((c as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_abx(opcode: Opcode, a: u8, bx: u16) -> u32 {
    ((bx as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_asbx(opcode: Opcode, a: u8, sbx: i16) -> u32 {
    (((sbx as u16) as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn main() {
    println!("Testing Pareto 80% property access superinstructions with IC");
    println!("===========================================================\n");

    // Test 1: GET_PROP2_IC - obj.slot1.slot2
    test_get_prop2_ic();

    // Test 2: GET_PROP3_IC - obj.slot1.slot2.slot3
    test_get_prop3_ic();

    // Test 3: GET_ELEM - arr[index]
    test_get_elem();

    // Test 4: SET_ELEM - arr[index] = value
    test_set_elem();

    println!("\nAll tests completed!");
}

fn test_get_prop2_ic() {
    println!("Test 1: GET_PROP2_IC - obj.slot1.slot2");

    // Create bytecode for: r1 = obj.slot1.slot2
    // We'll simulate: const obj = { a: { b: 42 } }; const result = obj.a.b;
    let bytecode = vec![
        // Load object into r0 (we'll create it in const pool)
        encode_abx(Opcode::LoadK, 0, 0),
        // GET_PROP2_IC dst=1, obj=0, slot1=0, slot2=1
        // slot1=0 means property id 0 ("a"), slot2=1 means property id 1 ("b")
        encode_abc(Opcode::GetProp2Ic, 1, 0, 0),
        // Return result in ACC
        encode_abc(Opcode::LoadAcc, 1, 0, 0),
        Opcode::Ret.as_u8() as u32,
    ];

    // We need to create the object in the VM after it's initialized
    // For now, just test that the opcode doesn't crash
    let const_pool = vec![
        make_undefined(), // const[0] - placeholder for object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("  Running GET_PROP2_IC opcode...");
    vm.run(false);

    println!("  Test completed without crash");
    println!();
}

fn test_get_prop3_ic() {
    println!("Test 2: GET_PROP3_IC - obj.slot1.slot2.slot3");

    // Create bytecode for: r1 = obj.a.b.c
    let bytecode = vec![
        // Load object into r0
        encode_abx(Opcode::LoadK, 0, 0),
        // GET_PROP3_IC dst=1, obj=0, slot1=0, slot2=1, slot3=2
        encode_abc(Opcode::GetProp3Ic, 1, 0, 0),
        // Return result in ACC
        encode_abc(Opcode::LoadAcc, 1, 0, 0),
        Opcode::Ret.as_u8() as u32,
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - placeholder for object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("  Running GET_PROP3_IC opcode...");
    vm.run(false);

    println!("  Test completed without crash");
    println!();
}

fn test_get_elem() {
    println!("Test 3: GET_ELEM - arr[index]");

    // Create bytecode for: r1 = arr[2]
    let bytecode = vec![
        // Load array into r0
        encode_abx(Opcode::LoadK, 0, 0),
        // Load index 2 into r2
        encode_asbx(Opcode::LoadI, 2, 2),
        // GET_ELEM dst=1, arr=0, index=2
        encode_abc(Opcode::GetElem, 1, 0, 2),
        // Return result in ACC
        encode_abc(Opcode::LoadAcc, 1, 0, 0),
        Opcode::Ret.as_u8() as u32,
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - placeholder for array
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("  Running GET_ELEM opcode...");
    vm.run(false);

    println!("  Test completed without crash");
    println!();
}

fn test_set_elem() {
    println!("Test 4: SET_ELEM - arr[index] = value");

    // Create bytecode for: arr[1] = 999
    let bytecode = vec![
        // Load array into r0
        encode_abx(Opcode::LoadK, 0, 0),
        // Load value 999 into r1
        encode_asbx(Opcode::LoadI, 1, 999),
        // Load index 1 into r2
        encode_asbx(Opcode::LoadI, 2, 1),
        // SET_ELEM arr=0, index=2, src=1
        encode_abc(Opcode::SetElem, 1, 0, 2),
        // Return ACC (which contains the set value)
        Opcode::Ret.as_u8() as u32,
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - placeholder for array
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    println!("  Running SET_ELEM opcode...");
    vm.run(false);

    println!("  Test completed without crash");
    println!();
}
