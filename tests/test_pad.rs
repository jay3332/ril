use ril::prelude::*;
use std::fs;
use std::time::Instant;

#[test]
fn test_padding() -> ril::Result<()> {
    let mut image = Image::from_fn(1024, 1024, |x, y| {
        Rgba::new((x % 256) as u8, (y % 256) as u8, 255, 255)
    });
    let time = Instant::now();
    image.pad(256, 256, 256, 256, Rgba::black());
    assert_eq!(image.dimensions(), (1536, 1536));
    Ok(())
}
