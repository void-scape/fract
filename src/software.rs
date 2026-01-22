use crate::{HEIGHT, MANDELBROT_XRANGE, MANDELBROT_YRANGE, PRECISION, WIDTH};
use rast::tint::{Color, Srgb};
use rug::{
    Assign, Float,
    float::{self, FreeCache},
    ops::CompleteRound,
};

// Implementation derived from:
// - https://en.wikipedia.org/wiki/Mandelbrot_set#Computer_drawings.
// - https://dirkwhoffmann.github.io/DeepDrill/docs/Theory/Perturbation.html
// - https://fractalforums.org/index.php?topic=4360.0
pub fn compute_mandelbrot(
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    // NOTE: When the zoom level is high, the perturbation algorithm I have
    // implemented becomes unstable, so it falls back to the original direct
    // implementation.
    if *zoom < 0.001 {
        mandelbrot_perturbation(frame_buffer, max_iteration, zoom, x, y);
    } else {
        mandelbrot(frame_buffer, max_iteration, zoom, x, y);
    }
}

fn mandelbrot_perturbation(
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: &Float,
    x: &Float,
    y: &Float,
) {
    let zoom = zoom.to_f64();
    let cx = x;
    let cy = y;

    let w = WIDTH as f64;
    let h = HEIGHT as f64;

    let scanline_width = WIDTH;
    let scanline_height = HEIGHT / 360;
    let scanline_len = scanline_width * scanline_height;
    assert!(frame_buffer.len().is_multiple_of(scanline_len));

    // Perturbation refernce orbit.
    let mut orbit = Vec::with_capacity(max_iteration);

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
    float::free_cache(FreeCache::All);

    // NOTE: Threading scanlines can be around 4-5x faster.
    //
    // With a scanline_height of 800 / 80, 10 threads are spawned. More or less
    // threads reduces performance.
    let orbit = &orbit;
    std::thread::scope(|s| {
        for i in 0..frame_buffer.len() / scanline_len {
            let frame_buffer = unsafe {
                let ptr = frame_buffer.as_mut_ptr().add(i * scanline_len);
                std::slice::from_raw_parts_mut(ptr, scanline_len)
            };

            let yoffset = (i * scanline_height) as f64;
            s.spawn(move || {
                for py in 0..scanline_height {
                    let dy0 = ((yoffset + py as f64) / h * MANDELBROT_YRANGE - 1.12) * zoom;

                    for px in 0..scanline_width {
                        let dx0 = ((px as f64) / w * MANDELBROT_XRANGE - 2.00) * zoom;

                        // Compute the delta of (x0, y0) with respect to the
                        // reference orbit.
                        let mut dx = dx0;
                        let mut dy = dy0;
                        let mut iteration = 0;
                        let mut ref_iteration = 0;

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

                            if zmag > 4.0 {
                                break;
                            } else if zmag < dmag || ref_iteration == orbit.len() - 1 {
                                dx += x;
                                dy += y;
                                ref_iteration = 0;
                            }

                            iteration += 1;
                        }

                        let l = iteration as f32 / max_iteration as f32;
                        frame_buffer[py * WIDTH + px] =
                            rast::tint::Hsv::from_hsv(l, 1.0, 1.0).to_srgb();
                    }
                }
            });
        }
    });
}

fn mandelbrot(frame_buffer: &mut [Srgb], max_iteration: usize, zoom: &Float, x: &Float, y: &Float) {
    let zoom = zoom.to_f64();
    let cx = x.to_f64();
    let cy = y.to_f64();

    let w = WIDTH as f64;
    let h = HEIGHT as f64;

    let scanline_width = WIDTH;
    let scanline_height = HEIGHT / 360;
    let scanline_len = scanline_width * scanline_height;
    assert!(frame_buffer.len().is_multiple_of(scanline_len));

    // NOTE: Threading scanlines can be around 4-5x faster.
    //
    // With a scanline_height of 800 / 80, 10 threads are spawned. More or less
    // threads reduces performance.
    std::thread::scope(|s| {
        for i in 0..frame_buffer.len() / scanline_len {
            let frame_buffer = unsafe {
                let ptr = frame_buffer.as_mut_ptr().add(i * scanline_len);
                std::slice::from_raw_parts_mut(ptr, scanline_len)
            };

            let yoffset = (i * scanline_height) as f64;
            s.spawn(move || {
                for py in 0..scanline_height {
                    let y0 = ((yoffset + py as f64) / h * MANDELBROT_YRANGE - 1.12) * zoom + cy;

                    for px in 0..scanline_width {
                        let x0 = ((px as f64) / w * MANDELBROT_XRANGE - 2.00) * zoom + cx;

                        let mut x = 0f64;
                        let mut y = 0f64;
                        let mut iteration = 0;

                        while iteration < max_iteration {
                            // NOTE: Intrinsics saves ~5% execution time.
                            use std::intrinsics::*;
                            let x2 = unsafe { fmul_fast(x, x) };
                            let y2 = unsafe { fmul_fast(y, y) };

                            if unsafe { fadd_fast(x2, y2) } > 4.0 {
                                break;
                            }

                            y = unsafe { fadd_fast(fmul_fast(fmul_fast(2.0, x), y), y0) };
                            x = unsafe { fadd_fast(fsub_fast(x2, y2), x0) };
                            iteration += 1;
                        }

                        let l = iteration as f32 / max_iteration as f32;
                        frame_buffer[py * WIDTH + px] =
                            rast::tint::Hsv::from_hsv(l, 1.0, 1.0).to_srgb();
                    }
                }
            });
        }
    });
}
