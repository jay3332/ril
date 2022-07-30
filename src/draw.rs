use crate::image::OverlayMode;
use crate::{Image, Pixel};

pub trait Draw<P: Pixel> {
    /// Draws the object to the given image.
    fn draw(&self, image: &mut Image<P>);
}

/// Represents whether a border is inset, outset, or if it lays in the center.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderPosition {
    /// An inset border. May overlap the contents of inside the shape.
    Inset,
    /// A border that is balanced between the inside and outside of the shape.
    Center,
    /// An outset border. May overlap the contents of outside the shape. This is the default
    /// behavior because it is usually what you would expect.
    #[default]
    Outset,
}

/// Represents a shape border.
///
/// TODO: Add support for rounded borders
#[derive(Clone, Debug, Default)]
pub struct Border<P: Pixel> {
    /// The color of the border.
    pub color: P,
    /// The thickness of the border, in pixels.
    pub thickness: u32,
    /// The position of the border.
    pub position: BorderPosition,
}

impl<P: Pixel> Border<P> {
    /// todo!()
    /// 
    /// # Panics
    /// * Panics if border thickness is greater than 0.
    pub fn new(color: P, thickness: u32) -> Self {
        assert_ne!(thickness, 0, "border thickness cannot be 0");

        Self {
            color,
            thickness,
            position: BorderPosition::default(),
        }
    }

    #[must_use]
    pub const fn with_color(mut self, color: P) -> Self {
        self.color = color;
        self
    }

    /// todo!()
    /// 
    /// # Panics
    /// * Panics if border thickness is greater than 0.
    #[must_use]
    pub fn with_thickness(mut self, thickness: u32) -> Self {
        assert_ne!(thickness, 0, "border thickness cannot be 0");
        self.thickness = thickness;
        self
    }

    #[must_use]
    pub const fn with_position(mut self, position: BorderPosition) -> Self {
        self.position = position;
        self
    }
}

/// A rectangle.
///
/// Using any of the predefined construction methods will automatically set the position to
/// `(0, 0)`. If you want to specify a different position, use the `with_position` method.
///
/// # Note
/// You must specify a width and height for the rectangle with something such as [`with_size`].
/// If you don't, a panic will be raised during drawing. You can also try using
/// [`from_bounding_box`] to create a rectangle from a bounding box, which automatically fills
/// in the size.
///
/// Additionally, a panic will be raised during drawing if you do not specify either a fill color
/// or a border. these can be set with [`with_fill`] and [`with_border`] subsequently.
#[derive(Clone, Debug, Default)]
pub struct Rectangle<P: Pixel> {
    pub position: (u32, u32),
    pub size: (u32, u32),
    pub border: Option<Border<P>>,
    pub fill: Option<P>,
    pub overlay: Option<OverlayMode>,
}

impl<P: Pixel> Rectangle<P> {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// todo!()
    /// 
    /// # Panics
    /// * Panics if the bounding box is invalid.
    #[must_use]
    pub fn from_bounding_box(x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        assert!(x2 >= x1, "invalid bounding box");
        assert!(y2 >= y1, "invalid bounding box");

        Self::default()
            .with_position(x1, y1)
            .with_size(x2 - x1, y2 - y1)
    }

    #[must_use]
    pub const fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    #[must_use]
    pub const fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    #[must_use]
    pub const fn with_border(mut self, border: Border<P>) -> Self {
        self.border = Some(border);
        self
    }

    #[must_use]
    pub const fn with_fill(mut self, fill: P) -> Self {
        self.fill = Some(fill);
        self
    }

    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = Some(mode);
        self
    }
}

impl<P: Pixel> Draw<P> for Rectangle<P> {
    fn draw(&self, image: &mut Image<P>) {
        assert!(
            self.fill.is_some() || self.border.is_some(),
            "must provide one of either fill or border, try calling .with_fill()"
        );
        assert!(
            self.size.0 > 0 && self.size.1 > 0,
            "rectangle must have a non-zero width and height, have you called .with_size() yet?"
        );

        let (x1, y1) = self.position;
        let (w, h) = self.size;
        let (x2, y2) = (x1 + w, y1 + h);
        let overlay = self.overlay.unwrap_or(image.overlay);

        let border = self.border.as_ref().map(
            |Border {
                 color,
                 thickness,
                 position,
             }| {
                let (inner, outer) = match position {
                    BorderPosition::Outset => (0_u32, *thickness),
                    BorderPosition::Inset => (*thickness, 0),
                    BorderPosition::Center => {
                        let offset = thickness / 2;
                        // This way, the two will still sum to offset
                        (offset, thickness - offset)
                    }
                };

                (inner, outer, color)
            },
        );

        image.map_in_place(|x, y, pixel| {
            if let Some((inner, outer, color)) = border {
                if x < x1 + inner
                    && x >= x1 - outer
                    && y >= y1 - outer
                    && y <= y2 + outer
                    // Right border
                    || x > x2 - inner
                    && x <= x2 + outer
                    && y >= y1 - outer
                    && y <= y2 + outer
                    // Top border
                    || y < y1 + inner
                    && y >= y1 - outer
                    && x >= x1
                    && x <= x2
                    // Bottom border
                    || y > y2 - inner
                    && y <= y2 + outer
                    && x >= x1
                    && x <= x2
                {
                    *pixel = pixel.overlay(*color, overlay);
                    return;
                }
            }

            if let Some(fill) = self.fill {
                if x >= x1 && x <= x2 && y >= y1 && y <= y2 {
                    *pixel = pixel.overlay(fill, overlay);
                }
            }
        });
    }
}
