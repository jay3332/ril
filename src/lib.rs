pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;

pub use error::{Error, Result};
pub use image::{Image, ImageFormat};
pub use pixel::{Pixel, BitPixel, Dynamic, L, Rgb, Rgba};

pub mod prelude {
    pub use super::{Image, ImageFormat, Pixel, BitPixel, Dynamic, L, Rgb, Rgba};
}
