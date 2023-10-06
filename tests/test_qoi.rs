use ril::prelude::*;

#[test]
fn test_qoi() -> ril::Result<()> {
    let image: Image<Rgba> = Image::open("tests/sample.qoi")?;
    assert_eq!(image.dimensions(), (1024, 1024));
    image.save_inferred("tests/out/qoi_encode_output.qoi")
}
