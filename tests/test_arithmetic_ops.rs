use vm::codegen::compile_source;
use vm::js_value::to_f64;
use vm::vm::VM;

const ACC: usize = 255;

fn run_number(source: &str) -> f64 {
    let compiled = compile_source(source).expect("source should compile");
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    to_f64(vm.frame.regs[ACC]).expect("result should be numeric")
}

#[test]
fn generic_modulo_expression_runs() {
    let result = run_number("5 % 2;");
    assert_eq!(result, 1.0);
}
