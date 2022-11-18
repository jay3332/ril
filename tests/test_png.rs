use ril::prelude::*;
use std::time::Duration;

const COLORS: [Rgb; 12] = [
    Rgb::new(255, 0, 0),
    Rgb::new(255, 128, 0),
    Rgb::new(255, 255, 0),
    Rgb::new(128, 255, 0),
    Rgb::new(0, 255, 0),
    Rgb::new(0, 255, 128),
    Rgb::new(0, 255, 255),
    Rgb::new(0, 128, 255),
    Rgb::new(0, 0, 255),
    Rgb::new(128, 0, 255),
    Rgb::new(255, 0, 255),
    Rgb::new(255, 0, 128),
];

#[test]
fn test_static_png() -> ril::Result<()> {
    let image = Image::<Rgb>::open("tests/sample.png")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/png_encode_output.png")?;

    Ok(())
}

#[test]
fn test_animated_png_encode() -> ril::Result<()> {
    let mut seq = ImageSequence::new();

    for color in COLORS.into_iter() {
        seq.push_frame(
            Frame::from_image(Image::new(256, 256, color)).with_delay(Duration::from_millis(100)),
        )
    }

    seq.save_inferred("tests/out/apng_encode_output.png")?;

    Ok(())
}

#[test]
fn test_animated_png_decode() -> ril::Result<()> {
    for (i, frame) in ImageSequence::<Rgb>::open("tests/apng_sample.png")?.enumerate() {
        let frame = frame?.into_image();

        assert_eq!(frame.dimensions(), (256, 256));
        assert_eq!(frame.pixel(0, 0), &COLORS[i]);
    }

    Ok(())
}

#[test]
fn test_paletted_png_encode() -> ril::Result<()> {
    let mut image = Image::<PalettedRgb>::from_paletted_pixels(
        2,
        vec![Rgb::new(255, 255, 255), Rgb::new(0, 0, 0)],
        vec![0, 1, 1, 0, 1, 0, 0, 1, 1, 0],
    );
    // palette mutation test
    let palette = image
        .palette_mut()
        .expect("palette was not registered properly");
    palette[0] = Rgb::new(128, 128, 128);

    assert_eq!(image.pixel(0, 0).color(), Rgb::new(128, 128, 128));
    assert_eq!(image.pixel(1, 0).color(), Rgb::new(0, 0, 0));

    image.save_inferred("tests/out/png_palette_encode_output.png")?;

    Ok(())
}
