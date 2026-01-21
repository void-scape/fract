#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

#[cfg(feature = "compute")]
pub mod compute;

use rast::tint::{Color, Srgb};

pub const WIDTH: usize = 1600;
pub const HEIGHT: usize = 1600;

pub const MANDELBROT_XRANGE: f64 = 2.00 + 0.47;
pub const MANDELBROT_YRANGE: f64 = 1.12 + 1.12;

// Implementation derived from: https://en.wikipedia.org/wiki/Mandelbrot_set#Computer_drawings.
//
// NOTE: There is no difference in execution time between f64 and f32 on my machine.
pub fn compute_mandelbrot(
    frame_buffer: &mut [Srgb],
    max_iteration: usize,
    zoom: f64,
    x: f64,
    y: f64,
) {
    let cx = x;
    let cy = y;

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
