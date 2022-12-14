//! Contains encoder and decoder implementations for various image formats.

#[cfg(feature = "gif")]
pub mod gif;
#[cfg(feature = "jpeg")]
pub mod jpeg;
#[cfg(feature = "png")]
pub mod png;
#[cfg(feature = "webp")]
pub mod webp;

/// Represents an arbitrary color type. Note that this does not store the bit-depth or the type used
/// to store the value of each channel, although it can specify the number of channels.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorType {
    /// A single-channel pixel that holds one value, typically representing luminance. Typically
    /// used for grayscale images.
    L,
    /// A two-channel [`L`][Self::L] pixel that holds an additional alpha value, for transparency.
    LA,
    /// A three-channel pixel that holds red, green, and blue values. This is a common pixel type
    /// used for true-color images.
    Rgb,
    /// A four-channel pixel that holds red, green, blue, and alpha values. This is a common pixel
    /// used for true-color images with transparency.
    Rgba,
    /// A single-channel pixel that holds an index into a palette of RGB colors.
    PaletteRgb,
    /// A single-channel pixel that holds an index into a palette of RGBA colors.
    PaletteRgba,
    /// Dynamic color type that can be used to store any color type. The bit depth of all color
    /// types this can represent should be the same, for example an 8-bit dynamic pixel cannot
    /// represent Rgb16.
    ///
    /// This is never used during decoding. When encoding, this should not be resolved statically
    /// but instead during runtime, where the color type is known.
    Dynamic,
}

impl ColorType {
    /// Returns the number of channels in this color type.
    #[must_use]
    pub const fn channels(&self) -> usize {
        match self {
            Self::L | Self::PaletteRgb | Self::PaletteRgba => 1,
            Self::LA => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
            Self::Dynamic => 0,
        }
    }

    /// Returns whether this color type can have a dynamic alpha value.
    #[must_use]
    pub const fn has_alpha(&self) -> bool {
        matches!(
            self,
            Self::LA | Self::Rgba | Self::PaletteRgba | Self::Dynamic
        )
    }

    #[must_use]
    pub const fn is_paletted(&self) -> bool {
        matches!(self, Self::PaletteRgb | Self::PaletteRgba)
    }

    #[must_use]
    pub const fn is_dynamic(&self) -> bool {
        matches!(self, Self::Dynamic)
    }
}
