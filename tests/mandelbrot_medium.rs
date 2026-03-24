use vm::mandelbrot::{reference_escape_grid, run_escape_grid};

#[test]
fn mandelbrot_medium_grid_matches_reference() {
    let x_min = -2.0;
    let x_max = 0.5;
    let y_min = -1.0;
    let y_max = 1.0;
    let width = 8usize;
    let height = 6usize;
    let max_iter = 24u16;

    let actual = run_escape_grid(x_min, x_max, y_min, y_max, width, height, max_iter);
    let expected = reference_escape_grid(x_min, x_max, y_min, y_max, width, height, max_iter);

    assert_eq!(actual, expected);
    assert!(
        actual.iter().flatten().any(|&value| value == max_iter),
        "grid should include stable interior points"
    );
    assert!(
        actual.iter().flatten().any(|&value| value <= 3),
        "grid should include fast-escaping points"
    );
}
