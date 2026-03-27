use vm::js_value::make_number;
use vm::opt::optimize_peephole;

fn main() {
    let constants = vec![make_number(0.0), make_number(1.0), make_number(2.0)];

    println!("=== Testing All Peephole Optimizations ===\n");

    // Test 1: LoadAdd (LoadI + AddI -> LoadAdd)
    println!("1. LoadAdd (LoadI + AddI -> LoadAdd):");
    let bytecode = vec![encode_load_i(1, 42), encode_add_i(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadAdd", 176);

    // Test 2: LoadSub (LoadI + SubI -> LoadSub)
    println!("\n2. LoadSub (LoadI + SubI -> LoadSub):");
    let bytecode = vec![encode_load_i(1, 42), encode_sub_i(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadSub", 177);

    // Test 3: LoadMul (LoadI + MulI -> LoadMul)
    println!("\n3. LoadMul (LoadI + MulI -> LoadMul):");
    let bytecode = vec![encode_load_i(1, 42), encode_mul_i(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadMul", 178);

    // Test 4: LoadInc (LoadI + Inc -> LoadInc)
    println!("\n4. LoadInc (LoadI + Inc -> LoadInc):");
    let bytecode = vec![encode_load_i(1, 42), encode_inc(1)];
    test_optimization(&bytecode, &constants, "LoadInc", 179);

    // Test 5: LoadDec (LoadI + Dec -> LoadDec)
    println!("\n5. LoadDec (LoadI + Dec -> LoadDec):");
    let bytecode = vec![encode_load_i(1, 42), encode_dec(1)];
    test_optimization(&bytecode, &constants, "LoadDec", 180);

    // Test 6: LoadCmpEq (LoadI + Eq -> LoadCmpEq)
    println!("\n6. LoadCmpEq (LoadI + Eq -> LoadCmpEq):");
    let bytecode = vec![encode_load_i(1, 42), encode_eq(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadCmpEq", 181);

    // Test 7: LoadCmpLt (LoadI + Lt -> LoadCmpLt)
    println!("\n7. LoadCmpLt (LoadI + Lt -> LoadCmpLt):");
    let bytecode = vec![encode_load_i(1, 42), encode_lt(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadCmpLt", 182);

    // Test 8: LoadJfalse (LoadI + JmpFalse -> LoadJfalse)
    println!("\n8. LoadJfalse (LoadI + JmpFalse -> LoadJfalse):");
    let bytecode = vec![encode_load_i(1, 42), encode_jmpfalse(1, 1)];
    test_optimization(&bytecode, &constants, "LoadJfalse", 183);

    // Test 9: LoadCmpEqJfalse (LoadI + Eq + JmpFalse -> LoadCmpEqJfalse)
    println!("\n9. LoadCmpEqJfalse (LoadI + Eq + JmpFalse -> LoadCmpEqJfalse):");
    let bytecode = vec![
        encode_load_i(1, 42),
        encode_eq(2, 1, 0),
        encode_jmpfalse(2, 1),
    ];
    test_optimization(&bytecode, &constants, "LoadCmpEqJfalse", 184);

    // Test 10: LoadCmpLtJfalse (LoadI + Lt + JmpFalse -> LoadCmpLtJfalse)
    println!("\n10. LoadCmpLtJfalse (LoadI + Lt + JmpFalse -> LoadCmpLtJfalse):");
    let bytecode = vec![
        encode_load_i(1, 42),
        encode_lt(2, 1, 0),
        encode_jmpfalse(2, 1),
    ];
    test_optimization(&bytecode, &constants, "LoadCmpLtJfalse", 185);

    // Test 11: LoadGetProp (LoadK + GetProp -> LoadGetProp)
    println!("\n11. LoadGetProp (LoadK + GetProp -> LoadGetProp):");
    let bytecode = vec![encode_load_k(1, 0), encode_getprop(2, 1, 0)];
    test_optimization(&bytecode, &constants, "LoadGetProp", 186);

    // Test 12: LoadGetPropCmpEq (LoadK + GetProp + Eq -> LoadGetPropCmpEq)
    println!("\n12. LoadGetPropCmpEq (LoadK + GetProp + Eq -> LoadGetPropCmpEq):");
    let bytecode = vec![
        encode_load_k(1, 0),
        encode_getprop(2, 1, 0),
        encode_eq(3, 2, 0),
    ];
    test_optimization(&bytecode, &constants, "LoadGetPropCmpEq", 187);

    // Test 13: GetProp2Ic (GetProp + GetProp -> GetProp2Ic)
    println!("\n13. GetProp2Ic (GetProp + GetProp -> GetProp2Ic):");
    let bytecode = vec![encode_getprop(1, 0, 0), encode_getprop(2, 1, 1)];
    test_optimization(&bytecode, &constants, "GetProp2Ic", 188);

    // Test 14: GetProp3Ic (GetProp + GetProp + GetProp -> GetProp3Ic)
    println!("\n14. GetProp3Ic (GetProp + GetProp + GetProp -> GetProp3Ic):");
    let bytecode = vec![
        encode_getprop(1, 0, 0),
        encode_getprop(2, 1, 1),
        encode_getprop(3, 2, 2),
    ];
    test_optimization(&bytecode, &constants, "GetProp3Ic", 189);

    // Test 15: GetElem (LoadK + GetIdxFast -> GetElem)
    println!("\n15. GetElem (LoadK + GetIdxFast -> GetElem):");
    let bytecode = vec![encode_load_k(1, 0), encode_getidxfast(2, 1, 0)];
    test_optimization(&bytecode, &constants, "GetElem", 190);

    // Test 16: SetElem (LoadK + SetIdxFast -> SetElem)
    println!("\n16. SetElem (LoadK + SetIdxFast -> SetElem):");
    let bytecode = vec![encode_load_k(1, 0), encode_setidxfast(2, 1, 0)];
    test_optimization(&bytecode, &constants, "SetElem", 191);

    // Test 17: GetPropElem (GetProp + GetIdxFast -> GetPropElem)
    println!("\n17. GetPropElem (GetProp + GetIdxFast -> GetPropElem):");
    let bytecode = vec![encode_getprop(1, 0, 0), encode_getidxfast(2, 1, 0)];
    test_optimization(&bytecode, &constants, "GetPropElem", 192);

    // Test 18: CallMethodIc (GetProp + Call -> CallMethodIc)
    println!("\n18. CallMethodIc (GetProp + Call -> CallMethodIc):");
    let bytecode = vec![encode_getprop(1, 0, 0), encode_call(1, 0)];
    test_optimization(&bytecode, &constants, "CallMethodIc", 193);

    // Test 19: CallMethod2Ic (GetProp + GetProp + Call -> CallMethod2Ic)
    println!("\n19. CallMethod2Ic (GetProp + GetProp + Call -> CallMethod2Ic):");
    let bytecode = vec![
        encode_getprop(1, 0, 0),
        encode_getprop(2, 1, 1),
        encode_call(2, 0),
    ];
    test_optimization(&bytecode, &constants, "CallMethod2Ic", 194);

    println!("\n=== All tests completed ===");
}

fn test_optimization(
    bytecode: &[u32],
    constants: &[vm::js_value::JSValue],
    name: &str,
    expected_opcode: u32,
) {
    let (optimized, _) = optimize_peephole(bytecode.to_vec(), constants.to_vec());

    if optimized.len() < bytecode.len() {
        print!(
            "✓ {}: {} -> {} instructions",
            name,
            bytecode.len(),
            optimized.len()
        );

        if let Some(&first_insn) = optimized.first() {
            let opcode = first_insn & 0xFF;
            if opcode == expected_opcode {
                println!(" (correct opcode {})", opcode);
            } else {
                println!(" ✗ WRONG OPCODE: {} (expected {})", opcode, expected_opcode);
            }
        } else {
            println!(" (no instructions?)");
        }
    } else {
        println!(
            "✗ {}: No optimization ({} -> {} instructions)",
            name,
            bytecode.len(),
            optimized.len()
        );
    }
}

// Encoding helper functions
fn encode_load_i(dst: u8, value: i16) -> u32 {
    ((value as u16 as u32) << 16) | ((dst as u32) << 8) | 6
}

fn encode_add_i(dst: u8, src1: u8, src2: u8) -> u32 {
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 36
}

fn encode_sub_i(dst: u8, src1: u8, src2: u8) -> u32 {
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 37
}

fn encode_mul_i(dst: u8, src1: u8, src2: u8) -> u32 {
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 38
}

fn encode_inc(dst: u8) -> u32 {
    ((dst as u32) << 8) | 42
}

fn encode_dec(dst: u8) -> u32 {
    ((dst as u32) << 8) | 43
}

fn encode_eq(dst: u8, src1: u8, src2: u8) -> u32 {
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 15
}

fn encode_lt(dst: u8, src1: u8, src2: u8) -> u32 {
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 16
}

fn encode_jmpfalse(reg: u8, offset: i16) -> u32 {
    ((offset as u16 as u32) << 16) | ((reg as u32) << 8) | 8
}

fn encode_load_k(dst: u8, index: u16) -> u32 {
    ((index as u32) << 16) | ((dst as u32) << 8) | 1
}

fn encode_getprop(dst: u8, src: u8, prop: u8) -> u32 {
    ((prop as u32) << 24) | ((src as u32) << 16) | ((dst as u32) << 8) | 70
}

fn encode_getidxfast(dst: u8, src: u8, idx: u8) -> u32 {
    ((idx as u32) << 24) | ((src as u32) << 16) | ((dst as u32) << 8) | 48
}

fn encode_setidxfast(dst: u8, src: u8, idx: u8) -> u32 {
    ((idx as u32) << 24) | ((src as u32) << 16) | ((dst as u32) << 8) | 49
}

fn encode_call(func: u8, argc: u8) -> u32 {
    ((argc as u32) << 16) | ((func as u32) << 8) | 4
}
