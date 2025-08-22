use ril::encodings::gif::GifEncoder;
use ril::prelude::*;

#[test]
fn test_filter() -> ril::Result<()> {
    let mut image = Image::<Rgb>::open("tests/puffins.jpg")?;

    let metadata = EncoderMetadata::from(&image)
        .with_frame_count(20)
        .with_loop_count(LoopCount::Exactly(1));

    // let convolution = Convolution::<9, 9, _, _>::box_blur();
    let convolution = DynamicConvolution::new(vec![1.0 / 81.0; 81], 9);
    image.apply_filter(&convolution);

    let mut encoder = GifEncoder::new(std::fs::File::create("dih.gif")?, metadata)?;
    for i in 0..20 {
        let frame = Frame::from_image(image.clone().brightened(i * 5))
            .with_delay(std::time::Duration::from_millis(100));

        encoder.add_frame(&frame)?;
    }
    encoder.finish()?;

    image.save_inferred("tests/out/convolution_output.png")?;
    Ok(())
}
