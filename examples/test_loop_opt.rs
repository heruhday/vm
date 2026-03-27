use vm::emit::BytecodeBuilder;
use vm::js_value::make_number;
use vm::opt::optimize_bytecode;

fn main() {
    // Create a simple bytecode that can be optimized
    let mut builder = BytecodeBuilder::new();
    builder.emit_load_i(1, 10);
    builder.emit_load_i(2, 20);
    builder.emit_add(1, 2);
    builder.emit_mov(3, 255);
    builder.emit_load_i(4, 30);
    builder.emit_add(3, 4);
    builder.emit_jmp(-6);
    let (bytecode, _) = builder.build();

    let constants = vec![make_number(1.0), make_number(2.0), make_number(3.0)];

    println!("Original bytecode length: {}", bytecode.len());
    println!("Original constants length: {}", constants.len());

    // Run optimization
    let (optimized_bytecode, optimized_constants) = optimize_bytecode(bytecode, constants);

    println!("Optimized bytecode length: {}", optimized_bytecode.len());
    println!("Optimized constants length: {}", optimized_constants.len());

    // Show some statistics
    println!("\nOptimization completed successfully!");
    println!("The optimizer will run up to 10 iterations or until no changes occur.");
    println!("This allows multiple optimization passes to work together:");
    println!("1. Simplify branches (jump threading)");
    println!("2. Copy propagation");
    println!("3. Peephole optimization (including superinstructions 200-255)");
    println!("4. Register coalescing");
    println!("5. Copy propagation (again)");
    println!("6. Fold temporary checks (isUndef/isNull to constants)");
    println!("7. Eliminate dead code");
    println!("8. Simplify branches (again)");

    println!("\nThe loop optimization ensures that:");
    println!(
        "- Optimizations can build on each other (e.g., dead code elimination creates opportunities for more copy propagation)"
    );
    println!("- The process continues until a fixed point is reached (no more changes)");
    println!("- A maximum of 10 iterations prevents infinite loops in case of bugs");
}
