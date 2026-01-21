use criterion::{Criterion, criterion_group, criterion_main};
use fract::compute_mandelbrot;
use rast::tint::Srgb;
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = [Srgb::default(); fract::WIDTH * fract::HEIGHT];
    c.bench_function("mandelbrot", |b| {
        b.iter(|| {
            compute_mandelbrot(
                black_box(fract::WIDTH),
                black_box(fract::HEIGHT),
                black_box(&mut fb),
                100,
                1.0,
                0.0,
                0.0,
            )
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
