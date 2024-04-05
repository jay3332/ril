#![cfg(feature = "webp")]

mod common;
use common::COLORS;
use ril::prelude::*;
use std::time::Duration;

#[test]
fn test_static_webp_encode() -> ril::Result<()> {
    let image = Image::from_fn(256, 256, |x, _| L(x as u8));

    image.save_inferred("tests/out/webp_encode_output.webp")
}

#[test]
fn test_animated_webp_encode() -> ril::Result<()> {
    let mut seq = ImageSequence::new();

    for color in COLORS {
        seq.push_frame(
            Frame::from_image(Image::new(256, 256, color)).with_delay(Duration::from_millis(100)),
        );
    }

    seq.save_inferred("tests/out/animated_webp_encode_output.webp")
}

#[test]
fn test_static_webp_decode() -> ril::Result<()> {
    let image = Image::<Rgb>::open("tests/sample.webp")?;

    assert_eq!(image.dimensions(), (256, 256));
    assert_eq!(image.pixel(0, 0), &Rgb::new(255, 0, 0));

    Ok(())
}

#[test]
fn test_animated_webp_lossless() -> ril::Result<()> {
    for (i, frame) in ImageSequence::<Rgb>::open("tests/animated_lossless.webp")?.enumerate() {
        let frame = frame?.into_image();

        let reference =
            Image::<Rgb>::open(format!("tests/reference/random_lossless-{}.png", i + 1))?;

        frame.pixels().zip(reference.pixels()).for_each(|(a, b)| {
            assert_eq!(a, b);
        });
    }

    Ok(())
}

#[test]
fn test_animated_webp_lossy() -> ril::Result<()> {
    for (i, frame) in ImageSequence::<Rgb>::open("tests/animated_lossy.webp")?.enumerate() {
        let frame = frame?.into_image();

        let reference = Image::<Rgb>::open(format!("tests/reference/random_lossy-{}.png", i + 1))?;

        frame.pixels().zip(reference.pixels()).for_each(|(a, b)| {
            assert_eq!(a, b);
        });
    }

    Ok(())
}
