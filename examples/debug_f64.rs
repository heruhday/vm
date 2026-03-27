use std::f64::consts::PI;

use vm::js_value::Value;
use vm::runtime_trait::ArithmeticOps;

fn main() {
    println!("Testing Value::f64()...");

    let v1 = Value::f64(PI);
    let v2 = Value::f64(2.71);

    println!("v1: {:?}", v1);
    println!("v2: {:?}", v2);

    println!("v1.is_f64(): {}", v1.is_f64());
    println!("v2.is_f64(): {}", v2.is_f64());

    println!("v1.as_f64(): {:?}", v1.as_f64());
    println!("v2.as_f64(): {:?}", v2.as_f64());

    // Test addition
    let result = v1.add(&v2);
    println!("{PI} + 2.71 = {:?}", result);
    println!("Result as_f64(): {:?}", result.as_f64());

    // Test with ints
    let i1 = Value::i32(10);
    let i2 = Value::i32(20);
    println!("\ni1: {:?}, is_int: {}", i1, i1.is_int());
    println!("i2: {:?}, is_int: {}", i2, i2.is_int());

    let int_result = i1.add(&i2);
    println!("10 + 20 = {:?}", int_result);
    println!("Result as_i32(): {:?}", int_result.as_i32());

    // Test mixed
    let mixed_result = i1.add(&v1);
    println!("\n10 + {PI} = {:?}", mixed_result);
    println!("Result as_f64(): {:?}", mixed_result.as_f64());
}
