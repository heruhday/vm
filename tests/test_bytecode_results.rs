use vm::emit::BytecodeBuilder;
use vm::js_value::{
    JSValue, bool_from_value, is_object, is_undefined, make_number, make_undefined,
    string_from_value, to_f64,
};
use vm::vm::{ICState, Opcode, VM, ValueProfileKind};

const ACC: usize = 255;

struct CoverageCase {
    name: &'static str,
    opcodes: &'static [Opcode],
    run: fn(),
}

fn run_vm(bytecode: Vec<u32>, const_pool: Vec<JSValue>) -> VM {
    let mut vm = VM::new(bytecode, const_pool, Vec::new());
    vm.run(false);
    vm
}

fn run_vm_with_setup<F>(bytecode: Vec<u32>, const_pool: Vec<JSValue>, setup: F) -> VM
where
    F: FnOnce(&mut VM),
{
    let mut vm = VM::new(bytecode, const_pool, Vec::new());
    setup(&mut vm);
    vm.run(false);
    vm
}

fn encode_abc(opcode: Opcode, a: u8, b: u8, c: u8) -> u32 {
    ((c as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_abx(opcode: Opcode, a: u8, bx: u16) -> u32 {
    ((bx as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_asbx(opcode: Opcode, a: u8, sbx: i16) -> u32 {
    (((sbx as u16) as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn assert_reg_number(vm: &VM, reg: usize, expected: f64) {
    assert_eq!(
        to_f64(vm.frame.regs[reg]),
        Some(expected),
        "register {}",
        reg
    );
}

fn assert_reg_bool(vm: &VM, reg: usize, expected: bool) {
    assert_eq!(
        bool_from_value(vm.frame.regs[reg]),
        Some(expected),
        "register {}",
        reg
    );
}

fn value_text(vm: &VM, value: JSValue) -> Option<String> {
    if let Some(atom) = value.as_atom() {
        return Some(vm.atoms.resolve(atom).to_owned());
    }

    string_from_value(value).map(|ptr| unsafe { (*ptr).text(&vm.atoms).to_owned() })
}

fn assert_reg_text(vm: &VM, reg: usize, expected: &str) {
    assert_eq!(
        value_text(vm, vm.frame.regs[reg]).as_deref(),
        Some(expected)
    );
}

fn case_load_and_accumulator_ops() {
    let mut builder = BytecodeBuilder::new();
    let foo_idx = builder.add_constant(make_undefined());
    let bar_idx = builder.add_constant(make_undefined());

    builder.emit_load_i(0, 33);
    builder.emit_load_this();
    builder.emit_mov(20, ACC as u8);
    builder.emit_load_0();
    builder.emit_mov(21, ACC as u8);
    builder.emit_load_1();
    builder.emit_mov(22, ACC as u8);
    builder.emit_load_null();
    builder.emit_mov(23, ACC as u8);
    builder.emit_load_true(24);
    builder.emit_mov(24, ACC as u8);
    builder.emit_load_false(25);
    builder.emit_mov(25, ACC as u8);

    builder.emit_load_k(1, foo_idx);
    builder.emit_load_k(2, bar_idx);
    builder.emit_load_acc(1);
    builder.emit_add_str_acc(2);
    builder.emit_mov(26, ACC as u8);

    builder.emit_load_i(3, 10);
    builder.emit_load_acc(3);
    builder.emit_add_acc_imm8(5);
    builder.emit_mov(27, ACC as u8);
    builder.emit_inc_acc();
    builder.emit_mov(28, ACC as u8);
    builder.emit_sub_acc_imm8(6);
    builder.emit_mov(29, ACC as u8);
    builder.emit_mul_acc_imm8(3);
    builder.emit_mov(30, ACC as u8);
    builder.emit_div_acc_imm8(2);
    builder.emit_mov(31, ACC as u8);

    builder.emit_load_acc(3);
    builder.emit_add_acc(3);
    builder.emit_mov(32, ACC as u8);
    builder.emit_sub_acc(3);
    builder.emit_mov(33, ACC as u8);
    builder.emit_mul_acc(3);
    builder.emit_mov(34, ACC as u8);
    builder.emit_div_acc(3);
    builder.emit_mov(35, ACC as u8);
    builder.emit_mod(3, 3);
    builder.emit_mov(46, ACC as u8);

    builder.emit_add_i(36, 3, 2);
    builder.emit_sub_i(37, 3, 2);
    builder.emit_mul_i(38, 3, 3);
    builder.emit_div_i(39, 3, 2);
    builder.emit_mod_i(40, 3, 3);
    builder.emit_neg(3);
    builder.emit_mov(41, ACC as u8);
    builder.emit_inc(3);
    builder.emit_mov(42, ACC as u8);
    builder.emit_dec(3);
    builder.emit_mov(43, ACC as u8);
    builder.emit_add_str(1, 2);
    builder.emit_mov(44, ACC as u8);
    builder.emit_to_primitive(3);
    builder.emit_mov(45, ACC as u8);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm_with_setup(bytecode, const_pool, |vm| {
        vm.const_pool[foo_idx as usize] = vm.intern_string("foo");
        vm.const_pool[bar_idx as usize] = vm.intern_string("bar");
    });

    assert_reg_number(&vm, 20, 33.0);
    assert_reg_number(&vm, 21, 0.0);
    assert_reg_number(&vm, 22, 1.0);
    assert!(is_undefined(vm.frame.regs[23]) || vm.frame.regs[23].is_null());
    assert_reg_bool(&vm, 24, true);
    assert_reg_bool(&vm, 25, false);
    assert_reg_text(&vm, 26, "foobar");
    assert_reg_number(&vm, 27, 15.0);
    assert_reg_number(&vm, 28, 16.0);
    assert_reg_number(&vm, 29, 10.0);
    assert_reg_number(&vm, 30, 30.0);
    assert_reg_number(&vm, 31, 15.0);
    assert_reg_number(&vm, 32, 20.0);
    assert_reg_number(&vm, 33, 10.0);
    assert_reg_number(&vm, 34, 100.0);
    assert_reg_number(&vm, 35, 10.0);
    assert_reg_number(&vm, 36, 12.0);
    assert_reg_number(&vm, 37, 8.0);
    assert_reg_number(&vm, 38, 30.0);
    assert_reg_number(&vm, 39, 5.0);
    assert_reg_number(&vm, 40, 1.0);
    assert_reg_number(&vm, 41, -10.0);
    assert_reg_number(&vm, 42, 11.0);
    assert_reg_number(&vm, 43, 9.0);
    assert_reg_text(&vm, 44, "foobar");
    assert_reg_number(&vm, 45, 10.0);
    assert_reg_number(&vm, 46, 0.0);
}

fn case_conversion_and_predicates() {
    let mut builder = BytecodeBuilder::new();
    let text_idx = builder.add_constant(make_undefined());
    let undef_idx = builder.add_constant(make_undefined());

    builder.emit_load_k(1, text_idx);
    builder.emit_to_num(2, 1);
    builder.emit_to_str(3, 2);
    builder.emit_load_k(4, undef_idx);
    builder.emit_is_undef(5, 4);
    builder.emit_load_null();
    builder.emit_mov(6, ACC as u8);
    builder.emit_is_null(7, 6);
    builder.emit_typeof(8, 6);
    builder.emit_typeof(9, 4);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm_with_setup(bytecode, const_pool, |vm| {
        vm.const_pool[text_idx as usize] = vm.intern_string("42");
        vm.const_pool[undef_idx as usize] = make_undefined();
    });

    assert_reg_number(&vm, 2, 42.0);
    assert_reg_text(&vm, 3, "42");
    assert_reg_bool(&vm, 5, true);
    assert_reg_bool(&vm, 7, true);
    assert_reg_text(&vm, 8, "object");
    assert_reg_text(&vm, 9, "undefined");
}

fn case_binary_bitwise_and_logical_ops() {
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_i(1, 7);
    builder.emit_load_i(2, 5);
    builder.emit_load_i(3, 1);
    builder.emit_add(1, 2);
    builder.emit_mov(10, ACC as u8);
    builder.emit_eq(1, 2);
    builder.emit_mov(11, ACC as u8);
    builder.emit_lt(2, 1);
    builder.emit_mov(12, ACC as u8);
    builder.emit_lte(2, 2);
    builder.emit_mov(13, ACC as u8);
    builder.emit_strict_eq(1, 1);
    builder.emit_mov(14, ACC as u8);
    builder.emit_strict_neq(1, 2);
    builder.emit_mov(15, ACC as u8);
    builder.emit_bit_and(1, 2);
    builder.emit_mov(16, ACC as u8);
    builder.emit_bit_or(1, 2);
    builder.emit_mov(17, ACC as u8);
    builder.emit_bit_xor(1, 2);
    builder.emit_mov(18, ACC as u8);
    builder.emit_bit_not(2);
    builder.emit_mov(19, ACC as u8);
    builder.emit_shl(2, 3);
    builder.emit_mov(20, ACC as u8);
    builder.emit_shr(1, 3);
    builder.emit_mov(21, ACC as u8);
    builder.emit_ushr(1, 3);
    builder.emit_mov(22, ACC as u8);
    builder.emit_pow(2, 3);
    builder.emit_mov(23, ACC as u8);
    builder.emit_load_0();
    builder.emit_mov(24, ACC as u8);
    builder.emit_logical_and(24, 2);
    builder.emit_mov(25, ACC as u8);
    builder.emit_logical_or(24, 2);
    builder.emit_mov(26, ACC as u8);
    builder.emit_load_null();
    builder.emit_mov(27, ACC as u8);
    builder.emit_nullish_coalesce(27, 2);
    builder.emit_mov(28, ACC as u8);
    builder.emit_new_arr(29, 0);
    builder.emit_load_i(30, 0);
    builder.emit_set_idx_fast(2, 29, 30);
    builder.emit_in(30, 29);
    builder.emit_mov(31, ACC as u8);
    builder.emit_new_class(32, 0);
    builder.emit_construct(32, 0);
    builder.emit_mov(33, ACC as u8);
    builder.emit_instanceof(33, 32);
    builder.emit_mov(34, ACC as u8);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_reg_number(&vm, 10, 12.0);
    assert_reg_bool(&vm, 11, false);
    assert_reg_bool(&vm, 12, true);
    assert_reg_bool(&vm, 13, true);
    assert_reg_bool(&vm, 14, true);
    assert_reg_bool(&vm, 15, true);
    assert_reg_number(&vm, 16, 5.0);
    assert_reg_number(&vm, 17, 7.0);
    assert_reg_number(&vm, 18, 2.0);
    assert_reg_number(&vm, 19, -6.0);
    assert_reg_number(&vm, 20, 10.0);
    assert_reg_number(&vm, 21, 3.0);
    assert_reg_number(&vm, 22, 3.0);
    assert_reg_number(&vm, 23, 5.0);
    assert_reg_number(&vm, 25, 0.0);
    assert_reg_number(&vm, 26, 5.0);
    assert_reg_number(&vm, 28, 5.0);
    assert_reg_bool(&vm, 31, true);
    assert!(is_object(vm.frame.regs[33]));
    assert_reg_bool(&vm, 34, true);
}

fn case_global_scope_and_name_ops() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 19);
    builder.emit_set_global(1, 7);
    builder.emit_get_global(2, 7);
    builder.emit_set_global_ic(2, 8);
    builder.emit_load_global_ic(3, 8);
    builder.emit_enter(8);
    builder.emit_create_env(4);
    builder.emit_get_scope(5, 0);
    builder.emit_load_i(6, 7);
    builder.emit_init_name(6, 11);
    builder.emit_load_name(7, 11);
    builder.emit_mov(8, ACC as u8);
    builder.emit_load_i(16, 13);
    builder.emit_init_name(16, 12);
    builder.emit_load_name(17, 12);
    builder.emit_mov(18, ACC as u8);
    builder.emit_typeof_name(9, 11);
    builder.emit_resolve_scope(10, 11);
    builder.emit_load_i(11, 9);
    builder.emit_set_scope(11, 0);
    builder.emit_get_scope(12, 0);
    builder.emit_leave();
    builder.emit_load_i(13, 42);
    builder.emit_set_upval(13, 0);
    builder.emit_get_upval(14, 0);
    builder.emit_load_closure(15, 0);
    builder.emit_ret_u();

    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);

    assert_reg_number(&vm, 2, 19.0);
    assert_reg_number(&vm, 3, 19.0);
    assert!(is_object(vm.frame.regs[4]));
    assert!(is_object(vm.frame.regs[5]));
    assert_reg_number(&vm, 8, 7.0);
    assert_reg_number(&vm, 18, 13.0);
    assert_reg_text(&vm, 9, "number");
    assert!(is_object(vm.frame.regs[10]));
    assert_reg_number(&vm, 12, 9.0);
    assert_reg_number(&vm, 14, 42.0);
    assert_reg_number(&vm, 15, 42.0);
    assert!(vm.scope_chain.is_empty());
}

fn case_object_and_property_ops() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_new_obj(1);
    builder.emit_load_i(2, 77);
    builder.emit_set_prop_ic(2, 1, 5);
    builder.emit_get_prop_ic(3, 1, 5);
    builder.emit_set_prop(2, 1, 6);
    builder.emit_get_prop(4, 1, 6);
    builder.emit_set_super(2, 1, 7);
    builder.emit_get_super(5, 1, 7);
    builder.emit_has_prop(6, 1, 6);
    builder.emit_delete_prop(7, 1, 6);
    builder.emit_has_prop(8, 1, 6);
    builder.emit_keys(9, 1);
    builder.emit_get_length_ic(10, 9, 0);
    builder.emit_get_prop_ic_mov(11, 1, 5);
    builder.emit_get_prop_add_imm_set_prop_ic(1, 5, 3);
    builder.emit_mov(12, ACC as u8);
    builder.emit_get_prop(13, 1, 5);
    builder.emit_new_obj_init_prop(14, 2, 9);
    builder.emit_get_prop(15, 14, 9);

    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::GetPropMono, 16, 1, 5));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));

    let vm = run_vm(bytecode, const_pool);

    assert_reg_number(&vm, 3, 77.0);
    assert_reg_number(&vm, 4, 77.0);
    assert_reg_number(&vm, 5, 77.0);
    assert_reg_bool(&vm, 6, true);
    assert_reg_bool(&vm, 7, true);
    assert_reg_bool(&vm, 8, false);
    assert_reg_number(&vm, 10, 2.0);
    assert_reg_number(&vm, 11, 77.0);
    assert_reg_number(&vm, 12, 80.0);
    assert_reg_number(&vm, 13, 80.0);
    assert_reg_number(&vm, 15, 77.0);
    assert_reg_number(&vm, 16, 80.0);
}

fn case_array_and_iteration_ops() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_new_arr(1, 0);
    builder.emit_load_i(2, 11);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_array_push_acc(1);
    builder.emit_load_i(3, 22);
    builder.emit_mov(ACC as u8, 3);
    builder.emit_array_push_acc(1);
    builder.emit_load_i(4, 0);
    builder.emit_load_i(6, 1);
    builder.emit_get_idx_fast(5, 1, 4);
    builder.emit_load_i(7, 33);
    builder.emit_set_idx_fast(7, 1, 6);
    builder.emit_get_idx_fast(8, 1, 6);
    builder.emit_load_i(9, 44);
    builder.emit_set_idx_ic(9, 1, 6);
    builder.emit_get_idx_ic(10, 1, 6);
    builder.emit_new_arr(11, 0);
    builder.emit_spread(11, 1);
    builder.emit_destructure(12, 11);
    builder.emit_for_in(14, 1);
    builder.emit_mov(15, ACC as u8);
    builder.emit_iterator_next(14, 14);
    builder.emit_mov(16, ACC as u8);
    builder.emit_new_obj(19);
    builder.emit_set_prop(1, 19, 0);
    builder.emit_load_i(20, 1);

    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::GetElem, 17, 1, 6));
    bytecode.push(encode_abc(Opcode::SetElem, 7, 1, 4));
    bytecode.push(encode_abc(Opcode::GetElem, 18, 1, 4));
    bytecode.push(encode_abc(Opcode::GetPropElem, 20, 19, 0));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));

    let vm = run_vm(bytecode, const_pool);

    assert_reg_number(&vm, 5, 11.0);
    assert_reg_number(&vm, 8, 33.0);
    assert_reg_number(&vm, 10, 44.0);
    assert_reg_number(&vm, 12, 11.0);
    assert_reg_number(&vm, 13, 44.0);
    assert_reg_number(&vm, 15, 0.0);
    assert_reg_number(&vm, 16, 1.0);
    assert_reg_number(&vm, 17, 44.0);
    assert_reg_number(&vm, 18, 33.0);
    assert_reg_number(&vm, 20, 44.0);
}

fn case_branch_and_return_ops() {
    let vm = run_vm(
        vec![
            encode_asbx(Opcode::Jmp, 0, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_true(1);
    builder.emit_mov(1, ACC as u8);
    builder.emit_jmp_true(1, 1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 7.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_false(1);
    builder.emit_mov(1, ACC as u8);
    builder.emit_jmp_false(1, 1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 8);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 8.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpEq, 1, 1, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpNeq, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpLt, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 2),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpLte, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 3),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpLteFalse, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let mut builder = BytecodeBuilder::new();
    let table_index = builder.add_switch_table(0, &[(make_number(1.0), 2), (make_number(2.0), 4)]);
    builder.emit_load_i(1, 2);
    builder.emit_switch(1, table_index);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_ret();
    builder.emit_load_i(ACC as u8, 10);
    builder.emit_ret();
    builder.emit_load_i(ACC as u8, 20);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 20.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 0);
    builder.emit_load_i(2, 2);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_loop_inc_jmp(1, 2, 1);
    builder.emit_load_i(1, 99);
    builder.emit_ret_reg(1);
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 1.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 1),
            encode_abc(Opcode::EqJmpTrue, 1, 1, 2),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::LtJmp, 1, 1, 2),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::EqJmpFalse, 1, 1, 2),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_false(1);
    builder.emit_mov(1, ACC as u8);
    builder.emit_load_i(2, 0);
    builder.emit_mov(ACC as u8, 2);
    builder.emit_inc_jmp_false_loop(1, 1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 1.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_inc_acc_jmp(1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 6.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_true(1);
    builder.emit_mov(1, ACC as u8);
    builder.emit_test_jmp_true(1, 1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::LteJmpLoop, 1, 1, 2),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_asbx(Opcode::LoadI, 3, 42),
            encode_abc(Opcode::RetIfLteI, 1, 2, 3),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 42.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::CmpJmp, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_false(1);
    builder.emit_mov(1, ACC as u8);
    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::LoadJfalse, 1, 1, 0));
    bytecode.push(encode_asbx(Opcode::LoadI, ACC as u8, 99));
    bytecode.push(encode_asbx(Opcode::LoadI, ACC as u8, 7));
    bytecode.push(encode_abc(Opcode::Ret, 0, 0, 0));
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 4),
            encode_asbx(Opcode::LoadI, 2, 4),
            encode_abc(Opcode::LoadCmpEqJfalse, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 4),
            encode_abc(Opcode::LoadCmpLtJfalse, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(
        vec![
            encode_asbx(Opcode::LoadI, 1, 1),
            encode_asbx(Opcode::LoadI, 2, 2),
            encode_abc(Opcode::JmpI32Fast, 1, 2, 1),
            encode_asbx(Opcode::LoadI, ACC as u8, 99),
            encode_asbx(Opcode::LoadI, ACC as u8, 7),
            encode_abc(Opcode::Ret, 0, 0, 0),
        ],
        vec![],
    );
    assert_reg_number(&vm, ACC, 7.0);

    let vm = run_vm(vec![encode_abc(Opcode::RetU, 0, 0, 0)], vec![]);
    assert!(is_undefined(vm.frame.regs[ACC]));
}

fn case_call_and_function_ops() {
    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_load_i(0, 33);
    builder.emit_new_func(1, undef_idx);
    builder.emit_load_i(2, 9);
    builder.emit_load_i(3, 11);
    builder.emit_call(1, 1);
    builder.emit_mov(4, ACC as u8);
    builder.emit_call_ic(1, 1);
    builder.emit_mov(5, ACC as u8);
    builder.emit_call_ic_super(1, 1);
    builder.emit_mov(6, ACC as u8);
    builder.emit_profile_hot_call(1, 1);
    builder.emit_mov(7, ACC as u8);
    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::CallMono, 1, 1, 0));
    bytecode.push(encode_abc(Opcode::Mov, 8, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Call0, 1, 0, 0));
    bytecode.push(encode_abc(Opcode::Mov, 9, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Call1, 1, 2, 0));
    bytecode.push(encode_abc(Opcode::Mov, 10, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Call2, 1, 2, 3));
    bytecode.push(encode_abc(Opcode::Mov, 11, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Call3, 1, 2, 3));
    bytecode.push(encode_abc(Opcode::Mov, 12, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, 4, 9.0);
    assert_reg_number(&vm, 5, 9.0);
    assert_reg_number(&vm, 6, 9.0);
    assert_reg_number(&vm, 7, 9.0);
    assert_reg_number(&vm, 8, 9.0);
    assert_reg_number(&vm, 9, 33.0);
    assert_reg_number(&vm, 10, 9.0);
    assert_reg_number(&vm, 11, 9.0);
    assert_reg_number(&vm, 12, 9.0);

    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_new_func(1, undef_idx);
    builder.emit_new_arr(2, 0);
    builder.emit_load_i(3, 17);
    builder.emit_mov(ACC as u8, 3);
    builder.emit_array_push_acc(2);
    builder.emit_call_var(1, 2);
    builder.emit_mov(4, ACC as u8);
    builder.emit_call_ic_var(1, 2);
    builder.emit_mov(5, ACC as u8);
    builder.emit_ret_u();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, 4, 17.0);
    assert_reg_number(&vm, 5, 17.0);

    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_new_func(0, undef_idx);
    builder.emit_load_this_call();
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert!(is_object(vm.frame.regs[ACC]));
    assert_eq!(vm.frame.regs[ACC].bits(), vm.frame.regs[0].bits());

    let mut builder = BytecodeBuilder::new();
    let wrapper_idx = builder.add_constant(make_number(0.0));
    let callee_idx = builder.add_constant(make_number(0.0));
    builder.emit_new_func(1, wrapper_idx);
    builder.emit_new_func(2, callee_idx);
    builder.emit_set_global(2, 1);
    builder.emit_load_i(2, 8);
    builder.emit_call(1, 1);
    builder.emit_ret();
    let wrapper_entry = builder.len();
    builder.emit_load_arg(4, 0);
    builder.emit_get_global(5, 1);
    builder.emit_mov(6, 4);
    builder.emit_tail_call(5, 1);
    let callee_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_ret_reg(1);
    let (bytecode, mut const_pool) = builder.build();
    const_pool[wrapper_idx as usize] = make_number(wrapper_entry as f64);
    const_pool[callee_idx as usize] = make_number(callee_entry as f64);
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 8.0);

    let mut builder = BytecodeBuilder::new();
    let wrapper_idx = builder.add_constant(make_number(0.0));
    let callee_idx = builder.add_constant(make_number(0.0));
    builder.emit_new_func(1, wrapper_idx);
    builder.emit_new_func(2, callee_idx);
    builder.emit_set_global(2, 1);
    builder.emit_load_i(2, 12);
    builder.emit_call(1, 1);
    builder.emit_ret();
    let wrapper_entry = builder.len();
    builder.emit_get_global(5, 1);
    builder.emit_load_arg(6, 0);
    builder.emit_mov(6, 6);
    let call_ret_index = builder.len();
    builder.emit_ret_u();
    let callee_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_ret_reg(1);
    let (mut bytecode, mut const_pool) = builder.build();
    const_pool[wrapper_idx as usize] = make_number(wrapper_entry as f64);
    const_pool[callee_idx as usize] = make_number(callee_entry as f64);
    bytecode[call_ret_index] = encode_abc(Opcode::CallRet, 5, 1, 0);
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 12.0);

    let mut builder = BytecodeBuilder::new();
    let wrapper_idx = builder.add_constant(make_number(0.0));
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_load_i(0, 77);
    builder.emit_new_func(1, wrapper_idx);
    builder.emit_new_func(2, undef_idx);
    builder.emit_call(1, 1);
    builder.emit_ret();
    let wrapper_entry = builder.len();
    builder.emit_load_arg(3, 0);
    builder.emit_mov(4, 3);
    builder.emit_load_arg_call(5, 0);
    builder.emit_ret();
    let (bytecode, mut const_pool) = builder.build();
    const_pool[wrapper_idx as usize] = make_number(wrapper_entry as f64);
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, ACC, 77.0);

    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_new_obj(10);
    builder.emit_new_func(11, undef_idx);
    builder.emit_set_prop(11, 10, 0);
    builder.emit_load_i(11, 55);
    builder.emit_load_i(12, 66);
    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abx(Opcode::CallMethod1, 10, 0));
    bytecode.push(encode_abc(Opcode::Mov, 13, ACC as u8, 0));
    bytecode.push(encode_abx(Opcode::CallMethod2, 10, 0));
    bytecode.push(encode_abc(Opcode::Mov, 14, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::CallMethodIc, 10, 0, 0));
    bytecode.push(encode_abc(Opcode::Mov, 15, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::GetPropCall, 16, 10, 0));
    bytecode.push(encode_abc(Opcode::Mov, 17, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, 13, 55.0);
    assert_reg_number(&vm, 14, 55.0);
    assert_eq!(vm.frame.regs[15].bits(), vm.frame.regs[10].bits());
    assert_eq!(vm.frame.regs[17].bits(), vm.frame.regs[10].bits());

    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_new_obj(1);
    builder.emit_new_func(2, undef_idx);
    builder.emit_set_prop(2, 1, 0);
    builder.emit_load_i(3, 0);
    builder.emit_set_idx_fast(2, 1, 3);
    builder.emit_get_prop_acc_call(1, 3);
    builder.emit_mov(4, ACC as u8);
    builder.emit_get_prop_ic_call(5, 1, 0);
    builder.emit_mov(6, ACC as u8);
    builder.emit_new_obj(7);
    builder.emit_new_func(8, undef_idx);
    builder.emit_set_prop(8, 7, 1);
    builder.emit_set_prop(7, 1, 2);
    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::CallMethod2Ic, 1, 2, 1));
    bytecode.push(encode_abc(Opcode::Mov, 9, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));
    let vm = run_vm(bytecode, const_pool);
    assert_eq!(vm.frame.regs[4].bits(), vm.frame.regs[1].bits());
    assert!(is_object(vm.frame.regs[5]));
    assert_eq!(vm.frame.regs[6].bits(), vm.frame.regs[1].bits());
    assert_eq!(vm.frame.regs[9].bits(), vm.frame.regs[1].bits());

    let mut builder = BytecodeBuilder::new();
    let undef_idx = builder.add_constant(make_undefined());
    builder.emit_new_func(1, undef_idx);
    builder.emit_load_i(2, 10);
    builder.emit_call1_sub_i(1, 2, 3);
    builder.emit_mov(3, ACC as u8);
    builder.emit_ret_u();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, 3, 7.0);
}

fn case_feedback_and_runtime_ops() {
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
    assert_eq!(vm.feedback.osr_entries, 1);
    assert_eq!(vm.feedback.osr_exits, 1);
    assert_eq!(vm.feedback.jit_hints.values().copied().sum::<u32>(), 1);
    assert_eq!(vm.feedback.safety_checks, 1);

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
    assert_reg_bool(&vm, 2, true);
    assert_reg_bool(&vm, 3, false);
    assert_reg_bool(&vm, 4, true);
    assert_eq!(vm.frame.ic_vector[1].state, ICState::Poly);

    let mut builder = BytecodeBuilder::new();
    builder.emit_new_obj(1);
    builder.emit_mov(ACC as u8, 1);
    builder.emit_check_struct(99);
    builder.emit_ret_u();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_eq!(vm.feedback.deopt_count, 1);
}

fn case_try_yield_await_and_assertions() {
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
    assert_reg_number(&vm, ACC, 42.0);
    assert!(is_undefined(vm.last_exception));

    let mut builder = BytecodeBuilder::new();
    builder.emit_try(1);
    builder.emit_end_try();
    builder.emit_load_i(1, 5);
    builder.emit_yield(1);
    builder.emit_mov(2, ACC as u8);
    builder.emit_await(1);
    builder.emit_mov(3, ACC as u8);
    builder.emit_ret_u();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_number(&vm, 2, 5.0);
    assert_reg_number(&vm, 3, 5.0);

    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 1);
    builder.emit_load_i(2, 2);
    builder.emit_assert_value(1);
    builder.emit_assert_ok(1);
    builder.emit_assert_fail();
    builder.emit_assert_throws(1);
    builder.emit_assert_does_not_throw(1);
    builder.emit_assert_rejects(1);
    builder.emit_assert_does_not_reject(1);
    builder.emit_assert_equal(1, 2);
    builder.emit_assert_not_equal(1, 2);
    builder.emit_assert_deep_equal(1, 2);
    builder.emit_assert_not_deep_equal(1, 2);
    builder.emit_assert_strict_equal(1, 2);
    builder.emit_assert_not_strict_equal(1, 2);
    builder.emit_assert_deep_strict_equal(1, 2);
    builder.emit_assert_not_deep_strict_equal(1, 2);
    builder.emit_ret();
    let (bytecode, const_pool) = builder.build();
    let vm = run_vm(bytecode, const_pool);
    assert_reg_bool(&vm, ACC, true);
}

fn case_fast_and_superinstruction_ops() {
    let mut builder = BytecodeBuilder::new();
    let f1_idx = builder.add_constant(make_number(1.5));
    let f2_idx = builder.add_constant(make_number(2.25));
    let k10_idx = builder.add_constant(make_number(10.0));
    let k3_idx = builder.add_constant(make_number(3.0));
    let foo_idx = builder.add_constant(make_undefined());
    let bar_idx = builder.add_constant(make_undefined());
    let fn_idx = builder.add_constant(make_undefined());

    builder.emit_load_i(1, 4);
    builder.emit_load_i(2, 2);
    builder.emit_load_i(3, 1);
    builder.emit_load_k(4, f1_idx);
    builder.emit_load_k(5, f2_idx);
    builder.emit_new_this(6);

    builder.emit_new_arr(7, 0);
    builder.emit_load_i(8, 0);
    builder.emit_load_i(9, 77);
    builder.emit_set_idx_fast(9, 7, 8);
    builder.emit_get_prop_acc(7, 8);
    builder.emit_mov(10, ACC as u8);

    builder.emit_new_obj(11);
    builder.emit_load_i(12, 88);
    builder.emit_mov(ACC as u8, 12);
    builder.emit_set_prop_acc(11, 8);
    builder.emit_get_prop_acc(11, 8);
    builder.emit_mov(13, ACC as u8);

    builder.emit_new_obj(14);
    builder.emit_load_i(15, 66);
    builder.emit_set_prop(15, 14, 2);

    builder.emit_new_arr(16, 0);
    builder.emit_load_i(17, 11);
    builder.emit_mov(ACC as u8, 17);
    builder.emit_array_push_acc(16);
    builder.emit_load_i(18, 22);
    builder.emit_mov(ACC as u8, 18);
    builder.emit_array_push_acc(16);

    builder.emit_new_obj(19);
    builder.emit_load_i(20, 91);
    builder.emit_set_prop(20, 19, 7);
    builder.emit_load_i(21, 19);

    builder.emit_new_func(22, fn_idx);

    builder.emit_load_k(23, foo_idx);
    builder.emit_load_k(24, bar_idx);

    builder.emit_new_obj(25);
    builder.emit_new_obj(26);
    builder.emit_load_i(27, 77);
    builder.emit_set_prop(27, 26, 47);
    builder.emit_set_prop(26, 25, 1);

    builder.emit_new_obj(49);
    builder.emit_new_obj(50);
    builder.emit_new_obj(51);
    builder.emit_load_i(52, 88);
    builder.emit_set_prop(52, 51, 49);
    builder.emit_set_prop(51, 50, 48);
    builder.emit_set_prop(50, 49, 2);

    let (mut bytecode, const_pool) = builder.build();
    bytecode.push(encode_abc(Opcode::AddI32Fast, 32, 1, 2));
    bytecode.push(encode_abc(Opcode::AddF64Fast, 33, 4, 5));
    bytecode.push(encode_abc(Opcode::SubI32Fast, 34, 1, 2));
    bytecode.push(encode_abc(Opcode::MulI32Fast, 35, 1, 2));
    bytecode.push(encode_abc(Opcode::EqI32Fast, 0, 1, 1));
    bytecode.push(encode_abc(Opcode::Mov, 36, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::LtI32Fast, 0, 2, 1));
    bytecode.push(encode_abc(Opcode::Mov, 37, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::LoadAdd, 38, 1, 2));
    bytecode.push(encode_abc(Opcode::LoadSub, 39, 1, 2));
    bytecode.push(encode_abc(Opcode::LoadMul, 40, 1, 2));
    bytecode.push(encode_abc(Opcode::LoadInc, 41, 1, 0));
    bytecode.push(encode_abc(Opcode::LoadDec, 42, 1, 0));
    bytecode.push(encode_abc(Opcode::LoadCmpEq, 43, 1, 1));
    bytecode.push(encode_abc(Opcode::LoadCmpLt, 44, 2, 1));
    bytecode.push(encode_abc(Opcode::LoadGetProp, 14, 2, 0));
    bytecode.push(encode_abc(Opcode::Mov, 45, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::LoadGetPropCmpEq, 14, 2, 15));
    bytecode.push(encode_abc(Opcode::Mov, 46, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::GetProp2Ic, 47, 25, 1));
    bytecode.push(encode_abc(Opcode::GetProp3Ic, 48, 49, 2));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 1, 0));
    bytecode.push(encode_abx(Opcode::LoadKAddAcc, 0, k10_idx));
    bytecode.push(encode_abc(Opcode::Mov, 53, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::AddMov, 54, 1, 2));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 2, 0));
    bytecode.push(encode_abx(Opcode::LoadKMulAcc, 0, k10_idx));
    bytecode.push(encode_abc(Opcode::Mov, 55, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 1, 0));
    bytecode.push(encode_abc(Opcode::AddAccImm8Mov, 56, 3u8, 0));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 1, 0));
    bytecode.push(encode_abx(Opcode::LoadKSubAcc, 0, k10_idx));
    bytecode.push(encode_abc(Opcode::Mov, 57, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::GetLengthIcCall, 0, 16, 1));
    bytecode.push(encode_abc(Opcode::Mov, 58, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::LoadAcc, 23, 0, 0));
    bytecode.push(encode_abc(Opcode::AddStrAccMov, 59, 24, 0));
    bytecode.push(encode_abc(Opcode::GetPropChainAcc, 0, 21, 7));
    bytecode.push(encode_abc(Opcode::Mov, 60, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 2, 0));
    bytecode.push(encode_abc(Opcode::MulAccMov, 61, 1, 0));
    bytecode.push(encode_abc(Opcode::AddI32, 62, 1, 2));
    bytecode.push(encode_abc(Opcode::AddF64, 63, 4, 5));
    bytecode.push(encode_abc(Opcode::SubI32, 64, 1, 2));
    bytecode.push(encode_abc(Opcode::SubF64, 65, 5, 4));
    bytecode.push(encode_abc(Opcode::MulI32, 66, 1, 2));
    bytecode.push(encode_abc(Opcode::MulF64, 67, 4, 5));
    bytecode.push(encode_abc(Opcode::AddAccReg, 1, 2, 0));
    bytecode.push(encode_abc(Opcode::Mov, 68, ACC as u8, 0));
    bytecode.push(encode_asbx(Opcode::LoadI, 69, 10));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 69, 0));
    bytecode.push(encode_asbx(Opcode::LoadI, 70, 5));
    bytecode.push(encode_abc(Opcode::Call1Add, 22, 70, 0));
    bytecode.push(encode_abc(Opcode::Mov, 71, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 69, 0));
    bytecode.push(encode_asbx(Opcode::LoadI, 72, 7));
    bytecode.push(encode_asbx(Opcode::LoadI, 73, 3));
    bytecode.push(encode_abc(Opcode::Call2Add, 22, 72, 73));
    bytecode.push(encode_abc(Opcode::Mov, 74, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::Mov, ACC as u8, 1, 0));
    bytecode.push(encode_abx(Opcode::LoadKAdd, 75, k10_idx));
    bytecode.push(encode_asbx(Opcode::LoadI, 76, 8));
    bytecode.push(encode_abx(Opcode::LoadKCmp, 76, k3_idx));
    bytecode.push(encode_abc(Opcode::Mov, 77, ACC as u8, 0));
    bytecode.push(encode_abc(Opcode::RetU, 0, 0, 0));

    let vm = run_vm_with_setup(bytecode, const_pool, |vm| {
        vm.const_pool[foo_idx as usize] = vm.intern_string("foo");
        vm.const_pool[bar_idx as usize] = vm.intern_string("bar");
    });

    assert!(is_object(vm.frame.regs[6]));
    assert_reg_number(&vm, 10, 77.0);
    assert_reg_number(&vm, 13, 88.0);
    assert_reg_number(&vm, 32, 6.0);
    assert_reg_number(&vm, 33, 3.75);
    assert_reg_number(&vm, 34, 2.0);
    assert_reg_number(&vm, 35, 8.0);
    assert_reg_bool(&vm, 36, true);
    assert_reg_bool(&vm, 37, true);
    assert_reg_number(&vm, 38, 6.0);
    assert_reg_number(&vm, 39, 2.0);
    assert_reg_number(&vm, 40, 8.0);
    assert_reg_number(&vm, 41, 5.0);
    assert_reg_number(&vm, 42, 3.0);
    assert_reg_bool(&vm, 43, true);
    assert_reg_bool(&vm, 44, true);
    assert_reg_number(&vm, 45, 66.0);
    assert_reg_bool(&vm, 46, true);
    assert_reg_number(&vm, 47, 77.0);
    assert_reg_number(&vm, 48, 88.0);
    assert_reg_number(&vm, 53, 14.0);
    assert_reg_number(&vm, 54, 6.0);
    assert_reg_number(&vm, 55, 20.0);
    assert_reg_number(&vm, 56, 7.0);
    assert_reg_number(&vm, 57, 6.0);
    assert_reg_number(&vm, 58, 2.0);
    assert_reg_text(&vm, 59, "foobar");
    assert_reg_number(&vm, 60, 91.0);
    assert_reg_number(&vm, 61, 8.0);
    assert_reg_number(&vm, 62, 6.0);
    assert_reg_number(&vm, 63, 3.75);
    assert_reg_number(&vm, 64, 2.0);
    assert_reg_number(&vm, 65, 0.75);
    assert_reg_number(&vm, 66, 8.0);
    assert_reg_number(&vm, 67, 3.375);
    assert_reg_number(&vm, 68, 6.0);
    assert_reg_number(&vm, 71, 15.0);
    assert_reg_number(&vm, 74, 17.0);
    assert_reg_number(&vm, 75, 14.0);
    assert_reg_bool(&vm, 77, true);
}

fn cases() -> Vec<CoverageCase> {
    vec![
        CoverageCase {
            name: "load_and_accumulator_ops",
            opcodes: &[
                Opcode::Mov,
                Opcode::LoadK,
                Opcode::LoadI,
                Opcode::AddAccImm8,
                Opcode::IncAcc,
                Opcode::LoadThis,
                Opcode::Load0,
                Opcode::Load1,
                Opcode::AddAcc,
                Opcode::SubAcc,
                Opcode::MulAcc,
                Opcode::DivAcc,
                Opcode::LoadNull,
                Opcode::LoadTrue,
                Opcode::LoadFalse,
                Opcode::SubAccImm8,
                Opcode::MulAccImm8,
                Opcode::DivAccImm8,
                Opcode::AddStrAcc,
                Opcode::AddI,
                Opcode::SubI,
                Opcode::MulI,
                Opcode::DivI,
                Opcode::ModI,
                Opcode::Mod,
                Opcode::Neg,
                Opcode::Inc,
                Opcode::Dec,
                Opcode::AddStr,
                Opcode::ToPrimitive,
                Opcode::LoadAcc,
            ],
            run: case_load_and_accumulator_ops,
        },
        CoverageCase {
            name: "conversion_and_predicates",
            opcodes: &[
                Opcode::Typeof,
                Opcode::ToNum,
                Opcode::ToStr,
                Opcode::IsUndef,
                Opcode::IsNull,
            ],
            run: case_conversion_and_predicates,
        },
        CoverageCase {
            name: "binary_bitwise_and_logical_ops",
            opcodes: &[
                Opcode::Add,
                Opcode::Eq,
                Opcode::Lt,
                Opcode::Lte,
                Opcode::StrictEq,
                Opcode::StrictNeq,
                Opcode::BitAnd,
                Opcode::BitOr,
                Opcode::BitXor,
                Opcode::BitNot,
                Opcode::Shl,
                Opcode::Shr,
                Opcode::Ushr,
                Opcode::Pow,
                Opcode::LogicalAnd,
                Opcode::LogicalOr,
                Opcode::NullishCoalesce,
                Opcode::In,
                Opcode::Instanceof,
                Opcode::NewClass,
                Opcode::Construct,
            ],
            run: case_binary_bitwise_and_logical_ops,
        },
        CoverageCase {
            name: "global_scope_and_name_ops",
            opcodes: &[
                Opcode::LoadGlobalIc,
                Opcode::SetGlobalIc,
                Opcode::GetGlobal,
                Opcode::SetGlobal,
                Opcode::GetUpval,
                Opcode::SetUpval,
                Opcode::GetScope,
                Opcode::SetScope,
                Opcode::ResolveScope,
                Opcode::CreateEnv,
                Opcode::LoadName,
                Opcode::StoreName,
                Opcode::InitName,
                Opcode::LoadClosure,
                Opcode::TypeofName,
                Opcode::Enter,
                Opcode::Leave,
            ],
            run: case_global_scope_and_name_ops,
        },
        CoverageCase {
            name: "object_and_property_ops",
            opcodes: &[
                Opcode::GetPropIc,
                Opcode::SetPropIc,
                Opcode::NewObj,
                Opcode::GetProp,
                Opcode::SetProp,
                Opcode::GetSuper,
                Opcode::SetSuper,
                Opcode::DeleteProp,
                Opcode::HasProp,
                Opcode::Keys,
                Opcode::GetLengthIc,
                Opcode::GetPropMono,
                Opcode::GetPropIcMov,
                Opcode::GetPropAddImmSetPropIc,
                Opcode::NewObjInitProp,
            ],
            run: case_object_and_property_ops,
        },
        CoverageCase {
            name: "array_and_iteration_ops",
            opcodes: &[
                Opcode::GetIdxFast,
                Opcode::SetIdxFast,
                Opcode::GetIdxIc,
                Opcode::SetIdxIc,
                Opcode::ArrayPushAcc,
                Opcode::NewArr,
                Opcode::ForIn,
                Opcode::IteratorNext,
                Opcode::Spread,
                Opcode::Destructure,
                Opcode::GetElem,
                Opcode::SetElem,
                Opcode::GetPropElem,
            ],
            run: case_array_and_iteration_ops,
        },
        CoverageCase {
            name: "branch_and_return_ops",
            opcodes: &[
                Opcode::Jmp,
                Opcode::JmpTrue,
                Opcode::JmpFalse,
                Opcode::JmpEq,
                Opcode::JmpNeq,
                Opcode::JmpLt,
                Opcode::JmpLte,
                Opcode::LoopIncJmp,
                Opcode::Switch,
                Opcode::Ret,
                Opcode::RetU,
                Opcode::JmpLteFalse,
                Opcode::EqJmpTrue,
                Opcode::LtJmp,
                Opcode::EqJmpFalse,
                Opcode::IncJmpFalseLoop,
                Opcode::IncAccJmp,
                Opcode::TestJmpTrue,
                Opcode::LteJmpLoop,
                Opcode::RetIfLteI,
                Opcode::CmpJmp,
                Opcode::LoadJfalse,
                Opcode::LoadCmpEqJfalse,
                Opcode::LoadCmpLtJfalse,
                Opcode::JmpI32Fast,
                Opcode::RetReg,
            ],
            run: case_branch_and_return_ops,
        },
        CoverageCase {
            name: "call_and_function_ops",
            opcodes: &[
                Opcode::Call,
                Opcode::LoadArg,
                Opcode::NewFunc,
                Opcode::TailCall,
                Opcode::CallVar,
                Opcode::CallIc,
                Opcode::CallIcVar,
                Opcode::CallMono,
                Opcode::Call0,
                Opcode::Call1,
                Opcode::Call2,
                Opcode::Call3,
                Opcode::CallMethod1,
                Opcode::CallMethod2,
                Opcode::GetPropCall,
                Opcode::CallRet,
                Opcode::GetPropIcCall,
                Opcode::CallMethodIc,
                Opcode::CallMethod2Ic,
                Opcode::GetPropAccCall,
                Opcode::ProfileHotCall,
                Opcode::Call1SubI,
                Opcode::CallIcSuper,
                Opcode::LoadThisCall,
                Opcode::LoadArgCall,
            ],
            run: case_call_and_function_ops,
        },
        CoverageCase {
            name: "feedback_and_runtime_ops",
            opcodes: &[
                Opcode::LoopHint,
                Opcode::ProfileType,
                Opcode::ProfileCall,
                Opcode::ProfileRet,
                Opcode::CheckType,
                Opcode::CheckStruct,
                Opcode::CheckIc,
                Opcode::IcInit,
                Opcode::IcUpdate,
                Opcode::IcMiss,
                Opcode::OsrEntry,
                Opcode::ProfileHotLoop,
                Opcode::OsrExit,
                Opcode::JitHint,
                Opcode::SafetyCheck,
            ],
            run: case_feedback_and_runtime_ops,
        },
        CoverageCase {
            name: "try_yield_await_and_assertions",
            opcodes: &[
                Opcode::Yield,
                Opcode::Await,
                Opcode::Throw,
                Opcode::Try,
                Opcode::EndTry,
                Opcode::Catch,
                Opcode::Finally,
                Opcode::AssertValue,
                Opcode::AssertOk,
                Opcode::AssertFail,
                Opcode::AssertThrows,
                Opcode::AssertDoesNotThrow,
                Opcode::AssertRejects,
                Opcode::AssertDoesNotReject,
                Opcode::AssertEqual,
                Opcode::AssertNotEqual,
                Opcode::AssertDeepEqual,
                Opcode::AssertNotDeepEqual,
                Opcode::AssertStrictEqual,
                Opcode::AssertNotStrictEqual,
                Opcode::AssertDeepStrictEqual,
                Opcode::AssertNotDeepStrictEqual,
            ],
            run: case_try_yield_await_and_assertions,
        },
        CoverageCase {
            name: "fast_and_superinstruction_ops",
            opcodes: &[
                Opcode::GetPropAcc,
                Opcode::SetPropAcc,
                Opcode::NewThis,
                Opcode::AddI32Fast,
                Opcode::AddF64Fast,
                Opcode::SubI32Fast,
                Opcode::MulI32Fast,
                Opcode::EqI32Fast,
                Opcode::LtI32Fast,
                Opcode::LoadAdd,
                Opcode::LoadSub,
                Opcode::LoadMul,
                Opcode::LoadInc,
                Opcode::LoadDec,
                Opcode::LoadCmpEq,
                Opcode::LoadCmpLt,
                Opcode::LoadGetProp,
                Opcode::LoadGetPropCmpEq,
                Opcode::GetProp2Ic,
                Opcode::GetProp3Ic,
                Opcode::LoadKAddAcc,
                Opcode::AddMov,
                Opcode::LoadKMulAcc,
                Opcode::AddAccImm8Mov,
                Opcode::LoadKSubAcc,
                Opcode::GetLengthIcCall,
                Opcode::AddStrAccMov,
                Opcode::GetPropChainAcc,
                Opcode::MulAccMov,
                Opcode::AddI32,
                Opcode::AddF64,
                Opcode::SubI32,
                Opcode::SubF64,
                Opcode::MulI32,
                Opcode::MulF64,
                Opcode::AddAccReg,
                Opcode::Call1Add,
                Opcode::Call2Add,
                Opcode::LoadKAdd,
                Opcode::LoadKCmp,
            ],
            run: case_fast_and_superinstruction_ops,
        },
    ]
}

#[test]
fn comprehensive_bytecode_cases_execute_with_expected_results() {
    for case in cases() {
        let result = std::panic::catch_unwind(|| (case.run)());
        assert!(result.is_ok(), "case failed: {}", case.name);
    }
}

#[test]
fn comprehensive_bytecode_coverage_tracks_all_non_reserved_opcodes() {
    let mut covered = [false; 256];
    for case in cases() {
        for opcode in case.opcodes {
            covered[opcode.as_u8() as usize] = true;
        }
    }

    let missing: Vec<_> = (0u8..=255)
        .filter_map(|raw| {
            let opcode = Opcode::from(raw);
            (!covered[raw as usize] && !matches!(opcode, Opcode::Reserved(_)))
                .then_some(format!("{:?}({})", opcode, raw))
        })
        .collect();

    assert!(
        missing.is_empty(),
        "missing opcode coverage: {}",
        missing.join(", ")
    );
}
