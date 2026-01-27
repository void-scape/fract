#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

use rug::Float;

mod compute;
pub mod config;
mod orbit;
pub mod palette;
pub mod pipeline;
mod ssaa;
pub mod viewer;

pub fn precision(z: &Float) -> u32 {
    53 + z.get_exp().unwrap_or(0).unsigned_abs()
}

pub fn byte_slice<T>(slice: &[T]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(slice.as_ptr().cast(), std::mem::size_of_val(slice)) }
}
