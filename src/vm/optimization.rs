use crate::js_value::JSValue;

pub fn simplify_branches(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::simplify_branches(bytecode, constants)
}

pub fn eliminate_dead_code(
    bytecode: Vec<u32>,
    constants: Vec<JSValue>,
) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::eliminate_dead_code(bytecode, constants)
}

pub fn copy_propagation(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::copy_propagation(bytecode, constants)
}

pub fn coalesce_registers(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::coalesce_registers(bytecode, constants)
}

pub fn fold_temporary_checks(
    bytecode: Vec<u32>,
    constants: Vec<JSValue>,
) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::fold_temporary_checks(bytecode, constants)
}

pub fn optimize_peephole(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::optimize_peephole(bytecode, constants)
}

pub fn reuse_registers_linear_scan(
    bytecode: Vec<u32>,
    constants: Vec<JSValue>,
) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::reuse_registers_linear_scan(bytecode, constants)
}

pub fn relocate_jumps(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::relocate_jumps(bytecode, constants)
}

pub fn optimize_bytecode(bytecode: Vec<u32>, constants: Vec<JSValue>) -> (Vec<u32>, Vec<JSValue>) {
    crate::opt::optimize_bytecode(bytecode, constants)
}
