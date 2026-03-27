use vm::emit::BytecodeBuilder;
use vm::js_value::to_f64;
use vm::vm::VM;

const ACC: usize = 255;

#[test]
fn arithmetic_example_sum_loop_exits_and_returns_15() {
    let mut builder = BytecodeBuilder::new();

    builder.emit_load_0();
    builder.emit_mov(1, ACC as u8); // r1 = sum = 0
    builder.emit_load_1();
    builder.emit_mov(2, ACC as u8); // r2 = i = 1
    builder.emit_load_i(3, 5); // r3 = limit = 5

    let loop_start = builder.len();

    builder.emit_lte(2, 3); // ACC = i <= 5
    builder.emit_jmp_false(ACC as u8, 6); // jump to the return path when the loop is done

    builder.emit_add(1, 2); // ACC = sum + i
    builder.emit_mov(1, ACC as u8); // sum = ACC

    builder.emit_mov(ACC as u8, 2); // ACC = i
    builder.emit_inc_acc(); // ACC = i + 1
    builder.emit_mov(2, ACC as u8); // i = ACC

    builder.emit_jmp(-(builder.len() as i16 - loop_start as i16 + 1));

    builder.emit_mov(ACC as u8, 1);
    builder.emit_ret();

    let (bytecode, constants) = builder.build();
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    assert_eq!(to_f64(vm.frame.regs[ACC]), Some(15.0));
}
