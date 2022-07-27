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
#[derive(Clone, Debug)]
pub struct Border<P: Pixel> {
    /// The color of the border.
    color: P,
    /// The thickness of the border, in pixels.
    thickness: u32,
    /// The position of the border.
    position: BorderPosition,
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
    pub const fn color(&self) -> P {
        self.color
    }

    #[must_use]
    pub const fn thickness(&self) -> u32 {
        self.thickness
    }

    #[must_use]
    pub const fn position(&self) -> BorderPosition {
        self.position
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

#[derive(Clone, Debug)]
pub struct Rectangle<P: Pixel> {
    position: (u32, u32),
    size: (u32, u32),
    border: Option<Border<P>>,
    fill: P,
}
