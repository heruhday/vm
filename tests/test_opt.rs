use vm::emit::BytecodeBuilder;
use vm::js_value::{bool_from_value, make_null, make_undefined, to_f64};
use vm::vm::optimization::{
    coalesce_registers, copy_propagation, eliminate_dead_code, fold_temporary_checks,
    optimize_peephole, relocate_jumps, simplify_branches,
};
use vm::vm::{Opcode, VM};

const ACC: usize = 255;

fn run_vm(bytecode: Vec<u32>) -> VM {
    run_vm_with_constants(bytecode, Vec::new())
}

fn run_vm_with_constants(bytecode: Vec<u32>, const_pool: Vec<vm::js_value::JSValue>) -> VM {
    let mut vm = VM::new(bytecode, const_pool, Vec::new());
    vm.run(false);
    vm
}

fn decode_opcode(raw: u32) -> Opcode {
    Opcode::from((raw & 0xFF) as u8)
}

fn decode_a(raw: u32) -> u8 {
    ((raw >> 8) & 0xFF) as u8
}

fn decode_b(raw: u32) -> u8 {
    ((raw >> 16) & 0xFF) as u8
}

fn decode_sbx(raw: u32) -> i16 {
    ((raw >> 16) & 0xFFFF) as u16 as i16
}

#[test]
fn optimized_constant_fold_collapses_dead_loads() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 2);
    builder.emit_load_i(2, 3);
    builder.emit_add(1, 2);
    builder.emit_ret();

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[0]), ACC as u8);
    assert_eq!(decode_sbx(bytecode[0]), 5);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::Ret);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn optimized_constant_fold_can_sink_into_destination_move() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 2);
    builder.emit_load_i(2, 3);
    builder.emit_add(1, 2);
    builder.emit_mov(4, ACC as u8);
    builder.emit_ret_reg(4);

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_sbx(bytecode[0]), 5);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn optimized_redundant_self_move_is_removed() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 9);
    builder.emit_mov(1, 1);
    builder.emit_ret_reg(1);

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(9.0));
}

#[test]
fn optimized_load_move_rewrites_to_direct_load() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_mov(2, 1);
    builder.emit_ret_reg(2);

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_sbx(bytecode[0]), 5);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn optimized_duplicate_constant_loads_share_registers() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 5);
    builder.emit_ret_reg(2);

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_sbx(bytecode[0]), 5);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn optimized_known_nullish_checks_fold_to_boolean_loads() {
    let mut builder = BytecodeBuilder::new();
    let undefined = builder.add_constant(make_undefined());
    builder.emit_load_k(1, undefined);
    builder.emit_is_undef(2, 1);
    builder.emit_ret_reg(2);

    let (bytecode, const_pool) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadK);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);

    let vm = run_vm_with_constants(bytecode, const_pool);
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), Some(true));
}

#[test]
fn optimized_reverse_moves_collapse_safely() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(2, 7);
    builder.emit_mov(1, 2);
    builder.emit_mov(2, 1);
    builder.emit_ret_reg(1);

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_sbx(bytecode[0]), 7);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(7.0));
}

#[test]
fn optimized_jump_threading_skips_intermediate_jump() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_jmp(0);
    builder.emit_jmp(1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();

    let (bytecode, _) = builder.build_optimized();
    assert_eq!(decode_opcode(bytecode[0]), Opcode::Jmp);
    assert_eq!(decode_sbx(bytecode[0]), 2);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(7.0));
}

#[test]
fn vm_optimization_simplify_branches_threads_unconditional_jumps() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_jmp(0);
    builder.emit_jmp(1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = simplify_branches(bytecode, constants);

    assert_eq!(decode_opcode(bytecode[0]), Opcode::Jmp);
    assert_eq!(decode_sbx(bytecode[0]), 2);
}

#[test]
fn vm_optimization_optimize_peephole_rewrites_load_move() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_mov(2, 1);
    builder.emit_ret_reg(2);

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = optimize_peephole(bytecode, constants);

    assert_eq!(bytecode.len(), 3);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[1]), 2);
    assert_eq!(decode_sbx(bytecode[1]), 5);
}

#[test]
fn vm_optimization_eliminate_dead_code_removes_unused_loads() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 9);
    builder.emit_ret_reg(1);

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = eliminate_dead_code(bytecode, constants);

    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[0]), 1);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
}

#[test]
fn vm_optimization_copy_propagation_rewrites_register_uses() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_mov(2, 1);
    builder.emit_ret_reg(2);

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = copy_propagation(bytecode, constants);

    assert_eq!(bytecode.len(), 3);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::Mov);
    assert_eq!(decode_opcode(bytecode[2]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[2]), 1);
}

#[test]
fn vm_optimization_coalesce_registers_rewrites_duplicate_loads_to_move() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 5);
    builder.emit_ret_reg(2);

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = coalesce_registers(bytecode, constants);

    assert_eq!(bytecode.len(), 3);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::Mov);
    assert_eq!(decode_a(bytecode[1]), 2);
    assert_eq!(decode_b(bytecode[1]), 1);
}

#[test]
fn vm_optimization_fold_temporary_checks_rewrites_known_null_tests() {
    let mut builder = BytecodeBuilder::new();
    let null_value = builder.add_constant(make_null());
    builder.emit_load_k(1, null_value);
    builder.emit_is_null(2, 1);
    builder.emit_ret_reg(2);

    let (bytecode, constants) = builder.build();
    let (bytecode, const_pool) = fold_temporary_checks(bytecode, constants);

    assert_eq!(bytecode.len(), 3);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::LoadK);

    let vm = run_vm_with_constants(bytecode, const_pool);
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), Some(true));
}

#[test]
fn vm_optimization_relocate_jumps_preserves_branch_targets() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_jmp(1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = relocate_jumps(bytecode, constants);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(7.0));
}
