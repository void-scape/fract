#![allow(clippy::too_many_arguments)]
#![allow(internal_features)]
#![feature(core_intrinsics)]

pub mod pipeline;
pub use pipeline::*;

pub const WIDTH: usize = 1600;
pub const HEIGHT: usize = 1600;

pub const PRECISION: u32 = 1024;
pub const MANDELBROT_XRANGE: f64 = 2.00 + 0.47;
pub const MANDELBROT_YRANGE: f64 = 1.12 + 1.12;

/// Compute the series approximation coefficients for a given reference `orbit`.
///
/// A = 2XA + 1
/// B = 2XB + A * A
/// C = 2XC + 2AB
pub fn series_approximation_coefficients<T>(
    orbit: &[(T, T)],
    sdx: f64,
    sdy: f64,
    xstep: f64,
    ystep: f64,
) -> (
    num::Complex<f64>,
    num::Complex<f64>,
    num::Complex<f64>,
    usize,
)
where
    T: Into<f64> + Copy,
{
    let mut a = num::Complex::<f64>::new(1.0, 0.0);
    let mut b = num::Complex::<f64>::default();
    let mut c = num::Complex::<f64>::default();
    let mut approx_iteration = 0;

    fn push_point(
        px: f64,
        py: f64,
        sdx: f64,
        sdy: f64,
        xstep: f64,
        ystep: f64,
        points: &mut [[num::Complex<f64>; 3]],
        index: usize,
    ) {
        let dx0 = sdx + px * xstep;
        let dy0 = sdy + py * ystep;
        let d = num::Complex::new(dx0, dy0);
        let d2 = d * d;
        let d3 = d2 * d;
        points[index] = [d, d2, d3];
    }

    let w = WIDTH as f64 / 2.0;
    let h = HEIGHT as f64 / 2.0;
    let mut points = [[num::Complex::default(); 3]; 4];
    push_point(0.0, 0.0, sdx, sdy, xstep, ystep, &mut points, 0);
    push_point(w, 0.0, sdx, sdy, xstep, ystep, &mut points, 1);
    push_point(0.0, h, sdx, sdy, xstep, ystep, &mut points, 2);
    push_point(w, h, sdx, sdy, xstep, ystep, &mut points, 3);

    'outer: while approx_iteration < orbit.len() {
        let (re, im) = orbit[approx_iteration];
        let x = num::Complex::new(re.into(), im.into());
        let x2 = x * 2.0;
        let aa = x2 * a + 1.0;
        let bb = x2 * b + a * a;
        let cc = x2 * c + 2.0 * a * b;

        for [d, d2, d3] in points.iter() {
            // D = Ad + Bd^2 + Cd^3
            if (cc * d3).norm() > ((aa * d).norm() + (bb * d2).norm()) * xstep * 10000.0 {
                break 'outer;
            }
        }

        a = aa;
        b = bb;
        c = cc;

        approx_iteration += 1;
    }

    (a, b, c, approx_iteration)
}
