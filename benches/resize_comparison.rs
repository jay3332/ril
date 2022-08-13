use criterion::{criterion_group, criterion_main, Criterion};
use ril::prelude::*;
use std::time::Duration;

macro_rules! bench_ril {
    ($c:expr, $w:literal, $h:literal, $algorithm:ident) => {{
        ($c).bench_function(
            format!(
                "resize RGBA 600x600 to {}x{} (ril, {})",
                $w,
                $h,
                stringify!($algorithm).to_ascii_lowercase()
            ),
            |b| {
                let image = Image::<Rgba>::open("benches/resize_sample.png").unwrap();

                b.iter(|| {
                    image.clone().resize($w, $h, ResizeAlgorithm::$algorithm);
                });
            },
        );
    }};
}

macro_rules! bench_image_rs {
    ($c:expr, $w:literal, $h:literal, $algorithm:ident) => {{
        ($c).bench_function(
            format!(
                "resize RGBA 600x600 to {}x{} (image-rs, {})",
                $w,
                $h,
                stringify!($algorithm).to_ascii_lowercase()
            ),
            |b| {
                let image = image::open("benches/resize_sample.png").unwrap();

                b.iter(|| {
                    image
                        .clone()
                        .resize($w, $h, image::imageops::FilterType::$algorithm);
                });
            },
        );
    }};
}

pub fn bench_resize(c: &mut Criterion) {
    let mut c = c.benchmark_group("resize");
    c.sample_size(10)
        .warm_up_time(Duration::from_millis(1500))
        .measurement_time(Duration::from_secs(10));

    // Downscaling
    bench_ril!(c, 64, 64, Nearest);
    bench_image_rs!(c, 64, 64, Nearest);
    bench_ril!(c, 64, 64, Lanczos3);
    bench_image_rs!(c, 64, 64, Lanczos3);

    // Upscaling
    bench_ril!(c, 2048, 2048, Nearest);
    bench_image_rs!(c, 2048, 2048, Nearest);
    bench_ril!(c, 2048, 2048, Lanczos3);
    bench_image_rs!(c, 2048, 2048, Lanczos3);

    c.finish();
}

criterion_group!(benches, bench_resize);
criterion_main!(benches);
