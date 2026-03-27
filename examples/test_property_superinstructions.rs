//! Test for Pareto 80% property access superinstructions with IC
//!
//! This test demonstrates the new property access superinstructions:
//! - GET_PROP2_IC: obj.slot1.slot2
//! - GET_PROP3_IC: obj.slot1.slot2.slot3
//! - GET_ELEM: arr[index]
//! - SET_ELEM: arr[index] = value
//! - GET_PROP_ELEM: obj.slot[index]
//! - CALL_METHOD_IC: obj.slot()
//! - CALL_METHOD2_IC: obj.slot1.slot2()

use vm::js_value::{make_number, make_undefined};
use vm::vm::VM;

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

    // Test 5: GET_PROP_ELEM - obj.slot[index]
    test_get_prop_elem();

    // Test 6: CALL_METHOD_IC - obj.slot()
    test_call_method_ic();

    // Test 7: CALL_METHOD2_IC - obj.slot1.slot2()
    test_call_method2_ic();

    println!("\nAll tests completed!");
}

fn test_get_prop2_ic() {
    println!("Test 1: GET_PROP2_IC - obj.slot1.slot2");

    // Create bytecode for: r1 = obj.slot1.slot2
    let bytecode = vec![
        // Create object with nested properties
        0x00000000, // Placeholder for object creation
        // GET_PROP2_IC dst=1, obj=0, slot1=100, slot2=101
        (188 | (1 << 8)) | (100 << 24), // GET_PROP2_IC r1, r0, 100, 101
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test object: obj = { slot1: { slot2: 42 } }
    let obj = vm.alloc_object();
    let inner = vm.alloc_object();
    vm.obj_set_prop(inner, 101, make_number(42.0)); // slot2 = 42
    vm.obj_set_prop(obj, 100, inner); // slot1 = inner object

    // Replace placeholder in const pool
    vm.const_pool[0] = obj;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[1];
    println!("  Result: {:?}", result);
    println!("  Expected: number 42");
    println!();
}

fn test_get_prop3_ic() {
    println!("Test 2: GET_PROP3_IC - obj.slot1.slot2.slot3");

    // Create bytecode for: r1 = obj.slot1.slot2.slot3
    let bytecode = vec![
        // Create object with deeply nested properties
        0x00000000, // Placeholder for object creation
        // GET_PROP3_IC dst=1, obj=0, slot1=100, slot2=101, slot3=102
        (189 | (1 << 8)) | (100 << 24), // GET_PROP3_IC r1, r0, 100, 101, 102
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test object: obj = { slot1: { slot2: { slot3: "hello" } } }
    let obj = vm.alloc_object();
    let middle = vm.alloc_object();
    let inner = vm.alloc_object();
    let hello = vm.intern_string("hello");
    vm.obj_set_prop(inner, 102, hello); // slot3 = "hello"
    vm.obj_set_prop(middle, 101, inner); // slot2 = inner object
    vm.obj_set_prop(obj, 100, middle); // slot1 = middle object

    // Replace placeholder in const pool
    vm.const_pool[0] = obj;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[1];
    println!("  Result: {:?}", result);
    println!("  Expected: string \"hello\"");
    println!();
}

fn test_get_elem() {
    println!("Test 3: GET_ELEM - arr[index]");

    // Create bytecode for: r1 = arr[2]
    let bytecode = vec![
        // Create array
        0x00000000, // Placeholder for array creation
        // Load index 2 into r2
        6 | (2 << 8) | (2 << 16), // LOAD_I r2, 2
        // GET_ELEM dst=1, arr=0, index=2
        (190 | (1 << 8)) | (2 << 24), // GET_ELEM r1, r0, r2
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with array
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test array: arr = [10, 20, 30, 40]
    let arr = vm.alloc_array(4);
    for i in 0..4 {
        vm.obj_set_prop(arr, i as u16, make_number((i as f64 + 1.0) * 10.0));
    }

    // Replace placeholder in const pool
    vm.const_pool[0] = arr;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[1];
    println!("  Result: {:?}", result);
    println!("  Expected: number 30 (arr[2])");
    println!();
}

fn test_set_elem() {
    println!("Test 4: SET_ELEM - arr[index] = value");

    // Create bytecode for: arr[1] = 999
    let bytecode = vec![
        // Create array
        0x00000000, // Placeholder for array creation
        // Load value 999 into r1
        6 | (1 << 8) | (999 << 16), // LOAD_I r1, 999
        // Load index 1 into r2
        6 | (2 << 8) | (1 << 16), // LOAD_I r2, 1
        // SET_ELEM arr=0, index=2, src=1
        191 | (2 << 24), // SET_ELEM r0, r2, r1
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with array
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test array: arr = [10, 20, 30, 40]
    let arr = vm.alloc_array(4);
    for i in 0..4 {
        vm.obj_set_prop(arr, i as u16, make_number((i as f64 + 1.0) * 10.0));
    }

    // Replace placeholder in const pool
    vm.const_pool[0] = arr;

    // Run VM
    vm.run(false);

    // Check if arr[1] was updated to 999
    let updated_value = vm.obj_get_prop(arr, 1);
    println!("  arr[1] after SET_ELEM: {:?}", updated_value);
    println!("  Expected: number 999");
    println!();
}

fn test_get_prop_elem() {
    println!("Test 5: GET_PROP_ELEM - obj.slot[index]");

    // Create bytecode for: r1 = obj.data[2]
    let bytecode = vec![
        // Create object with array property
        0x00000000, // Placeholder for object creation
        // Load index 2 into r2
        6 | (2 << 8) | (2 << 16), // LOAD_I r2, 2
        // GET_PROP_ELEM dst=1, obj=0, slot=100, index=2
        (192 | (1 << 8)) | (100 << 24), // GET_PROP_ELEM r1, r0, 100, r2
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test object: obj = { data: [100, 200, 300, 400] }
    let obj = vm.alloc_object();
    let data = vm.alloc_array(4);
    for i in 0..4 {
        vm.obj_set_prop(data, i as u16, make_number((i as f64 + 1.0) * 100.0));
    }
    vm.obj_set_prop(obj, 100, data); // data = array

    // Replace placeholder in const pool
    vm.const_pool[0] = obj;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[1];
    println!("  Result: obj.data[2] = {:?}", result);
    println!("  Expected: number 300");
    println!();
}

fn test_call_method_ic() {
    println!("Test 6: CALL_METHOD_IC - obj.slot()");

    // Create bytecode for: obj.method()
    let bytecode = vec![
        // Create object with method
        0x00000000, // Placeholder for object creation
        // CALL_METHOD_IC obj=0, slot=100
        193 | (100 << 16), // CALL_METHOD_IC r0, 100
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test object with method that returns 42
    let obj = vm.alloc_object();

    // Create a simple function that returns 42
    // For this test, we'll use a placeholder
    let method = make_number(42.0); // In real implementation, this would be a function

    vm.obj_set_prop(obj, 100, method); // method = function that returns 42

    // Replace placeholder in const pool
    vm.const_pool[0] = obj;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[255]; // ACC register
    println!("  Result of obj.method(): {:?}", result);
    println!("  Expected: number 42");
    println!();
}

fn test_call_method2_ic() {
    println!("Test 7: CALL_METHOD2_IC - obj.slot1.slot2()");

    // Create bytecode for: obj.utils.math.add()
    let bytecode = vec![
        // Create object with nested method
        0x00000000, // Placeholder for object creation
        // CALL_METHOD2_IC obj=0, slot1=100, slot2=101
        194 | (100 << 16) | (101 << 24), // CALL_METHOD2_IC r0, 100, 101
    ];

    let const_pool = vec![
        make_undefined(), // const[0] - will be replaced with object
    ];

    let args = vec![];

    let mut vm = VM::new(bytecode, const_pool, args);

    // Create test object: obj = { utils: { math: { add: function() { return 100 } } } }
    let obj = vm.alloc_object();
    let utils = vm.alloc_object();
    let math = vm.alloc_object();

    let add_method = make_number(100.0); // In real implementation, this would be a function

    vm.obj_set_prop(math, 101, add_method); // add = function
    vm.obj_set_prop(utils, 100, math); // math = object with add method
    vm.obj_set_prop(obj, 100, utils); // utils = object with math property

    // Replace placeholder in const pool
    vm.const_pool[0] = obj;

    // Run VM
    vm.run(false);

    let result = vm.frame.regs[255]; // ACC register
    println!("  Result of obj.utils.math.add(): {:?}", result);
    println!("  Expected: number 100");
    println!();
}
