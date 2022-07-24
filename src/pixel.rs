/// Represents any type of pixel in an image.
pub trait Pixel: Copy + Clone + Default + PartialEq + Eq {
    /// Returns the alpha, or opacity level of the pixel.
    ///
    /// This is a value between 0 and 255.
    /// 0 is completely transparent, and 255 is completely opaque.
    fn alpha(&self) -> u8;

    /// Returns the inverted value of this pixel.
    ///
    /// This does not independently invert the alpha value, instead you may need to
    /// split the image and isolate the alpha channel into an Image<L>, invert it, then merge the
    /// bands.
    ///
    /// # Why not invert alpha?
    /// In most cases the alpha channel is favored to be not inverted.
    ///
    /// In the case that a pixel is completely transparent, this behavior is almost always the case.
    /// The user cannot actually see the color of the pixel, so inverting it to a seemingly
    /// unknown color could be unexpected.
    ///
    /// For translucent pixels, in most cases the expected behavior is to only invert the color
    /// shown for the translucent pixel and not change the alpha itself.
    ///
    /// Finally, the most obvious reasoning is that for fully opaque pixels, those pixels will
    /// become fully transparent which is obviously not favored.
    fn inverted(&self) -> Self;

    /// The luminance of the pixel.
    fn luminance(&self) -> u8
    where
        Self: Into<L>,
    {
        let L(value) = self.into();

        value
    }
}

/// Represents a single-bit pixel that represents either a pixel that is on or off.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct BitPixel(
    /// Whether the pixel is on.
    pub bool,
);

impl BitPixel {
    /// Returns a new `BitPixel` with the given value.
    pub fn new(value: bool) -> Self {
        BitPixel(value)
    }

    /// Returns the value of the pixel.
    pub fn value(&self) -> bool {
        self.0
    }
}

impl Pixel for BitPixel {
    fn alpha(&self) -> u8 {
        255
    }

    fn inverted(&self) -> Self {
        BitPixel(!self.0)
    }
}

/// Represents an L, or luminance pixel that is stored as only one single
/// number representing how bright, or intense, the pixel is.
///
/// This can be thought of as the "unit channel" as this represents only
/// a single channel in which other pixel types can be composed of.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct L(
    /// The luminance value of the pixel, between 0 and 255.
    pub u8,
);

impl Pixel for L {
    fn alpha(&self) -> u8 {
        255
    }

    fn inverted(&self) -> Self {
        Self(255 - self.0)
    }
}

impl L {
    /// Creates a new L pixel with the given luminance value.
    pub fn new(l: u8) -> Self {
        Self(l)
    }

    /// Returns the luminance value of the pixel.
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Represents an RGB pixel.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Rgb {
    /// The red component of the pixel.
    pub r: u8,
    /// The green component of the pixel.
    pub g: u8,
    /// The blue component of the pixel.
    pub b: u8,
}

impl Pixel for Rgb {
    fn alpha(&self) -> u8 {
        255
    }

    fn inverted(&self) -> Self {
        Self {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
        }
    }
}

impl Rgb {
    /// Creates a new RGB pixel.
    pub fn new(r: u8, g: u8, b: u8) -> Rgb {
        Self { r, g, b }
    }

    /// Creates a completely black pixel.
    pub fn black() -> Self {
        Self::default()
    }

    /// Creates a completely white pixel.
    pub fn white() -> Self {
        Self::new(255, 255, 255)
    }
}

/// Represents an RGBA pixel.
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct Rgba {
    /// The red component of the pixel.
    pub r: u8,
    /// The green component of the pixel.
    pub g: u8,
    /// The blue component of the pixel.
    pub b: u8,
    /// The alpha component of the pixel.
    pub a: u8,
}

impl Pixel for Rgba {
    fn alpha(&self) -> u8 {
        self.a
    }

    fn inverted(&self) -> Self {
        Rgba {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
            a: self.a,
        }
    }
}

impl Rgba {
    /// Creates a new RGBA pixel.
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Self { r, g, b, a }
    }

    /// Creates an opaque pixel from an RGB pixel.
    pub fn from_rgb(Rgb { r, g, b }: Rgb) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Creates a completely transparent pixel.
    pub fn transparent() -> Self {
        Self::default()
    }

    /// Creates an opaque black pixel.
    pub fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }

    /// Creates an opaque white pixel.
    pub fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }
}

impl From<Rgb> for BitPixel {
    fn from(rgb: Rgb) -> Self {
        BitPixel(rgb.luminance() > 127)
    }
}

impl From<BitPixel> for Rgb {
    fn from(bit: BitPixel) -> Self {
        if bit.value() {
            Rgb::new(255, 255, 255)
        } else {
            Rgb::new(0, 0, 0)
        }
    }
}

impl From<Rgb> for L {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        Self(((r as f32 * 0.299) + (g as f32 * 0.587) + (b as f32 * 0.114)) as u8)
    }
}

impl From<L> for Rgb {
    fn from(L(l): L) -> Self {
        Self { r: l, g: l, b: l }
    }
}

impl From<Rgba> for Rgb {
    fn from(Rgba { r, g, b, .. }: Rgba) -> Rgb {
        Self { r, g, b }
    }
}

impl<T: Pixel + Into<Rgb>> From<T> for Rgba {
    fn from(p: T) -> Rgba {
        let a = p.alpha();
        let Rgb { r, g, b } = p.into();

        Self { r, g, b, a }
    }
}
