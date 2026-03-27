//! Runtime Operations Demo
//! Demonstrates all operations from runtime_trait.rs

fn main() {
    println!("=== Runtime Operations Demo ===\n");

    // Create some test values (just for display)
    println!("Test values that would be used:");
    println!("  num5: make_number(5.0)");
    println!("  num3: make_number(3.0)");
    println!("  num2: make_number(2.0)");
    println!("  str_hello: vm.intern_string(\"Hello\")");
    println!("  str_world: vm.intern_string(\"World\")");
    println!("  bool_true: make_true()");
    println!("  bool_false: make_false()");
    println!("  null: make_null()");
    println!("  undefined: make_undefined()");
    println!();

    // Note: The traits are implemented in vm.rs for VmValue.
    // VmValue requires a VM context pointer, so we can't demonstrate
    // actual usage here. Instead, we show the concepts and API.

    println!("=== Operation Categories ===\n");

    println!("1. ArithmeticOps:");
    println!("   - add: 5 + 3 = 8");
    println!("   - sub: 5 - 3 = 2");
    println!("   - mul: 5 * 3 = 15");
    println!("   - div: 5 / 2 = 2.5");
    println!("   - rem: 5 % 2 = 1");
    println!("   - pow: 5 ** 2 = 25");
    println!("   - inc: ++5 = 6");
    println!("   - dec: --5 = 4");
    println!("   - unary_plus: +5 = 5");
    println!("   - unary_minus: -5 = -5");
    println!();

    println!("2. ComparisonOps:");
    println!("   - eq: 5 == 3 = false");
    println!("   - ne: 5 != 3 = true");
    println!("   - strict_eq: 5 === 5 = true");
    println!("   - strict_ne: 5 !== '5' = true");
    println!("   - gt: 5 > 3 = true");
    println!("   - lt: 5 < 3 = false");
    println!("   - ge: 5 >= 5 = true");
    println!("   - le: 5 <= 3 = false");
    println!();

    println!("3. LogicalOps:");
    println!("   - logical_and: true && false = false");
    println!("   - logical_or: true || false = true");
    println!("   - logical_not: !true = false");
    println!();

    println!("4. BitwiseOps:");
    println!("   - bit_and: 5 & 3 = 1 (0b101 & 0b011 = 0b001)");
    println!("   - bit_or: 5 | 3 = 7 (0b101 | 0b011 = 0b111)");
    println!("   - bit_xor: 5 ^ 3 = 6 (0b101 ^ 0b011 = 0b110)");
    println!("   - bit_not: ~5 = -6");
    println!("   - shl: 5 << 1 = 10");
    println!("   - shr: 5 >> 1 = 2");
    println!("   - ushr: -5 >>> 1 = 2147483645");
    println!();

    println!("5. AssignmentOps:");
    println!("   - assign: x = 5");
    println!("   - add_assign: x += 3 (x = 8)");
    println!("   - sub_assign: x -= 2 (x = 6)");
    println!("   - mul_assign: x *= 2 (x = 12)");
    println!("   - div_assign: x /= 3 (x = 4)");
    println!("   - rem_assign: x %= 3 (x = 1)");
    println!("   - pow_assign: x **= 2 (x = 1)");
    println!("   - shl_assign: x <<= 2 (x = 4)");
    println!("   - shr_assign: x >>= 1 (x = 2)");
    println!("   - ushr_assign: x >>>= 1 (x = 1)");
    println!("   - bit_and_assign: x &= 3 (x = 1)");
    println!("   - bit_or_assign: x |= 2 (x = 3)");
    println!("   - bit_xor_assign: x ^= 1 (x = 2)");
    println!();

    println!("6. LogicalAssignOps:");
    println!("   - and_assign: x &&= false (x = false)");
    println!("   - or_assign: x ||= true (x = true)");
    println!();

    println!("7. NullishOps:");
    println!("   - nullish_coalesce: null ?? 'default' = 'default'");
    println!("   - nullish_assign: x ??= 'value' (if x is null/undefined)");
    println!();

    println!("8. TypeOps:");
    println!("   - typeof_: typeof 5 = 'number'");
    println!("   - instanceof: [] instanceof Array = true");
    println!("   - in_: 'length' in [] = true");
    println!("   - delete: delete obj.prop = true");
    println!();

    println!("9. CoercionOps:");
    println!("   - to_number: Number('5') = 5");
    println!("   - to_string: String(5) = '5'");
    println!("   - to_boolean: Boolean(1) = true");
    println!("   - to_primitive: Object to primitive value");
    println!();

    println!("10. PropertyOps:");
    println!("   - get: obj.prop or obj['prop']");
    println!("   - set: obj.prop = value");
    println!("   - has: 'prop' in obj");
    println!("   - delete_property: delete obj.prop");
    println!();

    println!("11. CallOps:");
    println!("   - call: func.call(this, arg1, arg2)");
    println!("   - construct: new Constructor(arg1, arg2)");
    println!();

    println!("12. Ternary:");
    println!("   - ternary: condition ? a : b");
    println!();

    println!("=== Implementation Notes ===\n");

    println!("The traits are implemented in vm.rs for VmValue:");
    println!("- VmValue implements all ValueOps traits");
    println!("- Each operation corresponds to JavaScript semantics");
    println!("- Operations handle type coercion automatically");
    println!("- The VM uses these traits during bytecode execution");
    println!();

    println!("=== Example VM Usage ===\n");

    println!("In the VM, operations look like:");
    println!("  let vm_value = VmValue::new(vm_ptr, js_value);");
    println!("  let result = vm_value.add(&other_value);");
    println!("  let is_equal = vm_value.eq(&other_value);");
    println!("  let coerced = vm_value.to_number();");
    println!();

    println!("=== Bytecode Correspondence ===\n");

    println!("Each trait method corresponds to bytecode instructions:");
    println!("  add -> Opcode::Add, Opcode::AddAcc, Opcode::AddI");
    println!("  sub -> Opcode::SubAcc, Opcode::SubI");
    println!("  mul -> Opcode::MulAcc, Opcode::MulI");
    println!("  eq -> Opcode::Eq, Opcode::StrictEq");
    println!("  lt -> Opcode::Lt");
    println!("  lte -> Opcode::Lte");
    println!("  to_number -> Opcode::ToNum");
    println!("  to_string -> Opcode::ToStr");
    println!("  typeof_ -> Opcode::Typeof");
    println!("  get -> Opcode::GetProp, Opcode::GetPropIc");
    println!("  set -> Opcode::SetProp, Opcode::SetPropIc");
    println!();

    println!("=== Complete Runtime API ===\n");

    println!("The ValueOps trait combines all operations:");
    println!("  pub trait ValueOps:");
    println!("      ArithmeticOps");
    println!("      + ComparisonOps");
    println!("      + LogicalOps");
    println!("      + BitwiseOps");
    println!("      + AssignmentOps");
    println!("      + LogicalAssignOps");
    println!("      + NullishOps");
    println!("      + TypeOps");
    println!("      + CoercionOps");
    println!("      + PropertyOps");
    println!("      + CallOps");
    println!("      + Ternary");
    println!();

    println!("This provides a complete JavaScript runtime API!");
}
