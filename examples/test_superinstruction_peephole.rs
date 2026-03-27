use vm::js_value::make_number;
use vm::opt::optimize_peephole;

fn main() {
    // Test LoadAdd optimization: LoadI + AddI -> LoadAdd
    // LoadI r1, 0 (opcode 6, a=1, value=0)
    // AddI r2, r1, r0 (opcode 36, a=2, b=1, c=0)
    let bytecode = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_add_i(2, 1, 0), // AddI r2, r1, r0
    ];
    let constants = vec![make_number(0.0), make_number(1.0)];

    let (optimized, _) = optimize_peephole(bytecode.clone(), constants.clone());

    println!("Original bytecode: {:?}", bytecode);
    println!("Optimized bytecode: {:?}", optimized);
    println!(
        "Optimization successful: {}",
        optimized.len() < bytecode.len()
    );

    // Test LoadSub optimization: LoadI + SubI -> LoadSub
    let bytecode2 = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_sub_i(2, 1, 0), // SubI r2, r1, r0
    ];

    let (optimized2, _) = optimize_peephole(bytecode2.clone(), constants.clone());

    println!("\nOriginal bytecode2: {:?}", bytecode2);
    println!("Optimized bytecode2: {:?}", optimized2);
    println!(
        "Optimization successful: {}",
        optimized2.len() < bytecode2.len()
    );

    // Test LoadMul optimization: LoadI + MulI -> LoadMul
    let bytecode3 = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_mul_i(2, 1, 0), // MulI r2, r1, r0
    ];

    let (optimized3, _) = optimize_peephole(bytecode3.clone(), constants.clone());

    println!("\nOriginal bytecode3: {:?}", bytecode3);
    println!("Optimized bytecode3: {:?}", optimized3);
    println!(
        "Optimization successful: {}",
        optimized3.len() < bytecode3.len()
    );

    // Test LoadInc optimization: LoadI + Inc -> LoadInc
    let bytecode4 = vec![
        encode_load_i(1, 0), // LoadI r1, 0
        encode_inc(1),       // Inc r1
    ];

    let (optimized4, _) = optimize_peephole(bytecode4.clone(), constants.clone());

    println!("\nOriginal bytecode4: {:?}", bytecode4);
    println!("Optimized bytecode4: {:?}", optimized4);
    println!(
        "Optimization successful: {}",
        optimized4.len() < bytecode4.len()
    );

    // Test LoadDec optimization: LoadI + Dec -> LoadDec
    let bytecode5 = vec![
        encode_load_i(1, 0), // LoadI r1, 0
        encode_dec(1),       // Dec r1
    ];

    let (optimized5, _) = optimize_peephole(bytecode5.clone(), constants.clone());

    println!("\nOriginal bytecode5: {:?}", bytecode5);
    println!("Optimized bytecode5: {:?}", optimized5);
    println!(
        "Optimization successful: {}",
        optimized5.len() < bytecode5.len()
    );

    // Test LoadCmpEq optimization: LoadI + Eq -> LoadCmpEq
    let bytecode6 = vec![
        encode_load_i(1, 0), // LoadI r1, 0
        encode_eq(2, 1, 0),  // Eq r2, r1, r0
    ];

    let (optimized6, _) = optimize_peephole(bytecode6.clone(), constants.clone());

    println!("\nOriginal bytecode6: {:?}", bytecode6);
    println!("Optimized bytecode6: {:?}", optimized6);
    println!(
        "Optimization successful: {}",
        optimized6.len() < bytecode6.len()
    );

    // Test LoadCmpLt optimization: LoadI + Lt -> LoadCmpLt
    let bytecode7 = vec![
        encode_load_i(1, 0), // LoadI r1, 0
        encode_lt(2, 1, 0),  // Lt r2, r1, r0
    ];

    let (optimized7, _) = optimize_peephole(bytecode7.clone(), constants.clone());

    println!("\nOriginal bytecode7: {:?}", bytecode7);
    println!("Optimized bytecode7: {:?}", optimized7);
    println!(
        "Optimization successful: {}",
        optimized7.len() < bytecode7.len()
    );

    // Test LoadJfalse optimization: LoadI + JmpFalse -> LoadJfalse
    let bytecode8 = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_jmpfalse(1, 1), // JmpFalse r1, 1
    ];

    let (optimized8, _) = optimize_peephole(bytecode8.clone(), constants.clone());

    println!("\nOriginal bytecode8: {:?}", bytecode8);
    println!("Optimized bytecode8: {:?}", optimized8);
    println!(
        "Optimization successful: {}",
        optimized8.len() < bytecode8.len()
    );

    // Test LoadCmpEqJfalse optimization: LoadI + Eq + JmpFalse -> LoadCmpEqJfalse
    let bytecode9 = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_eq(2, 1, 0),    // Eq r2, r1, r0
        encode_jmpfalse(2, 1), // JmpFalse r2, 1
    ];

    let (optimized9, _) = optimize_peephole(bytecode9.clone(), constants.clone());

    println!("\nOriginal bytecode9: {:?}", bytecode9);
    println!("Optimized bytecode9: {:?}", optimized9);
    println!(
        "Optimization successful: {}",
        optimized9.len() < bytecode9.len()
    );

    // Test LoadCmpLtJfalse optimization: LoadI + Lt + JmpFalse -> LoadCmpLtJfalse
    let bytecode10 = vec![
        encode_load_i(1, 0),   // LoadI r1, 0
        encode_lt(2, 1, 0),    // Lt r2, r1, r0
        encode_jmpfalse(2, 1), // JmpFalse r2, 1
    ];

    let (optimized10, _) = optimize_peephole(bytecode10.clone(), constants.clone());

    println!("\nOriginal bytecode10: {:?}", bytecode10);
    println!("Optimized bytecode10: {:?}", optimized10);
    println!(
        "Optimization successful: {}",
        optimized10.len() < bytecode10.len()
    );
}

fn encode_load_i(dst: u8, value: i16) -> u32 {
    // LoadI format: opcode=6, a=dst, sbx=value
    ((value as u16 as u32) << 16) | ((dst as u32) << 8) | 6
}

fn encode_add_i(dst: u8, src1: u8, src2: u8) -> u32 {
    // AddI format: opcode=36, a=dst, b=src1, c=src2
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 36
}

fn encode_sub_i(dst: u8, src1: u8, src2: u8) -> u32 {
    // SubI format: opcode=37, a=dst, b=src1, c=src2
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 37
}

fn encode_mul_i(dst: u8, src1: u8, src2: u8) -> u32 {
    // MulI format: opcode=38, a=dst, b=src1, c=src2
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 38
}

fn encode_inc(dst: u8) -> u32 {
    // Inc format: opcode=42, a=dst
    ((dst as u32) << 8) | 42
}

fn encode_dec(dst: u8) -> u32 {
    // Dec format: opcode=43, a=dst
    ((dst as u32) << 8) | 43
}

fn encode_eq(dst: u8, src1: u8, src2: u8) -> u32 {
    // Eq format: opcode=15, a=dst, b=src1, c=src2
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 15
}

fn encode_lt(dst: u8, src1: u8, src2: u8) -> u32 {
    // Lt format: opcode=16, a=dst, b=src1, c=src2
    ((src2 as u32) << 24) | ((src1 as u32) << 16) | ((dst as u32) << 8) | 16
}

fn encode_jmpfalse(reg: u8, offset: i16) -> u32 {
    // JmpFalse format: opcode=8, a=reg, sbx=offset
    ((offset as u16 as u32) << 16) | ((reg as u32) << 8) | 8
}
