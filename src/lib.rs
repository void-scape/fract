#![allow(clippy::too_many_arguments)]

use crate::{encoder::Encoder, pipeline::Pipeline};
use indicatif::ProgressBar;
use malachite::{Rational, base::rounding_modes::RoundingMode};
use malachite_float::Float;

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
    encoder::png(output, &pixels, w, h, true)
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

/// Cast a slice to bytes.
pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}

pub fn to_f32_exp(x: &Float) -> (f32, i32) {
    if let Some((m, e, _)) = x.sci_mantissa_and_exponent_round::<f32>(RoundingMode::Nearest) {
        let signum = if x.is_sign_positive() { 1.0 } else { -1.0 };
        // NOTE: Convert [1.0, 2.0) normalization to [0.5, 1.0) to work with the
        // current `rug` implementation.
        (m * 0.5 * signum, e + 1)
    } else {
        (0.0, 0)
    }
}

pub fn float_from_str(str: &str) -> Float {
    let mut digits = str
        .chars()
        .take_while(|c| *c != 'e')
        .filter(|c| c.is_ascii_digit())
        .count() as u32;

    if let Some(index) = str.find("e") {
        digits += str::parse::<i32>(&str[index + 1..]).unwrap().unsigned_abs();
    }

    let prec = (digits as f64 * 3.322).ceil() as u64 + 16;
    let prec = prec.max(53);

    Float::from_rational_prec_round(
        Rational::from_sci_string_simplest(str).unwrap(),
        prec,
        RoundingMode::Nearest,
    )
    .0
}
