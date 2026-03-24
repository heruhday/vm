use vm::mandelbrot::{reference_escape_grid, run_escape_grid};

fn main() {
    println!("=== Mandelbrot Medium Test ===\n");

    let x_min = -2.0;
    let x_max = 0.5;
    let y_min = -1.0;
    let y_max = 1.0;
    let width = 8usize;
    let height = 6usize;
    let max_iter = 24u16;

    println!(
        "Grid: {width}x{height}, x=[{x_min}, {x_max}], y=[{y_min}, {y_max}], max_iter={max_iter}"
    );

    let actual = run_escape_grid(x_min, x_max, y_min, y_max, width, height, max_iter);
    let expected = reference_escape_grid(x_min, x_max, y_min, y_max, width, height, max_iter);

    println!("\n=== Iteration Grid ===");
    for row in &actual {
        let line = row
            .iter()
            .map(|value| format!("{value:>2}"))
            .collect::<Vec<_>>()
            .join(" ");
        println!("{line}");
    }

    let checksum: u32 = actual.iter().flatten().map(|&value| value as u32).sum();
    println!("\nChecksum: {checksum}");

    assert_eq!(actual, expected, "medium grid mismatch");
    println!("All medium-grid values matched the Rust reference.");
}
