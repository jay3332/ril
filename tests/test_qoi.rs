use std::time::Instant;
use ril::prelude::*;

#[test]
fn test_qoi() -> ril::Result<()> {
    let image: Image<Rgba> = Image::open("tests/sample.qoi")?;
    let elapsed = Instant::now();
    assert_eq!(image.dimensions(), (1024, 1024));
    image.save_inferred("tests/out/qoi_encode_output.qoi")?;
    println!("{:?}", elapsed);
    Ok(())
}
