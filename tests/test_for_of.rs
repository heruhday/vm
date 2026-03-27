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
fn for_of_sums_array_values() {
    let result = run_number(
        "function f() { let sum = 0; for (let x of [1, 2, 3]) { sum += x; } return sum; } f();",
    );
    assert_eq!(result, 6.0);
}

#[test]
fn for_of_respects_continue_and_break() {
    let result = run_number(
        "function f() { let sum = 0; for (let x of [1, 2, 3, 4]) { if (x === 3) continue; if (x === 4) break; sum += x; } return sum; } f();",
    );
    assert_eq!(result, 3.0);
}
