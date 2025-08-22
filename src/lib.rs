//! The Rust Imaging Library. A performant and high-level image processing crate for Rust.
//!
//! See benchmarks and more by viewing the [README](https://github.com/jay3332/ril#ril). It should
//! also be noted that the README is updated more frequently than the documentation.
//!
//! Similarly, you can see the changelog [here](https://github.com/jay3332/ril/blob/main/CHANGELOG.md).
//!
//! # Installation
//! The MSRV (Minimum Supported Rust Version) of this crate is **v1.61.0**.
//!
//! Add the following to your `Cargo.toml` dependencies:
//! ```toml
//! ril = { version = "0", features = ["all"] ]
//! ```
//!
//! ## Installing from GitHub
//! You can also use the unstable but latest version by installing from GitHub:
//! ```toml
//! ril = { git = "https://github.com/jay3332/ril", branch = "main", features = ["all"] }
//! ```
//!
//! ## Using `cargo add`
//! If you have cargo >= 1.62.0, you can use `cargo add ril --features=all`.
//!
//! ## Cargo Features
//! RIL currently depends on a few dependencies for certain features - especially for various image encodings.
//! By default RIL comes with no encoding dependencies but with the `text` and `resize` dependencies, which give you text
//! and resizing capabilities respectively.
//!
//! You can use the `all` feature to enable all features, including encoding features. This enables the widest range of
//! image format support, but adds a lot of dependencies you may not need.
//!
//! For every image encoding that requires a dependency, a corresponding feature can be enabled for it:
//!
//! | Encoding      | Feature | Dependencies                   | Default? |
//! |---------------|---------|--------------------------------|----------|
//! | PNG and APNG  | `png`   | `png`                          | no       |
//! | JPEG          | `jpeg`  | `jpeg-decoder`, `jpeg-encoder` | no       |
//! | GIF           | `gif`   | `gif`                          | no       |
//! | WebP          | `webp`  | `libwebp-sys2`                 | no       |
//!
//! Other features:
//!
//! | Description                                               | Feature    | Dependencies        | Default? |
//! |-----------------------------------------------------------|------------|---------------------|----------|
//! | Font/Text Rendering                                       | `text`     | `fontdue`           | yes      |
//! | Image Resizing                                            | `resize`   | `fast_image_resize` | yes      |
//! | Color Quantization (using NeuQuant)                       | `quantize` | `color_quant`       | yes      |
//! | Gradients                                                 | `gradient` | `colorgrad`         | yes      |
//! | Enable all features,<br/> including all encoding features | `all`      |                     | no       |
//!
//! ### WebP Support limitations
//! WebP support uses `libwebp`, which is a native library. This means that if you try to use the
//! `webp` feature when compiling to a WebAssembly target, it might fail. We plan on making a
//! pure-Rust port of `libwebp` in the future.
//!
//! For ease of use, the `all-pure` feature is provided, which is the equivalent of `all` minus the
//! `webp` feature.
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
//! which are found in the prelude. There are also grayscale counterparts, such as [`Luma`].
//!
//! ### Reading from a byte stream
//! You can also read from raw bytes using [`from_bytes`][Image::decode_from_bytes]:
//!
//! ```ignore
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! let bytes = include_bytes!("image.png") as &[u8]; // Replace this with your own image data
//! let image = Image::<Rgb>::from_bytes(ImageFormat::Png, bytes)?;
//! # Ok(()) }
//! ```
//!
//! The first argument is the encoding of the image, and the second is a slice of bytes, or anything
//! that implements [`AsRef<[u8]>`].
//!
//! You can also use [`from_bytes_inferred`][Image::from_bytes_inferred] to
//! infer the format from the byte slice without having to explicitly provide an encoding:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! # let bytes = [0].as_slice();
//! let image = Image::<Rgb>::from_bytes_inferred(bytes)?;
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
//! We can also use the [`std::ops::Not`] trait to invert an image:
//!
//! ```no_run
//! # use ril::prelude::*;
//! # fn main() -> ril::Result<()> {
//! let image = Image::new(256, 256, Rgb::new(255, 0, 0));
//! (!image).save_inferred("output.png")?;
//! # Ok(()) }
//! ```
//!
//! Seems to be a bit cleaner than the first way, but it really just comes down to preference...
//! and whether or not you have ownership of the image object; you likely want to stay away from
//! cloning images for no benefit as it is a very expensive operation.
//!
//! TODO: finish guide

#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::module_name_repetitions,
    clippy::similar_names,
    clippy::many_single_char_names,
    clippy::doc_markdown
)]

pub mod colors;
pub mod draw;
pub mod encode;
pub mod encodings;
pub mod error;
pub mod fill;
pub mod filter;
mod format;
#[cfg(feature = "gradient")]
pub mod gradient;
mod image;
pub mod pixel;
pub mod quantize;
#[cfg(feature = "resize")]
mod resize;
pub mod sequence;
#[cfg(feature = "text")]
pub mod text;
pub mod vector;

macro_rules! inline_doc {
    ($($token:item)*) => {
        $(#[doc(inline)] $token)*
    }
}

inline_doc! {
    pub use crate::image::{Banded, Image, OverlayMode};
    pub use draw::{Border, BorderPosition, Draw, Ellipse, Line, Paste, Polygon, Rectangle};
    pub use encode::{Decoder, Encoder, EncoderMetadata, SingleFrameIterator, FrameIterator};
    pub use encodings::ColorType;
    pub use error::{Error, Result};
    #[cfg(feature = "gradient")]
    pub use gradient::{
        BlendMode as GradientBlendMode,
        Interpolation as GradientInterpolation,
        GradientPosition,
        LinearGradient,
        RadialGradient,
        ConicGradient,
        RadialGradientCover,
    };
    pub use fill::{Fill, IntoFill};
    pub use filter::{Convolution, DynamicConvolution};
    pub use format::ImageFormat;
    pub use pixel::{
        Alpha, BitPixel, Dynamic, DynamicSubpixel, Paletted, PalettedRgb, PalettedRgba, Pixel, Rgb,
        Rgba, TrueColor, Luma,
    };
    pub use quantize::Quantizer;
    #[cfg(feature = "resize")]
    pub use resize::FilterType as ResizeAlgorithm;
    pub use sequence::{DisposalMethod, Frame, ImageSequence, LoopCount};
    #[cfg(feature = "text")]
    pub use text::{
        Font, HorizontalAnchor, TextAlign, TextLayout, TextSegment, VerticalAnchor, WrapStyle,
    };
    pub use vector::{FromVector, IntoVector, Vector};
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
        Alpha, Banded, BitPixel, Border, BorderPosition, ColorType, Convolution, Decoder,
        DisposalMethod, Draw, Dynamic, DynamicConvolution, DynamicSubpixel, Ellipse, Encoder,
        EncoderMetadata, Fill, Frame, FrameIterator, FromVector, Image, ImageFormat, ImageSequence,
        IntoFill, IntoVector, Line, LoopCount, Luma, OverlayMode, Paletted, PalettedRgb,
        PalettedRgba, Paste, Pixel, Polygon, Rectangle, Rgb, Rgba, SingleFrameIterator, TrueColor,
        Vector,
    };

    #[cfg(feature = "resize")]
    pub use super::ResizeAlgorithm;
    #[cfg(feature = "gradient")]
    pub use super::{
        ConicGradient, GradientBlendMode, GradientInterpolation, GradientPosition, LinearGradient,
        RadialGradient, RadialGradientCover,
    };
    #[cfg(feature = "text")]
    pub use super::{
        Font, HorizontalAnchor, TextAlign, TextLayout, TextSegment, VerticalAnchor, WrapStyle,
    };
}
