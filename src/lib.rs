pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod image;
pub mod pixel;

pub use error::{Error, Result};
pub use image::{Image, ImageFormat};
pub use pixel::{BitPixel, Dynamic, Pixel, Rgb, Rgba, L};

pub mod prelude {
    pub use super::{BitPixel, Dynamic, Image, ImageFormat, Pixel, Rgb, Rgba, L};
}
