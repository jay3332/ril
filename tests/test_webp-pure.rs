#![cfg(feature = "webp-pure")]

mod common;

use ril::prelude::*;

#[test]
fn test_static_webp_encode() -> ril::Result<()> {
    let image = Image::from_fn(256, 256, |x, _| L(x as u8));

    image.save_inferred("tests/out/webp_pure_encode_output.webp")
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

        frame.map_pixels_with_coords(|x, y, rgb| {
            assert_eq!(&rgb, reference.get_pixel(x, y).unwrap());
            rgb
        });
    }

    Ok(())
}

#[test]
fn test_animated_webp_lossy() -> ril::Result<()> {
    for (i, frame) in ImageSequence::<Rgb>::open("tests/animated_lossy.webp")?.enumerate() {
        let frame = frame?.into_image();

        let reference = Image::<Rgb>::open(format!("tests/reference/random_lossy-{}.png", i + 1))?;

        let (width, height) = frame.dimensions();

        // https://github.com/image-rs/image-webp/blob/4020925b7002bac88cda9f951eb725f6a7fcd3d8/tests/decode.rs#L56-L59
        let pixels = frame.pixels();
        let num_bytes_different = pixels
            .zip(reference.pixels())
            .filter(|(a, b)| a != b)
            .count();

        assert!(
            100 * num_bytes_different / ((width * height) as usize) < 1,
            "More than 1% of pixels differ"
        );
    }

    Ok(())
}
