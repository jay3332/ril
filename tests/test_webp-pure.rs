#![cfg(feature = "webp-pure")]

mod test_png;

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
