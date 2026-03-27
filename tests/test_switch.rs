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
fn switch_selects_matching_case() {
    let result = run_number(
        "function f() { switch (2) { case 1: return 10; case 2: return 20; default: return 30; } } f();",
    );
    assert_eq!(result, 20.0);
}

#[test]
fn switch_falls_through_without_break() {
    let result = run_number(
        "function f() { let out = 0; switch (1) { case 1: out = 10; case 2: out = out + 5; break; default: out = 99; } return out; } f();",
    );
    assert_eq!(result, 15.0);
}
