#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

#[cfg(feature = "compute")]
pub mod compute;
// #[cfg(not(feature = "compute"))]
pub mod software;

pub const WIDTH: usize = 1600;
pub const HEIGHT: usize = 1600;

pub const PRECISION: u32 = 1024;
pub const MANDELBROT_XRANGE: f64 = 2.00 + 0.47;
pub const MANDELBROT_YRANGE: f64 = 1.12 + 1.12;
