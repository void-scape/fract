use crate::{PRECISION, series_approximation_coefficients};
use rayon::prelude::*;
use rug::{Assign, Float, ops::CompleteRound};
use tint::{Color, LinearRgb, Sbgr};

// Implementation derived from:
// - https://en.wikipedia.org/wiki/Mandelbrot_set#Computer_drawings.
// - https://dirkwhoffmann.github.io/DeepDrill/docs/Theory/Perturbation.html
// - https://fractalforums.org/index.php?topic=4360.0
pub fn compute_mandelbrot(
    pipeline: &mut Pipeline,
    frame_buffer: &mut [Sbgr],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
    palette: &[Sbgr],
    width: usize,
    height: usize,
) {
    if let Some(buffered) = pipeline.buffered.as_mut() {
        if buffered.buffer.is_empty() {
            for _ in 0..width * height {
                buffered.buffer.push(Sbgr::default());
            }
        }

        if *zoom == buffered.zoom && *x == buffered.x && *y == buffered.y {
            frame_buffer.copy_from_slice(&buffered.buffer);
            return;
        }

        buffered.zoom.assign(zoom);
        buffered.x.assign(x);
        buffered.y.assign(y);

        // NOTE: When the zoom level is high, the perturbation algorithm I have
        // implemented becomes unstable, so it falls back to the original direct
        // implementation.
        if *zoom < 0.001 {
            mandelbrot_perturbation(
                &mut pipeline.orbit,
                &mut buffered.buffer,
                max_iteration,
                zoom,
                x,
                y,
                palette,
                width,
                height,
                pipeline.super_sampled,
            );
        } else {
            mandelbrot(
                &mut buffered.buffer,
                max_iteration,
                zoom,
                x,
                y,
                palette,
                width,
                height,
            );
        }

        frame_buffer.copy_from_slice(&buffered.buffer);
    } else {
        // NOTE: When the zoom level is high, the perturbation algorithm I have
        // implemented becomes unstable, so it falls back to the original direct
        // implementation.
        if *zoom < 0.001 {
            mandelbrot_perturbation(
                &mut pipeline.orbit,
                frame_buffer,
                max_iteration,
                zoom,
                x,
                y,
                palette,
                width,
                height,
                pipeline.super_sampled,
            );
        } else {
            mandelbrot(
                frame_buffer,
                max_iteration,
                zoom,
                x,
                y,
                palette,
                width,
                height,
            );
        }
    }
}

pub struct Pipeline {
    orbit: Vec<(f64, f64)>,
    buffered: Option<Buffered>,
    super_sampled: bool,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self {
            orbit: Vec::new(),
            buffered: Some(Buffered::default()),
            super_sampled: false,
        }
    }
}

impl Pipeline {
    pub fn unbuffered() -> Self {
        Self {
            orbit: Vec::new(),
            buffered: None,
            super_sampled: false,
        }
    }

    pub fn super_sampled(mut self) -> Self {
        self.super_sampled = true;
        self
    }
}

struct Buffered {
    buffer: Vec<Sbgr>,
    zoom: Float,
    x: Float,
    y: Float,
}

impl Default for Buffered {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            zoom: Float::with_val(PRECISION, 0.0),
            x: Float::with_val(PRECISION, 0.0),
            y: Float::with_val(PRECISION, 0.0),
        }
    }
}

fn mandelbrot_perturbation(
    orbit: &mut Vec<(f64, f64)>,
    frame_buffer: &mut [Sbgr],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
    palette: &[Sbgr],
    width: usize,
    height: usize,
    super_sampled: bool,
) {
    let cx = x;
    let cy = y;

    let w = width as f64;
    let h = height as f64;

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

    let aspect = w / h;
    let z = zoom.to_f64();
    let xstep = 2.0 * z * aspect / w;
    let ystep = 2.0 * z / h;
    let sdx = -z * aspect;
    let sdy = -z;

    let (a, b, c, approx_iteration) =
        series_approximation_coefficients(orbit, sdx, sdy, xstep, ystep);

    if super_sampled {
        frame_buffer
            .par_chunks_mut(height)
            .enumerate()
            .for_each(|(py, scanline_buffer)| {
                scanline_buffer
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(px, pixel)| {
                        let px = px as f64;
                        let py = py as f64;

                        let mut p1 = LinearRgb::default();
                        let mut p2 = LinearRgb::default();
                        let mut p3 = LinearRgb::default();
                        let mut p4 = LinearRgb::default();

                        for (pixel, px, py) in [
                            (&mut p1, px + 0.25, py + 0.25),
                            (&mut p2, px + 0.75, py + 0.25),
                            (&mut p3, px + 0.25, py + 0.75),
                            (&mut p4, px + 0.75, py + 0.75),
                        ] {
                            let dx0 = sdx + px * xstep;
                            let dy0 = sdy + py * ystep;

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
                            *pixel = iteration_to_srgb(
                                iteration,
                                x + dx,
                                y + dy,
                                max_iteration,
                                palette,
                            )
                            .to_linear();
                        }

                        let [r1, g1, b1, _] = p1.to_array();
                        let [r2, g2, b2, _] = p2.to_array();
                        let [r3, g3, b3, _] = p3.to_array();
                        let [r4, g4, b4, _] = p4.to_array();
                        *pixel = LinearRgb::from_rgb(
                            (r1 + r2 + r3 + r4) / 4.0,
                            (g1 + g2 + g3 + g4) / 4.0,
                            (b1 + b2 + b3 + b4) / 4.0,
                        )
                        .to_sbgr();
                    });
            });
    } else {
        frame_buffer
            .par_chunks_mut(height)
            .enumerate()
            .for_each(|(py, scanline_buffer)| {
                let dy0 = sdy + py as f64 * ystep;
                scanline_buffer
                    .par_iter_mut()
                    .enumerate()
                    .for_each(|(px, pixel)| {
                        let dx0 = sdx + px as f64 * xstep;

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
                        *pixel =
                            iteration_to_srgb(iteration, x + dx, y + dy, max_iteration, palette);
                    });
            });
    }
}

fn mandelbrot(
    frame_buffer: &mut [Sbgr],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
    palette: &[Sbgr],
    width: usize,
    height: usize,
) {
    let zoom = zoom.to_f64();
    let cx = x.to_f64();
    let cy = y.to_f64();
    let w = width as f64;
    let h = height as f64;
    let aspect = w / h;

    frame_buffer
        .par_chunks_mut(height)
        .enumerate()
        .for_each(|(py, scanline_buffer)| {
            let y0 = ((py as f64) / h * 2.0 - 1.0) * zoom + cy;
            scanline_buffer
                .par_iter_mut()
                .enumerate()
                .for_each(|(px, pixel)| {
                    let x0 = ((px as f64) / w * 2.0 - 1.0) * zoom * aspect + cx;
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
                    *pixel = iteration_to_srgb(iteration, x, y, max_iteration, palette);
                });
        });
}

fn iteration_to_srgb(
    iteration: usize,
    x: f64,
    y: f64,
    max_iteration: usize,
    palette: &[Sbgr],
) -> Sbgr {
    if iteration == max_iteration {
        return Sbgr::from_rgb(0, 0, 0);
    }

    // https://en.wikipedia.org/wiki/Plotting_algorithms_for_the_Mandelbrot_set#Continuous_(smooth)_coloring
    let zn = x * x + y * y;
    let nu = (zn.ln() * 0.5).ln() / std::f64::consts::LN_2;
    let iter = iteration as f64 + 1.0 - nu;
    let index = iter % palette.len() as f64;
    let c1 = index.floor() as usize;
    let c2 = (c1 + 1) % palette.len();
    let t = index.fract() as f32;

    let color1 = palette[c1];
    let color2 = palette[c2];
    (color1.to_linear() * (1.0 - t) + color2.to_linear() * t).to_sbgr()
}
