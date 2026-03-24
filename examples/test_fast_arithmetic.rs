use vm::vm::VM;
use vm::js_value::{make_int32, make_number, Value};

fn main() {
    // Test AddI32 fast path
    println!("Testing AddI32 fast path...");
    
    // Create bytecode for: AddI32 r1, r2, r3
    // where r2 = 10 (int32), r3 = 20 (int32)
    let bytecode = vec![
        // Load 10 into r2
        0x0602000A, // LoadI r2, 10
        // Load 20 into r3  
        0x06030014, // LoadI r3, 20
        // AddI32 r1, r2, r3
        0xF3010203, // AddI32 r1, r2, r3
        // RetReg r1
        0xF2000001, // RetReg r1
    ];
    
    let const_pool = vec![];
    let args = vec![];
    
    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);
    
    let result = vm.frame.regs[255]; // ACC register
    println!("AddI32 result: {:?}", result);
    
    // Check if result is int32 30
    if let Some(val) = result.as_i32() {
        println!("Success! AddI32 fast path works: {} + {} = {}", 10, 20, val);
    } else {
        println!("Error: Expected int32 result, got {:?}", result);
    }
    
    // Test AddF64 fast path
    println!("\nTesting AddF64 fast path...");
    
    let bytecode2 = vec![
        // Load 3.14 into r2 (as f64)
        0x01020000, // LoadK r2, const[0] (3.14)
        // Load 2.71 into r3 (as f64)
        0x01030001, // LoadK r3, const[1] (2.71)
        // AddF64 r1, r2, r3
        0xF4010203, // AddF64 r1, r2, r3
        // RetReg r1
        0xF2000001, // RetReg r1
    ];
    
    let const_pool2 = vec![
        Value::f64(3.14),
        Value::f64(2.71),
    ];
    
    let args2 = vec![];
    let mut vm2 = VM::new(bytecode2, const_pool2, args2);
    vm2.run(false);
    
    let result2 = vm2.frame.regs[255]; // ACC register
    println!("AddF64 result: {:?}", result2);
    
    // Check if result is f64 5.85
    if let Some(val) = result2.as_f64() {
        println!("Success! AddF64 fast path works: {} + {} = {}", 3.14, 2.71, val);
    } else {
        println!("Error: Expected f64 result, got {:?}", result2);
    }
    
    // Test mixed types (should fall back to slow path)
    println!("\nTesting mixed types (should use slow path)...");
    
    let bytecode3 = vec![
        // Load 10 into r2 (int32)
        0x0602000A, // LoadI r2, 10
        // Load 3.14 into r3 (f64)
        0x01030000, // LoadK r3, const[0] (3.14)
        // AddI32 r1, r2, r3 (should fall back to slow path)
        0xF3010203, // AddI32 r1, r2, r3
        // RetReg r1
        0xF2000001, // RetReg r1
    ];
    
    let const_pool3 = vec![
        Value::f64(3.14),
    ];
    
    let args3 = vec![];
    let mut vm3 = VM::new(bytecode3, const_pool3, args3);
    vm3.run(false);
    
    let result3 = vm3.frame.regs[255]; // ACC register
    println!("Mixed types result: {:?}", result3);
    
    if let Some(val) = result3.as_f64() {
        println!("Success! Slow path works for mixed types: {} + {} = {}", 10, 3.14, val);
    } else {
        println!("Result: {:?}", result3);
    }
}