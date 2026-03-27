use vm::codegen::compile_source;
use vm::js_value::{bool_from_value, is_undefined};
use vm::vm::VM;

const ACC: usize = 255;

fn run_vm(source: &str) -> VM {
    let compiled = compile_source(source).expect("source should compile");
    let mut vm = VM::from_compiled(compiled, vec![]);
    vm.run(false);
    vm
}

#[test]
fn delete_local_identifier_returns_false() {
    let vm = run_vm("function f(a) { return delete a; } f(1);");
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), Some(false));
}

#[test]
fn delete_plain_value_returns_true() {
    let vm = run_vm("delete 1;");
    assert_eq!(bool_from_value(vm.frame.regs[ACC]), Some(true));
}

#[test]
fn delete_named_property_removes_it() {
    let vm = run_vm("let obj = { a: 1 }; delete obj.a; obj.a;");
    assert!(is_undefined(vm.frame.regs[ACC]));
}
