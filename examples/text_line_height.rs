use ril::prelude::*;

fn main() -> ril::Result<()> {
    let font = Font::open("./examples/assets/Arial.ttf", 12.0)?;

    let mut image = Image::new(72, 72, Rgba::black());

    let (x, y) = image.center();
    let layout = TextLayout::new()
        .centered() // Shorthand for centering horizontally and vertically
        .with_wrap(WrapStyle::Word, image.width()) // Wrap the text such that it doesn't overflow the image
        .with_position(x, y) // Position the anchor (which is the center) at the center of the image
        .with_line_height(0.8) // Set the line height to 80% of the default
        .with_basic_text(&font, "Super long overflowing line of text", Rgba::white());

    image.draw(&layout);
    image.save_inferred("example_text_line_height.png")?;

    Ok(())
}
