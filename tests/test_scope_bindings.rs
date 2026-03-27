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
fn recursive_function_frames_keep_their_own_bindings() {
    let result = run_number(
        "function fib(n) { if (n <= 1) return n; return fib(n - 1) + fib(n - 2); } fib(5);",
    );
    assert_eq!(result, 5.0);
}

#[test]
fn assignment_inside_loop_updates_outer_binding() {
    let result = run_number("let result; for (let i = 0; i < 3; i++) { result = 7; } result;");
    assert_eq!(result, 7.0);
}
