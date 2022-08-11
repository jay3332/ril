//! The Rust Imaging Library. A performant and high-level image processing crate for Rust.
//!
//! See benchmarks and more by viewing the [README](https://github.com/jay3332/ril#ril).
//!
//! # Installation
//! The MSRV (Minimum Support Rust Version) of this crate is **v1.61.0**.
//!
//! Add the following to your `Cargo.toml` dependencies:
//! ```toml
//! ril = "0"
//! ```
//!
//! ## Installing from GitHub
//! You can also use the unstable but latest version by installing from GitHub:
//! ```toml
//! ril = { git = "https://github.com/jay3332/ril", branch = "main" }
//! ```
//!
//! ## Using `cargo add`
//! If you have cargo >= 1.62.0, you can use `cargo add ril`.
//!
//! # Getting Started
//! Import the prelude which brings commonly used types and crucial traits into scope:
//!
//! ```no_run
//! use ril::prelude::*;
//! ```
//!
//! Because all errors from this crate are of the same type, ril provides a `Result` type
//! which you can use in any function that leverages ril, such as the `main` function:
//!
//! ```no_run
//! use ril::prelude::*;
//!
//! fn main() -> ril::Result<()> {
//!     // code goes here...
//!
//!     Ok(())
//! }
//! ```
//!
//! Now you can use the `?` operator on anything that returns a ``Result`` for convenience.
//!
//! # Brief Guide
//! A quick guide and overview of ril's interface.
//!
//! ## Opening an image
//! The [`open`][Image::open] method should suit your needs:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! let image = Image::<Rgb>::open("my_image.png")?;
//! # Ok(()) }
//! ```
//!
//! The sole argument can be anything that implements [`AsRef<Path>`][AsRef], such as
//! a string or a file.
//!
//! You might have also noticed that [`Image`] is generic with one type parameter,
//! which can be anything that implements [`Pixel`]. It represents what type of pixel
//! this image has - in this case, the image has RGB pixels.
//!
//! Common pixel formats are [`Rgb`] (colored) and [`Rgba`] (colored with transparency),
//! which are found in the prelude. There are also grayscale counterparts, such as [`L`].
//!
//! ### Reading from a byte stream
//! You can also read from a byte stream using [`decode_from_bytes`][Image::decode_from_bytes]:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! let bytes = [0; 10000].as_slice(); // Replace this with your own image data
//! let image = Image::<Rgb>::decode_from_bytes(ImageFormat::Png, bytes)?;
//! # Ok(()) }
//! ```
//!
//! The first argument is the encoding of the image, and the second is the byte stream
//! that implements [`Read`][std::io::Read].
//!
//! You can also use [`decode_inferred_from_bytes`][Image::decode_inferred_from_bytes] to
//! infer the format from the byte stream without having to explicitly provide an encoding:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! # let bytes = [0].as_slice();
//! let image = Image::<Rgb>::decode_inferred_from_bytes(bytes)?;
//! # Ok(()) }
//! ```
//!
//! ## Creating your own image
//! You can create your own image using the [`Image::new`][Image::new] method:
//!
//! ```no_run
//! # use ril::prelude::*;
//! let image = Image::new(256, 256, Rgb::new(255, 0, 0));
//! ```
//!
//! The above creates a 256x256 RGB image with all pixels set to red.
//!
//! The first argument is the width of the image, the second is the height, and the third is the
//! fill color. The pixel type of the image can be inferred from this argument, which is why we
//! don't have to specify it explicitly as a type argument - Rust type inference is powerful and
//! infers this for us.
//!
//! ### The `from_fn` method
//! The [`from_fn`][Image::from_fn] method is a shortcut for creating an image from a function:
//!
//! ```no_run
//! # use ril::prelude::*;
//! let image = Image::from_fn(256, 256, |x, y| {
//!     // Do something, maybe with `x` and `y`, and return a pixel
//!     Rgb::new(x as u8, y as u8, 0)
//! });
//! ```
//!
//! The above is just an example. You specify the width, height, and the function that
//! generates the pixels. It should take two parameters - `x` and `y`, which specify the position
//! of the pixel to generate - and return a pixel.
//!
//! ## Encoding and saving images
//! You can encode and save an image to a file with the [`save`][Image::save] method:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! // Here's the red image from before:
//! let image = Image::new(256, 256, Rgb::new(255, 0, 0));
//!
//! image.save(ImageFormat::Png, "output.png")?;
//! # Ok(()) }
//! ```
//!
//! The first argument is the encoding of the image, and the second is the path to the file.
//!
//! You may have noticed this is a bit repetitive, and that it is possible to infer the encoding
//! from the file extension. In cases like this, you can use the slightly slower
//! [`save_inferred`][Image::save_inferred] method:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! # let image = Image::new(256, 256, Rgb::new(255, 0, 0));
//! image.save_inferred("output.png")?;
//! # Ok(()) }
//! ```
//!
//! Now, you do not have to explicitly specify the encoding as it is inferred from the output path.
//!
//! ### Encoding and saving images to memory
//! You can encode images to a memory buffer by using the [`encode`][Image::encode] method:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! # let image = Image::new(256, 256, Rgb::new(255, 0, 0));
//! let mut out = Vec::new();
//! image.encode(ImageFormat::Png, &mut out)?;
//! # Ok(()) }
//!
//! // Do something with `out`
//! ```
//!
//! The first argument is the encoding of the image, and the second is the output buffer that must
//! implement [`Write`][std::io::Write].
//!
//! There is no filename to infer the encoding from, so in this case you have to explicitly
//! specify the encoding.
//!
//! ## Manipulating images
//! Now that you know how to create and save images, let's look at some of the ways we can modify
//! them!
//!
//! ### Inverting images
//! A common manipulation method would be inverting every pixel in the image. To do this, there are
//! two methods which you can use:
//!
//! - [`invert`][Image::invert]: Inverts the image in-place
//! - [`inverted`][Image::inverted]: Consumes the image and returns a new image with the inverted pixels
//!
//! A common pattern you'll see in this crate is that many methods have an in-place method and a
//! not-in-place counterpart, in which the former can be useful for method chaining. One usually does
//! not have any memory or performance benefits than the other.
//!
//! Anyhow, here's how you'd invert an image:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! let mut image = Image::new(256, 256, Rgb::new(255, 0, 0));
//! image.invert();
//! image.save_inferred("output.png")?;
//! # Ok(()) }
//! ```
//!
//! `(255, 0, 0)` (red) inverts to `(0, 255, 255)` (cyan), so that should be the color of the
//! output image.
//!
//! We can also use [`inverted`][Image::inverted] to use method chaining:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! Image::new(256, 256, Rgb::new(255, 0, 0))
//!     .inverted()
//!     .save_inferred("output.png")?;
//! # Ok(()) }
//! ```
//!
//! Seems to be a bit cleaner than the first way, but it really just comes down to preference...
//! and whether or not you have ownership of the image object - you likely want to stay away from
//! cloning images for no benefit as it is a very expensive operation.
//!
//! TODO: finish guide

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
pub mod text;

macro_rules! inline_doc {
    ($($token:item)*) => {
        $(#[doc(inline)] $token)*
    }
}

inline_doc! {
    pub use crate::image::{Banded, Image, ImageFormat, OverlayMode};
    pub use draw::{Border, BorderPosition, Draw, Ellipse, Paste, Rectangle};
    pub use encode::{Decoder, DynamicFrameIterator, Encoder, FrameIterator};
    pub use error::{Error, Result};
    pub use pixel::{Alpha, BitPixel, Dynamic, Pixel, Rgb, Rgba, L};
    pub use resize::FilterType as ResizeAlgorithm;
    pub use sequence::{DisposalMethod, Frame, ImageSequence, LoopCount};
    pub use text::{Font, HorizontalAnchor, TextLayout, TextSegment, VerticalAnchor, WrapStyle};
}

/// The crate prelude exports. Importing this with a wildcard will import most items from RIL that
/// can be useful for image processing, along with bringing crucial traits into scope.
///
/// # Example
/// ```no_run
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
        DynamicFrameIterator, Ellipse, Font, Frame, FrameIterator, HorizontalAnchor, Image,
        ImageFormat, ImageSequence, LoopCount, OverlayMode, Paste, Pixel, Rectangle,
        ResizeAlgorithm, Rgb, Rgba, TextLayout, TextSegment, VerticalAnchor, WrapStyle, L,
    };
}
