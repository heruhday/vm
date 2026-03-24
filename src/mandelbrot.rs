use crate::emit::BytecodeBuilder;
use crate::js_value::{JSValue, make_number, to_f64};
use crate::vm::VM;

const ACC_REG: u8 = 255;
const ACC_SLOT: usize = ACC_REG as usize;

const CX_REG: u8 = 1;
const CY_REG: u8 = 2;
const ZR_REG: u8 = 3;
const ZI_REG: u8 = 4;
const ITER_REG: u8 = 5;
const MAX_ITER_REG: u8 = 6;
const ZR2_REG: u8 = 7;
const ZI2_REG: u8 = 8;
const ZR_ZI_REG: u8 = 9;
const MAG2_REG: u8 = 10;
const TWO_REG: u8 = 11;
const NEXT_ZR_REG: u8 = 12;
const NEXT_ZI_REG: u8 = 13;
const ESCAPE_RADIUS2_REG: u8 = 14;

/// Build a bytecode program that returns the Mandelbrot escape-iteration count for one point.
pub fn build_escape_iterations_program(
    cx: f64,
    cy: f64,
    max_iter: u16,
) -> (Vec<u32>, Vec<JSValue>) {
    assert!(max_iter <= i16::MAX as u16, "max_iter must fit in i16");

    let mut builder = BytecodeBuilder::new();

    let cx_const = builder.add_constant(make_number(cx));
    let cy_const = builder.add_constant(make_number(cy));
    let escape_radius2_const = builder.add_constant(make_number(4.0));

    builder.emit_load_k(CX_REG, cx_const);
    builder.emit_load_k(CY_REG, cy_const);

    builder.emit_load_0();
    builder.emit_mov(ZR_REG, ACC_REG);
    builder.emit_load_0();
    builder.emit_mov(ZI_REG, ACC_REG);
    builder.emit_load_0();
    builder.emit_mov(ITER_REG, ACC_REG);

    builder.emit_load_i(MAX_ITER_REG, max_iter as i16);
    builder.emit_load_i(TWO_REG, 2);
    builder.emit_load_k(ESCAPE_RADIUS2_REG, escape_radius2_const);

    let loop_start = builder.len();

    // Stop once iter == max_iter.
    builder.emit_lt(ITER_REG, MAX_ITER_REG);
    builder.emit_jmp_false(ACC_REG, 29);

    // Compute zr^2, zi^2, and |z|^2.
    builder.emit_mov(ACC_REG, ZR_REG);
    builder.emit_mul_acc(ZR_REG);
    builder.emit_mov(ZR2_REG, ACC_REG);

    builder.emit_mov(ACC_REG, ZI_REG);
    builder.emit_mul_acc(ZI_REG);
    builder.emit_mov(ZI2_REG, ACC_REG);

    builder.emit_add(ZR2_REG, ZI2_REG);
    builder.emit_mov(MAG2_REG, ACC_REG);

    // Stop once |z|^2 > 4.
    builder.emit_lte(MAG2_REG, ESCAPE_RADIUS2_REG);
    builder.emit_jmp_false(ACC_REG, 19);

    // next_zr = zr^2 - zi^2 + cx
    builder.emit_mov(ACC_REG, ZR_REG);
    builder.emit_mul_acc(ZI_REG);
    builder.emit_mov(ZR_ZI_REG, ACC_REG);

    builder.emit_mov(ACC_REG, ZR2_REG);
    builder.emit_sub_acc(ZI2_REG);
    builder.emit_mov(NEXT_ZR_REG, ACC_REG);
    builder.emit_add(NEXT_ZR_REG, CX_REG);
    builder.emit_mov(NEXT_ZR_REG, ACC_REG);

    // next_zi = 2 * zr * zi + cy
    builder.emit_mov(ACC_REG, ZR_ZI_REG);
    builder.emit_mul_acc(TWO_REG);
    builder.emit_mov(NEXT_ZI_REG, ACC_REG);
    builder.emit_add(NEXT_ZI_REG, CY_REG);
    builder.emit_mov(NEXT_ZI_REG, ACC_REG);

    builder.emit_mov(ZR_REG, NEXT_ZR_REG);
    builder.emit_mov(ZI_REG, NEXT_ZI_REG);

    builder.emit_mov(ACC_REG, ITER_REG);
    builder.emit_inc_acc();
    builder.emit_mov(ITER_REG, ACC_REG);

    builder.emit_jmp(-(builder.len() as i16 - loop_start as i16 + 1));

    debug_assert_eq!(builder.len() - loop_start, 31);

    builder.emit_mov(ACC_REG, ITER_REG);
    builder.emit_ret();

    builder.build()
}

/// Run the Mandelbrot escape-iteration program and return the integer result.
pub fn run_escape_iterations(cx: f64, cy: f64, max_iter: u16) -> u16 {
    let (bytecode, constants) = build_escape_iterations_program(cx, cy, max_iter);
    let mut vm = VM::new(bytecode, constants, vec![]);
    vm.run(false);

    to_f64(vm.frame.regs[ACC_SLOT])
        .map(|value| value as u16)
        .expect("mandelbrot program should return a numeric iteration count")
}

/// Reference Rust implementation using the same loop semantics as the VM program.
pub fn reference_escape_iterations(cx: f64, cy: f64, max_iter: u16) -> u16 {
    let mut zr = 0.0;
    let mut zi = 0.0;
    let mut iter = 0;

    while iter < max_iter && zr * zr + zi * zi <= 4.0 {
        let next_zr = zr * zr - zi * zi + cx;
        let next_zi = 2.0 * zr * zi + cy;
        zr = next_zr;
        zi = next_zi;
        iter += 1;
    }

    iter
}

fn sample_axis_value(start: f64, end: f64, count: usize, index: usize) -> f64 {
    assert!(count > 0, "sample count must be non-zero");
    assert!(index < count, "sample index out of bounds");

    if count == 1 {
        return start;
    }

    let t = index as f64 / (count - 1) as f64;
    start + (end - start) * t
}

/// Run the Mandelbrot program across a rectangular grid and return escape iterations per cell.
pub fn run_escape_grid(
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    width: usize,
    height: usize,
    max_iter: u16,
) -> Vec<Vec<u16>> {
    let mut rows = Vec::with_capacity(height);

    for y in 0..height {
        let cy = sample_axis_value(y_min, y_max, height, y);
        let mut row = Vec::with_capacity(width);

        for x in 0..width {
            let cx = sample_axis_value(x_min, x_max, width, x);
            row.push(run_escape_iterations(cx, cy, max_iter));
        }

        rows.push(row);
    }

    rows
}

/// Reference Mandelbrot grid using the Rust implementation for each sampled cell.
pub fn reference_escape_grid(
    x_min: f64,
    x_max: f64,
    y_min: f64,
    y_max: f64,
    width: usize,
    height: usize,
    max_iter: u16,
) -> Vec<Vec<u16>> {
    let mut rows = Vec::with_capacity(height);

    for y in 0..height {
        let cy = sample_axis_value(y_min, y_max, height, y);
        let mut row = Vec::with_capacity(width);

        for x in 0..width {
            let cx = sample_axis_value(x_min, x_max, width, x);
            row.push(reference_escape_iterations(cx, cy, max_iter));
        }

        rows.push(row);
    }

    rows
}
