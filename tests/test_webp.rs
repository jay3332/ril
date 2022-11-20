mod test_png;

use ril::prelude::*;
use std::time::Duration;
use test_png::COLORS;

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
