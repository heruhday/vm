//! Assertion Instructions Demo
//! Shows how to use the new assertion instructions in the VM

use vm::emit::BytecodeBuilder;
use vm::vm::VM;

fn main() {
    println!("=== Assertion Instructions Demo ===\n");

    // Test 1: assert_value - check if value is truthy
    println!("1. Assert Value (truthy):");
    let mut builder1 = BytecodeBuilder::new();
    builder1.emit_load_i(1, 5); // load_i r1, 5
    builder1.emit_assert_value(1); // assert_value r1 (5 is truthy)
    builder1.emit_ret(); // ret

    let (bytecode1, constants1) = builder1.build();
    let mut vm1 = VM::new(bytecode1, constants1, vec![]);
    vm1.run(false);
    println!("  ✓ assert_value(5) passed (5 is truthy)");

    // Test 2: assert_equal - check if values are equal
    println!("\n2. Assert Equal:");
    let mut builder2 = BytecodeBuilder::new();
    builder2.emit_load_i(1, 5); // load_i r1, 5
    builder2.emit_load_i(2, 5); // load_i r2, 5
    builder2.emit_assert_equal(1, 2); // assert_equal r1, r2
    builder2.emit_ret(); // ret

    let (bytecode2, constants2) = builder2.build();
    let mut vm2 = VM::new(bytecode2, constants2, vec![]);
    vm2.run(false);
    println!("  ✓ assert_equal(5, 5) passed");

    // Test 3: assert_not_equal - check if values are not equal
    println!("\n3. Assert Not Equal:");
    let mut builder3 = BytecodeBuilder::new();
    builder3.emit_load_i(1, 5); // load_i r1, 5
    builder3.emit_load_i(2, 3); // load_i r2, 3
    builder3.emit_assert_not_equal(1, 2); // assert_not_equal r1, r2
    builder3.emit_ret(); // ret

    let (bytecode3, constants3) = builder3.build();
    let mut vm3 = VM::new(bytecode3, constants3, vec![]);
    vm3.run(false);
    println!("  ✓ assert_not_equal(5, 3) passed");

    // Test 4: assert_strict_equal - check strict equality
    println!("\n4. Assert Strict Equal:");
    let mut builder4 = BytecodeBuilder::new();
    builder4.emit_load_i(1, 0); // load_i r1, 0
    builder4.emit_load_i(2, 0); // load_i r2, 0
    builder4.emit_assert_strict_equal(1, 2); // assert_strict_equal r1, r2
    builder4.emit_ret(); // ret

    let (bytecode4, constants4) = builder4.build();
    let mut vm4 = VM::new(bytecode4, constants4, vec![]);
    vm4.run(false);
    println!("  ✓ assert_strict_equal(0, 0) passed");

    // Test 5: assert_ok - check if value is not undefined/null
    println!("\n5. Assert Ok:");
    let mut builder5 = BytecodeBuilder::new();
    builder5.emit_load_i(1, 42); // load_i r1, 42
    builder5.emit_assert_ok(1); // assert_ok r1
    builder5.emit_ret(); // ret

    let (bytecode5, constants5) = builder5.build();
    let mut vm5 = VM::new(bytecode5, constants5, vec![]);
    vm5.run(false);
    println!("  ✓ assert_ok(42) passed");

    // Test 6: assert_fail - should always panic
    println!("\n6. Assert Fail (should panic):");
    let mut builder6 = BytecodeBuilder::new();
    builder6.emit_assert_fail(); // assert_fail
    builder6.emit_ret(); // ret

    let (bytecode6, constants6) = builder6.build();
    println!("  Running assert_fail...");
    let result = std::panic::catch_unwind(|| {
        let mut vm6 = VM::new(bytecode6.clone(), constants6.clone(), vec![]);
        vm6.run(false);
    });

    match result {
        Ok(_) => println!("  ✗ assert_fail should have panicked but didn't"),
        Err(_) => println!("  ✓ assert_fail correctly panicked"),
    }

    println!("\n=== Summary ===");
    println!("Assertion instructions added to VM:");
    println!("  - assert_value (opcode 225) - checks if value is truthy");
    println!("  - assert_ok (opcode 226) - checks if value is not undefined/null");
    println!("  - assert_equal (opcode 227) - checks abstract equality");
    println!("  - assert_not_equal (opcode 228) - checks not equal");
    println!("  - assert_deep_equal (opcode 229) - checks deep equality");
    println!("  - assert_not_deep_equal (opcode 230) - checks not deep equal");
    println!("  - assert_strict_equal (opcode 231) - checks strict equality");
    println!("  - assert_not_strict_equal (opcode 232) - checks not strict equal");
    println!("  - assert_deep_strict_equal (opcode 233) - checks deep strict equality");
    println!("  - assert_not_deep_strict_equal (opcode 234) - checks not deep strict equal");
    println!("  - assert_throws (opcode 235) - checks if function throws");
    println!("  - assert_does_not_throw (opcode 236) - checks if function doesn't throw");
    println!("  - assert_rejects (opcode 237) - checks if promise rejects");
    println!("  - assert_does_not_reject (opcode 238) - checks if promise doesn't reject");
    println!("  - assert_fail (opcode 239) - always fails");
    println!("\nThese instructions are useful for debug/test builds to add runtime assertions.");
    println!(
        "\nNote: Using BytecodeBuilder provides a cleaner, type-safe API for generating bytecode."
    );
}
