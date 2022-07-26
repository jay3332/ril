use crate::{
    encodings::PixelData,
    Error::{InvalidHexCode, UnsupportedColorType},
    Result,
};

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
        let L(value) = self.clone().into();

        value
    }

    /// Creates this pixel from raw data.
    fn from_pixel_data(data: PixelData) -> Result<Self>;
}

/// Represents a single-bit pixel that represents either a pixel that is on or off.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BitPixel(
    /// Whether the pixel is on.
    pub bool,
);

impl BitPixel {
    /// Returns a new `BitPixel` with the given value.
    pub const fn new(value: bool) -> Self {
        BitPixel(value)
    }

    /// Returns the value of the pixel.
    pub fn value(&self) -> bool {
        self.0
    }

    /// Returns a new `BitPixel` that is on.
    pub const fn on() -> Self {
        BitPixel(true)
    }

    /// Returns a new `BitPixel` that is off.
    pub const fn off() -> Self {
        BitPixel(false)
    }
}

impl Pixel for BitPixel {
    fn alpha(&self) -> u8 {
        255
    }

    fn inverted(&self) -> Self {
        BitPixel(!self.0)
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        // Before, this supported L, however this implicit conversion is not supported anymore
        // as the result will be completely different
        match data {
            PixelData::Bit(value) => Ok(Self(value)),
            _ => Err(UnsupportedColorType),
        }
    }
}

/// Represents an L, or luminance pixel that is stored as only one single
/// number representing how bright, or intense, the pixel is.
///
/// This can be thought of as the "unit channel" as this represents only
/// a single channel in which other pixel types can be composed of.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        match data {
            PixelData::L(value) => Ok(Self(value)),
            // Currently, losing alpha implicitly is allowed, but I may change my mind about this
            // in the future.
            PixelData::LA(value, _) => Ok(Self(value)),
            PixelData::Bit(value) => Ok(Self(value.then_some(255).unwrap_or(0))),
            _ => Err(UnsupportedColorType),
        }
    }
}

impl L {
    /// Creates a new L pixel with the given luminance value.
    pub const fn new(l: u8) -> Self {
        Self(l)
    }

    /// Returns the luminance value of the pixel.
    pub fn value(&self) -> u8 {
        self.0
    }
}

/// Represents an RGB pixel.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        match data {
            PixelData::Rgb(r, g, b) => Ok(Self { r, g, b }),
            PixelData::Rgba(r, g, b, _) => Ok(Self { r, g, b }),
            PixelData::L(l) => Ok(Self { r: l, g: l, b: l }),
            PixelData::LA(l, _) => Ok(Self { r: l, g: l, b: l }),
            PixelData::Bit(value) => Ok(if value { Self::white() } else { Self::black() }),
            _ => Err(UnsupportedColorType),
        }
    }
}

impl Rgb {
    /// Creates a new RGB pixel.
    pub const fn new(r: u8, g: u8, b: u8) -> Rgb {
        Self { r, g, b }
    }

    /// Parses an RGB pixel from a hex value.
    ///
    /// The hex value can be in one of the following formats:
    /// - RRGGBB
    /// - RGB
    ///
    /// These can be optionally padded with #, for example "#FF0000" is the same as as "FF0000".
    pub fn from_hex(hex: impl AsRef<str>) -> Result<Self> {
        let hex = hex.as_ref();

        // Strip # from the hex code
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        // Expand the hex code to 6 characters
        //
        // We can use .len() instead of .chars().count() since it's both faster
        // performance-wise and also because characters of a hex-string will never
        // take up more than one byte.
        let hex = if hex.len() == 3 {
            let mut expanded = String::with_capacity(6);

            for c in hex.chars() {
                expanded.push(c);
                expanded.push(c);
            }

            expanded
        } else if hex.len() != 6 {
            return Err(InvalidHexCode(hex.to_string()));
        } else {
            hex.to_string()
        };

        let err = |_| InvalidHexCode(hex.to_string());

        Ok(Rgb {
            r: u8::from_str_radix(&hex[0..2], 16).map_err(err)?,
            g: u8::from_str_radix(&hex[2..4], 16).map_err(err)?,
            b: u8::from_str_radix(&hex[4..6], 16).map_err(err)?,
        })
    }

    /// Creates a completely black pixel.
    pub const fn black() -> Self {
        Self::new(0, 0, 0)
    }

    /// Creates a completely white pixel.
    pub const fn white() -> Self {
        Self::new(255, 255, 255)
    }
}

/// Represents an RGBA pixel.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
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

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        match data {
            PixelData::Rgb(r, g, b) => Ok(Self { r, g, b, a: 255 }),
            PixelData::Rgba(r, g, b, a) => Ok(Self { r, g, b, a }),
            PixelData::L(l) => Ok(Self {
                r: l,
                g: l,
                b: l,
                a: 255,
            }),
            PixelData::LA(l, a) => Ok(Self {
                r: l,
                g: l,
                b: l,
                a,
            }),
            PixelData::Bit(value) => Ok(if value { Self::white() } else { Self::black() }),
            _ => Err(UnsupportedColorType),
        }
    }
}

impl Rgba {
    /// Creates a new RGBA pixel.
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Rgba {
        Self { r, g, b, a }
    }

    /// Creates an opaque pixel from an RGB pixel.
    pub fn from_rgb(Rgb { r, g, b }: Rgb) -> Self {
        Self::new(r, g, b, 255)
    }

    /// Parses an RGBA pixel from a hex value.
    ///
    /// The hex value can be in one of the following formats:
    /// - RRGGBBAA
    /// - RRGGBB
    /// - RGBA
    /// - RGB
    ///
    /// These can be optionally padded with #, for example "#FF0000" is the same as as "FF0000".
    pub fn from_hex(hex: &str) -> Result<Self> {
        let hex = hex.strip_prefix('#').unwrap_or(hex);

        match hex.len() {
            3 | 6 => Rgb::from_hex(hex).map(Self::from_rgb),
            len @ (4 | 8) => {
                let hex = if len == 4 {
                    let mut expanded = String::with_capacity(8);

                    for c in hex.chars() {
                        expanded.push(c);
                        expanded.push(c);
                    }

                    expanded
                } else {
                    hex.to_string()
                };

                let err = |_| InvalidHexCode(hex.to_string());

                Ok(Self {
                    r: u8::from_str_radix(&hex[0..2], 16).map_err(err)?,
                    g: u8::from_str_radix(&hex[2..4], 16).map_err(err)?,
                    b: u8::from_str_radix(&hex[4..6], 16).map_err(err)?,
                    a: u8::from_str_radix(&hex[6..8], 16).map_err(err)?,
                })
            }
            _ => Err(InvalidHexCode(hex.to_string())),
        }
    }

    /// Creates a completely transparent pixel.
    pub const fn transparent() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Creates an opaque black pixel.
    pub const fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }

    /// Creates an opaque white pixel.
    pub const fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }
}

/// Represents a pixel type that is dynamically resolved.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Dynamic {
    BitPixel(BitPixel),
    L(L),
    Rgb(Rgb),
    Rgba(Rgba),
}

impl Default for Dynamic {
    fn default() -> Self {
        panic!("Dynamic pixel type must be known, try using a concrete pixel type instead");
    }
}

impl Pixel for Dynamic {
    fn alpha(&self) -> u8 {
        match self {
            Dynamic::BitPixel(pixel) => pixel.alpha(),
            Dynamic::L(pixel) => pixel.alpha(),
            Dynamic::Rgb(pixel) => pixel.alpha(),
            Dynamic::Rgba(pixel) => pixel.alpha(),
        }
    }

    fn inverted(&self) -> Self {
        match self {
            Dynamic::BitPixel(pixel) => Dynamic::BitPixel(pixel.inverted()),
            Dynamic::L(pixel) => Dynamic::L(pixel.inverted()),
            Dynamic::Rgb(pixel) => Dynamic::Rgb(pixel.inverted()),
            Dynamic::Rgba(pixel) => Dynamic::Rgba(pixel.inverted()),
        }
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        Ok(match data {
            PixelData::Bit(value) => Dynamic::BitPixel(BitPixel(value)),
            PixelData::L(l) => Dynamic::L(L(l)),
            // TODO: LA pixel type
            PixelData::LA(l, _a) => Dynamic::L(L(l)),
            PixelData::Rgb(r, g, b) => Dynamic::Rgb(Rgb { r, g, b }),
            PixelData::Rgba(r, g, b, a) => Dynamic::Rgba(Rgba { r, g, b, a }),
            _ => return Err(UnsupportedColorType),
        })
    }
}

macro_rules! impl_dynamic {
    ($($t:ident),+) => {
        $(
            impl From<Dynamic> for $t {
                fn from(pixel: Dynamic) -> Self {
                    match pixel {
                        Dynamic::BitPixel(pixel) => pixel.into(),
                        Dynamic::L(pixel) => pixel.into(),
                        Dynamic::Rgb(pixel) => pixel.into(),
                        Dynamic::Rgba(pixel) => pixel.into(),
                    }
                }
            }

            impl From<$t> for Dynamic {
                fn from(pixel: $t) -> Self {
                    Dynamic::$t(pixel)
                }
            }
        )+
    };
}

impl_dynamic!(BitPixel, L, Rgb, Rgba);

impl From<Rgb> for BitPixel {
    fn from(rgb: Rgb) -> Self {
        BitPixel(rgb.luminance() > 127)
    }
}

impl From<Rgba> for BitPixel {
    fn from(rgba: Rgba) -> Self {
        BitPixel(rgba.luminance() > 127)
    }
}

macro_rules! impl_from_bitpixel {
    ($($t:ident),+) => {
        $(
            impl From<BitPixel> for $t {
                fn from(pixel: BitPixel) -> Self {
                    if pixel.value() {
                        <$t>::white()
                    } else {
                        <$t>::black()
                    }
                }
            }
        )+
    };
}

impl_from_bitpixel!(Rgb, Rgba);

impl From<BitPixel> for L {
    fn from(bit: BitPixel) -> Self {
        Self(bit.value().then_some(255).unwrap_or(0))
    }
}

impl From<L> for BitPixel {
    fn from(l: L) -> Self {
        Self(l.value() > 127)
    }
}

impl From<Rgb> for L {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        Self(((r as f32 * 0.299) + (g as f32 * 0.587) + (b as f32 * 0.114)) as u8)
    }
}

impl From<Rgba> for L {
    fn from(rgba: Rgba) -> Self {
        Self(Rgb::from(rgba).luminance())
    }
}

impl From<L> for Rgb {
    fn from(L(l): L) -> Self {
        Self { r: l, g: l, b: l }
    }
}

impl From<L> for Rgba {
    fn from(L(l): L) -> Self {
        Self {
            r: l,
            g: l,
            b: l,
            a: 255,
        }
    }
}

impl From<Rgba> for Rgb {
    fn from(Rgba { r, g, b, .. }: Rgba) -> Self {
        Self { r, g, b }
    }
}

impl From<Rgb> for Rgba {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        Self { r, g, b, a: 255 }
    }
}
