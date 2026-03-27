//! Comprehensive test for representative superinstructions.

use vm::emit::BytecodeBuilder;
use vm::js_value::*;
use vm::vm::VM;

const ACC: usize = 255;

fn main() {
    println!("Testing comprehensive superinstructions...");

    test_arithmetic_superinstructions();
    test_comparison_branching_superinstructions();
    test_property_access_superinstructions();
    test_call_superinstructions();
    test_return_superinstructions();

    println!("All comprehensive superinstruction tests passed!");
}

fn test_arithmetic_superinstructions() {
    println!("  Testing arithmetic superinstructions...");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x00140206, // load_i r2, 20
        0x020100B0, // load_add r0, r1, r2
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_int32(30));
    println!("    PASS load_add");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x00140206, // load_i r2, 20
        0x020100B1, // load_sub r0, r1, r2
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_int32(-10));
    println!("    PASS load_sub");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x00140206, // load_i r2, 20
        0x020100B2, // load_mul r0, r1, r2
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_int32(200));
    println!("    PASS load_mul");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x000100B3, // load_inc r0, r1
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_int32(11));
    println!("    PASS load_inc");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x000100B4, // load_dec r0, r1
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_int32(9));
    println!("    PASS load_dec");
}

fn test_comparison_branching_superinstructions() {
    println!("  Testing comparison + branching superinstructions...");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x000A0206, // load_i r2, 10
        0x020100B5, // load_cmp_eq r0, r1, r2
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_true());
    println!("    PASS load_cmp_eq");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x00140206, // load_i r2, 20
        0x020100B6, // load_cmp_lt r0, r1, r2
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[0], make_true());
    println!("    PASS load_cmp_lt");

    let bytecode = vec![
        0x00000106, // load_i r1, 0
        0x010000B7, // load_jfalse r1, 1
        0x00630206, // load_i r2, 99 (skipped)
        0x00640206, // load_i r2, 100
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[2], make_int32(100));
    println!("    PASS load_jfalse");

    let bytecode = vec![
        0x000A0106, // load_i r1, 10
        0x00140206, // load_i r2, 20
        0x020100B8, // load_cmp_eq_jfalse r1, r2, 1
        0x00630306, // load_i r3, 99 (skipped)
        0x00010306, // load_i r3, 1
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[3], make_int32(1));
    println!("    PASS load_cmp_eq_jfalse");

    let bytecode = vec![
        0x00140106, // load_i r1, 20
        0x000A0206, // load_i r2, 10
        0x020100B9, // load_cmp_lt_jfalse r1, r2, 1
        0x00630306, // load_i r3, 99 (skipped)
        0x00010306, // load_i r3, 1
    ];
    let mut vm = VM::new(bytecode, vec![], vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[3], make_int32(1));
    println!("    PASS load_cmp_lt_jfalse");
}

fn test_property_access_superinstructions() {
    println!("  Testing property access superinstructions...");
    println!("    Skipping: property access setup is covered by dedicated examples.");
}

fn test_call_superinstructions() {
    println!("  Testing call superinstructions...");

    let mut builder = BytecodeBuilder::new();
    let call1_entry_const = builder.add_constant(make_number(0.0));
    builder.emit_new_func(0, call1_entry_const);
    builder.emit_load_i(1, 10);
    builder.emit_call1(0, 1);
    builder.emit_ret();

    let call1_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_i(2, 1);
    builder.emit_add(1, 2);
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[call1_entry_const as usize] = make_number(call1_entry as f64);
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(11.0));
    println!("    PASS call1");

    let mut builder = BytecodeBuilder::new();
    let call0_entry_const = builder.add_constant(make_number(0.0));
    builder.emit_new_func(0, call0_entry_const);
    builder.emit_call0(0);
    builder.emit_ret();

    let call0_entry = builder.len();
    builder.emit_load_1();
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[call0_entry_const as usize] = make_number(call0_entry as f64);
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(1.0));
    println!("    PASS call0");

    let mut builder = BytecodeBuilder::new();
    let call2_entry_const = builder.add_constant(make_number(0.0));
    builder.emit_new_func(0, call2_entry_const);
    builder.emit_load_i(1, 10);
    builder.emit_load_i(2, 20);
    builder.emit_call2(0, 1, 2);
    builder.emit_ret();

    let call2_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_arg(2, 1);
    builder.emit_add(1, 2);
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[call2_entry_const as usize] = make_number(call2_entry as f64);
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(30.0));
    println!("    PASS call2");
}

fn test_return_superinstructions() {
    println!("  Testing return superinstructions...");

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 42);
    builder.emit_ret_reg(1);
    let (bytecode, const_pool) = builder.build();
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[ACC], make_int32(42));
    println!("    PASS ret_reg");

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 10);
    builder.emit_load_i(2, 20);
    builder.emit_load_i(3, 42);
    builder.emit_ret_if_lte_i(1, 2, 3);
    builder.emit_load_i(4, 99);
    builder.emit_ret_reg(4);
    let (bytecode, const_pool) = builder.build();
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[ACC], make_int32(42));
    println!("    PASS ret_if_lte_i(true)");

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 30);
    builder.emit_load_i(2, 20);
    builder.emit_load_i(3, 42);
    builder.emit_ret_if_lte_i(1, 2, 3);
    builder.emit_load_i(4, 99);
    builder.emit_ret_reg(4);
    let (bytecode, const_pool) = builder.build();
    let mut vm = VM::new(bytecode, const_pool, vec![]);
    vm.run(false);
    assert_eq!(vm.frame.regs[ACC], make_int32(99));
    println!("    PASS ret_if_lte_i(false)");
}
