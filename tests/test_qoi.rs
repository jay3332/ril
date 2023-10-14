use ril::prelude::*;

#[test]
fn test_qoi_rgb() -> ril::Result<()> {
    let image = Image::<Rgb>::open("tests/sample_rgb.qoi")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/qoi_encode_output_rgb.qoi")
}

#[test]
fn test_qoi_rgba() -> ril::Result<()> {
    let image = Image::<Rgba>::open("tests/sample_rgba.qoi")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/qoi_encode_output_rgba.qoi")
}

#[test]
fn test_qoi_rgba_conv() -> ril::Result<()> {
    let image = Image::<L>::open("tests/sample_rgba.qoi")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/qoi_encode_output_rgb_conv.qoi")
}

#[test]
fn test_qoi_rgb_conv() -> ril::Result<()> {
    let image = Image::<Rgba>::open("tests/sample_rgb.qoi")?;
    assert_eq!(image.dimensions(), (1024, 1024));

    image.save_inferred("tests/out/qoi_encode_output_rgba_conv.qoi")
}
