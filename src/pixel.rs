//! Encloses pixel-related traits and pixel type implementations.

use crate::Error::DecodingError;
use crate::{
    encodings::ColorType,
    image::OverlayMode,
    Error::{InvalidHexCode, InvalidPaletteIndex, UnsupportedColorType},
    Result,
};
use std::borrow::Cow;
use std::fmt::{self, Debug, Formatter};
use std::hash::Hash;

mod sealed {
    use super::{BitPixel, Dynamic, NoOp, PalettedRgb, PalettedRgba, Rgb, Rgba, L};
    pub trait Sealed {}

    macro_rules! sealed {
        ($($t:ty)+) => {
            $(impl Sealed for $t {})+
        };
    }

    pub trait MaybeSealed {
        const SEALED: bool = false;
    }

    impl<P: Sealed> MaybeSealed for P {
        const SEALED: bool = true;
    }

    sealed!(NoOp BitPixel L Rgb Rgba Dynamic PalettedRgb<'_> PalettedRgba<'_>);
}

pub(crate) use sealed::MaybeSealed;

/// Represents any type of pixel in an image.
///
/// Generally speaking, the values enclosed inside of each pixel are designed to be immutable.
pub trait Pixel: Copy + Clone + Debug + Default + PartialEq + Eq + Hash + MaybeSealed {
    /// The color type of the pixel.
    const COLOR_TYPE: ColorType;

    /// The bit depth of the pixel.
    const BIT_DEPTH: u8;

    /// The type of a single component in the pixel.
    type Subpixel: Copy + Into<usize>;

    /// The resolved color type of the palette. This is `Self` for non-paletted pixels.
    type Color: Pixel;

    /// The iterator type this pixel uses.
    type Data: IntoIterator<Item = u8> + AsRef<[u8]>;

    /// Resolves the color type of this pixel at runtime. This is used for dynamic color types.
    /// If you are certain the pixel is not dynamic, you can use the [`Self::COLOR_TYPE`] constant
    /// instead.
    fn color_type(&self) -> ColorType {
        Self::COLOR_TYPE
    }

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

    /// Creates this pixel from the given color type, bit depth, and data. This may require a lossy
    /// conversion.
    ///
    /// # Errors
    /// * If the color type is not supported by the pixel type.
    /// * An error occurs when trying to convert the data to the pixel type.
    fn from_raw_parts(color_type: ColorType, bit_depth: u8, data: &[u8]) -> Result<Self> {
        Self::from_raw_parts_paletted::<NoOp>(color_type, bit_depth, data, None)
    }

    /// Creates this pixel from the given color type, bit depth, data, and possibly a color palette.
    /// This may require a lossy xonversion.
    ///
    /// A palette should be supplied if the color type is paletted, else `None`.
    ///
    /// # Errors
    /// * If the color type is not supported by the pixel type.
    /// * An error occurs when trying to convert the data to the pixel type.
    #[allow(unused_variables)]
    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        if color_type != Self::COLOR_TYPE {
            return Err(UnsupportedColorType);
        }
        if bit_depth != Self::BIT_DEPTH {
            return Err(UnsupportedColorType);
        }
        Ok(Self::from_bytes(data))
    }

    // noinspection RsConstantConditionIf
    /// Creates this pixel from the given palette and index, but the conversion is done at runtime.
    ///
    /// # Errors
    /// * The pixel index is invalid/out of bounds.
    /// * If the color type is not supported by the pixel type.
    /// * An error occurs when trying to convert the data to the pixel type.
    fn from_arbitrary_palette<P: Pixel>(palette: &[P], index: usize) -> Result<Self> {
        let pixel = palette.get(index).ok_or(InvalidPaletteIndex)?;

        if P::SEALED && P::COLOR_TYPE == ColorType::Dynamic {
            // SAFETY: upheld by the `Sealed` trait
            Ok(Self::from_dynamic(unsafe { *(pixel as *const P).cast() }))
        } else {
            Self::from_raw_parts(P::COLOR_TYPE, P::BIT_DEPTH, pixel.as_bytes().as_ref())
        }
    }

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

    /// Creates this pixel from any dynamic pixel, dynamically at runtime. Different from the
    /// From/Into traits.
    #[allow(unused_variables)]
    #[must_use]
    fn from_dynamic(dynamic: Dynamic) -> Self {
        panic!("cannot convert from dynamic pixel for this pixel type");
    }

    /// Returns this pixel as RGB despite its type. This can panic on some pixel types, you must
    /// be sure this pixel is able to be converted into RGB before using this.
    ///
    /// You should use [`Rgb::from`] or ensure that the pixel is [`TrueColor`], as they are safer
    /// methods, checked at compile time. This is primarily used for internal purposes, for example when an encoder
    /// can guarantee that a pixel is convertable into RGB.
    ///
    /// # Panics
    /// * If the pixel is not convertable into RGB.
    fn as_rgb(&self) -> Rgb;

    /// Returns this pixel as RGBA despite its type. This can panic on some pixel types, you must
    /// be sure this pixel is able to be converted into RGBA before using this.
    ///
    /// You should use [`Rgba::from`] or ensure that the pixel is [`TrueColor`], as they are safer
    /// methods, checked at compile time. This is primarily used for internal purposes, for example
    /// when an encoder can guarantee that a pixel is convertable into RGBA.
    ///
    /// # Panics
    /// * If the pixel is not convertable into RGBA.
    fn as_rgba(&self) -> Rgba;
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

/// A pixel type that does and stores nothing. This pixel type is useless and will behave weirdly
/// with your code. This is usually only used for internal or polyfill purposes.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NoOp;

/// Extension of [`NoOp`], used for internal purposes only. This is a ZST that implements
/// `Into<u8>`.
#[derive(Copy, Clone)]
pub struct NoOpSubpixel;

impl From<NoOpSubpixel> for usize {
    fn from(_: NoOpSubpixel) -> Self {
        0
    }
}

impl Pixel for NoOp {
    const COLOR_TYPE: ColorType = ColorType::L;
    const BIT_DEPTH: u8 = 0;

    type Subpixel = NoOpSubpixel;
    type Color = Self;
    type Data = [u8; 0];

    fn inverted(&self) -> Self {
        Self
    }

    fn map_subpixels<F, A>(self, _f: F, _a: A) -> Self
    where
        F: Fn(Self::Subpixel) -> Self::Subpixel,
        A: Fn(Self::Subpixel) -> Self::Subpixel,
    {
        Self
    }

    fn from_bytes(_bytes: &[u8]) -> Self {
        Self
    }

    fn as_bytes(&self) -> Self::Data {
        []
    }

    fn merge_with_alpha(self, _other: Self, _alpha: u8) -> Self {
        Self
    }

    fn from_dynamic(_dynamic: Dynamic) -> Self {
        Self
    }

    fn as_rgb(&self) -> Rgb {
        panic!("NoOp is a private pixel type and should not be used")
    }

    fn as_rgba(&self) -> Rgba {
        panic!("NoOp is a private pixel type and should not be used")
    }
}

impl Alpha for NoOp {
    fn alpha(&self) -> u8 {
        255
    }

    fn with_alpha(self, _alpha: u8) -> Self {
        Self
    }
}

impl TrueColor for NoOp {
    fn as_rgb_tuple(&self) -> (u8, u8, u8) {
        (0, 0, 0)
    }

    fn as_rgba_tuple(&self) -> (u8, u8, u8, u8) {
        (0, 0, 0, 0)
    }

    fn from_rgb_tuple(_: (u8, u8, u8)) -> Self {
        Self
    }

    fn from_rgba_tuple(_: (u8, u8, u8, u8)) -> Self {
        Self
    }

    fn into_rgb(self) -> Rgb {
        Rgb::default()
    }

    fn into_rgba(self) -> Rgba {
        Rgba::default()
    }
}

macro_rules! propagate_palette {
    ($p:expr, $data:expr) => {{
        if let Some(palette) = $p {
            return Self::from_arbitrary_palette(palette, $data[0] as usize);
        }
    }};
}

/// Represents a single-bit pixel that represents either a pixel that is on or off.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
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

macro_rules! force_into_impl {
    () => {
        fn as_rgb(&self) -> Rgb {
            (*self).into()
        }

        fn as_rgba(&self) -> Rgba {
            (*self).into()
        }
    };
}

impl Pixel for BitPixel {
    const COLOR_TYPE: ColorType = ColorType::L;
    const BIT_DEPTH: u8 = 1;

    type Subpixel = bool;
    type Color = Self;
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

    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        propagate_palette!(palette, data);
        // Before, this supported L, however this implicit conversion is not supported anymore
        // as the result will be completely different
        match (color_type, bit_depth, data.is_empty()) {
            (_, 1, false) => Ok(Self(data[0] != 0)),
            _ => Err(UnsupportedColorType),
        }
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

    force_into_impl!();
}

macro_rules! scale_subpixels {
    ($src_depth:expr, $target_depth:expr, $data:expr) => {{
        if $src_depth == $target_depth {
            Cow::from($data)
        } else {
            if !$src_depth.is_power_of_two() {
                return Err(DecodingError(format!(
                    "source depth {} is not a power of two",
                    $src_depth
                )));
            }
            debug_assert!(
                $target_depth.is_power_of_two(),
                "target depth {} is not a power of two",
                $target_depth,
            );

            if $src_depth <= 8 && $target_depth <= 8 {
                Cow::from(if $src_depth < $target_depth {
                    let scale = $target_depth / $src_depth;
                    $data.iter().map(|n| *n * scale).collect::<Vec<_>>()
                } else {
                    let scale = $src_depth / $target_depth;
                    $data.iter().map(|n| *n / scale).collect::<Vec<_>>()
                })
            } else if $src_depth < $target_depth {
                let scale = $target_depth as usize / $src_depth as usize;
                let mut result = Vec::with_capacity($data.len() * scale);

                for n in $data {
                    result.extend((*n as usize * scale).to_be_bytes());
                }

                Cow::from(result)
            } else {
                let scale = $src_depth as usize / $target_depth as usize;
                let mut result = Vec::with_capacity($data.len() / scale);

                for chunk in $data.chunks_exact(scale) {
                    let sum = chunk
                        .iter()
                        .rev()
                        .enumerate()
                        .map(|(i, &x)| (x as usize) << (8 * i))
                        .sum::<usize>();
                    result.push((sum / scale) as u8);
                }

                Cow::from(result)
            }
        }
    }};
}

macro_rules! propagate_data {
    ($data:expr, $expected:expr) => {{
        if $data.len() < $expected {
            return Err(DecodingError(format!(
                "malformed pixel data for {}: expected at least {} component(s) but received {}",
                std::any::type_name::<Self>(),
                $expected,
                $data.len(),
            )));
        }
    }};
    ($data:expr) => {{
        propagate_data!($data, Self::COLOR_TYPE.channels());
    }};
}

/// Represents an L, or luminance pixel that is stored as only one single
/// number representing how bright, or intense, the pixel is.
///
/// This can be thought of as the "unit channel" as this represents only
/// a single channel in which other pixel types can be composed of.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct L(
    /// The luminance value of the pixel, between 0 and 255.
    pub u8,
);

impl Pixel for L {
    const COLOR_TYPE: ColorType = ColorType::L;
    const BIT_DEPTH: u8 = 8;

    type Subpixel = u8;
    type Color = Self;
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

    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        propagate_palette!(palette, data);

        let data = scale_subpixels!(bit_depth, Self::BIT_DEPTH, data);
        propagate_data!(data);

        match color_type {
            // Currently, losing alpha implicitly is allowed, but I may change my mind about this
            // in the future.
            ColorType::L | ColorType::LA => Ok(Self(data[0])),
            _ => Err(UnsupportedColorType),
        }
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

    force_into_impl!();
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
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    /// The red component of the pixel.
    pub r: u8,
    /// The green component of the pixel.
    pub g: u8,
    /// The blue component of the pixel.
    pub b: u8,
}

impl Pixel for Rgb {
    const COLOR_TYPE: ColorType = ColorType::Rgb;
    const BIT_DEPTH: u8 = 8;

    type Subpixel = u8;
    type Color = Self;
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

    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        propagate_palette!(palette, data);
        let data = scale_subpixels!(bit_depth, Self::BIT_DEPTH, data);

        match color_type {
            ColorType::Rgb | ColorType::Rgba => {
                propagate_data!(data, 3);
                Ok(Self {
                    r: data[0],
                    g: data[1],
                    b: data[2],
                })
            }
            ColorType::L | ColorType::LA => {
                propagate_data!(data, 1);
                Ok(Self {
                    r: data[0],
                    g: data[0],
                    b: data[0],
                })
            }
            _ => Err(UnsupportedColorType),
        }
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

    force_into_impl!();
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
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
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
    const COLOR_TYPE: ColorType = ColorType::Rgba;
    const BIT_DEPTH: u8 = 8;

    type Subpixel = u8;
    type Color = Self;
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

    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        propagate_palette!(palette, data);
        let data = scale_subpixels!(bit_depth, Self::BIT_DEPTH, data);

        match color_type {
            ColorType::Rgb => {
                propagate_data!(data, 3);
                Ok(Self {
                    r: data[0],
                    g: data[1],
                    b: data[2],
                    a: 255,
                })
            }
            ColorType::Rgba => {
                propagate_data!(data, 4);
                Ok(Self {
                    r: data[0],
                    g: data[1],
                    b: data[2],
                    a: data[3],
                })
            }
            ColorType::L => {
                propagate_data!(data, 1);
                Ok(Self {
                    r: data[0],
                    g: data[0],
                    b: data[0],
                    a: 255,
                })
            }
            ColorType::LA => {
                propagate_data!(data, 2);
                Ok(Self {
                    r: data[0],
                    g: data[0],
                    b: data[0],
                    a: data[1],
                })
            }
            _ => Err(UnsupportedColorType),
        }
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

    // TODO: SIMD could speed this up significantly
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

    fn overlay_with_alpha(self, other: Self, mode: OverlayMode, alpha: u8) -> Self {
        match mode {
            OverlayMode::Replace => other.with_alpha(alpha),
            OverlayMode::Merge => self.merge_with_alpha(other, alpha),
        }
    }

    fn from_dynamic(dynamic: Dynamic) -> Self {
        match dynamic {
            Dynamic::Rgba(value) => value,
            Dynamic::Rgb(value) => value.into(),
            Dynamic::L(value) => value.into(),
            Dynamic::BitPixel(value) => value.into(),
        }
    }

    force_into_impl!();
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

impl From<DynamicSubpixel> for usize {
    fn from(v: DynamicSubpixel) -> Self {
        match v {
            DynamicSubpixel::U8(v) => v.into(),
            DynamicSubpixel::Bool(v) => v.into(),
        }
    }
}

/// Represents a pixel type that is dynamically resolved.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
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
    const COLOR_TYPE: ColorType = ColorType::Dynamic;
    const BIT_DEPTH: u8 = 8;

    type Subpixel = DynamicSubpixel;
    type Color = Self;
    type Data = Vec<u8>;

    fn color_type(&self) -> ColorType {
        match self {
            Self::BitPixel(_) | Self::L(_) => ColorType::L,
            Self::Rgb(_) => ColorType::Rgb,
            Self::Rgba(_) => ColorType::Rgba,
        }
    }

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

    fn from_raw_parts_paletted<P: Pixel>(
        color_type: ColorType,
        bit_depth: u8,
        data: &[u8],
        palette: Option<&[P]>,
    ) -> Result<Self> {
        propagate_palette!(palette, data);

        Ok(if bit_depth == 1 {
            propagate_data!(data, 1);
            Self::BitPixel(BitPixel(data[0] != 0))
        } else {
            let data = scale_subpixels!(bit_depth, Self::BIT_DEPTH, data);

            match color_type {
                ColorType::L | ColorType::LA => {
                    propagate_data!(data, 1);
                    Self::L(L(data[0]))
                }
                ColorType::Rgb => {
                    propagate_data!(data, 3);
                    Self::Rgb(Rgb {
                        r: data[0],
                        g: data[1],
                        b: data[2],
                    })
                }
                ColorType::Rgba => {
                    propagate_data!(data, 4);
                    Self::Rgba(Rgba {
                        r: data[0],
                        g: data[1],
                        b: data[2],
                        a: data[3],
                    })
                }
                _ => return Err(UnsupportedColorType),
            }
        })
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
        match self {
            Self::BitPixel(pixel) => pixel.as_bytes().to_vec(),
            Self::L(pixel) => pixel.as_bytes().to_vec(),
            Self::Rgb(pixel) => pixel.as_bytes().to_vec(),
            Self::Rgba(pixel) => pixel.as_bytes().to_vec(),
        }
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

    force_into_impl!();
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
        Self::from_raw_parts(pixel.color_type(), P::BIT_DEPTH, pixel.as_bytes().as_ref())
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

/// A trait representing all pixels that can be safely represented as either RGB or RGBA true color.
pub trait TrueColor: Pixel {
    /// Returns the pixel as an (r, g, b) tuple.
    fn as_rgb_tuple(&self) -> (u8, u8, u8);

    /// Returns the pixel as an (r, g, b, a) tuple.
    fn as_rgba_tuple(&self) -> (u8, u8, u8, u8);

    /// Creates a new pixel from an (r, g, b) tuple.
    fn from_rgb_tuple(rgb: (u8, u8, u8)) -> Self;

    /// Creates a new pixel from an (r, g, b, a) tuple.
    fn from_rgba_tuple(rgba: (u8, u8, u8, u8)) -> Self;

    /// Returns the pixel casted into an Rgb pixel.
    fn into_rgb(self) -> Rgb;

    /// Returns the pixel casted into an Rgba pixel.
    fn into_rgba(self) -> Rgba;
}

#[allow(clippy::trait_duplication_in_bounds)]
impl<P: Pixel + Copy + From<Rgb> + From<Rgba> + Into<Rgb> + Into<Rgba>> TrueColor for P {
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

    fn into_rgb(self) -> Rgb {
        self.into()
    }

    fn into_rgba(self) -> Rgba {
        self.into()
    }
}

#[allow(dead_code)]
pub(crate) unsafe fn assume_pixel_from_palette<'p, P, Index: Into<usize>>(
    palette: &'p [P::Color],
    index: Index,
) -> Result<P>
where
    P: 'p + Pixel,
{
    macro_rules! unsafe_cast {
        ($t:ty => $out:ty) => {{
            let length = palette.len();
            let ptr = palette.as_ptr().cast::<$t>();
            // SAFETY: upheld by the caller
            let palette = std::slice::from_raw_parts(ptr, length);

            // SAFETY: mostly upheld by the caller, but transmute_copy can be used since all Pixels
            // implement Copy.
            Ok(std::mem::transmute_copy(&<$out>::from_palette(
                palette,
                index.into() as u8,
            )))
        }};
    }

    match P::COLOR_TYPE {
        ColorType::PaletteRgb => unsafe_cast!(Rgb => PalettedRgb),
        ColorType::PaletteRgba => unsafe_cast!(Rgba => PalettedRgba),
        _ => P::from_arbitrary_palette(palette, index.into()),
    }
}

/// A trait representing a paletted pixel. [`Pixel::Subpixel`] is the type of the palette index.
///
/// The generic lifetime parameter `'p` represents the lifetime of a palette the type will hold a
/// reference to.
pub trait Paletted<'p>: Pixel
where
    Self: 'p,
{
    /// Creates this pixel from the given palette and index. For unpaletted pixels, use
    /// [`Pixel::from_arbitrary_palette`] instead.
    ///
    /// # Errors
    /// * The pixel index is invalid/out of bounds.
    /// * If the color type is not supported by the pixel type.
    /// * An error occurs when trying to convert the data to the pixel type.
    fn from_palette(palette: &'p [Self::Color], index: Self::Subpixel) -> Self;

    /// Returns the palette lookup as a slice.
    fn palette(&self) -> &'p [Self::Color];

    /// Returns the index in the palette this pixel is of.
    fn palette_index(&self) -> Self::Subpixel;

    /// Resolves the color of the pixel. Because invalid palette values are supposed to be
    /// propagated prior to calling this, this will panic
    // TODO: this could potentially just use an index to avoid the expect
    fn color(&self) -> Self::Color {
        *self
            .palette()
            .get(self.palette_index().into())
            .expect("invalid palette index")
    }

    /// Resolves the color of the pixel. Invalid palette values *should* be propagated prior to
    /// to calling this, but it isn't guaranteed, for example if the palette pixel was manually
    /// initialized.
    ///
    /// This results in undefined behavior if the palette index is invalid.
    ///
    /// # Safety
    /// * The palette index must be valid.
    unsafe fn color_unchecked(&self) -> Self::Color {
        *self.palette().get_unchecked(self.palette_index().into())
    }
}

macro_rules! impl_palette_default {
    ($($t:ty),+) => {
        $(
            impl Default for $t {
                fn default() -> Self {
                    panic!("cannot use default for paletted pixel");
                }
            }
        )+
    }
}

macro_rules! try_palette {
    ($self:ident, $action:literal, $filter:expr) => {{
        Self {
            index: $self.palette().iter().position($filter).unwrap_or_else(|| {
                panic!(
                    "could not find a color in the palette with the same value once {}, \
                     try converting this pixel or image to a true color format first. paletted \
                     pixels can only transform to other colors in the same palette.",
                    $action,
                )
            }) as u8,
            palette: $self.palette,
        }
    }};
}

macro_rules! panic_unpaletted {
    () => {{
        panic!(
            "currently, pixels are resolved independent from other pixels, meaning that RIL \
                cannot quantize non-paletted pixels into a palette, since it must be aware of all \
                other pixels in the image. Consider manually quantizing the image, or using a \
                non-paletted pixel format, such as RGBA, instead - RIL can automatically flatten \
                paletted images into true color images, but not the opposite.",
        )
    }};
}

macro_rules! impl_palette_cast {
    ($t:ty: $($f:ty)+) => {
        $(
            impl From<$t> for $f {
                fn from(pixel: $t) -> Self {
                    pixel.color().into()
                }
            }
        )+
    };
}

// Suffixed with an 8 to indicate that it's an 8-bit palette
macro_rules! impl_palette8 {
    ($name:ident: $color_type:ident $cast:ident $tgt:ty) => {
        #[derive(Copy, Clone, PartialEq, Eq, Hash)]
        #[doc = concat!(
            "Represents a paletted pixel, holding an index to a palette of ",
            stringify!($tgt),
            " colors represented as a `&'p [",
            stringify!($tgt),
            "]`, where `'p` is the lifetime of the palette.",
        )]
        pub struct $name<'p> {
            pub index: u8,
            palette: &'p [$tgt],
        }

        impl fmt::Debug for $name<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_struct(stringify!($name))
                    .field("index", &self.index)
                    .finish()
            }
        }

        impl_palette_default!($name<'_>);

        impl Pixel for $name<'_> {
            const COLOR_TYPE: ColorType = ColorType::$color_type;
            const BIT_DEPTH: u8 = 8;

            type Subpixel = u8;
            type Color = $tgt;
            type Data = [u8; 1];

            fn inverted(&self) -> Self {
                let target = &self.color().inverted();

                try_palette!(self, "inverted", |color| color == target)
            }

            fn map_subpixels<F, A>(self, f: F, a: A) -> Self
            where
                F: Fn(Self::Subpixel) -> Self::Subpixel,
                A: Fn(Self::Subpixel) -> Self::Subpixel,
            {
                let target = &self.color().map_subpixels(f, a);

                try_palette!(self, "mapped", |color| color == target)
            }

            fn from_raw_parts_paletted<P: Pixel>(
                _color_type: ColorType,
                _bit_depth: u8,
                _data: &[u8],
                _palette: Option<&[P]>,
            ) -> Result<Self> {
                panic_unpaletted!()
            }

            fn from_bytes(_bytes: &[u8]) -> Self {
                panic!("cannot initialize a paletted pixel without being aware of its palette")
            }

            fn as_bytes(&self) -> Self::Data {
                [self.index]
            }

            fn merge_with_alpha(self, other: Self, alpha: u8) -> Self {
                let target = &self.color().merge_with_alpha(other.color(), alpha);

                try_palette!(self, "merged", |color| color == target)
            }

            fn from_dynamic(_dynamic: Dynamic) -> Self {
                todo!("implement dynamic palettes")
            }

            force_into_impl!();
        }

        impl<'p> Paletted<'p> for $name<'p> {
            fn from_palette(palette: &'p [Self::Color], index: Self::Subpixel) -> Self {
                Self {
                    index: usize::from(index) as u8,
                    palette,
                }
            }

            fn palette(&self) -> &'p [Self::Color] {
                self.palette
            }

            fn palette_index(&self) -> u8 {
                self.index
            }
        }

        impl_palette_cast!($name<'_>: Rgb Rgba L BitPixel Dynamic);
    }
}

impl_palette8!(PalettedRgb: PaletteRgb into_rgb Rgb);
impl_palette8!(PalettedRgba: PaletteRgba into_rgba Rgba);
