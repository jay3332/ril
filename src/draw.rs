use crate::{Image, Pixel};

pub trait Draw<P: Pixel> {
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
    pub fn new(color: P, thickness: u32) -> Self {
        Self {
            color,
            thickness,
            position: BorderPosition::default(),
        }
    }

    #[must_use]
    pub fn with_color(mut self, color: P) -> Self {
        self.color = color;
        self
    }

    #[must_use]
    pub fn with_thickness(mut self, thickness: u32) -> Self {
        self.thickness = thickness;
        self
    }

    #[must_use]
    pub fn with_position(mut self, position: BorderPosition) -> Self {
        self.position = position;
        self
    }
}

#[derive(Clone, Debug, Default)]
pub struct Rectangle<P: Pixel> {
    pub position: (u32, u32),
    pub size: (u32, u32),
    pub border: Option<Border<P>>,
    pub fill: P,
}

impl<P: Pixel> Rectangle<P> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_position(mut self, x: u32, y: u32) -> Self {
        self.position = (x, y);
        self
    }

    pub fn with_size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    pub fn with_border(mut self, border: Border<P>) -> Self {
        self.border = Some(border);
        self
    }

    pub fn with_fill(mut self, fill: P) -> Self {
        self.fill = fill;
        self
    }
}

impl<P: Pixel> Draw<P> for Rectangle<P> {
    fn draw(&self, image: &mut Image<P>) {
        todo!()
    }
}
