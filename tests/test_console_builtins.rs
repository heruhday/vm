use vm::codegen::compile_source;
use vm::opt::optimize_compiled;
use vm::vm::VM;

fn run_vm(source: &str) -> VM {
    let compiled = compile_source(source).expect("source should compile");
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    vm
}

fn run_optimized_vm(source: &str) -> VM {
    let compiled = optimize_compiled(compile_source(source).expect("source should compile"));
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    vm
}

#[test]
fn console_log_error_and_warn_capture_output() {
    let vm = run_vm("console.log(1, 2, true); console.error(3); console.warn(4); 0;");
    assert_eq!(vm.console_output, vec!["1 2 true", "3", "4"]);
}

#[test]
fn console_log_supports_runtime_string_literals() {
    let vm = run_vm("console.log('hello'); 0;");
    assert_eq!(vm.console_output, vec!["hello"]);
}

#[test]
fn console_log_supports_template_literals() {
    let vm = run_vm("let a = 'heru'; console.log(`hello${a}`); 0;");
    assert_eq!(vm.console_output, vec!["helloheru"]);
}

#[test]
fn console_time_end_and_assert_capture_output() {
    let vm = run_vm("console.time(1); console.timeEnd(1); console.assert(0, 2, true); 0;");
    assert_eq!(vm.console_output.len(), 2);
    assert!(vm.console_output[0].starts_with("1: "));
    assert!(vm.console_output[0].ends_with("ms"));
    assert_eq!(vm.console_output[1], "Assertion failed: 2 true");
}

#[test]
fn console_count_group_and_clear_work() {
    let vm = run_vm(
        "console.count(); console.count(); console.group(1); console.log(2); console.groupEnd(); console.clear(); console.log(3); 0;",
    );
    assert_eq!(vm.console_output, vec!["3"]);
}

#[test]
fn optimized_console_log_preserves_method_call() {
    let vm = run_optimized_vm("console.log(1); 0;");
    assert_eq!(vm.console_output, vec!["1"]);
}

#[test]
fn optimized_console_log_two_args_preserves_method_call() {
    let vm = run_optimized_vm("console.log(1, 2); 0;");
    assert_eq!(vm.console_output, vec!["1 2"]);
}
