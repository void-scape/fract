#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

pub mod palette;
pub mod pipeline;
pub mod software;

use rug::Complex;
use rug::Float;

pub const PRECISION: u32 = 1024;

/// Compute the series approximation coefficients for a given reference `orbit`.
///
/// A = 2XA + 1
/// B = 2XB + A * A
/// C = 2XC + 2AB
pub fn series_approximation_coefficients<T>(
    orbit: &[(T, T)],
    sdx: f64,
    sdy: f64,
    zoom: &Float,
) -> (
    num::Complex<f64>,
    num::Complex<f64>,
    num::Complex<f64>,
    usize,
)
where
    T: Into<f64> + Copy,
{
    const PREC: u32 = 256;

    let mut a = Complex::with_val(PREC, (1.0, 0.0));
    let mut b = Complex::with_val(PREC, (0.0, 0.0));
    let mut c = Complex::with_val(PREC, (0.0, 0.0));
    let mut approx_iteration = 0;

    let d0 = Complex::with_val(PREC, (sdx, sdy));
    let d2 = Complex::with_val(PREC, &d0 * &d0);
    let d3 = Complex::with_val(PREC, &d2 * &d0);

    let one = Complex::with_val(PREC, (1.0, 0.0));
    let two = Complex::with_val(PREC, (2.0, 0.0));
    let tofactor = Float::with_val(PREC, 0.01 / zoom);

    'outer: while approx_iteration < orbit.len().saturating_sub(2) {
        let (re, im) = orbit[approx_iteration];
        let x = Complex::with_val(PREC, (re.into(), im.into()));
        let x2 = Complex::with_val(PREC, &x * &two);

        let aa = Complex::with_val(PREC, &x2 * &a + &one);
        let a2 = Complex::with_val(PREC, &a * &a);
        let bb = Complex::with_val(PREC, &x2 * &b + a2);
        let ab = Complex::with_val(PREC, &a * &b);
        let cc = Complex::with_val(PREC, &x2 * &c + &two * ab);

        let cc_d3 = Complex::with_val(PREC, &cc * &d3);
        let left = Float::with_val(PREC, cc_d3.abs().real() * &tofactor);

        let aa_d0 = Complex::with_val(PREC, &aa * &d0);
        let bb_d2 = Complex::with_val(PREC, &bb * &d2);
        let right = Float::with_val(PREC, aa_d0.abs().real() + bb_d2.abs().real());

        // |(cc * d3)| * 10_000 > |(aa * d0)| + |(bb * d2)|
        if left > right {
            break 'outer;
        }

        a = aa;
        b = bb;
        c = cc;
        approx_iteration += 1;
    }

    let a = num::Complex::new(a.real().to_f64(), a.imag().to_f64());
    let b = num::Complex::new(b.real().to_f64(), b.imag().to_f64());
    let c = num::Complex::new(c.real().to_f64(), c.imag().to_f64());
    (a, b, c, approx_iteration)
}
