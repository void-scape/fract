use crate::{
    HEIGHT, MANDELBROT_XRANGE, MANDELBROT_YRANGE, PRECISION, WIDTH,
    series_approximation_coefficients,
};
use rayon::prelude::*;
use rug::{Assign, Float, ops::CompleteRound};
use tint::{Color, Srgb};

// Implementation derived from:
// - https://en.wikipedia.org/wiki/Mandelbrot_set#Computer_drawings.
// - https://dirkwhoffmann.github.io/DeepDrill/docs/Theory/Perturbation.html
// - https://fractalforums.org/index.php?topic=4360.0
pub fn compute_mandelbrot(
    pipeline: &mut Pipeline,
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    if *zoom == pipeline.zoom && *x == pipeline.x && *y == pipeline.y {
        frame_buffer.copy_from_slice(&pipeline.buffer);
        return;
    }

    pipeline.zoom.assign(zoom);
    pipeline.x.assign(x);
    pipeline.y.assign(y);

    // NOTE: When the zoom level is high, the perturbation algorithm I have
    // implemented becomes unstable, so it falls back to the original direct
    // implementation.
    if *zoom < 0.001 {
        mandelbrot_perturbation(
            &mut pipeline.orbit,
            &mut pipeline.buffer,
            max_iteration,
            zoom,
            x,
            y,
        );
    } else {
        mandelbrot(&mut pipeline.buffer, max_iteration, zoom, x, y);
    }
    frame_buffer.copy_from_slice(&pipeline.buffer);
}

pub struct Pipeline {
    buffer: Vec<Srgb>,
    orbit: Vec<(f64, f64)>,
    zoom: Float,
    x: Float,
    y: Float,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            buffer: vec![Srgb::default(); WIDTH * HEIGHT],
            orbit: Vec::new(),
            zoom: Float::with_val(PRECISION, 0.0),
            x: Float::with_val(PRECISION, 0.0),
            y: Float::with_val(PRECISION, 0.0),
        }
    }
}

fn mandelbrot_perturbation(
    orbit: &mut Vec<(f64, f64)>,
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    let cx = x;
    let cy = y;

    let w = WIDTH as f64;
    let h = HEIGHT as f64;

    let scanline_width = WIDTH;
    let scanline_height = HEIGHT / 80;
    let scanline_len = scanline_width * scanline_height;
    assert!(frame_buffer.len().is_multiple_of(scanline_len));

    // Perturbation reference orbit.
    orbit.clear();
    let x0 = cx;
    let y0 = cy;
    let mut x = Float::with_val(PRECISION, 0.0);
    let mut y = Float::with_val(PRECISION, 0.0);
    let mut x2 = Float::with_val(PRECISION, 0.0);
    let mut y2 = Float::with_val(PRECISION, 0.0);
    let mut xy = Float::with_val(PRECISION, 0.0);
    for _ in 0..max_iteration {
        orbit.push((x.to_f64(), y.to_f64()));
        x2.assign(&x * &x);
        y2.assign(&y * &y);
        if (&x2 + &y2).complete(PRECISION) > 4.0 {
            break;
        }
        xy.assign(&x * &y);
        y.assign(&xy * 2.0);
        y += y0;
        x.assign(&x2 - &y2);
        x += x0;
    }

    let xstep = (Float::with_val(PRECISION, MANDELBROT_XRANGE) * zoom / w).to_f64();
    let ystep = (Float::with_val(PRECISION, MANDELBROT_YRANGE) * zoom / h).to_f64();
    let sdx = (Float::with_val(PRECISION, -2.00) * zoom).to_f64();
    let sdy = (Float::with_val(PRECISION, -1.12) * zoom).to_f64();
    let (a, b, c, approx_iteration) =
        series_approximation_coefficients(orbit, sdx, sdy, xstep, ystep);

    // println!("x: {}", cx.to_string_radix(10, Some(50)));
    // println!("y: {}", cy.to_string_radix(10, Some(50)));
    // println!("z: {}", zoom.to_string_radix(10, Some(50)));
    // println!("i: {}", max_iteration);
    //
    // println!("a: {}", a);
    // println!("b: {}", b);
    // println!("c: {}", c);
    //
    // println!("approx: {}", approx_iteration);
    // println!("orbit len: {}", orbit.len());
    // println!();

    frame_buffer
        .par_chunks_mut(HEIGHT)
        .enumerate()
        .for_each(|(py, scanline_buffer)| {
            let dy0 = sdy + py as f64 * ystep;
            scanline_buffer
                .par_iter_mut()
                .enumerate()
                .for_each(|(px, pixel)| {
                    let dx0 = sdx + (px as f64) * xstep;

                    // Compute the delta of (x0, y0) with respect to the
                    // reference orbit.
                    let mut dx = dx0;
                    let mut dy = dy0;
                    let mut iteration = 0;
                    let mut ref_iteration = 0;

                    // If there are coefficients present, approximate the position
                    // of (dx, dy) at iteration `approx_iteration`.
                    if approx_iteration > 0 {
                        // D = Ad = Bd^2 + Cd^3
                        let d = num::Complex::new(dx0, dy0);
                        let d2 = d * d;
                        let d3 = d2 * d;
                        let dd = a * d + b * d2 + c * d3;
                        dx = dd.re;
                        dy = dd.im;
                        iteration = approx_iteration;
                        ref_iteration = approx_iteration;
                    }

                    while iteration < max_iteration {
                        let (mut ax, mut ay) = orbit[ref_iteration];
                        ax *= 2.0;
                        ay *= 2.0;

                        // ad = a * d
                        let adx = ax * dx - ay * dy;
                        let ady = ax * dy + ay * dx;

                        // a = a * d + d * d
                        ax = adx + dx * dx - dy * dy;
                        ay = ady + dx * dy + dy * dx;

                        // d = a * d + d * d + d0
                        dx = ax + dx0;
                        dy = ay + dy0;

                        ref_iteration += 1;

                        // The full value of (x0, y0) is reconstructed from
                        // the reference orbit and checked for escape time.
                        let (x, y) = orbit[ref_iteration];
                        let zmag = (dx + x) * (dx + x) + (dy + y) * (dy + y);
                        let dmag = dx * dx + dy * dy;

                        if zmag > 10000.0 {
                            break;
                        } else if zmag < dmag || ref_iteration == orbit.len() - 1 {
                            dx += x;
                            dy += y;
                            ref_iteration = 0;
                        }

                        iteration += 1;
                    }

                    let (x, y) = orbit[ref_iteration];
                    *pixel = iteration_to_srgb(iteration, x + dx, y + dy, max_iteration);
                });
        });
}

fn mandelbrot(frame_buffer: &mut [Srgb], max_iteration: usize, zoom: &Float, x: &Float, y: &Float) {
    // println!("x: {}", x.to_string_radix(10, Some(50)));
    // println!("y: {}", y.to_string_radix(10, Some(50)));
    // println!("z: {}", zoom.to_string_radix(10, Some(50)));
    // println!("i: {}", max_iteration);
    // println!();

    let zoom = zoom.to_f64();
    let cx = x.to_f64();
    let cy = y.to_f64();
    let w = WIDTH as f64;
    let h = HEIGHT as f64;

    frame_buffer
        .par_chunks_mut(HEIGHT)
        .enumerate()
        .for_each(|(py, scanline_buffer)| {
            let y0 = ((py as f64) / h * MANDELBROT_YRANGE - 1.12) * zoom + cy;
            scanline_buffer
                .par_iter_mut()
                .enumerate()
                .for_each(|(px, pixel)| {
                    let x0 = ((px as f64) / w * MANDELBROT_XRANGE - 2.00) * zoom + cx;
                    let mut x = 0f64;
                    let mut y = 0f64;
                    let mut iteration = 0;
                    while iteration < max_iteration {
                        use std::intrinsics::*;
                        let x2 = unsafe { fmul_fast(x, x) };
                        let y2 = unsafe { fmul_fast(y, y) };
                        if unsafe { fadd_fast(x2, y2) } > 10000.0 {
                            break;
                        }
                        y = unsafe { fadd_fast(fmul_fast(fmul_fast(2.0, x), y), y0) };
                        x = unsafe { fadd_fast(fsub_fast(x2, y2), x0) };
                        iteration += 1;
                    }
                    *pixel = iteration_to_srgb(iteration, x, y, max_iteration);
                });
        });
}

// https://stackoverflow.com/a/16505538
const MAPPING: [Srgb; 16] = [
    Srgb::from_rgb(66, 30, 15),
    Srgb::from_rgb(25, 7, 26),
    Srgb::from_rgb(9, 1, 47),
    Srgb::from_rgb(4, 4, 73),
    Srgb::from_rgb(0, 7, 100),
    Srgb::from_rgb(12, 44, 138),
    Srgb::from_rgb(24, 82, 177),
    Srgb::from_rgb(57, 125, 209),
    Srgb::from_rgb(134, 181, 229),
    Srgb::from_rgb(211, 236, 248),
    Srgb::from_rgb(241, 233, 191),
    Srgb::from_rgb(248, 201, 95),
    Srgb::from_rgb(255, 170, 0),
    Srgb::from_rgb(204, 128, 0),
    Srgb::from_rgb(153, 87, 0),
    Srgb::from_rgb(106, 52, 3),
];

fn iteration_to_srgb(iteration: usize, x: f64, y: f64, max_iteration: usize) -> Srgb {
    if iteration == max_iteration {
        return Srgb::from_rgb(0, 0, 0);
    }

    // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring
    let zn = x * x + y * y;
    let nu = (zn.ln() * 0.5).ln() / std::f64::consts::LN_2;
    let iter = iteration as f64 + 1.0 - nu;
    let index = iter % 16.0;
    let c1 = index.floor() as usize;
    let c2 = (c1 + 1) % 16;
    let t = index.fract() as f32;

    let color1 = MAPPING[c1];
    let color2 = MAPPING[c2];
    (color1.to_linear() * (1.0 - t) + color2.to_linear() * t).to_srgb()
}
