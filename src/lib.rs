#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;
pub mod resize;
pub mod sequence;

pub use draw::{Border, BorderPosition, Draw, Ellipse, Paste, Rectangle};
pub use encode::{Decoder, Encoder, FrameIterator, DynamicFrameIterator};
pub use error::{Error, Result};
pub use image::{Banded, Image, ImageFormat, OverlayMode};
pub use pixel::{Alpha, BitPixel, Dynamic, Pixel, Rgb, Rgba, L};
pub use resize::FilterType as ResizeAlgorithm;
pub use sequence::{DisposalMethod, Frame, ImageSequence, LoopCount};

pub mod prelude {
    pub use super::{
        Alpha, Banded, BitPixel, Border, BorderPosition, Draw, Dynamic, Ellipse, Image,
        ImageFormat, OverlayMode, Paste, Pixel, Rectangle, ResizeAlgorithm, Rgb, Rgba, L,
        ImageSequence, Frame, DisposalMethod, LoopCount, FrameIterator, DynamicFrameIterator,
    };
}
