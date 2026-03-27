use std::f64::consts::PI;

use vm::js_value::Value;
use vm::vm::VM;

fn main() {
    println!("Testing LoadK with f64 values...");

    // Simple bytecode: LoadK r1, const[0] (3.14), RetReg r1
    let bytecode = vec![
        0x00000101, // LoadK r1, const[0] (little-endian: [0x01, 0x01, 0x00, 0x00])
        0x000001F2, // RetReg r1 (little-endian: [0xF2, 0x01, 0x00, 0x00])
    ];

    let const_pool = vec![Value::f64(PI)];

    let args = vec![];
    let mut vm = VM::new(bytecode, const_pool, args);
    vm.run(false);

    let result = vm.frame.regs[255]; // ACC register
    println!("Result: {:?}", result);
    println!("Result.is_f64(): {}", result.is_f64());
    println!("Result.as_f64(): {:?}", result.as_f64());

    // Also check r1
    let r1 = vm.frame.regs[1];
    println!("r1: {:?}", r1);
    println!("r1.is_f64(): {}", r1.is_f64());
    println!("r1.as_f64(): {:?}", r1.as_f64());
}
