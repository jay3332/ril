use criterion::{criterion_group, criterion_main, Criterion};
use std::time::Duration;

use gif::{DecodeOptions, Encoder};
use image::codecs::gif::{GifDecoder, GifEncoder};
use image::AnimationDecoder;
use ril::prelude::*;

pub fn bench_invert_gif(c: &mut Criterion) {
    let mut c = c.benchmark_group("invert_gif");
    c.sample_size(10)
        .warm_up_time(Duration::from_millis(1500))
        .measurement_time(Duration::from_secs(10));

    c.bench_function("invert_gif ril (combinator)", |b| {
        b.iter(|| {
            ImageSequence::open("benches/invert_sample.gif")
                .unwrap()
                .map(|frame| frame.unwrap().map_image(|img| img.inverted()))
                .collect::<ImageSequence<Rgb>>()
                .save_inferred("benches/out/invert_ril_combinator.gif")
                .unwrap();
        })
    });

    c.bench_function("invert_gif ril (for loop)", |b| {
        b.iter(|| {
            let mut out = ImageSequence::<Rgb>::new();

            for frame in ImageSequence::open("benches/invert_sample.gif").unwrap() {
                out.push_frame(frame.unwrap().map_image(|img| img.inverted()));
            }

            out.save_inferred("benches/out/invert_ril_for.gif").unwrap();
        })
    });

    c.bench_function("invert_gif ril (raw)", |b| {
        b.iter(|| {
            let mut decoder = DecodeOptions::new();
            decoder.set_color_output(gif::ColorOutput::RGBA);

            let mut decoder = decoder
                .read_info(std::fs::File::open("benches/invert_sample.gif").unwrap())
                .unwrap();

            let mut encoder = Encoder::new(
                std::fs::File::create("benches/out/invert_ril_raw.gif").unwrap(),
                decoder.width(),
                decoder.height(),
                &[],
            )
            .unwrap();

            while let Some(frame) = decoder.read_next_frame().unwrap() {
                let data = frame
                    .buffer
                    .chunks(4)
                    .map(|p| Rgba {
                        r: p[0],
                        g: p[1],
                        b: p[2],
                        a: p[3],
                    })
                    .collect::<Vec<_>>();

                let image = Image::<Rgba>::from_pixels(decoder.width() as u32, data);
                let mut data = image
                    .data
                    .iter()
                    .flat_map(|p| p.inverted().as_bytes())
                    .collect::<Vec<_>>();

                let frame =
                    gif::Frame::from_rgba_speed(decoder.width(), decoder.height(), &mut data, 30);

                encoder.write_frame(&frame).unwrap();
            }
        })
    });

    c.bench_function("invert_gif image-rs (low-level)", |b| {
        b.iter(|| {
            let file = std::fs::File::open("benches/invert_sample.gif").unwrap();
            let decoder = GifDecoder::new(file).unwrap();
            let mut out =
                GifEncoder::new(std::fs::File::create("benches/out/invert_image-rs.gif").unwrap());

            decoder.into_frames().for_each(|frame| {
                let mut frame = frame.unwrap();
                let buffer = frame.buffer_mut();
                image::imageops::invert(buffer);

                out.encode_frame(frame).unwrap();
            });
        })
    });

    c.finish();
}

criterion_group!(benches, bench_invert_gif);
criterion_main!(benches);
