use vm::asm::disassemble_clean;
use vm::mandelbrot::{
    build_escape_iterations_program, reference_escape_iterations, run_escape_iterations,
};

#[test]
fn mandelbrot_small_points_match_reference() {
    let cases = [
        (0.0, 0.0, 8u16),
        (1.0, 0.0, 8u16),
        (-1.0, 0.0, 8u16),
        (-0.75, 0.1, 16u16),
    ];

    for (cx, cy, max_iter) in cases {
        let expected = reference_escape_iterations(cx, cy, max_iter);
        let actual = run_escape_iterations(cx, cy, max_iter);
        assert_eq!(
            actual, expected,
            "VM Mandelbrot result mismatch for c=({cx}, {cy}), max_iter={max_iter}"
        );
    }
}

#[test]
fn mandelbrot_program_disassembles_to_expected_shape() {
    let (bytecode, constants) = build_escape_iterations_program(1.0, 0.0, 8);
    let asm = disassemble_clean(&bytecode, &constants);

    assert_eq!(asm.len(), 44);
    assert_eq!(asm[0], "load_k r1, const[0]");
    assert_eq!(asm[1], "load_k r2, const[1]");
    assert!(asm.iter().any(|line| line == "mul_acc r3"));
    assert!(asm.iter().any(|line| line == "mul_acc r4"));
    assert!(asm.iter().any(|line| line == "sub_acc r8"));
    assert_eq!(asm[41], "jmp -> L-31");
    assert_eq!(asm[43], "ret");
}
