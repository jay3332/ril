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
pub mod resize;
pub mod sequence;

pub use draw::{Border, BorderPosition, Draw, Ellipse, Paste, Rectangle};
pub use encode::{Decoder, DynamicFrameIterator, Encoder, FrameIterator};
pub use error::{Error, Result};
pub use image::{Banded, Image, ImageFormat, OverlayMode};
pub use pixel::{Alpha, BitPixel, Dynamic, Pixel, Rgb, Rgba, L};
pub use resize::FilterType as ResizeAlgorithm;
pub use sequence::{DisposalMethod, Frame, ImageSequence, LoopCount};

pub mod prelude {
    pub use super::{
        Alpha, Banded, BitPixel, Border, BorderPosition, DisposalMethod, Draw, Dynamic,
        DynamicFrameIterator, Ellipse, Frame, FrameIterator, Image, ImageFormat, ImageSequence,
        LoopCount, OverlayMode, Paste, Pixel, Rectangle, ResizeAlgorithm, Rgb, Rgba, L,
    };
}
