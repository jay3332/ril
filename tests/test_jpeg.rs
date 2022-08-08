use ril::prelude::*;

#[test]
fn test_jpeg() -> ril::Result<()> {
    let image = Image::<Rgb>::open("tests/sample.jpg")?;
    assert_eq!(image.dimensions(), (1024, 1024));
    
    image.save_inferred("tests/out/jpg_encode_output.jpg")?;

    Ok(())
}
