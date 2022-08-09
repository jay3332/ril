//! The Rust Imaging Library. A high-level image processing crate for Rust.

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::many_single_char_names
)]

pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;
mod resize;
pub mod sequence;

pub use crate::image::{Banded, Image, ImageFormat, OverlayMode};
pub use draw::{Border, BorderPosition, Draw, Ellipse, Paste, Rectangle};
pub use encode::{Decoder, DynamicFrameIterator, Encoder, FrameIterator};
pub use error::{Error, Result};
pub use pixel::{Alpha, BitPixel, Dynamic, Pixel, Rgb, Rgba, L};
pub use resize::FilterType as ResizeAlgorithm;
pub use sequence::{DisposalMethod, Frame, ImageSequence, LoopCount};

/// The crate prelude exports. Importing this with a wildcard will import most items from RIL that
/// can be useful for image processing, along with bringing crucial traits into scope.
///
/// # Example
/// ```rust
/// use ril::prelude::*;
///
/// // Prelude imported Image and Rgb
/// let image = Image::new(100, 100, Rgb::new(255, 0, 0));
/// // Prelude imported the Banded trait
/// let (r, g, b) = image.bands();
/// ```
pub mod prelude {
    pub use super::{
        Alpha, Banded, BitPixel, Border, BorderPosition, DisposalMethod, Draw, Dynamic,
        DynamicFrameIterator, Ellipse, Frame, FrameIterator, Image, ImageFormat, ImageSequence,
        LoopCount, OverlayMode, Paste, Pixel, Rectangle, ResizeAlgorithm, Rgb, Rgba, L,
    };
}
