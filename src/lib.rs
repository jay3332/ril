#![allow(clippy::module_name_repetitions)]

pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;

pub use draw::{Border, BorderPosition, Draw, Rectangle};
pub use encode::{ByteStream, Decoder};
pub use error::{Error, Result};
pub use image::{Image, ImageFormat, OverlayMode};
pub use pixel::{BitPixel, Dynamic, Pixel, Rgb, Rgba, L};

pub mod prelude {
    pub use super::{
        BitPixel, Dynamic, Image, ImageFormat, Pixel, Rgb, Rgba, L,
        OverlayMode, Draw, Border, BorderPosition, Rectangle,
    };
}
