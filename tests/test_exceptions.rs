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
fn try_catch_returns_thrown_value() {
    let result = run_number("function f() { try { throw 42; } catch (err) { return err; } } f();");
    assert_eq!(result, 42.0);
}

#[test]
fn catch_parameter_can_be_read_inside_handler() {
    let result =
        run_number("function f() { try { throw 41; } catch (err) { return err + 1; } } f();");
    assert_eq!(result, 42.0);
}
