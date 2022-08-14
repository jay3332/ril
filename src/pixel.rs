//! Encloses pixel-related traits and pixel type implementations.

use crate::{
    encodings::PixelData,
    image::OverlayMode,
    Error::{InvalidHexCode, UnsupportedColorType},
    Result,
};

/// Represents any type of pixel in an image.
///
/// Generally speaking, the values enclosed inside of each pixel are designed to be immutable.
pub trait Pixel: Copy + Clone + Default + PartialEq + Eq {
    /// The type of a single component in the pixel.
    type Subpixel;

    /// The iterator type this pixel uses.
    type Data: IntoIterator<Item = u8>;

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
    #[must_use]
    fn inverted(&self) -> Self;

    /// The luminance of the pixel.
    #[must_use]
    fn luminance(&self) -> u8
    where
        Self: Into<L>,
    {
        let L(value) = (*self).into();

        value
    }

    /// Maps the pixel's components and returns a new pixel with the mapped components.
    ///
    /// Alpha is intentionally mapped separately. If no alpha component exists, the alpha function
    /// is ignored.
    #[must_use]
    fn map_subpixels<F, A>(self, f: F, a: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel;

    /// Creates this pixel from raw data.
    ///
    /// # Errors
    /// todo!()
    fn from_pixel_data(data: PixelData) -> Result<Self>;

    /// Creates raw pixel data from this pixel type.
    fn as_pixel_data(&self) -> PixelData;

    /// Creates this pixel from a raw bytes. This is used internally and is unchecked - it panics
    /// if the data is not of the correct length.
    fn from_bytes(bytes: &[u8]) -> Self;

    /// Turns this pixel into bytes.
    fn as_bytes(&self) -> Self::Data;

    /// Merges this pixel with the given overlay pixel, taking into account alpha.
    #[must_use]
    fn merge(self, other: Self) -> Self {
        other
    }

    /// Overlays this pixel with the given overlay pixel, abiding by the given overlay mode.
    #[must_use]
    fn overlay(self, other: Self, mode: OverlayMode) -> Self {
        match mode {
            OverlayMode::Replace => other,
            OverlayMode::Merge => self.merge(other),
        }
    }

    /// Merges this pixel with the given overlay pixel, where the alpha of the overlay pixel is
    /// known. This is used in anti-aliasing.
    #[must_use]
    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self;

    /// Overlays this pixel with the given overlay pixel, abiding by the given overlay mode with
    /// the given alpha.
    ///
    /// This is used in anti-aliasing.
    #[must_use]
    fn overlay_with_alpha(self, other: Self, mode: OverlayMode, alpha: u8) -> Self {
        match mode {
            OverlayMode::Replace => other,
            OverlayMode::Merge => self.merge_with_alpha(other, alpha),
        }
    }

    /// Creates this pixel from any dynamic pixel...dynamically at runtime. Different from the
    /// From/Into traits.
    fn from_dynamic(dynamic: Dynamic) -> Self;
}

/// Represents a pixel that supports alpha, or transparency values.
pub trait Alpha: Pixel {
    /// Returns the alpha, or opacity level of the pixel.
    ///
    /// This is a value between 0 and 255.
    /// 0 is completely transparent, and 255 is completely opaque.
    #[must_use]
    fn alpha(&self) -> u8;

    /// Clones this pixel with the given alpha value.
    #[must_use]
    fn with_alpha(self, alpha: u8) -> Self;
}

/// Represents a single-bit pixel that represents either a pixel that is on or off.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct BitPixel(
    /// Whether the pixel is on.
    pub bool,
);

impl BitPixel {
    /// Returns a new `BitPixel` with the given value.
    #[must_use]
    pub const fn new(value: bool) -> Self {
        Self(value)
    }

    /// Returns the value of the pixel.
    #[must_use]
    pub const fn value(&self) -> bool {
        self.0
    }

    /// Returns a new `BitPixel` that is on.
    #[must_use]
    pub const fn on() -> Self {
        Self(true)
    }

    /// Returns a new `BitPixel` that is off.
    #[must_use]
    pub const fn off() -> Self {
        Self(false)
    }
}

impl Pixel for BitPixel {
    type Subpixel = bool;
    type Data = [u8; 1];

    fn inverted(&self) -> Self {
        Self(!self.0)
    }

    fn map_subpixels<F, A>(self, f: F, _: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        Self(f(self.0))
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        // Before, this supported L, however this implicit conversion is not supported anymore
        // as the result will be completely different
        match data {
            PixelData::Bit(value) => Ok(Self(value)),
            _ => Err(UnsupportedColorType),
        }
    }

    fn as_pixel_data(&self) -> PixelData {
        PixelData::Bit(self.0)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0] > 127)
    }

    fn as_bytes(&self) -> Self::Data {
        [if self.0 { 255 } else { 0 }]
    }

    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
        if alpha < 128 {
            self
        } else {
            other
        }
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        match dynamic {
            Dynamic::BitPixel(value) => value,
            Dynamic::L(value) => value.into(),
            Dynamic::Rgb(value) => value.into(),
            Dynamic::Rgba(value) => value.into(),
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
    type Subpixel = u8;
    type Data = [u8; 1];

    fn inverted(&self) -> Self {
        Self(!self.0)
    }

    fn map_subpixels<F, A>(self, f: F, _: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        Self(f(self.0))
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        match data {
            // Currently, losing alpha implicitly is allowed, but I may change my mind about this
            // in the future.
            PixelData::L(value) | PixelData::LA(value, _) => Ok(Self(value)),
            PixelData::Bit(value) => Ok(Self(if value { 255 } else { 0 })),
            _ => Err(UnsupportedColorType),
        }
    }

    fn as_pixel_data(&self) -> PixelData {
        PixelData::L(self.0)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes[0])
    }

    fn as_bytes(&self) -> Self::Data {
        [self.0]
    }

    #[allow(clippy::cast_lossless)]
    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
        let alpha = alpha as f32 / 255.;
        let base_l = self.0 as f32 / 255.;
        let overlay_l = other.0 as f32 / 255.;

        let a_diff = 1. - alpha;
        let a = a_diff.mul_add(255., alpha);
        let l = (a_diff * 255.).mul_add(base_l, alpha * overlay_l) / a;

        Self((l * 255.) as u8)
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        match dynamic {
            Dynamic::L(value) => value,
            Dynamic::BitPixel(value) => value.into(),
            Dynamic::Rgb(value) => value.into(),
            Dynamic::Rgba(value) => value.into(),
        }
    }
}

impl L {
    /// Creates a new L pixel with the given luminance value.
    #[must_use]
    pub const fn new(l: u8) -> Self {
        Self(l)
    }

    /// Returns the luminance value of the pixel.
    #[must_use]
    pub const fn value(&self) -> u8 {
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
    type Subpixel = u8;
    type Data = [u8; 3];

    fn inverted(&self) -> Self {
        Self {
            r: !self.r,
            g: !self.g,
            b: !self.b,
        }
    }

    fn map_subpixels<F, A>(self, f: F, _: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        Self {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
        }
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        match data {
            PixelData::Rgb(r, g, b) | PixelData::Rgba(r, g, b, _) => Ok(Self { r, g, b }),
            PixelData::L(l) | PixelData::LA(l, _) => Ok(Self { r: l, g: l, b: l }),
            PixelData::Bit(value) => Ok(if value { Self::white() } else { Self::black() }),
            _ => Err(UnsupportedColorType),
        }
    }

    fn as_pixel_data(&self) -> PixelData {
        PixelData::Rgb(self.r, self.g, self.b)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
        }
    }

    fn as_bytes(&self) -> Self::Data {
        [self.r, self.g, self.b]
    }

    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
        Rgba::from_rgb(self)
            .merge_with_alpha(Rgba::from_rgb(other), alpha)
            .into()
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        match dynamic {
            Dynamic::Rgb(value) => value,
            Dynamic::Rgba(value) => value.into(),
            Dynamic::BitPixel(value) => value.into(),
            Dynamic::L(value) => value.into(),
        }
    }
}

impl Rgb {
    /// Creates a new RGB pixel.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parses an RGB pixel from a hex value.
    ///
    /// The hex value can be in one of the following formats:
    /// - RRGGBB
    /// - RGB
    ///
    /// These can be optionally padded with #, for example "#FF0000" is the same as as "FF0000".
    ///
    /// # Errors
    /// * Received a malformed hex code.
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

        Ok(Self {
            r: u8::from_str_radix(&hex[0..2], 16).map_err(err)?,
            g: u8::from_str_radix(&hex[2..4], 16).map_err(err)?,
            b: u8::from_str_radix(&hex[4..6], 16).map_err(err)?,
        })
    }

    /// Creates a completely black pixel.
    #[must_use]
    pub const fn black() -> Self {
        Self::new(0, 0, 0)
    }

    /// Creates a completely white pixel.
    #[must_use]
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
    type Subpixel = u8;
    type Data = [u8; 4];

    fn inverted(&self) -> Self {
        Self {
            r: !self.r,
            g: !self.g,
            b: !self.b,
            a: self.a,
        }
    }

    fn map_subpixels<F, A>(self, f: F, a: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        Self {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
            a: a(self.a),
        }
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        #[allow(clippy::match_wildcard_for_single_variants)]
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
            PixelData::Bit(value) => Ok(value.then(Self::white).unwrap_or_else(Self::black)),
            _ => Err(UnsupportedColorType),
        }
    }

    fn as_pixel_data(&self) -> PixelData {
        PixelData::Rgba(self.r, self.g, self.b, self.a)
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            r: bytes[0],
            g: bytes[1],
            b: bytes[2],
            a: bytes[3],
        }
    }

    fn as_bytes(&self) -> Self::Data {
        [self.r, self.g, self.b, self.a]
    }

    #[allow(clippy::cast_lossless)]
    fn merge(self, other: Self) -> Self {
        // Optimize for common cases
        if other.a == 255 {
            return other;
        } else if other.a == 0 {
            return self;
        }

        let (base_r, base_g, base_b, base_a) = (
            self.r as f32 / 255.,
            self.g as f32 / 255.,
            self.b as f32 / 255.,
            self.a as f32 / 255.,
        );

        let (overlay_r, overlay_g, overlay_b, overlay_a) = (
            other.r as f32 / 255.,
            other.g as f32 / 255.,
            other.b as f32 / 255.,
            other.a as f32 / 255.,
        );

        let a_diff = 1. - overlay_a;
        let a = a_diff.mul_add(base_a, overlay_a);

        let a_ratio = a_diff * base_a;
        let r = a_ratio.mul_add(base_r, overlay_a * overlay_r) / a;
        let g = a_ratio.mul_add(base_g, overlay_a * overlay_g) / a;
        let b = a_ratio.mul_add(base_b, overlay_a * overlay_b) / a;

        Self {
            r: (r * 255.) as u8,
            g: (g * 255.) as u8,
            b: (b * 255.) as u8,
            a: (a * 255.) as u8,
        }
    }

    #[allow(clippy::cast_lossless)]
    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
        self.merge(other.with_alpha((other.a as f32 * (alpha as f32 / 255.)) as u8))
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        match dynamic {
            Dynamic::Rgba(value) => value,
            Dynamic::Rgb(value) => value.into(),
            Dynamic::L(value) => value.into(),
            Dynamic::BitPixel(value) => value.into(),
        }
    }
}

impl Alpha for Rgba {
    fn alpha(&self) -> u8 {
        self.a
    }

    fn with_alpha(mut self, alpha: u8) -> Self {
        self.a = alpha;
        self
    }
}

impl Rgba {
    /// Creates a new RGBA pixel.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    /// Creates an opaque pixel from an RGB pixel.
    #[must_use]
    pub const fn from_rgb(Rgb { r, g, b }: Rgb) -> Self {
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
    ///
    /// # Errors
    /// * Received a malformed hex code.
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
    #[must_use]
    pub const fn transparent() -> Self {
        Self::new(0, 0, 0, 0)
    }

    /// Creates an opaque black pixel.
    #[must_use]
    pub const fn black() -> Self {
        Self::new(0, 0, 0, 255)
    }

    /// Creates an opaque white pixel.
    #[must_use]
    pub const fn white() -> Self {
        Self::new(255, 255, 255, 255)
    }
}

/// Represents a subpixel of a dynamic pixel.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DynamicSubpixel {
    /// A u8 subpixel.
    U8(u8),
    /// A boolean subpixel.
    Bool(bool),
}

macro_rules! impl_num_op {
    ($err:literal; $self:expr, $other:expr; $a:ident, $b:ident; $out:expr) => {{
        match ($self, $other) {
            (DynamicSubpixel::U8($a), DynamicSubpixel::U8($b)) => DynamicSubpixel::U8($out),
            (DynamicSubpixel::Bool(_), DynamicSubpixel::Bool(_)) => panic!(
                "cannot {} DynamicPixel boolean variants. You should try converting to a \
                    concrete pixel type so these runtime panics are not triggered.",
                $err,
            ),
            _ => panic!(
                "cannot {} different or incompatible DynamicSubpixel variants. You should \
                    try converting to a concrete pixel type so these runtime panics are not \
                    triggered.",
                $err,
            ),
        }
    }};
}

impl std::ops::Add for DynamicSubpixel {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        impl_num_op!("add"; self, other; a, b; a + b)
    }
}

impl std::ops::AddAssign for DynamicSubpixel {
    fn add_assign(&mut self, other: Self) {
        *self = *self + other;
    }
}

impl std::ops::Sub for DynamicSubpixel {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        impl_num_op!("subtract"; self, other; a, b; a - b)
    }
}

impl std::ops::SubAssign for DynamicSubpixel {
    fn sub_assign(&mut self, other: Self) {
        *self = *self - other;
    }
}

impl std::ops::Mul for DynamicSubpixel {
    type Output = Self;

    fn mul(self, other: Self) -> Self {
        impl_num_op!("multiply"; self, other; a, b; a * b)
    }
}

impl std::ops::MulAssign for DynamicSubpixel {
    fn mul_assign(&mut self, other: Self) {
        *self = *self * other;
    }
}

impl std::ops::Div for DynamicSubpixel {
    type Output = Self;

    fn div(self, other: Self) -> Self {
        impl_num_op!("divide"; self, other; a, b; a / b)
    }
}

impl std::ops::DivAssign for DynamicSubpixel {
    fn div_assign(&mut self, other: Self) {
        *self = *self / other;
    }
}

impl std::ops::Rem for DynamicSubpixel {
    type Output = Self;
    fn rem(self, other: Self) -> Self {
        impl_num_op!("remainder"; self, other; a, b; a % b)
    }
}

impl std::ops::RemAssign for DynamicSubpixel {
    fn rem_assign(&mut self, other: Self) {
        *self = *self % other;
    }
}

impl num_traits::SaturatingAdd for DynamicSubpixel {
    fn saturating_add(&self, v: &Self) -> Self {
        impl_num_op!("saturating_add"; self, v; a, b; a.saturating_add(b))
    }
}

impl num_traits::SaturatingSub for DynamicSubpixel {
    fn saturating_sub(&self, v: &Self) -> Self {
        impl_num_op!("saturating_sub"; self, v; a, b; a.saturating_sub(b))
    }
}

impl num_traits::SaturatingMul for DynamicSubpixel {
    fn saturating_mul(&self, v: &Self) -> Self {
        impl_num_op!("saturating_mul"; self, v; a, b; a.saturating_mul(b))
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
        panic!(
            "Dynamic pixel type must be known, try using a concrete pixel type instead so these \
                runtime panics are not triggered."
        );
    }
}

impl Pixel for Dynamic {
    type Subpixel = DynamicSubpixel;
    type Data = Vec<u8>;

    fn inverted(&self) -> Self {
        match self {
            Self::BitPixel(pixel) => Self::BitPixel(pixel.inverted()),
            Self::L(pixel) => Self::L(pixel.inverted()),
            Self::Rgb(pixel) => Self::Rgb(pixel.inverted()),
            Self::Rgba(pixel) => Self::Rgba(pixel.inverted()),
        }
    }

    // noinspection ALL
    fn map_subpixels<F, A>(self, f: F, a: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        macro_rules! subpixel {
            ($pixel:expr, $variant:ident) => {{
                ($pixel).map_subpixels(
                    |pixel| match f(DynamicSubpixel::$variant(pixel)) {
                        DynamicSubpixel::$variant(pixel) => pixel,
                        _ => panic!("dynamic subpixel map returned something different"),
                    },
                    |alpha| match a(DynamicSubpixel::$variant(alpha)) {
                        DynamicSubpixel::$variant(alpha) => alpha,
                        _ => panic!("dynamic subpixel map returned something different"),
                    },
                )
            }};
        }

        match self {
            Self::BitPixel(pixel) => Self::BitPixel(subpixel!(pixel, Bool)),
            Self::L(pixel) => Self::L(subpixel!(pixel, U8)),
            Self::Rgb(pixel) => Self::Rgb(subpixel!(pixel, U8)),
            Self::Rgba(pixel) => Self::Rgba(subpixel!(pixel, U8)),
        }
    }

    fn from_pixel_data(data: PixelData) -> Result<Self> {
        #[allow(clippy::match_wildcard_for_single_variants)]
        Ok(match data {
            PixelData::Bit(value) => Self::BitPixel(BitPixel(value)),
            PixelData::L(l) => Self::L(L(l)),
            // TODO: LA pixel type
            PixelData::LA(l, _a) => Self::L(L(l)),
            PixelData::Rgb(r, g, b) => Self::Rgb(Rgb { r, g, b }),
            PixelData::Rgba(r, g, b, a) => Self::Rgba(Rgba { r, g, b, a }),
            _ => return Err(UnsupportedColorType),
        })
    }

    fn as_pixel_data(&self) -> PixelData {
        match *self {
            Self::BitPixel(BitPixel(value)) => PixelData::Bit(value),
            Self::L(L(l)) => PixelData::L(l),
            Self::Rgb(Rgb { r, g, b }) => PixelData::Rgb(r, g, b),
            Self::Rgba(Rgba { r, g, b, a }) => PixelData::Rgba(r, g, b, a),
        }
    }

    fn from_bytes(bytes: &[u8]) -> Self {
        match bytes.len() {
            1 => Self::L(Pixel::from_bytes(bytes)),
            3 => Self::Rgb(Pixel::from_bytes(bytes)),
            4 => Self::Rgba(Pixel::from_bytes(bytes)),
            _ => panic!("Invalid pixel data length"),
        }
    }

    fn as_bytes(&self) -> Self::Data {
        self.as_pixel_data().data()
    }

    fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
        match (self, other) {
            (Self::BitPixel(pixel), Self::BitPixel(other)) => {
                Self::BitPixel(pixel.merge_with_alpha(other, alpha))
            }
            (Self::L(pixel), Self::L(other)) => Self::L(pixel.merge_with_alpha(other, alpha)),
            (Self::Rgb(pixel), Self::Rgb(other)) => Self::Rgb(pixel.merge_with_alpha(other, alpha)),
            (Self::Rgba(pixel), Self::Rgba(other)) => {
                Self::Rgba(pixel.merge_with_alpha(other, alpha))
            }
            _ => panic!("Cannot overlay two foreign pixel types"),
        }
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        dynamic
    }
}

impl Alpha for Dynamic {
    fn alpha(&self) -> u8 {
        match self {
            Self::Rgba(pixel) => pixel.alpha(),
            _ => 255,
        }
    }

    fn with_alpha(self, alpha: u8) -> Self {
        match self {
            Self::Rgba(pixel) => Self::Rgba(pixel.with_alpha(alpha)),
            pixel => pixel,
        }
    }
}

impl Dynamic {
    /// Creates a new dynamic pixel...dynamically, from a concrete pixel.
    ///
    /// # Errors
    /// * The pixel type is not supported.
    pub fn from_pixel<P: Pixel>(pixel: P) -> Result<Self> {
        Self::from_pixel_data(pixel.as_pixel_data())
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
        Self(rgb.luminance() > 127)
    }
}

impl From<Rgba> for BitPixel {
    fn from(rgba: Rgba) -> Self {
        Self(rgba.luminance() > 127)
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
        Self(if bit.value() { 255 } else { 0 })
    }
}

impl From<L> for BitPixel {
    fn from(l: L) -> Self {
        Self(l.value() > 127)
    }
}

impl From<Rgb> for L {
    fn from(Rgb { r, g, b }: Rgb) -> Self {
        #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
        Self(f32::from(b).mul_add(0.114, f32::from(r).mul_add(0.299, f32::from(g) * 0.587)) as u8)
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

/// A trait representing all pixels that can be represented as either RGB or RGBA true color.
pub trait TrueColor {
    /// Returns the pixel as an (r, g, b) tuple.
    fn as_rgb_tuple(&self) -> (u8, u8, u8);
    
    /// Returns the pixel as an (r, g, b, a) tuple.
    fn as_rgba_tuple(&self) -> (u8, u8, u8, u8);
    
    /// Creates a new pixel from an (r, g, b) tuple.
    fn from_rgb_tuple(rgb: (u8, u8, u8)) -> Self;
    
    /// Creates a new pixel from an (r, g, b, a) tuple.
    fn from_rgba_tuple(rgba: (u8, u8, u8, u8)) -> Self;
}

impl<P: Copy + From<Rgb> + From<Rgba> + Into<Rgb> + Into<Rgba>> TrueColor for P {
    fn as_rgb_tuple(&self) -> (u8, u8, u8) {
        let Rgb { r, g, b } = (*self).into();
        
        (r, g, b)
    }
    
    fn as_rgba_tuple(&self) -> (u8, u8, u8, u8) {
        let Rgba { r, g, b, a } = (*self).into();
        
        (r, g, b, a)
    }
    
    fn from_rgb_tuple((r, g, b): (u8, u8, u8)) -> Self {
        From::<Rgb>::from(Rgb { r, g, b })
    }
    
    fn from_rgba_tuple((r, g, b, a): (u8, u8, u8, u8)) -> Self {
        From::<Rgba>::from(Rgba { r, g, b, a })
    }
} 
