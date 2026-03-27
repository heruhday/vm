use vm::asm::disassemble_clean;
use vm::codegen::compile_source;
use vm::emit::BytecodeBuilder;
use vm::js_value::{bool_from_value, make_null, make_undefined, to_f64};
use vm::opt::optimize_compiled;
use vm::vm::optimization::{
    coalesce_registers, copy_propagation, eliminate_dead_code, fold_temporary_checks,
    optimize_peephole, relocate_jumps, reuse_registers_linear_scan, simplify_branches,
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

fn run_optimized_source(source: &str) -> VM {
    let compiled = optimize_compiled(compile_source(source).expect("source should compile"));
    let mut vm = VM::from_compiled(compiled, Vec::new());
    vm.run(false);
    vm
}

fn optimize_without_peephole(
    mut bytecode: Vec<u32>,
    mut constants: Vec<vm::js_value::JSValue>,
) -> (Vec<u32>, Vec<vm::js_value::JSValue>) {
    for _ in 0..8 {
        let prev_bytecode = bytecode.clone();
        let prev_constants = constants.clone();
        (bytecode, constants) = coalesce_registers(bytecode, constants);
        (bytecode, constants) = copy_propagation(bytecode, constants);
        (bytecode, constants) = eliminate_dead_code(bytecode, constants);
        (bytecode, constants) = fold_temporary_checks(bytecode, constants);
        (bytecode, constants) = simplify_branches(bytecode, constants);
        if bytecode == prev_bytecode && constants == prev_constants {
            break;
        }
    }
    (bytecode, constants) = reuse_registers_linear_scan(bytecode, constants);
    relocate_jumps(bytecode, constants)
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

fn encode_abc(opcode: Opcode, a: u8, b: u8, c: u8) -> u32 {
    ((c as u32) << 24) | ((b as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_abx(opcode: Opcode, a: u8, bx: u16) -> u32 {
    ((bx as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
}

fn encode_asbx(opcode: Opcode, a: u8, sbx: i16) -> u32 {
    (((sbx as u16) as u32) << 16) | ((a as u32) << 8) | opcode.as_u8() as u32
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
fn optimized_loop_condition_preserves_distinct_load_name_values() {
    let vm = run_optimized_source(
        "const runs = 3; let result = 0; for (let i = 0; i < runs; i++) { result = i + 1; } result;",
    );
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(3.0));
}

#[test]
fn optimized_recursive_fib_source_returns_five() {
    let vm = run_optimized_source(
        "function fib(n) { if (n <= 1) return n; return fib(n - 1) + fib(n - 2); } fib(5);",
    );
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn optimized_iterative_fib_source_returns_two_thirty_three() {
    let vm = run_optimized_source(
        "function fib(n) { if (n <= 1) return n; let a = 0; let b = 1; for (let i = 2; i <= n; i++) { let c = a + b; a = b; b = c; } return b; } fib(13);",
    );
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(233.0));
}

#[test]
fn compiled_recursive_fib_source_uses_fused_return_instructions() {
    let compiled = compile_source(
        "function fib(n) { if (n <= 1) return n; return fib(n - 1) + fib(n - 2); } fib(5);",
    )
    .expect("source should compile");
    let opcodes = compiled
        .bytecode
        .iter()
        .map(|&raw| decode_opcode(raw))
        .collect::<Vec<_>>();

    assert!(opcodes.contains(&Opcode::RetIfLteI));
    assert!(opcodes.contains(&Opcode::RetReg));
    assert!(
        opcodes
            .iter()
            .filter(|&&opcode| opcode == Opcode::Call1SubI)
            .count()
            >= 2
    );
}

#[test]
fn compiled_iterative_fib_source_uses_fused_loop_branch() {
    let compiled = compile_source(
        "function fib(n) { if (n <= 1) return n; let a = 0; let b = 1; for (let i = 2; i <= n; i++) { let c = a + b; a = b; b = c; } return b; } fib(13);",
    )
    .expect("source should compile");
    if std::env::var_os("PRINT_OPT_ITER_FIB_DISASM").is_some() {
        for line in disassemble_clean(&compiled.bytecode, &compiled.constants) {
            println!("{line}");
        }
    }
    let opcodes = compiled
        .bytecode
        .iter()
        .map(|&raw| decode_opcode(raw))
        .collect::<Vec<_>>();

    assert!(opcodes.contains(&Opcode::RetIfLteI));
    assert!(opcodes.contains(&Opcode::JmpLteFalse));
}

#[test]
fn optimized_iterative_fib_source_disassembles_cleanly() {
    let compiled = optimize_compiled(
        compile_source(
            "function fib(n) { if (n <= 1) return n; let a = 0; let b = 1; for (let i = 2; i <= n; i++) { let c = a + b; a = b; b = c; } return b; } fib(13);",
        )
        .expect("source should compile"),
    );
    if std::env::var_os("PRINT_OPT_ITER_FIB_OPT_DISASM").is_some() {
        for line in disassemble_clean(&compiled.bytecode, &compiled.constants) {
            println!("{line}");
        }
    }

    assert!(!compiled.bytecode.is_empty());
}

#[test]
fn optimized_source_uses_method_call_superinstructions() {
    let compiled = optimize_compiled(
        compile_source("console.log(1); console.log(2, 3); 0;").expect("source should compile"),
    );
    let opcodes = compiled
        .bytecode
        .iter()
        .map(|&raw| decode_opcode(raw))
        .collect::<Vec<_>>();

    assert!(opcodes.contains(&Opcode::CallMethod1));
    assert!(opcodes.contains(&Opcode::CallMethod2));
}

#[test]
fn optimized_recursive_fib_script_disassembles_cleanly() {
    let source = include_str!("../fib_recursive.qjs");
    let compiled = optimize_compiled(compile_source(source).expect("source should compile"));
    let disasm = disassemble_clean(&compiled.bytecode, &compiled.constants);
    if std::env::var_os("PRINT_OPT_FIB_DISASM").is_some() {
        for line in &disasm {
            println!("{line}");
        }
    }

    assert!(!compiled.bytecode.is_empty());
    assert!(!disasm.is_empty());
    assert!(compiled.bytecode.len() < 512);
}

#[test]
fn optimized_recursive_fib_script_runs_single_iteration() {
    let source =
        include_str!("../fib_recursive.qjs").replace("const runs = 10;", "const runs = 1;");
    let compiled = optimize_compiled(compile_source(&source).expect("source should compile"));
    let mut vm = VM::from_compiled(compiled, Vec::new());
    vm.run(false);

    assert!(
        vm.console_output
            .iter()
            .any(|line| line.contains("fib(25) = 75025"))
    );
}

#[test]
fn optimized_iterative_fib_script_disassembles_cleanly() {
    let source = include_str!("../fib.qjs");
    let compiled = optimize_compiled(compile_source(source).expect("source should compile"));
    let disasm = disassemble_clean(&compiled.bytecode, &compiled.constants);
    if std::env::var_os("PRINT_OPT_ITER_SCRIPT_DISASM").is_some() {
        for line in &disasm {
            println!("{line}");
        }
    }

    let opcodes = compiled
        .bytecode
        .iter()
        .map(|&raw| decode_opcode(raw))
        .collect::<Vec<_>>();

    assert!(opcodes.contains(&Opcode::JmpLteFalse));
    assert!(!compiled.bytecode.is_empty());
    assert!(!disasm.is_empty());
}

#[test]
fn optimized_iterative_fib_script_runs_single_iteration() {
    let source = include_str!("../fib.qjs").replace("const runs = 100;", "const runs = 1;");
    let compiled = optimize_compiled(compile_source(&source).expect("source should compile"));
    let mut vm = VM::from_compiled(compiled, Vec::new());
    vm.run(false);

    assert!(
        vm.console_output
            .iter()
            .any(|line| line.contains("fib(25) = 75025"))
    );
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

#[test]
fn vm_optimization_reuse_registers_linear_scan_compacts_live_ranges() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(10, 2);
    builder.emit_load_i(11, 3);
    builder.emit_add(10, 11);
    builder.emit_mov(12, ACC as u8);
    builder.emit_load_i(13, 4);
    builder.emit_add(12, 13);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = reuse_registers_linear_scan(bytecode, constants);

    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[0]), 1);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[1]), 2);
    assert_eq!(decode_opcode(bytecode[3]), Opcode::Mov);
    assert_eq!(decode_a(bytecode[3]), 1);
    assert_eq!(decode_opcode(bytecode[4]), Opcode::LoadI);
    assert_eq!(decode_a(bytecode[4]), 2);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(9.0));
}

#[test]
fn vm_optimization_reuse_registers_linear_scan_preserves_call_arg_bundle() {
    let mut builder = BytecodeBuilder::new();
    let function_entry_const = builder.add_constant(vm::js_value::make_number(0.0));

    builder.emit_new_func(10, function_entry_const);
    builder.emit_load_i(11, 4);
    builder.emit_call(10, 1);
    builder.emit_ret();

    let function_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_1();
    builder.emit_add(1, ACC as u8);
    builder.emit_ret();

    let (bytecode, mut const_pool) = builder.build();
    const_pool[function_entry_const as usize] = vm::js_value::make_number(function_entry as f64);

    let (bytecode, const_pool) = reuse_registers_linear_scan(bytecode, const_pool);

    assert_eq!(decode_opcode(bytecode[2]), Opcode::Call);
    assert_eq!(decode_a(bytecode[0]), decode_a(bytecode[2]));
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[2]) + 1);

    let vm = run_vm_with_constants(bytecode, const_pool);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));
}

#[test]
fn vm_optimization_pipeline_without_peephole_combines_remaining_passes() {
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 5);
    builder.emit_load_i(2, 5);
    builder.emit_mov(3, 2);
    builder.emit_load_i(4, 9);
    builder.emit_ret_reg(3);

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = optimize_without_peephole(bytecode, constants);

    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadI);
    assert_eq!(decode_sbx(bytecode[0]), 5);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(5.0));

    let mut builder = BytecodeBuilder::new();
    let null_value = builder.add_constant(make_null());
    builder.emit_load_k(1, null_value);
    builder.emit_is_null(2, 1);
    builder.emit_mov(3, 2);
    builder.emit_ret_reg(3);

    let (bytecode, constants) = builder.build();
    let (bytecode, const_pool) = optimize_without_peephole(bytecode, constants);

    assert_eq!(bytecode.len(), 2);
    assert_eq!(decode_opcode(bytecode[0]), Opcode::LoadK);
    assert_eq!(decode_opcode(bytecode[1]), Opcode::RetReg);
    assert_eq!(decode_a(bytecode[1]), decode_a(bytecode[0]));

    let vm = run_vm_with_constants(bytecode, const_pool);
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), Some(true));

    let mut builder = BytecodeBuilder::new();
    builder.emit_jmp(0);
    builder.emit_jmp(1);
    builder.emit_load_i(ACC as u8, 99);
    builder.emit_load_i(ACC as u8, 7);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let (bytecode, _) = optimize_without_peephole(bytecode, constants);

    assert_eq!(decode_opcode(bytecode[0]), Opcode::Jmp);
    assert_eq!(decode_sbx(bytecode[0]), 2);

    let vm = run_vm(bytecode);
    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(7.0));
}

#[test]
fn vm_optimization_peephole_matrix_covers_more_than_sixty_source_instructions() {
    struct Case {
        name: &'static str,
        bytecode: Vec<u32>,
        expected: Vec<Opcode>,
    }

    let cases = vec![
        Case {
            name: "load_add",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::AddI, 2, 1, 0),
            ],
            expected: vec![Opcode::LoadAdd],
        },
        Case {
            name: "load_sub",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::SubI, 2, 1, 0),
            ],
            expected: vec![Opcode::LoadSub],
        },
        Case {
            name: "load_mul",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::MulI, 2, 1, 0),
            ],
            expected: vec![Opcode::LoadMul],
        },
        Case {
            name: "load_inc",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Inc, 1, 0, 0),
            ],
            expected: vec![Opcode::LoadInc],
        },
        Case {
            name: "load_dec",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Dec, 1, 0, 0),
            ],
            expected: vec![Opcode::LoadDec],
        },
        Case {
            name: "load_cmp_eq",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Eq, 2, 1, 0),
            ],
            expected: vec![Opcode::LoadCmpEq],
        },
        Case {
            name: "load_cmp_lt",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Lt, 2, 1, 0),
            ],
            expected: vec![Opcode::LoadCmpLt],
        },
        Case {
            name: "load_jfalse",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 0),
                encode_asbx(Opcode::JmpFalse, 1, 1),
            ],
            expected: vec![Opcode::LoadJfalse],
        },
        Case {
            name: "load_get_prop",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::GetProp, 2, 1, 3),
            ],
            expected: vec![Opcode::LoadGetProp],
        },
        Case {
            name: "load_get_prop_cmp_eq",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::GetProp, 2, 1, 3),
                encode_abc(Opcode::Eq, 4, 2, 0),
            ],
            expected: vec![Opcode::LoadGetPropCmpEq],
        },
        Case {
            name: "get_prop_2_ic",
            bytecode: vec![
                encode_abc(Opcode::GetProp, 1, 0, 3),
                encode_abc(Opcode::GetProp, 2, 1, 4),
            ],
            expected: vec![Opcode::GetProp2Ic],
        },
        Case {
            name: "get_prop_3_ic",
            bytecode: vec![
                encode_abc(Opcode::GetProp, 1, 0, 3),
                encode_abc(Opcode::GetProp, 2, 1, 4),
                encode_abc(Opcode::GetProp, 3, 2, 5),
            ],
            expected: vec![Opcode::GetProp3Ic],
        },
        Case {
            name: "get_elem",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::GetIdxFast, 2, 1, 3),
            ],
            expected: vec![Opcode::GetElem],
        },
        Case {
            name: "set_elem",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::SetIdxFast, 2, 1, 3),
            ],
            expected: vec![Opcode::SetElem],
        },
        Case {
            name: "get_prop_elem",
            bytecode: vec![
                encode_abc(Opcode::GetProp, 1, 0, 3),
                encode_abc(Opcode::GetIdxFast, 2, 1, 4),
            ],
            expected: vec![Opcode::GetPropElem],
        },
        Case {
            name: "call_method_ic",
            bytecode: vec![
                encode_abc(Opcode::GetProp, 1, 0, 3),
                encode_abc(Opcode::Call, 1, 0, 0),
            ],
            expected: vec![Opcode::CallMethodIc],
        },
        Case {
            name: "call_method_2_ic",
            bytecode: vec![
                encode_abc(Opcode::GetProp, 1, 0, 3),
                encode_abc(Opcode::GetProp, 2, 1, 4),
                encode_abc(Opcode::Call, 2, 0, 0),
            ],
            expected: vec![Opcode::CallMethod2Ic],
        },
        Case {
            name: "const_add_fold",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 2),
                encode_asbx(Opcode::LoadI, 2, 3),
                encode_abc(Opcode::Add, 0, 1, 2),
            ],
            expected: vec![Opcode::LoadI, Opcode::LoadI, Opcode::LoadI],
        },
        Case {
            name: "load_cmp_eq_jfalse_direct",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Eq, 2, 1, 0),
                encode_asbx(Opcode::JmpFalse, 2, 1),
            ],
            expected: vec![Opcode::LoadCmpEqJfalse],
        },
        Case {
            name: "load_cmp_lt_jfalse_direct",
            bytecode: vec![
                encode_asbx(Opcode::LoadI, 1, 42),
                encode_abc(Opcode::Lt, 2, 1, 0),
                encode_asbx(Opcode::JmpFalse, 2, 1),
            ],
            expected: vec![Opcode::LoadCmpLtJfalse],
        },
        Case {
            name: "get_prop_ic_call",
            bytecode: vec![
                encode_abc(Opcode::GetPropIc, 1, 0, 3),
                encode_abc(Opcode::Call, 1, 0, 0),
            ],
            expected: vec![Opcode::GetPropIcCall],
        },
        Case {
            name: "inc_jmp_false_loop",
            bytecode: vec![
                encode_abc(Opcode::IncAcc, 0, 0, 0),
                encode_asbx(Opcode::JmpFalse, ACC as u8, 1),
            ],
            expected: vec![Opcode::IncJmpFalseLoop],
        },
        Case {
            name: "load_k_add",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::Add, 0, 1, 2),
            ],
            expected: vec![Opcode::LoadKAdd],
        },
        Case {
            name: "load_k_cmp",
            bytecode: vec![
                encode_abx(Opcode::LoadK, 1, 0),
                encode_abc(Opcode::Eq, 0, 1, 2),
            ],
            expected: vec![Opcode::LoadKCmp],
        },
        Case {
            name: "load_cmp_eq_then_jfalse",
            bytecode: vec![
                encode_abc(Opcode::LoadCmpEq, 2, 1, 0),
                encode_asbx(Opcode::JmpFalse, 2, 1),
            ],
            expected: vec![Opcode::LoadCmpEqJfalse],
        },
        Case {
            name: "load_cmp_lt_then_jfalse",
            bytecode: vec![
                encode_abc(Opcode::LoadCmpLt, 2, 1, 0),
                encode_asbx(Opcode::JmpFalse, 2, 1),
            ],
            expected: vec![Opcode::LoadCmpLtJfalse],
        },
        Case {
            name: "add_acc_reg",
            bytecode: vec![
                encode_abc(Opcode::LoadAcc, 4, 0, 0),
                encode_abc(Opcode::Add, 0, ACC as u8, 5),
            ],
            expected: vec![Opcode::AddAccReg],
        },
        Case {
            name: "call_1_add",
            bytecode: vec![
                encode_abc(Opcode::Call, 3, 0, 0),
                encode_abc(Opcode::Add, 0, ACC as u8, 5),
            ],
            expected: vec![Opcode::Call1Add],
        },
        Case {
            name: "call_2_add",
            bytecode: vec![
                encode_abc(Opcode::Call, 3, 0, 0),
                encode_abc(Opcode::Call, 3, 0, 0),
                encode_abc(Opcode::Add, 0, ACC as u8, 5),
            ],
            expected: vec![Opcode::Call2Add],
        },
    ];

    let total_source_instructions: usize = cases.iter().map(|case| case.bytecode.len()).sum();
    assert!(
        total_source_instructions > 60,
        "expected more than 60 source instructions, got {total_source_instructions}"
    );

    for case in cases {
        let (optimized, _) = optimize_peephole(case.bytecode, vec![make_null()]);
        let actual: Vec<_> = optimized.into_iter().map(decode_opcode).collect();
        assert_eq!(actual, case.expected, "peephole case {}", case.name);
    }
}
