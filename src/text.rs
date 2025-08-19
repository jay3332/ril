//! Implements the font/text rasterizing and layout interface.

#![allow(clippy::cast_precision_loss, clippy::too_many_arguments)]

use crate::{Draw, Error::FontError, Image, OverlayMode, Pixel};

use fontdue::{
    layout::{CoordinateSystem, Layout, LayoutSettings, TextStyle},
    FontSettings,
};
use std::{fs::File, io::Read, ops::DerefMut, path::Path};

/// Represents a single font along with its alternatives used to render text.
/// Currently, this supports TrueType and OpenType fonts.
#[allow(clippy::doc_markdown)]
#[derive(Clone)]
pub struct Font {
    inner: fontdue::Font,
    settings: FontSettings,
}

impl Font {
    /// Opens the font from the given path.
    ///
    /// The optimal size is not the fixed size of the font - rather it is the size to optimize
    /// rasterizing the font for.
    ///
    /// Lower sizes will look worse but perform faster, while higher sizes will
    /// look better but perform slower. It is best to set this to the size that will likely be
    /// the most used.
    ///
    /// # Errors
    /// * Failed to load the font.
    pub fn open<P: AsRef<Path>>(path: P, optimal_size: f32) -> crate::Result<Self> {
        Self::from_reader(File::open(path)?, optimal_size)
    }

    /// Loads the font from the given byte slice. Useful for the `include_bytes!` macro.
    ///
    /// The optimal size is not the fixed size of the font - rather it is the size to optimize
    /// rasterizing the font for.
    ///
    /// Lower sizes will look worse but perform faster, while higher sizes will
    /// look better but perform slower. It is best to set this to the size that will likely be
    /// the most used.
    ///
    /// # Errors
    /// * Failed to load the font.
    pub fn from_bytes(bytes: &[u8], optimal_size: f32) -> crate::Result<Self> {
        let settings = FontSettings {
            scale: optimal_size,
            collection_index: 0,
        };
        let inner = fontdue::Font::from_bytes(bytes, settings).map_err(FontError)?;

        Ok(Self { inner, settings })
    }

    /// Loads the font from the given byte reader. See [`from_bytes`] if you already have a byte
    /// slice - that is much more performant.
    ///
    /// The optimal size is not the fixed size of the font - rather it is the size to optimize
    /// rasterizing the font for.
    ///
    /// Lower sizes will look worse but perform faster, while higher sizes will
    /// look better but perform slower. It is best to set this to the size that will likely be
    /// the most used.
    ///
    /// # Errors
    /// * Failed to load the font.
    pub fn from_reader<R: Read>(mut buffer: R, optimal_size: f32) -> crate::Result<Self> {
        let settings = FontSettings {
            scale: optimal_size,
            collection_index: 0,
        };
        let mut out = Vec::new();
        buffer.read_to_end(&mut out)?;

        let inner = fontdue::Font::from_bytes(out, settings).map_err(FontError)?;

        Ok(Self { inner, settings })
    }

    /// Returns a reference the [`fontdue::Font`] object associated with the font.
    /// It contains technical information about the font.
    #[must_use]
    pub const fn inner(&self) -> &fontdue::Font {
        &self.inner
    }

    /// Consumes this font and returns the [`fontdue::Font`] object associated with the font.
    /// It contains technical information about the font.
    #[must_use]
    #[allow(clippy::missing_const_for_fn)] // no destructors
    pub fn into_inner(self) -> fontdue::Font {
        self.inner
    }

    /// Returns the optimal size, in pixels, of this font.
    ///
    /// The optimal size is not the fixed size of the font - rather it is the size to optimize
    /// rasterizing the font for.
    ///
    /// Lower sizes will look worse but perform faster, while higher sizes will
    /// look better but perform slower. It is best to set this to the size that will likely be
    /// the most used.
    #[must_use]
    pub const fn optimal_size(&self) -> f32 {
        self.settings.scale
    }
}

/// Determines how text should be wrapped.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum WrapStyle {
    /// Do not wrap the text.
    None,
    /// Keep words together and do not break in the middle of words. This is usually what is
    /// desired. Breaks to a newline at the nearest word boundary.
    ///
    /// This is the default behavior.
    Word,
    /// Keep as many characters per line as possible, and allow breaks in the middle of words.
    /// Breaks to a newline at the nearest character.
    Character,
}

impl Default for WrapStyle {
    fn default() -> Self {
        Self::Word
    }
}

/// Represents a text segment that can be drawn.
///
/// See [`TextLayout`] for a more robust implementation that supports rendering text with multiple
/// styles. This type is for more simple and lightweight usages.
///
/// Additionally, accessing metrics such as the width and height of the text cannot be done here,
/// but can be done in [`TextLayout`] since it keeps a running copy of the layout.
/// Use [`TextLayout`] if you will be needing to calculate the width and height of the text.
/// Additionally, [`TextLayout`] supports text anchoring, which can be used to align text.
///
/// If you need none of these features, text segments should be used in favor of being much more
/// lightweight.
///
/// Note that [`TextLayout`] is not cloneable while text segments are, which is one advantage
/// of using this over [`TextLayout`].
#[derive(Clone)]
pub struct TextSegment<'a, P: Pixel> {
    /// The position the text will be rendered at. Ignored if this is used in a [`TextLayout`].
    pub position: (u32, u32),
    /// The width of the text box. If this is used in a [`TextLayout`], this is ignored and
    /// [`TextLayout::with_width`] is used instead. This is used for text wrapping and wrapping only.
    pub width: Option<u32>,
    /// The content of the text segment.
    pub text: String,
    /// The font to use to render the text.
    pub font: &'a Font,
    /// The fill color the text will be in.
    pub fill: P,
    /// The overlay mode of the text. Note that anti-aliasing is still a bit funky with
    /// [`OverlayMode::Replace`], so it is best to use [`OverlayMode::Merge`] for this, which is
    /// the default.
    pub overlay: OverlayMode,
    /// The size of the text in pixels.
    pub size: f32,
    /// The wrapping style of the text. Note that text will only wrap if [`width`] is set.
    /// If this is used in a [`TextLayout`], this is ignored and [`TextLayout::with_wrap`] is
    /// used instead.
    pub wrap: WrapStyle,
}

impl<'a, P: Pixel> TextSegment<'a, P> {
    /// Creates a new text segment with the given text, font, and fill color.
    /// The text can be anything that implements [`AsRef<str>`].
    ///
    /// If this is used to be directly drawn (as opposed to in a [`TextLayout`]), the position
    /// is set to ``(0, 0)`` by default. Use [`with_position`][TextSegment::with_position] to set
    /// the position.
    ///
    /// The size defaults to the font's optimal size.
    /// You can override this by using the [`with_size`][Self::with_size] method.
    #[must_use]
    pub fn new(font: &'a Font, text: impl AsRef<str>, fill: P) -> Self {
        Self {
            position: (0, 0),
            width: None,
            text: text.as_ref().to_string(),
            font,
            fill,
            overlay: OverlayMode::Merge,
            size: font.optimal_size(),
            wrap: WrapStyle::Word,
        }
    }

    /// Sets the position of the text segment. Ignored if this segment is used in a [`TextLayout`].
    #[must_use]
    pub const fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    /// Sets the size of the text segment.
    #[must_use]
    pub const fn with_size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    /// Sets the overlay mode of the text segment.
    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = mode;
        self
    }

    /// Sets the width of the text segment, used for text wrapping.
    /// If this is used in a [`TextLayout`], this is ignored and [`TextLayout::width`] is used instead.
    #[must_use]
    pub const fn with_width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Sets the wrapping style of the text segment.
    /// If this is used in a [`TextLayout`], this is ignored and [`TextLayout::wrap`] is used instead.
    #[must_use]
    pub const fn with_wrap(mut self, wrap: WrapStyle) -> Self {
        self.wrap = wrap;
        self
    }

    fn layout(&self) -> Layout<(P, OverlayMode)> {
        let mut layout = Layout::new(CoordinateSystem::PositiveYDown);
        layout.reset(&LayoutSettings {
            x: self.position.0 as f32,
            y: self.position.1 as f32,
            max_width: if self.wrap == WrapStyle::None {
                None
            } else {
                self.width.map(|w| w as f32)
            },
            wrap_style: match self.wrap {
                WrapStyle::None | WrapStyle::Word => fontdue::layout::WrapStyle::Word,
                WrapStyle::Character => fontdue::layout::WrapStyle::Letter,
            },
            ..LayoutSettings::default()
        });
        layout.append(
            &[self.font.inner()],
            &TextStyle::with_user_data(&self.text, self.size, 0, (self.fill, self.overlay)),
        );
        layout
    }
}

fn render_layout<P: Pixel>(
    image: &mut Image<P>,
    fonts: &[&fontdue::Font],
    layout: &Layout<(P, OverlayMode)>,
) {
    let glyphs = layout.glyphs();
    if glyphs.is_empty() {
        return;
    }

    // SAFETY: already checked before calling
    let lines = unsafe { layout.lines().unwrap_unchecked() };
    for line in lines {
        for glyph in &glyphs[line.glyph_start..=line.glyph_end] {
            let (fill, overlay) = glyph.user_data;
            let font = fonts[glyph.font_index];
            let (metrics, bitmap) = font.rasterize_config(glyph.key);

            if metrics.width == 0 || glyph.char_data.is_whitespace() || metrics.height == 0 {
                continue;
            }

            for (row, y) in bitmap.chunks_exact(metrics.width).zip(glyph.y as i32..) {
                for (value, x) in row.iter().zip(glyph.x as i32..) {
                    let (x, y) = if x < 0 || y < 0 {
                        continue;
                    } else {
                        (x as u32, y as u32)
                    };

                    let value = *value;
                    if value == 0 {
                        continue;
                    }

                    if let Some(pixel) = image.get_pixel(x, y) {
                        *image.pixel_mut(x, y) = pixel.overlay_with_alpha(fill, overlay, value);
                    }
                }
            }
        }
    }
}

fn render_layout_with_alignment<P: Pixel>(
    image: &mut Image<P>,
    fonts: &[&fontdue::Font],
    layout: &Layout<(P, OverlayMode)>,
    widths: Vec<u32>,
    max_width: u32,
    fx: f32,
    ox: f32,
    oy: f32,
) {
    let glyphs = layout.glyphs();
    if glyphs.is_empty() {
        return;
    }

    // SAFETY: this was checked before calling
    let lines = unsafe { layout.lines().unwrap_unchecked() };
    for (line, width) in lines.iter().zip(widths) {
        let ox = ((max_width - width) as f32).mul_add(fx, ox);

        for glyph in &glyphs[line.glyph_start..=line.glyph_end] {
            let (fill, overlay) = glyph.user_data;
            let font = fonts[glyph.font_index];
            let (metrics, bitmap) = font.rasterize_config(glyph.key);

            if metrics.width == 0 || glyph.char_data.is_whitespace() || metrics.height == 0 {
                continue;
            }

            let x = (glyph.x + ox) as i32;
            let y = (glyph.y + oy) as i32;

            for (row, y) in bitmap.chunks_exact(metrics.width).zip(y..) {
                for (value, x) in row.iter().zip(x..) {
                    let (x, y) = if x < 0 || y < 0 {
                        continue;
                    } else {
                        (x as u32, y as u32)
                    };

                    let value = *value;
                    if value == 0 {
                        continue;
                    }

                    if let Some(pixel) = image.get_pixel(x, y) {
                        *image.pixel_mut(x, y) = pixel.overlay_with_alpha(fill, overlay, value);
                    }
                }
            }
        }
    }
}

impl<'a, P: Pixel> Draw<P> for TextSegment<'a, P> {
    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        render_layout(&mut *image, &[self.font.inner()], &self.layout());
    }
}

/// Represents where text is anchored horizontally.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HorizontalAnchor {
    /// The x position is the left edge of the text. This is the default.
    Left,
    /// The x position is the center of the text. This also center-aligns the text.
    Center,
    /// The x position is the right edge of the text. This also right-aligns the text.
    Right,
}

impl Default for HorizontalAnchor {
    fn default() -> Self {
        Self::Left
    }
}

/// Represents how to align text horizontally within its bounding box.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TextAlign {
    /// Aligns the text to the left.
    Left,
    /// Aligns the text to the center.
    Center,
    /// Aligns the text to the right.
    Right,
}

impl Default for TextAlign {
    fn default() -> Self {
        Self::Left
    }
}

/// Represents where text is anchored vertically.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum VerticalAnchor {
    /// The y position is the top edge of the text. This is the default.
    Top,
    /// The y position is the center of the text.
    Center,
    /// The y position is the bottom edge of the text.
    Bottom,
}

impl Default for VerticalAnchor {
    fn default() -> Self {
        Self::Top
    }
}

/// Represents a high-level text layout that can layout text segments, maybe with different fonts.
///
/// This is a high-level layout that can be used to layout text segments. It can be used to layout
/// text segments with different fonts and styles, and has many features over [`TextSegment`] such
/// as text anchoring, which can be useful for text alignment. This also keeps track of font
/// metrics, meaning that unlike [`TextSegment`], this can be used to determine the width and height
/// of text before rendering it.
///
/// This is less efficient than [`TextSegment`] and you should use [`TextSegment`] if you don't need
/// any of the features [`TextLayout`] provides.
///
/// # Note
/// This is does not implement [`Clone`] and therefore it is not cloneable! Consider using
/// [`TextSegment`] if you require cloning functionality.
pub struct TextLayout<'a, P: Pixel> {
    inner: Layout<(P, OverlayMode)>,
    fonts: Vec<&'a fontdue::Font>,
    settings: LayoutSettings,
    x_anchor: HorizontalAnchor,
    y_anchor: VerticalAnchor,
    align: TextAlign,
}

impl<'a, P: Pixel> TextLayout<'a, P> {
    /// Creates a new text layout with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Layout::new(CoordinateSystem::PositiveYDown),
            fonts: Vec::new(),
            settings: LayoutSettings::default(),
            x_anchor: HorizontalAnchor::default(),
            y_anchor: VerticalAnchor::default(),
            align: TextAlign::default(),
        }
    }

    fn set_settings(&mut self, settings: LayoutSettings) {
        self.inner.reset(&settings);
        self.settings = settings;
    }

    /// Sets the position of the text layout.
    ///
    /// **This must be set before adding any text segments!**
    #[must_use]
    pub fn with_position(mut self, x: u32, y: u32) -> Self {
        self.set_settings(LayoutSettings {
            x: x as f32,
            y: y as f32,
            ..self.settings
        });
        self
    }

    /// Sets the wrapping style of the text. Make sure to also set the wrapping width using
    /// [`with_width`] for wrapping to work.
    ///
    /// **This must be set before adding any text segments!**
    #[must_use]
    pub fn with_wrap(mut self, wrap: WrapStyle) -> Self {
        self.set_settings(LayoutSettings {
            wrap_style: match wrap {
                WrapStyle::None | WrapStyle::Word => fontdue::layout::WrapStyle::Word,
                WrapStyle::Character => fontdue::layout::WrapStyle::Letter,
            },
            max_width: Some(self.settings.max_width.unwrap_or(f32::MAX)),
            ..self.settings
        });
        self
    }

    /// Sets the wrapping width of the text. This does not impact [`Self::width`] and [`Self::dimensions`].
    ///
    /// **This must be set before adding any text segments!**
    #[must_use]
    pub fn with_width(mut self, width: u32) -> Self {
        self.set_settings(LayoutSettings {
            max_width: Some(width as f32),
            ..self.settings
        });
        self
    }

    /// Sets the line height of the text.
    ///
    /// **This must be set before adding any text segments!**
    #[must_use]
    pub fn with_line_height(mut self, line_height: f32) -> Self {
        self.set_settings(LayoutSettings {
            line_height,
            ..self.settings
        });
        self
    }

    /// Adds a text segment to the text layout.
    pub fn push_segment(&mut self, segment: &TextSegment<'a, P>) {
        self.fonts.push(segment.font.inner());
        self.inner.append(
            &self.fonts,
            &TextStyle::with_user_data(
                &segment.text,
                segment.size,
                self.fonts.len() - 1,
                (segment.fill, segment.overlay),
            ),
        );
    }

    /// Takes this text layout and returns it with the given text segment added to the text layout.
    /// Useful for method chaining.
    #[must_use]
    pub fn with_segment(mut self, segment: &TextSegment<'a, P>) -> Self {
        self.push_segment(segment);
        self
    }

    /// Adds basic text to the text layout. This is a convenience method that creates a [`TextSegment`]
    /// with the given font, text, and fill and adds it to the text layout.
    ///
    /// The size of the text is determined by the font's optimal size.
    ///
    /// # Note
    /// The overlay mode is set to [`OverlayMode::Merge`] and not the image's overlay mode, since
    /// anti-aliasing is funky with the replace overlay mode.
    pub fn push_basic_text(&mut self, font: &'a Font, text: impl AsRef<str>, fill: P) {
        self.push_segment(&TextSegment::new(font, text, fill));
    }

    /// Takes this text layout and returns it with the given basic text added to the text layout.
    /// Useful for method chaining.
    ///
    /// # Note
    /// The overlay mode is set to [`OverlayMode::Merge`] and not the image's overlay mode, since
    /// anti-aliasing is funky with the replace overlay mode.
    ///
    /// # See Also
    /// * [`push_basic_text`][TextLayout::push_basic_text]
    #[must_use]
    pub fn with_basic_text(mut self, font: &'a Font, text: impl AsRef<str>, fill: P) -> Self {
        self.push_basic_text(font, text, fill);
        self
    }

    /// Sets the horizontal anchor of the text. The horizontal anchor determines where the x
    /// position of the text is anchored.
    #[must_use]
    pub const fn with_horizontal_anchor(mut self, anchor: HorizontalAnchor) -> Self {
        self.x_anchor = anchor;
        self
    }

    /// Sets the vertical anchor of the text. The vertical anchor determines where the y position of
    /// the text is anchored.
    #[must_use]
    pub const fn with_vertical_anchor(mut self, anchor: VerticalAnchor) -> Self {
        self.y_anchor = anchor;
        self
    }

    /// Sets the horizontal text alignment. This determines how text is aligned within its bounding
    /// box (which is determined by [`Self::width`] and [`Self::height`]).
    #[must_use]
    pub const fn with_align(mut self, align: TextAlign) -> Self {
        self.align = align;
        self
    }

    /// Sets the horizontal anchor and vertial anchor of the text to be centered. This makes the
    /// position of the text be the center as opposed to the top-left corner.
    #[must_use]
    pub const fn centered(self) -> Self {
        self.with_horizontal_anchor(HorizontalAnchor::Center)
            .with_vertical_anchor(VerticalAnchor::Center)
    }

    fn line_widths(&self) -> (Vec<u32>, u32, u32) {
        let glyphs = self.inner.glyphs();
        if glyphs.is_empty() {
            return (Vec::new(), 0, 0);
        }

        let mut widths = Vec::new();
        let mut max_width = 0;

        // SAFETY: checking glyphs.is_empty() above means that glyphs is not empty.
        for line in unsafe { self.inner.lines().unwrap_unchecked() } {
            let x = self.settings.x as u32;

            let glyph = &glyphs[line.glyph_end];
            let right = glyph.x + glyph.width as f32;
            let line_width = (right - x as f32).ceil() as u32;
            widths.push(line_width);
            max_width = max_width.max(line_width);
        }

        (widths, max_width, self.inner.height() as u32)
    }

    /// Returns the width of the text. This is a slightly expensive operation and is not a simple
    /// getter.
    ///
    /// If you want both width and height, use [`dimensions`][TextLayout::dimensions].
    #[must_use]
    pub fn width(&self) -> u32 {
        let glyphs = self.inner.glyphs();
        if glyphs.is_empty() {
            return 0;
        }

        let mut width = 0;

        // SAFETY: checking glyphs.is_empty() above means that glyphs is not empty.
        for line in unsafe { self.inner.lines().unwrap_unchecked() } {
            let x = self.settings.x as u32;

            for glyph in glyphs[line.glyph_start..=line.glyph_end].iter().rev() {
                if glyph.char_data.is_whitespace() {
                    continue;
                }

                let right = glyph.x + glyph.width as f32;
                let line_width = (right - x as f32).ceil() as u32;
                width = width.max(line_width);

                break;
            }
        }

        width
    }

    /// Returns the height of the text.
    ///
    /// If you want both width and height, use [`dimensions`][TextLayout::dimensions].
    #[must_use]
    pub fn height(&self) -> u32 {
        self.inner.height() as u32
    }

    /// Returns the width and height of the text. This is a slightly expensive operation and should
    /// be used sparingly - it is not a simple getter.
    #[must_use]
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Returns the bounding box of the text. Left and top bounds are inclusive; right and bottom
    /// bounds are exclusive.
    #[must_use]
    pub fn bounding_box(&self) -> (u32, u32, u32, u32) {
        let (width, height) = self.dimensions();

        let ox = match self.x_anchor {
            HorizontalAnchor::Left => 0.0,
            HorizontalAnchor::Center => width as f32 / -2.0,
            HorizontalAnchor::Right => -(width as f32),
        };
        let oy = match self.y_anchor {
            VerticalAnchor::Top => 0.0,
            VerticalAnchor::Center => height as f32 / -2.0,
            VerticalAnchor::Bottom => -(height as f32),
        };

        let x = (self.settings.x + ox) as u32;
        let y = (self.settings.y + oy) as u32;

        (x, y, x + width, y + height)
    }

    fn calculate_offsets(&self) -> (Vec<u32>, u32, f32, f32, f32) {
        let (widths, width, height) = self.line_widths();

        let fx = match self.align {
            TextAlign::Left => 0.0,
            TextAlign::Center => 0.5,
            TextAlign::Right => 1.0,
        };
        let ox = match self.x_anchor {
            HorizontalAnchor::Left => 0.0,
            HorizontalAnchor::Center => width as f32 / -2.0,
            HorizontalAnchor::Right => -(width as f32),
        };
        let oy = match self.y_anchor {
            VerticalAnchor::Top => 0.0,
            VerticalAnchor::Center => height as f32 / -2.0,
            VerticalAnchor::Bottom => -(height as f32),
        };

        (widths, width, fx, ox, oy)
    }
}

impl<'a, P: Pixel> Draw<P> for TextLayout<'a, P> {
    fn draw<I: DerefMut<Target = Image<P>>>(&self, mut image: I) {
        let image = &mut *image;

        // Skips the calculation of offsets
        if self.x_anchor == HorizontalAnchor::Left
            && self.y_anchor == VerticalAnchor::Top
            && self.align == TextAlign::Left
        {
            render_layout(image, &self.fonts, &self.inner);
            return;
        }

        let (widths, max_width, fx, ox, oy) = self.calculate_offsets();
        render_layout_with_alignment(
            image,
            &self.fonts,
            &self.inner,
            widths,
            max_width,
            fx,
            ox,
            oy,
        );
    }
}

impl<'a, P: Pixel> Default for TextLayout<'a, P> {
    fn default() -> Self {
        Self::new()
    }
}
