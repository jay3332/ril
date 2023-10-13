use ril::prelude::*;

#[test]
fn test_text_rendering() -> ril::Result<()> {
    let font = Font::open("tests/test_font_inter.ttf", 20.0)?;
    let mut image = Image::new(512, 1024, Rgba::black());

    let (cx, cy) = image.center();
    let layout = TextLayout::new()
        .with_wrap(WrapStyle::Word)
        .with_width(image.width())
        .with_position(cx, cy)
        .with_basic_text(&font, include_str!("sample_text.txt"), Rgba::white())
        .with_align(TextAlign::Center)
        .centered();

    let bounds = layout.bounding_box();
    assert_eq!(bounds, (4, 24, 507, 999));

    image.draw(&layout);
    image.save_inferred("tests/out/text_render_output.png")
}

#[test]
fn test_text_gradient() -> ril::Result<()> {
    let font = Font::open("tests/test_font_inter.ttf", 48.0)?;
    let mut mask = Image::new(256, 64, Rgba::transparent());

    let (cx, cy) = mask.center();
    let layout = TextLayout::new()
        .with_wrap(WrapStyle::Word)
        .with_width(mask.width())
        .with_position(cx, cy)
        .with_basic_text(&font, "gradient", Rgba::white())
        .centered();

    mask.draw(&layout);

    let gradient = RadialGradient::new()
        .with_color(Rgba::new(0, 0, 255, 255))
        .with_color_at(0.75, Rgba::new(0, 255, 128, 255));

    let mut image = Image::new(256, 64, Rgba::transparent())
        .with(&Rectangle::from_bounding_box(0, 0, 256, 64).with_fill(gradient));

    image.mask_alpha(&mask.bands().3);
    image.save_inferred("tests/out/text_gradient_output.png")
}

#[test]
fn test_resize_gradient() -> ril::Result<()> {
    let gradient = RadialGradient::new()
        .with_color(Rgba::new(0, 0, 255, 255))
        .with_color(Rgba::transparent())
        .with_color_at(0.75, Rgba::new(0, 255, 128, 255));

    Image::new(2048, 512, Rgba::transparent())
        .with(&Rectangle::from_bounding_box(0, 0, 2048, 512).with_fill(gradient.clone()))
        .save_inferred("tests/out/resize_gradient_output_control.png")?;

    Image::new(256, 64, Rgba::transparent())
        .with(&Rectangle::from_bounding_box(0, 0, 256, 64).with_fill(gradient))
        .resized(2048, 512, ResizeAlgorithm::Bilinear)
        .save_inferred("tests/out/resize_gradient_output_resized.png")
}
