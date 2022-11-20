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
        .centered();

    let bounds = layout.bounding_box();
    assert_eq!(bounds, (4, 24, 507, 999));

    image.draw(&layout);
    image.save_inferred("tests/out/text_render_output.png")
}
