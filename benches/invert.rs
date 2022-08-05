use std::time::Duration;

use criterion::{criterion_group, criterion_main, Criterion};
use ril::prelude::*;

const SAMPLES: [u32; 6] = [256, 512, 1024, 2048, 4096, 8192];

pub fn bench_invert(c: &mut Criterion) {
    let mut c = c.benchmark_group("invert");
    c.sample_size(10)
        .warm_up_time(Duration::from_millis(1500))
        .measurement_time(Duration::from_secs(3));

    for size in SAMPLES {
        let mut image = Image::new(size, size, Rgb::white());

        c.bench_function(format!("invert {0}x{0} RGB", size).as_str(), |b| {
            b.iter(|| image.invert())
        });

        let mut image = Image::new(size, size, L(0));

        c.bench_function(format!("invert {0}x{0} L", size).as_str(), |b| {
            b.iter(|| image.invert())
        });
    }

    c.finish();
}

criterion_group!(benches, bench_invert);
criterion_main!(benches);
