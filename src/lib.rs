#![allow(clippy::module_name_repetitions)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;

pub use draw::{Border, BorderPosition, Draw, Paste, Rectangle};
pub use encode::{ByteStream, Decoder};
pub use error::{Error, Result};
pub use image::{Image, ImageFormat, OverlayMode};
pub use pixel::{Alpha, BitPixel, Dynamic, Pixel, Rgb, Rgba, L};

pub mod prelude {
    pub use super::{
        Alpha, BitPixel, Border, BorderPosition, Draw, Dynamic, Image, ImageFormat, OverlayMode,
        Paste, Pixel, Rectangle, Rgb, Rgba, L,
    };
}
