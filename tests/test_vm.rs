use vm::emit::BytecodeBuilder;
use vm::js_value::{bool_from_value, is_undefined, make_number, to_f64};
use vm::vm::{ICState, Opcode, VM, ValueProfileKind};

const ACC: usize = 255;

fn run_vm(bytecode: Vec<u32>, const_pool: Vec<vm::js_value::JSValue>) -> VM {
    let mut vm = VM::new(bytecode, const_pool, Vec::new());
    vm.run(false);
    vm
}

#[test]
fn public_bytecode_builder_arithmetic_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 7);
    builder.emit_load_i(2, 5);
    builder.emit_add(1, 2);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(12.0));
}

#[test]
fn public_run_can_optimize_loaded_bytecode() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 2);
    builder.emit_load_i(2, 3);
    builder.emit_add(1, 2);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let mut vm = VM::new(bytecode, const_pool, Vec::new());

    assert_eq!(vm.bytecode.len(), 4);
    vm.run(true);

    assert_eq!(vm.bytecode.len(), 2);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn public_bytecode_function_call_smoke() {
    let mut builder = BytecodeBuilder::new();
    let function_entry_const = builder.add_constant(make_number(0.0));

    builder.emit_new_func(1, function_entry_const);
    builder.emit_load_i(2, 4);
    builder.emit_call(1, 1);
    builder.emit_ret();

    let function_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_1();
    builder.emit_add(1, ACC as u8);
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[function_entry_const as usize] = make_number(function_entry as f64);

    let vm = run_vm(bytecode, const_pool);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

fn patch_abc_offset(bytecode: &mut [u32], index: usize, opcode: u8, a: u8, b: u8, target: usize) {
    let offset = target as i16 - (index as i16 + 1);
    let offset = i8::try_from(offset).expect("jump offset must fit in i8");
    bytecode[index] =
        ((offset as u8 as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode as u32;
}

#[test]
fn public_recursive_fibonacci_smoke() {
    let expected = [0.0, 1.0, 1.0, 2.0, 3.0, 5.0, 8.0];

    for (n, expected_value) in expected.into_iter().enumerate() {
        let mut builder = BytecodeBuilder::new();
        let fib_entry_const = builder.add_constant(make_number(0.0));

        builder.emit_new_func(1, fib_entry_const);
        builder.emit_set_upval(1, 0);
        builder.emit_load_i(2, n as i16);
        builder.emit_call(1, 1);
        builder.emit_ret();

        let fib_entry = builder.len();
        builder.emit_load_arg(1, 0);
        builder.emit_load_i(2, 1);
        let recurse_jump = builder.len();
        builder.emit_jmp_lte_false(1, 2, 0);
        builder.emit_ret_reg(1);

        let recurse_label = builder.len();
        builder.emit_get_upval(4, 0);
        builder.emit_call1_sub_i(4, 1, 1);
        builder.emit_mov(3, ACC as u8);

        builder.emit_call1_sub_i(4, 1, 2);
        builder.emit_add(3, ACC as u8);
        builder.emit_ret();

        let (mut bytecode, mut const_pool) = builder.build();
        const_pool[fib_entry_const as usize] = make_number(fib_entry as f64);
        patch_abc_offset(
            &mut bytecode,
            recurse_jump,
            Opcode::JmpLteFalse.as_u8(),
            1,
            2,
            recurse_label,
        );

        let vm = run_vm(bytecode, const_pool);
        assert_eq!(to_f64(vm.frame.regs[ACC]), Some(expected_value));
    }
}

#[test]
fn public_global_round_trip_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 19);
    builder.emit_set_global(1, 7);
    builder.emit_get_global(2, 7);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(to_f64(vm.frame.regs[2]), Some(19.0));
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(19.0));
}

#[test]
fn public_bitwise_and_truthiness_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 10);
    builder.emit_load_i(2, 3);
    builder.emit_bit_and(1, 2);
    builder.emit_mov(3, ACC as u8);
    builder.emit_load_0();
    builder.emit_mov(4, ACC as u8);
    builder.emit_logical_or(4, 3);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(to_f64(vm.frame.regs[3]), Some(2.0));
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), None);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(2.0));
}

#[test]
fn public_switch_dispatch_smoke() {
    let mut builder = BytecodeBuilder::new();
    let table_index = builder.add_switch_table(0, &[(make_number(1.0), 3), (make_number(2.0), 6)]);

    builder.emit_load_i(1, 2);
    builder.emit_switch(1, table_index);
    builder.emit_load_i(2, 99);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_ret();
    builder.emit_load_i(2, 10);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_ret();
    builder.emit_load_i(2, 20);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(20.0));
}

#[test]
fn public_feedback_opcode_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 42);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_profile_type(0);
    builder.emit_check_type(3);
    builder.emit_loop_hint();
    builder.emit_profile_hot_loop();
    builder.emit_osr_entry();
    builder.emit_jit_hint();
    builder.emit_safety_check();
    builder.emit_osr_exit();
    builder.emit_profile_ret();
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(
        vm.feedback.type_slots[0].last,
        Some(ValueProfileKind::Number)
    );
    assert_eq!(vm.feedback.return_slot.last, Some(ValueProfileKind::Number));
    assert_eq!(vm.feedback.deopt_count, 0);
    assert_eq!(vm.feedback.osr_entries, 1);
    assert_eq!(vm.feedback.osr_exits, 1);
    assert!(!vm.feedback.osr_active);
    assert_eq!(
        vm.feedback.loop_hint_counts.values().copied().sum::<u32>(),
        1
    );
    assert_eq!(
        vm.feedback.hot_loop_counts.values().copied().sum::<u32>(),
        1
    );
    assert_eq!(vm.feedback.jit_hints.values().copied().sum::<u32>(), 1);
    assert_eq!(vm.feedback.safety_checks, 1);
    assert_eq!(vm.feedback.failed_safety_checks, 0);
}

#[test]
fn public_profile_call_and_ret_smoke() {
    let mut builder = BytecodeBuilder::new();
    let function_entry_const = builder.add_constant(make_number(0.0));

    builder.emit_new_func(1, function_entry_const);
    builder.emit_call(1, 0);
    builder.emit_profile_call(0);
    builder.emit_ret();

    let function_entry = builder.len();
    builder.emit_load_i(2, 5);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_profile_ret();
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[function_entry_const as usize] = make_number(function_entry as f64);

    let vm = run_vm(bytecode, const_pool);
    assert_eq!(
        vm.feedback.call_slots[0].last,
        Some(ValueProfileKind::Function)
    );
    assert_eq!(vm.feedback.call_slots[0].samples, 1);
    assert_eq!(vm.feedback.return_slot.last, Some(ValueProfileKind::Number));
}

#[test]
fn public_inline_cache_feedback_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_new_obj(1);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_ic_init(1);
    builder.emit_check_ic(1);
    builder.emit_mov(2, ACC as u8);
    builder.emit_load_i(5, 7);
    builder.emit_set_prop(5, 1, 5);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_check_ic(1);
    builder.emit_mov(3, ACC as u8);
    builder.emit_ic_miss();
    builder.emit_ic_update(1);
    builder.emit_check_ic(1);
    builder.emit_mov(4, ACC as u8);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(bool_from_value(vm.frame.regs[2]), Some(true));
    assert_eq!(bool_from_value(vm.frame.regs[3]), Some(false));
    assert_eq!(bool_from_value(vm.frame.regs[4]), Some(true));
    assert_eq!(vm.feedback.ic_misses, 1);
    assert_eq!(vm.frame.ic_vector[1].state, ICState::Poly);
}

#[test]
fn public_check_struct_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_new_obj(1);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_check_struct(99);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(vm.feedback.deopt_count, 1);
}

#[test]
fn public_leave_unwinds_scope_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_enter(8);
    builder.emit_create_env(1);
    builder.emit_load_i(2, 7);
    builder.emit_init_name(2, 11);
    builder.emit_leave();
    builder.emit_load_name(3, 11);
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert!(is_undefined(vm.frame.regs[ACC]));
    assert!(vm.scope_chain.is_empty());
}

#[test]
fn public_try_catch_finally_smoke() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_try(2);
    builder.emit_load_i(1, 42);
    builder.emit_throw(1);
    builder.emit_catch(2);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_finally();
    builder.emit_ret();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(42.0));
    assert!(is_undefined(vm.last_exception));
}
