use vm::asm::disassemble_clean;
use vm::emit::BytecodeBuilder;
use vm::js_value::{make_number, to_f64};
use vm::vm::{Opcode, VM};

const ACC: u8 = 255;

fn patch_asbx(bytecode: &mut [u32], index: usize, opcode: u8, a: u8, target: usize) {
    let offset = target as i16 - (index as i16 + 1);
    bytecode[index] = (((offset as u16) as u32) << 16) | ((a as u32) << 8) | opcode as u32;
}

fn build_control_flow_program(call_arg: i16) -> (Vec<u32>, Vec<vm::js_value::JSValue>) {
    let mut builder = BytecodeBuilder::new();
    let function_entry_const = builder.add_constant(make_number(0.0));

    // main:
    //   const controlFlowStress = function(n) { ... }
    //   return controlFlowStress(call_arg)
    builder.emit_new_func(1, function_entry_const);
    builder.emit_load_i(2, call_arg);
    builder.emit_call(1, 1);
    builder.emit_ret();

    // controlFlowStress(n):
    //   let sum = 0;
    //   for (let i = 0; i < n; i++) {
    //     if (i % 2 === 0) sum += i;
    //     else sum -= i;
    //   }
    //   return sum;
    let function_entry = builder.len();
    builder.emit_load_arg(1, 0);
    builder.emit_load_0();
    builder.emit_mov(2, ACC);
    builder.emit_load_0();
    builder.emit_mov(3, ACC);

    let loop_start = builder.len();
    builder.emit_lt(3, 1);
    let loop_exit = builder.len();
    builder.emit_jmp_false(ACC, 0);

    builder.emit_mod_i(4, 3, 2);
    builder.emit_load_0();
    builder.emit_mov(5, ACC);
    builder.emit_eq(4, 5);
    let else_jump = builder.len();
    builder.emit_jmp_false(ACC, 0);

    builder.emit_add(2, 3);
    builder.emit_mov(2, ACC);
    let after_if_jump = builder.len();
    builder.emit_jmp(0);

    let else_label = builder.len();
    builder.emit_mov(ACC, 2);
    builder.emit_sub_acc(3);
    builder.emit_mov(2, ACC);

    let after_if = builder.len();
    builder.emit_mov(ACC, 3);
    builder.emit_inc_acc();
    builder.emit_mov(3, ACC);
    builder.emit_jmp(-(builder.len() as i16 - loop_start as i16 + 1));

    let loop_end = builder.len();
    builder.emit_mov(ACC, 2);
    builder.emit_ret();

    let (mut bytecode, mut constants) = builder.build();
    constants[function_entry_const as usize] = make_number(function_entry as f64);
    patch_asbx(
        &mut bytecode,
        loop_exit,
        Opcode::JmpFalse.as_u8(),
        ACC,
        loop_end,
    );
    patch_asbx(
        &mut bytecode,
        else_jump,
        Opcode::JmpFalse.as_u8(),
        ACC,
        else_label,
    );
    patch_asbx(
        &mut bytecode,
        after_if_jump,
        Opcode::Jmp.as_u8(),
        0,
        after_if,
    );

    (bytecode, constants)
}

fn main() {
    let call_arg = 100;
    let expected = -50.0;
    let (bytecode, constants) = build_control_flow_program(call_arg);

    println!("=== Function Call Control Flow Example ===\n");
    println!("Source shape:");
    println!("  function controlFlowStress(n) {{ ... }}");
    println!("  return controlFlowStress({call_arg});");
    println!("\nBytecode generated: {} instructions", bytecode.len());

    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    let result = to_f64(vm.frame.regs[ACC as usize]).unwrap_or(f64::NAN);
    println!("\nResult: {result}");
    println!("Expected: {expected}");
    if (result - expected).abs() < f64::EPSILON {
        println!("Result matches expected value.");
    } else {
        println!("Result does not match expected value.");
    }

    println!("\nDisassembly:");
    for (index, line) in disassemble_clean(&vm.bytecode, &vm.const_pool)
        .iter()
        .enumerate()
    {
        println!("  {:3}: {}", index, line);
    }
}
