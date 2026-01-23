use criterion::{Criterion, criterion_group, criterion_main};
use fract::{
    PRECISION,
    software::{Pipeline, compute_mandelbrot},
};
use rast::tint::Srgb;
use rug::{Float, ops::CompleteRound};
use std::hint::black_box;

fn criterion_benchmark(c: &mut Criterion) {
    let mut fb = vec![Srgb::default(); fract::WIDTH * fract::HEIGHT];

    // let zoom = Float::with_val(PRECISION, 1.0);
    // let cx = Float::with_val(PRECISION, 0.0);
    // let cy = Float::with_val(PRECISION, 0.0);
    // c.bench_function("[SOFTWARE] no perturbation", |b| {
    //     b.iter(|| {
    //         let mut pipeline = Pipeline::default();
    //         compute_mandelbrot(
    //             black_box(&mut pipeline),
    //             black_box(&mut fb),
    //             black_box(1000),
    //             black_box(&zoom),
    //             black_box(&cx),
    //             black_box(&cy),
    //         );
    //     })
    // });

    let zoom = Float::parse("4.9369960548568338955566401331513647338919005732326e-5")
        .unwrap()
        .complete(PRECISION);
    let cx = Float::parse("-6.9550855300283617401720624898076838918553513221840e-1")
        .unwrap()
        .complete(PRECISION);
    let cy = Float::parse("3.6821253719040156918882966036467425177521358016978e-1")
        .unwrap()
        .complete(PRECISION);
    c.bench_function("[SOFTWARE] perturbation", |b| {
        b.iter(|| {
            let mut pipeline = Pipeline::default();
            compute_mandelbrot(
                black_box(&mut pipeline),
                black_box(&mut fb),
                black_box(1000),
                black_box(&zoom),
                black_box(&cx),
                black_box(&cy),
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
