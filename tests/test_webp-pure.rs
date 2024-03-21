#![cfg(feature = "webp-pure")]

mod common;
use common::COLORS;

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
fn test_animated_webp_decode() -> ril::Result<()> {
    for (frame, ref color) in ImageSequence::<Rgb>::open("tests/animated_sample.webp")?.zip(COLORS)
    {
        let frame = frame?.into_image();

        assert_eq!(frame.dimensions(), (256, 256));
        assert_eq!(frame.pixel(0, 0), color);
    }

    Ok(())
}
