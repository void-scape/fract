#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

use crate::{encoder::Encoder, pipeline::Pipeline};
use indicatif::ProgressBar;
use rug::{Float, ops::CompleteRound};

mod compute;
pub mod config;
pub mod encoder;
mod orbit;
mod palette;
pub mod pipeline;
mod ssaa;
pub mod viewer;

/// Render a png to `output` with the given `pipeline`.
///
/// If `progress_bar` is supplied, the number of pixels processed will be displayed.
/// You do NOT need to specify the length.
pub fn render_png(
    pipeline: &mut Pipeline,
    progress_bar: Option<&ProgressBar>,
    output: &str,
    frame: usize,
) -> std::io::Result<()> {
    let pixels = pipeline.total_pixels() as u64;

    if let Some(bar) = progress_bar {
        bar.set_length(pixels);
        bar.set_position(0);
    }

    while !pipeline.finished() {
        let remaining = pipeline.step_mandelbrot_headless();
        if let Some(bar) = progress_bar {
            bar.set_position(pixels - remaining as u64);
        }
    }

    pipeline.render_output();
    if let Some(bar) = progress_bar {
        bar.finish();
    }

    pipeline.log(frame)?;
    let pixels = pipeline.read_output_buffer_bytes();
    let (w, h) = pipeline.dimensions();
    encoder::png(output, &pixels, w, h)
}

/// Render an mp4 to `output` with the given `pipeline`.
///
/// If `progress_bar` is supplied, the number of processed frames will be displayed.
/// You do NOT need to specify the length.
pub fn render_mp4(
    pipeline: &mut Pipeline,
    progress_bar: Option<&ProgressBar>,
    mut encoder: Encoder,
    frames: usize,
    mut zoom: impl FnMut(&mut Float),
    output: &str,
) -> std::io::Result<()> {
    if let Some(bar) = progress_bar {
        bar.set_length(frames as u64);
        bar.set_position(0);
    }

    for i in 0..frames {
        pipeline.write_position(|_, _, z| {
            zoom(z);
        });
        while !pipeline.finished() {
            pipeline.step_mandelbrot_headless();
        }
        pipeline.render_output();
        let pixels = pipeline.read_output_buffer_bytes();
        if let Some(bar) = progress_bar {
            bar.inc(1);
        }
        encoder.render_frame(&pixels)?;
        pipeline.log(i)?;
    }

    if let Some(bar) = progress_bar {
        bar.finish();
    }

    encoder.finish(output)
}

/// Parses `f` and estimates the required precision.
pub fn float_from_str(f: &str) -> Float {
    let mut digits = f
        .chars()
        .take_while(|c| *c != 'e')
        .filter(|c| c.is_ascii_digit())
        .count() as u32;

    if let Some(index) = f.find("e") {
        digits += str::parse::<i32>(&f[index + 1..]).unwrap().unsigned_abs();
    }

    let prec = (digits as f64 * 3.322).ceil() as u32 + 16;
    let prec = prec.max(53);
    Float::parse(f).unwrap().complete(prec)
}

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}
