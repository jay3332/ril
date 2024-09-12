use ril::prelude::*;

fn main() -> ril::Result<()> {
    let font = Font::open("./examples/assets/Arial.ttf", 12.0)?;

    let mut image = Image::new(72, 72, Rgba::black());

    let (x, y) = image.center();
    let layout = TextLayout::new()
        .centered() // Shorthand for centering horizontally and vertically
        .with_wrap(WrapStyle::Word) // RIL supports word wrapping
        .with_width(image.width()) // This is the width to wrap text at. Only required if you want to wrap text.
        .with_position(x, y) // Position the anchor (which is the center) at the center of the image
        .with_segment(
            &TextSegment::new(&font, "Here is some multi line text.", Rgba::white())
                .with_max_height(30),
        );

    image.draw(&layout);

    image.save_inferred("sample_ellipsis.jpg")?;

    Ok(())
}
