use criterion::{criterion_group, criterion_main, Criterion};
use ril::prelude::*;
use std::time::Duration;

pub fn bench_text_rendering(c: &mut Criterion) {
    let mut c = c.benchmark_group("text_rendering");
    c.sample_size(10)
        .warm_up_time(Duration::from_millis(1500))
        .measurement_time(Duration::from_secs(10));

    let sample = include_str!("../tests/sample_text.txt");
    let bytes = include_bytes!("../tests/test_font_inter.ttf") as &[u8];

    c.bench_function("text_rendering (ril)", |b| {
        let mut image = Image::new(8192, 128, Rgba::white());
        let font = Font::from_bytes(bytes, 20.0).unwrap();

        b.iter(|| {
            TextSegment::new(&font, sample, Rgba::new(0, 0, 0, 255))
                .with_position(5, 5)
                .draw(&mut image);
        })
    });

    c.bench_function("text_rendering (image-rs + imageproc)", |b| {
        let mut image = image::RgbaImage::from_pixel(8192, 128, image::Rgba([255, 255, 255, 255]));
        let font = rusttype::Font::try_from_bytes(bytes).unwrap();

        b.iter(|| {
            imageproc::drawing::draw_text_mut(
                &mut image,
                image::Rgba([0, 0, 0, 255]),
                5,
                5,
                rusttype::Scale::uniform(20.0),
                &font,
                sample,
            );
        })
    });

    c.finish();
}

criterion_group!(benches, bench_text_rendering);
criterion_main!(benches);
