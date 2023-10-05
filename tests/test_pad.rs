use lazy_static::lazy_static;
use ril::prelude::*;

lazy_static! {
    static ref TEST_IMAGE: Image<Rgba> = Image::from_fn(
        256, 256, |x, y|
        Rgba::new(x as u8, y as u8, 255, 255)
    );
}

//noinspection RsAssertEqual
#[test]
fn test_padding() -> ril::Result<()> {
    let mut image = TEST_IMAGE.clone();
    image.pad(64, 32, 64, 32, Default::default());
    assert_eq!(image.dimensions(), (384, 320));
    // Using include_bytes here to prevent having to test with a feature enabled
    let bytes = image.data
        .iter().flat_map(|pixel| pixel.as_bytes())
        .collect::<Box<[u8]>>();
    // Not using assert_eq here, as it causes a gigantic error message
    // with the representations of each value
    assert!(
        bytes.as_ref() == include_bytes!("sample.bin"),
        "padded image was not identical to sample"
    );

    Ok(())
}

#[test]
#[should_panic(expected = "width overflowed")]
fn test_overflow_width_check() {
    let mut image: Image<Rgba> = TEST_IMAGE.clone();
    image.pad(u32::MAX, 0, 0, 0, Default::default());
}

#[test]
#[should_panic(expected = "padding overflowed")]
fn test_overflow_pad_check() {
    let mut image: Image<Rgba> = TEST_IMAGE.clone();
    image.pad(u32::MAX, 0, 1, 0, Default::default());
}