use vm::js_value::make_number;
use vm::opt::optimize_peephole;

fn main() {
    // Test LoadAdd optimization: LoadI + AddI -> LoadAdd
    // LoadI r1, 42 (opcode 6, a=1, value=42)
    // AddI r2, r1, r0 (opcode 36, a=2, b=1, c=0)
    let bytecode = vec![
        encode_load_i(1, 42),  // LoadI r1, 42
        encode_add_i(2, 1, 0), // AddI r2, r1, r0
    ];
    let constants = vec![make_number(0.0), make_number(1.0)];

    println!("Testing LoadAdd optimization (LoadI + AddI -> LoadAdd)");
    println!("Original bytecode: {:?}", bytecode);

    let (optimized, _) = optimize_peephole(bytecode.clone(), constants.clone());

    println!("Optimized bytecode: {:?}", optimized);
    println!(
        "Original length: {}, Optimized length: {}",
        bytecode.len(),
        optimized.len()
    );

    // Check if optimization happened
    if optimized.len() < bytecode.len() {
        println!(
            "✓ Optimization successful! Reduced from {} to {} instructions",
            bytecode.len(),
            optimized.len()
        );

        // Check if the optimized instruction is LoadAdd (opcode 176)
        if let Some(&first_insn) = optimized.first() {
            let opcode = first_insn & 0xFF;
            if opcode == 176 {
                // LoadAdd opcode
                println!("✓ Optimized to LoadAdd instruction");
            } else {
                println!("✗ Wrong opcode: {} (expected 176 for LoadAdd)", opcode);
            }
        }
    } else {
        println!("✗ No optimization occurred");
    }

    println!("\n--- Testing other patterns ---");

    // Test LoadSub optimization: LoadI + SubI -> LoadSub
    let bytecode2 = vec![encode_load_i(1, 42), encode_sub_i(2, 1, 0)];

    let (optimized2, _) = optimize_peephole(bytecode2.clone(), constants.clone());
    println!(
        "\nLoadSub test: {} -> {} instructions",
        bytecode2.len(),
        optimized2.len()
    );

    // Test LoadMul optimization: LoadI + MulI -> LoadMul
    let bytecode3 = vec![encode_load_i(1, 42), encode_mul_i(2, 1, 0)];

    let (optimized3, _) = optimize_peephole(bytecode3.clone(), constants.clone());
    println!(
        "LoadMul test: {} -> {} instructions",
        bytecode3.len(),
        optimized3.len()
    );

    // Test LoadInc optimization: LoadI + Inc -> LoadInc
    let bytecode4 = vec![
        encode_load_i(1, 42),
        encode_inc(1), // Inc r1
    ];

    let (optimized4, _) = optimize_peephole(bytecode4.clone(), constants.clone());
    println!(
        "LoadInc test: {} -> {} instructions",
        bytecode4.len(),
        optimized4.len()
    );

    // Test LoadDec optimization: LoadI + Dec -> LoadDec
    let bytecode5 = vec![
        encode_load_i(1, 42),
        encode_dec(1), // Dec r1
    ];

    let (optimized5, _) = optimize_peephole(bytecode5.clone(), constants.clone());
    println!(
        "LoadDec test: {} -> {} instructions",
        bytecode5.len(),
        optimized5.len()
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
