#![allow(clippy::wildcard_imports)]

use crate::{
    draw::Draw,
    error::{
        Error::{self, InvalidExtension},
        Result,
    },
    pixel::*,
    Dynamic, DynamicFrameIterator,
};

#[cfg(feature = "gif")]
use crate::encodings::gif;
#[cfg(feature = "jpeg")]
use crate::encodings::jpeg;
#[cfg(feature = "png")]
use crate::encodings::png;
#[cfg(feature = "qoi")]
use crate::encodings::qoi;
#[cfg(feature = "webp")]
use crate::encodings::webp;
#[cfg(feature = "resize")]
use crate::ResizeAlgorithm;
#[cfg(any(
    feature = "png",
    feature = "gif",
    feature = "jpeg",
    feature = "webp",
    feature = "qoi"
))]
use crate::{Decoder, Encoder};

use num_traits::{SaturatingAdd, SaturatingSub};
use std::{
    ffi::OsStr,
    fmt::{self, Display},
    fs::File,
    io::{Read, Write},
    num::NonZeroU32,
    path::Path,
};

/// The behavior to use when overlaying images on top of each other.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum OverlayMode {
    /// Replace alpha values with the alpha values of the overlay image. This is the default
    /// behavior.
    Replace,
    /// Merge the alpha values of overlay image with the alpha values of the base image.
    Merge,
}

impl Default for OverlayMode {
    fn default() -> Self {
        Self::Replace
    }
}

impl Display for OverlayMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Merge => write!(f, "merge"),
            Self::Replace => write!(f, "replace"),
        }
    }
}

/// A high-level image representation.
///
/// This represents a static, single-frame image.
/// See [`ImageSequence`] for information on opening animated or multi-frame images.
#[derive(Clone)]
pub struct Image<P: Pixel = Dynamic> {
    pub(crate) width: NonZeroU32,
    pub(crate) height: NonZeroU32,
    /// A 1-dimensional vector of pixels representing all pixels in the image. This is shaped
    /// according to the image's width and height to form the image.
    ///
    /// This data is a low-level, raw representation of the image. You can see the various pixel
    /// mapping functions, or use the [`pixels`] method directly for higher level representations
    /// of the data.
    pub data: Vec<P>,
    pub(crate) format: ImageFormat,
    pub(crate) overlay: OverlayMode,
    pub(crate) palette: Option<Box<[P::Color]>>,
}

macro_rules! assert_nonzero {
    ($width:expr) => {{
        debug_assert_ne!($width, 0, "width must be non-zero");
    }};
    ($width:expr, $height:expr) => {{
        assert_nonzero!($width);
        debug_assert_ne!($height, 0, "height must be non-zero");
    }};
}

impl<P: Pixel> Image<P> {
    /// Creates a new image with the given width and height, with all pixels being set
    /// initially to `fill`.
    ///
    /// Both the width and height must be non-zero, or else this will panic. You should validate
    /// the width and height before calling this function.
    ///
    /// # Panics
    /// * `width` or `height` is zero.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// // 16x16 RGB image with all pixels set to white
    /// let image = Image::new(16, 16, Rgb::white());
    ///
    /// assert_eq!(image.width(), 16);
    /// assert_eq!(image.height(), 16);
    /// assert_eq!(image.pixel(0, 0), &Rgb::white());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn new(width: u32, height: u32, fill: P) -> Self {
        assert_nonzero!(width, height);

        Self {
            width: NonZeroU32::new(width).unwrap(),
            height: NonZeroU32::new(height).unwrap(),
            data: vec![fill; (width * height) as usize],
            format: ImageFormat::default(),
            overlay: OverlayMode::default(),
            palette: None,
        }
    }

    /// Creates a new image with the given width and height. The pixels are then resolved through
    /// then given callback function which takes two parameters - the x and y coordinates of
    /// a pixel - and returns a pixel.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let gradient = Image::from_fn(256, 256, |x, _y| L(x as u8));
    ///
    /// assert_eq!(gradient.pixel(0, 0), &L(0));
    /// assert_eq!(gradient.pixel(255, 0), &L(255));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn from_fn(width: u32, height: u32, f: impl Fn(u32, u32) -> P) -> Self {
        Self::new(width, height, P::default()).map_pixels_with_coords(|x, y, _| f(x, y))
    }

    /// Creates a new image shaped with the given width and a 1-dimensional sequence of pixels
    /// which will be shaped according to the width.
    ///
    /// # Panics
    /// * The length of the pixels is not a multiple of the width.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::from_pixels(2, &[L(0), L(1), L(2), L(3)]);
    ///
    /// assert_eq!(image.width(), 2);
    /// assert_eq!(image.height(), 2);
    /// assert_eq!(image.pixel(1, 1), &L(3));
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn from_pixels(width: u32, pixels: impl AsRef<[P]>) -> Self {
        assert_nonzero!(width);
        let pixels = pixels.as_ref();

        assert_eq!(
            pixels.len() % width as usize,
            0,
            "length of pixels must be a multiple of the image width",
        );

        Self {
            width: NonZeroU32::new(width).unwrap(),
            // SAFETY: We have already asserted the width being non-zero above with addition to
            // the height being a multiple of the width, meaning that the height cannot be zero.
            height: unsafe { NonZeroU32::new_unchecked(pixels.len() as u32 / width) },
            data: pixels.to_vec(),
            format: ImageFormat::default(),
            overlay: OverlayMode::default(),
            palette: None,
        }
    }

    /// Creates a new image shaped with the given width and a 1-dimensional sequence of paletted
    /// pixels which will be shaped according to the width.
    ///
    /// # Panics
    /// * The length of the pixels is not a multiple of the width.
    /// * The palette is empty.
    /// * The a pixel index is out of bounds with regards to the palette.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::<PalettedRgb>::from_paletted_pixels(
    ///     2,
    ///     vec![Rgb::white(), Rgb::black()],
    ///     &[0, 1, 0, 1],
    /// );
    /// assert_eq!(image.pixel(1, 1).color(), Rgb::black());
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn from_paletted_pixels<'p>(
        width: u32,
        palette: impl ToOwned<Owned = Vec<P::Color>> + 'p,
        pixels: impl AsRef<[P::Subpixel]>,
    ) -> Self
    where
        P: Paletted<'p>,
    {
        assert_nonzero!(width);

        let pixels = pixels.as_ref();
        debug_assert_eq!(
            pixels.len() % width as usize,
            0,
            "length of pixels must be a multiple of the image width",
        );
        #[allow(clippy::redundant_clone)]
        let palette = palette.to_owned().into_boxed_slice();
        debug_assert!(!palette.is_empty(), "palette must not be empty");

        let mut slf = Self {
            width: NonZeroU32::new(width).unwrap(),
            // SAFETY: We have already asserted the width being non-zero above with addition to
            // the height being a multiple of the width, meaning that the height cannot be zero.
            height: unsafe { NonZeroU32::new_unchecked(pixels.len() as u32 / width) },
            data: Vec::new(),
            format: ImageFormat::default(),
            overlay: OverlayMode::default(),
            palette: Some(palette),
        };

        let palette = unsafe {
            slf.palette
                .as_deref()
                // SAFETY: references will be dropped when `Self` is dropped; we can guarantee that
                // 'p is only valid for the lifetime of `Self`.
                .map(|slice| std::slice::from_raw_parts(slice.as_ptr(), slice.len()))
                // SAFETY: declared palette as `Some` in struct declaration
                .unwrap_unchecked()
        };

        slf.data = pixels
            .iter()
            .map(|&p| P::from_palette(palette, p))
            .collect();
        slf
    }

    /// Decodes an image with the explicitly given image encoding from the raw byte stream.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let file = std::fs::File::open("image.png")?;
    /// let image = Image::<Rgb>::from_reader(ImageFormat::Png, file)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_reader(format: ImageFormat, bytes: impl Read) -> Result<Self> {
        format.run_decoder(bytes)
    }

    /// Decodes an image from the given read stream of bytes, inferring its encoding.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    /// * `UnknownEncodingFormat`: Could not infer the encoding from the image. Try explicitly
    /// specifying it.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let file = std::fs::File::open("image.png")?;
    /// let image = Image::<Rgb>::from_reader_inferred(file)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_reader_inferred(mut bytes: impl Read) -> Result<Self> {
        let buf = &mut [0; 12];
        let n = bytes.read(buf)?;

        match ImageFormat::infer_encoding(buf) {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            format => format.run_decoder((&buf[..n]).chain(bytes)),
        }
    }

    /// Decodes an image with the explicitly given image encoding from the given bytes.
    /// Could be useful in conjunction with the `include_bytes!` macro.
    ///
    /// Currently, this is not any different from [`from_reader`].
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    ///
    /// # Examples
    /// ```no_run,ignore
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let bytes = include_bytes!("sample.png") as &[u8];
    /// let image = Image::<Rgb>::from_bytes(ImageFormat::Png, bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes(format: ImageFormat, bytes: impl AsRef<[u8]>) -> Result<Self> {
        format.run_decoder(bytes.as_ref())
    }

    /// Decodes an image from the given bytes, inferring its encoding.
    /// Could be useful in conjunction with the `include_bytes!` macro.
    ///
    /// This is more efficient than [`from_reader_inferred`].
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    /// * `UnknownEncodingFormat`: Could not infer the encoding from the image. Try explicitly
    /// specifying it.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    ///
    /// # Examples
    /// ```no_run,ignore
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let bytes = include_bytes!("sample.png") as &[u8];
    /// let image = Image::<Rgb>::from_bytes_inferred(bytes)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_bytes_inferred(bytes: impl AsRef<[u8]>) -> Result<Self> {
        match ImageFormat::infer_encoding(bytes.as_ref()) {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            format => format.run_decoder(bytes.as_ref()),
        }
    }

    /// Opens a file from the given path and decodes it into an image.
    ///
    /// The encoding of the image is automatically inferred. You can explicitly pass in an encoding
    /// by using the [`from_reader`] method.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    /// * `UnknownEncodingFormat`: Could not infer the encoding from the image. Try explicitly
    /// specifying it.
    /// * `IoError`: The file could not be opened.
    ///
    /// # Panics
    /// * No decoder implementation for the given encoding format.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::<Rgb>::open("sample.png")?;
    /// println!("Image dimensions: {}x{}", image.width(), image.height());
    /// # Ok(())
    /// # }
    /// ```
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let buffer = &mut Vec::new();
        let mut file = File::open(path.as_ref())?;
        file.read_to_end(buffer)?;

        let format = match ImageFormat::from_path(path)? {
            ImageFormat::Unknown => match ImageFormat::infer_encoding(&buffer[0..12]) {
                ImageFormat::Unknown => return Err(Error::UnknownEncodingFormat),
                format => format,
            },
            format => format,
        };

        format.run_decoder(buffer.as_slice())
    }

    /// Encodes the image with the given encoding and writes it to the given write buffer.
    ///
    /// # Errors
    /// * An error occurred during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::new(100, 100, Rgb::new(255, 0, 0));
    /// let mut out = Vec::new();
    /// image.encode(ImageFormat::Png, &mut out)?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn encode(&self, encoding: ImageFormat, dest: &mut impl Write) -> Result<()> {
        encoding.run_encoder(self, dest)
    }

    /// Saves the image with the given encoding to the given path.
    /// You can try saving to a memory buffer by using the [`encode`] method.
    ///
    /// # Errors
    /// * An error occurred during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::new(100, 100, Rgb::new(255, 0, 0));
    /// image.save(ImageFormat::Png, "out.png")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save(&self, encoding: ImageFormat, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path).map_err(Error::IoError)?;
        self.encode(encoding, &mut file)
    }

    /// Saves the image to the given path, inferring the encoding from the path/filename extension.
    ///
    /// This is obviously slower than [`save`] since this method itself uses it. You should only
    /// use this method if the filename is dynamic, or if you do not know the desired encoding
    /// before runtime.
    ///
    /// See [`save`] for more information on how saving works.
    ///
    /// # Errors
    /// * Could not infer encoding format.
    /// * An error occurred during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::new(100, 100, Rgb::new(255, 0, 0));
    /// image.save_inferred("out.png")?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn save_inferred(&self, path: impl AsRef<Path>) -> Result<()> {
        let encoding = ImageFormat::from_path(path.as_ref())?;

        match encoding {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            _ => self.save(encoding, path),
        }
    }

    #[inline]
    #[must_use]
    const fn resolve_coordinate(&self, x: u32, y: u32) -> usize {
        if x >= self.width() || y >= self.height() {
            usize::MAX
        } else {
            (y * self.width() + x) as usize
        }
    }

    /// Returns the width of the image.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width.get()
    }

    /// Returns the height of the image.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height.get()
    }

    /// Returns the nearest pixel coordinates to the center of the image.
    ///
    /// This uses integer division which means if an image dimension is not even, then the value is
    /// rounded down - e.g. a 5x5 image returns ``(2, 2)``, rounded down from ``(2.5, 2.5)``.
    #[inline]
    #[must_use]
    pub const fn center(&self) -> (u32, u32) {
        (self.width() / 2, self.height() / 2)
    }

    /// Returns an iterator of slices representing the pixels of the image.
    /// Each slice in the Vec is a row. The returned slice should be of ``Vec<&[P; width]>``.
    #[inline]
    pub fn pixels(&self) -> impl Iterator<Item = &[P]> {
        self.data.chunks_exact(self.width() as usize)
    }

    /// Returns the encoding format of the image. This is nothing more but metadata about the image.
    /// When saving the image, you will still have to explicitly specify the encoding format.
    #[inline]
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// Returns the overlay mode of the image.
    #[inline]
    #[must_use]
    pub const fn overlay_mode(&self) -> OverlayMode {
        self.overlay
    }

    /// Returns the same image with its overlay mode set to the given value.
    #[must_use]
    pub const fn with_overlay_mode(mut self, mode: OverlayMode) -> Self {
        self.overlay = mode;
        self
    }

    /// Returns the dimensions of the image.
    #[inline]
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width(), self.height())
    }

    /// Returns the amount of pixels in the image.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.width() * self.height()
    }

    /// Returns true if the image contains no pixels.
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns a reference of the pixel at the given coordinates.
    #[inline]
    #[must_use]
    pub fn pixel(&self, x: u32, y: u32) -> &P {
        &self.data[self.resolve_coordinate(x, y)]
    }

    /// Returns a reference of the pixel at the given coordinates, but only if it exists.
    #[inline]
    #[must_use]
    pub fn get_pixel(&self, x: u32, y: u32) -> Option<&P> {
        self.data.get(self.resolve_coordinate(x, y))
    }

    /// Returns a mutable reference to the pixel at the given coordinates.
    #[inline]
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut P {
        let pos = self.resolve_coordinate(x, y);

        &mut self.data[pos]
    }

    /// Sets the pixel at the given coordinates to the given pixel.
    #[inline]
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: P) {
        let pos = self.resolve_coordinate(x, y);

        self.data[pos] = pixel;
    }

    /// Overlays the pixel at the given coordinates with the given pixel according to the overlay
    /// mode.
    #[inline]
    pub fn overlay_pixel(&mut self, x: u32, y: u32, pixel: P) {
        self.overlay_pixel_with_mode(x, y, pixel, self.overlay);
    }

    /// Overlays the pixel at the given coordinates with the given pixel according to the specified
    /// overlay mode.
    ///
    /// If the pixel is out of bounds, nothing occurs. This is expected, use [`set_pixel`] if you
    /// want this to panic, or to use a custom overlay mode use [`pixel_mut`].
    #[inline]
    pub fn overlay_pixel_with_mode(&mut self, x: u32, y: u32, pixel: P, mode: OverlayMode) {
        let pos = self.resolve_coordinate(x, y);

        if let Some(target) = self.data.get_mut(pos) {
            *target = target.overlay(pixel, mode);
        }
    }

    /// Overlays the pixel at the given coordinates with the given alpha intensity. This does not
    /// regard the overlay mode, since this is usually used for anti-aliasing.
    ///
    /// If the pixel is out of bounds, nothing occurs. This is expected, use [`set_pixel`] if you
    /// want this to panic, or to use a custom overlay mode use [`pixel_mut`].
    #[inline]
    pub fn overlay_pixel_with_alpha(
        &mut self,
        x: u32,
        y: u32,
        pixel: P,
        mode: OverlayMode,
        alpha: u8,
    ) {
        let pos = self.resolve_coordinate(x, y);

        if let Some(target) = self.data.get_mut(pos) {
            *target = target.overlay_with_alpha(pixel, mode, alpha);
        }
    }

    /// Inverts this image in place.
    pub fn invert(&mut self) {
        self.data.iter_mut().for_each(|p| *p = p.inverted());
    }

    /// Takes this image and inverts it. Useful for method chaining.
    #[must_use]
    pub fn inverted(self) -> Self {
        self.map_pixels(|pixel| pixel.inverted())
    }

    /// Brightens the image by increasing all pixels by the specified amount of subpixels in place.
    /// See [`darken`] to darken the image, since this usually does not take any negative values.
    ///
    /// A subpixel is a value of a pixel's component, for example in RGB, each subpixel is a value
    /// of either R, G, or B.
    ///
    /// For anything with alpha, alpha is not brightened.
    pub fn brighten(&mut self, amount: P::Subpixel)
    where
        P::Subpixel: SaturatingAdd + Copy,
    {
        self.data
            .iter_mut()
            .for_each(|p| *p = p.map_subpixels(|value| value.saturating_add(&amount), |a| a));
    }

    /// Darkens the image by decreasing all pixels by the specified amount of subpixels in place.
    /// See [`brighten`] to brighten the image, since this usually does not take any negative values.
    ///
    /// A subpixel is a value of a pixel's component, for example in RGB, each subpixel is a value
    /// of either R, G, or B.
    ///
    /// For anything with alpha, alpha is not brightened.
    pub fn darken(&mut self, amount: P::Subpixel)
    where
        P::Subpixel: SaturatingSub + Copy,
    {
        self.data
            .iter_mut()
            .for_each(|p| *p = p.map_subpixels(|value| value.saturating_sub(&amount), |a| a));
    }

    /// Takes this image and brightens it by increasing all pixels by the specified amount of
    /// subpixels. Negative values will darken the image. Useful for method chaining.
    ///
    /// See [`darkened`] to darken the image, since this usually does not take any negative values.
    ///
    /// A subpixel is a value of a pixel's component, for example in RGB, each subpixel is a value.
    ///
    /// For anything with alpha, alpha is not brightened.
    #[must_use]
    pub fn brightened(self, amount: P::Subpixel) -> Self
    where
        P::Subpixel: SaturatingAdd + Copy,
    {
        self.map_pixels(|pixel| pixel.map_subpixels(|value| value.saturating_add(&amount), |a| a))
    }

    /// Takes this image and darkens it by decreasing all pixels by the specified amount of
    /// subpixels. Negative values will brighten the image. Useful for method chaining.
    ///
    /// See [`brightened`] to brighten the image, since this usually does not take any negative
    /// values.
    ///
    /// A subpixel is a value of a pixel's component, for example in RGB, each subpixel is a value.
    ///
    /// For anything with alpha, alpha is not brightened.
    #[must_use]
    pub fn darkened(self, amount: P::Subpixel) -> Self
    where
        P::Subpixel: SaturatingSub + Copy,
    {
        self.map_pixels(|pixel| pixel.map_subpixels(|value| value.saturating_sub(&amount), |a| a))
    }

    //noinspection SpellCheckingInspection
    #[allow(clippy::cast_lossless)]
    fn prepare_hue_matrix(degrees: i32) -> (f64, f64, f64, f64, f64, f64, f64, f64, f64) {
        let degrees = (degrees % 360) as f64;
        let radians = degrees.to_radians();
        let sinv = radians.sin();
        let cosv = radians.cos();

        (
            sinv.mul_add(-0.213, cosv.mul_add(0.787, 0.213)),
            sinv.mul_add(-0.715, cosv.mul_add(-0.715, 0.715)),
            sinv.mul_add(0.928, cosv.mul_add(-0.072, 0.072)),
            sinv.mul_add(0.143, cosv.mul_add(-0.213, 0.213)),
            sinv.mul_add(0.140, cosv.mul_add(0.285, 0.715)),
            sinv.mul_add(-0.283, cosv.mul_add(-0.072, 0.072)),
            sinv.mul_add(-0.787, cosv.mul_add(-0.213, 0.213)),
            sinv.mul_add(0.715, cosv.mul_add(-0.715, 0.715)),
            sinv.mul_add(0.072, cosv.mul_add(0.928, 0.072)),
        )
    }

    /// Hue rotates the image by the specified amount of degrees in place.
    ///
    /// The hue is a standard angle degree, that is a value between 0 and 360, although values
    /// below and above will be wrapped using the modulo operator.
    ///
    /// For anything with alpha, alpha is not rotated.
    pub fn hue_rotate(&mut self, degrees: i32)
    where
        P: TrueColor,
    {
        let mat = Self::prepare_hue_matrix(degrees);

        self.data.iter_mut().for_each(|p| {
            let (r, g, b, a) = p.as_rgba_tuple();
            let (r, g, b) = (f64::from(r), f64::from(g), f64::from(b));

            *p = P::from_rgba_tuple((
                mat.2.mul_add(b, mat.0.mul_add(r, mat.1 * g)) as u8,
                mat.5.mul_add(b, mat.3.mul_add(r, mat.4 * g)) as u8,
                mat.8.mul_add(b, mat.6.mul_add(r, mat.7 * g)) as u8,
                a,
            ));
        });
    }

    /// Takes this image and hue rotates it by the specified amount of degrees.
    /// Useful for method chaining.
    ///
    /// See [`Self::hue_rotate`] for more information.
    #[must_use]
    pub fn hue_rotated(mut self, degrees: i32) -> Self
    where
        P: TrueColor,
    {
        self.hue_rotate(degrees);
        self
    }

    /// Returns the image replaced with the given data. It is up to you to make sure
    /// the data is the correct size.
    ///
    /// The function should take the current image data and return the new data.
    ///
    /// # Note
    /// This will *not* work for paletted images, nor will it work for conversion to paletted
    /// images. For conversion from paletted images, see the [`Self::flatten`] method to flatten
    /// the palette fist. For conversion to paletted images, try quantizing the image.
    pub fn map_data<T: Pixel>(self, f: impl FnOnce(Vec<P>) -> Vec<T>) -> Image<T> {
        Image {
            width: self.width,
            height: self.height,
            data: f(self.data),
            format: self.format,
            overlay: self.overlay,
            palette: None,
        }
    }

    /// Sets the data of this image to the new data. This is used a lot internally,
    /// but should rarely be used by you.
    ///
    /// # Panics
    /// * Panics if the data is malformed.
    pub fn set_data(&mut self, data: Vec<P>) {
        assert_eq!(
            self.width() * self.height(),
            data.len() as u32,
            "malformed data"
        );

        self.data = data;
    }

    /// Returns the image with each pixel in the image mapped to the given function.
    ///
    /// The function should take the pixel and return another pixel.
    pub fn map_pixels<T: Pixel>(self, f: impl FnMut(P) -> T) -> Image<T> {
        self.map_data(|data| data.into_iter().map(f).collect())
    }

    /// Returns the image with the each pixel in the image mapped to the given function, with
    /// the function taking additional data of the pixel.
    ///
    /// The function should take the x and y coordinates followed by the pixel and return the new
    /// pixel.
    pub fn map_pixels_with_coords<T: Pixel>(self, f: impl Fn(u32, u32, P) -> T) -> Image<T> {
        let width = self.width;

        self.map_data(|data| {
            data.into_iter()
                .zip(0..)
                .map(|(p, i)| f(i % width, i / width, p))
                .collect()
        })
    }

    /// Similar to [`map_pixels_with_coords`], but this maps the pixels in place.
    ///
    /// This means that the output pixel type must be the same.
    pub fn map_in_place(&mut self, f: impl Fn(u32, u32, &mut P)) {
        let width = self.width;

        self.data
            .iter_mut()
            .zip(0..)
            .for_each(|(p, i)| f(i % width, i / width, p));
    }

    /// Returns the image with each row of pixels represented as a slice mapped to the given
    /// function.
    ///
    /// The function should take the y coordinate followed by the row of pixels
    /// (represented as a slice) and return an Iterator of pixels.
    pub fn map_rows<I, T: Pixel>(self, f: impl Fn(u32, &[P]) -> I) -> Image<T>
    where
        I: IntoIterator<Item = T>,
    {
        let width = self.width();

        self.map_data(|data| {
            data.chunks(width as usize)
                .zip(0..)
                .flat_map(|(row, y)| f(y, row))
                .collect()
        })
    }

    /// Iterates over each row of pixels in the image.
    pub fn rows(&self) -> impl Iterator<Item = &[P]> {
        self.data.chunks_exact(self.width() as usize)
    }

    /// Converts the image into an image with the given pixel type.
    ///
    /// # Note
    /// Currently there is a slight inconsistency with paletted images - if you would like to
    /// convert from a paletted image to a paletted image with a different pixel type, you cannot
    /// use this method and must instead use the `From`/`Into` trait instead.
    ///
    /// That said, you can also use the `From`/`Into` trait regardless of the pixel type.
    #[must_use]
    pub fn convert<T: Pixel + From<P>>(self) -> Image<T> {
        self.map_pixels(T::from)
    }

    /// Sets the encoding format of this image. Note that when saving the file,
    /// an encoding format will still have to be explicitly specified.
    /// This is more or less image metadata.
    pub fn set_format(&mut self, format: ImageFormat) {
        self.format = format;
    }

    /// Crops this image in place to the given bounding box.
    ///
    /// # Panics
    /// * The width or height of the bounding box is less than 1.
    pub fn crop(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        self.data = self
            .pixels()
            .skip(y1 as usize)
            .zip(y1..y2)
            .flat_map(|(row, _)| &row[x1 as usize..x2 as usize])
            .copied()
            .collect();

        self.width = NonZeroU32::new(x2 - x1).unwrap();
        self.height = NonZeroU32::new(y2 - y1).unwrap();
    }

    /// Takes this image and crops it to the given box. Useful for method chaining.
    #[must_use]
    pub fn cropped(mut self, x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        self.crop(x1, y1, x2, y2);
        self
    }

    /// Mirrors, or flips this image horizontally (about the y-axis) in place.
    pub fn mirror(&mut self) {
        let width = self.width();

        self.data
            .chunks_exact_mut(width as usize)
            .for_each(<[P]>::reverse);
    }

    /// Takes this image and flips it horizontally (about the y-axis). Useful for method chaining.
    #[must_use]
    pub fn mirrored(mut self) -> Self {
        self.mirror();
        self
    }

    /// Flips this image vertically (about the x-axis) in place.
    pub fn flip(&mut self) {
        self.mirror();
        self.rotate_180();
    }

    /// Takes this image and flips it vertically, or about the x-axis. Useful for method chaining.
    #[must_use]
    pub fn flipped(mut self) -> Self {
        self.flip();
        self
    }

    fn rotate_iterator(&self) -> impl Iterator<Item = P> + DoubleEndedIterator + '_ {
        (0..self.width() as usize).flat_map(move |i| {
            (0..self.height() as usize)
                .map(move |j| self.data[j * self.width() as usize + i])
                .rev()
        })
    }

    /// Rotates this image by 90 degrees clockwise, or 270 degrees counterclockwise, in place.
    ///
    /// # See Also
    /// - [`Self::rotate`] for a version that can take any arbitrary amount of degrees
    /// - [`Self::rotated`] for the above method which does operate in-place - useful for method
    /// chaining
    pub fn rotate_90(&mut self) {
        self.data = self.rotate_iterator().collect();
        std::mem::swap(&mut self.width, &mut self.height);
    }

    /// Rotates this image by 180 degrees in place.
    ///
    /// # See Also
    /// - [`Self::rotate`] for a version that can take any arbitrary amount of degrees
    /// - [`Self::rotated`] for the above method which does operate in-place - useful for method
    /// chaining
    pub fn rotate_180(&mut self) {
        self.data.reverse();
    }

    /// Rotates this image by 270 degrees clockwise, or 90 degrees counterclockwise, in place.
    ///
    /// # See Also
    /// - [`Self::rotate`] for a version that can take any arbitrary amount of degrees
    /// - [`Self::rotated`] for the above method which does operate in-place - useful for method
    /// chaining
    pub fn rotate_270(&mut self) {
        self.data = self.rotate_iterator().rev().collect();
        std::mem::swap(&mut self.width, &mut self.height);
    }

    /// Rotates this image in place about its center. There are optimized rotating algorithms for
    /// 90, 180, and 270 degree rotations (clockwise).
    ///
    /// As mentioned, the argument is specified in degrees.
    ///
    /// # See Also
    /// - [`Self::rotated`] for this method which does operate in-place - useful for method chaining
    pub fn rotate(&mut self, mut degrees: i32) {
        degrees %= 360;

        match degrees {
            0 => (),
            90 => self.rotate_90(),
            180 => self.rotate_180(),
            270 => self.rotate_270(),
            _ => unimplemented!(
                "currently only rotations of 0, 90, 180, and 270 degrees are supported",
            ),
        }
    }

    /// Takes the image and rotates it by the specified amount of degrees about its center. Useful
    /// for method chaining. There are optimized rotating algorithms for 90, 180, and 270 degree
    /// rotations.
    #[must_use]
    pub fn rotated(mut self, degrees: i32) -> Self {
        self.rotate(degrees);
        self
    }

    /// Resizes this image in place to the given dimensions using the given resizing algorithm
    /// in place.
    ///
    /// `width` and `height` must be greater than 0, otherwise this method will panic. You should
    /// validate user input before calling this method.
    ///
    /// # Panics
    /// * `width` or `height` is zero.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let mut image = Image::new(256, 256, Rgb::white());
    /// assert_eq!(image.dimensions(), (256, 256));
    ///
    /// image.resize(64, 64, ResizeAlgorithm::Lanczos3);
    /// assert_eq!(image.dimensions(), (64, 64));
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "resize")]
    pub fn resize(&mut self, width: u32, height: u32, algorithm: ResizeAlgorithm) {
        assert_nonzero!(width, height);

        let width = NonZeroU32::new(width).unwrap();
        let height = NonZeroU32::new(height).unwrap();

        self.data = crate::resize::resize(
            &self.data,
            self.width,
            self.height,
            width,
            height,
            algorithm,
        );
        self.width = width;
        self.height = height;
    }

    /// Takes this image and resizes this image to the given dimensions using the given
    /// resizing algorithm. Useful for method chaining.
    ///
    /// `width` and `height` must be greater than 0, otherwise this method will panic. You should
    /// validate user input before calling this method.
    ///
    /// # Panics
    /// * `width` or `height` is zero.
    ///
    /// # See Also
    /// * [`Self::resize`] for a version that operates in-place
    #[must_use]
    #[cfg(feature = "resize")]
    pub fn resized(mut self, width: u32, height: u32, algorithm: ResizeAlgorithm) -> Self {
        self.resize(width, height, algorithm);
        self
    }

    /// Draws an object or shape onto this image.
    ///
    /// # Example
    /// ```
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let mut image = Image::new(256, 256, Rgb::white());
    /// let rectangle = Rectangle::at(64, 64)
    ///     .with_size(128, 128)
    ///     .with_fill(Rgb::black());
    ///
    /// image.draw(&rectangle);
    /// # Ok(())
    /// # }
    /// ```
    pub fn draw(&mut self, entity: &impl Draw<P>) {
        entity.draw(self);
    }

    /// Takes this image, draws the given object or shape onto it, and returns it.
    /// Useful for method chaining and drawing multiple objects at once.
    ///
    /// # See Also
    /// * [`Self::draw`] for a version that operates in-place
    #[must_use]
    pub fn with(mut self, entity: &impl Draw<P>) -> Self {
        self.draw(entity);
        self
    }

    /// Pastes the given image onto this image at the given x and y coordinates.
    /// This is a shorthand for using the [`draw`] method with [`Paste`].
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let mut image = Image::new(256, 256, Rgb::white());
    /// let overlay_image = Image::open("overlay.png")?;
    ///
    /// image.paste(64, 64, &overlay_image);
    /// # Ok(())
    /// # }
    /// ```
    pub fn paste(&mut self, x: u32, y: u32, image: &Self) {
        self.draw(&crate::Paste::new(image).with_position(x, y));
    }

    /// Pastes the given image onto this image at the given x and y coordinates,
    /// masked with the given masking image.
    ///
    /// Currently, only [`BitPixel`] images are supported for the masking image.
    ///
    /// This is a shorthand for using the [`draw`] method with [`Paste`].
    ///
    /// # Example
    /// ```no_run
    /// # use ril::prelude::*;
    /// # fn main() -> ril::Result<()> {
    /// let mut image = Image::new(256, 256, Rgb::white());
    /// let overlay_image = Image::open("overlay.png")?;
    ///
    /// let (w, h) = overlay_image.dimensions();
    /// let mut mask = Image::new(w, h, BitPixel::off());
    /// mask.draw(&Ellipse::from_bounding_box(0, 0, w, h).with_fill(BitPixel::on()));
    ///
    /// image.paste_with_mask(64, 64, &overlay_image, &mask);
    /// # Ok(())
    /// # }
    /// ```
    pub fn paste_with_mask(&mut self, x: u32, y: u32, image: &Self, mask: &Image<BitPixel>) {
        self.draw(&crate::Paste::new(image).with_position(x, y).with_mask(mask));
    }

    /// Masks the alpha values of this image with the luminance values of the given single-channel
    /// [`L`] image.
    ///
    /// If you want to mask using the alpha values of the image instead of providing an [`L`] image,
    /// you can split the bands of the image and extract the alpha band.
    ///
    /// This masking image must have the same dimensions as this image. If it doesn't, you will
    /// receive a panic.
    ///
    /// # Panics
    /// * The masking image has different dimensions from this image.
    pub fn mask_alpha(&mut self, mask: &Image<L>)
    where
        P: Alpha,
    {
        assert_eq!(
            self.dimensions(),
            mask.dimensions(),
            "Masking image with dimensions {:?} must have the \
            same dimensions as this image with dimensions {:?}",
            mask.dimensions(),
            self.dimensions()
        );

        self.data
            .iter_mut()
            .zip(mask.data.iter())
            .for_each(|(pixel, mask)| {
                *pixel = pixel.with_alpha(mask.value());
            });
    }

    /// Returns the palette associated with this image as a slice.
    /// If there is no palette, this returns `None`.
    #[must_use]
    pub fn palette(&self) -> Option<&[P::Color]> {
        self.palette.as_deref()
    }

    /// Returns the palette associated with this image as a mutable slice.
    /// If there is no palette, this returns `None`.
    #[must_use]
    pub fn palette_mut(&mut self) -> Option<&mut [P::Color]> {
        self.palette.as_deref_mut()
    }

    /// Returns the palette associated with this image as a slice. You must uphold the guarantee
    /// that the image is paletted, otherwise this will result in undefined behaviour.
    ///
    /// # Safety
    /// * The image must always be paletted.
    ///
    /// # See Also
    /// * [`Self::palette`] - A safe, checked alternative to this method.
    #[must_use]
    pub unsafe fn palette_unchecked(&self) -> &[P::Color] {
        self.palette.as_ref().unwrap_unchecked()
    }

    /// Returns the palette associated with this image as a mutable slice. You must uphold the
    /// guarantee that the image is paletted, otherwise this will result in undefined behaviour.
    ///
    /// # Safety
    /// * The image must always be paletted.
    ///
    /// # See Also
    /// * [`Self::palette_mut`] - A safe, checked alternative to this method.
    #[must_use]
    pub unsafe fn palette_mut_unchecked(&mut self) -> &mut [P::Color] {
        self.palette.as_mut().unwrap_unchecked()
    }

    /// Maps the palette of this image using the given function. If this image has no palette,
    /// this will do nothing.
    ///
    /// # Panics
    /// * Safe conversion of palette references failed.
    pub fn map_palette<'a, U, F, C: TrueColor>(self, mut f: F) -> Image<U>
    where
        Self: 'a,
        P: Paletted<'a>,
        U: Paletted<'a> + Pixel<Subpixel = P::Subpixel, Color = C>,
        F: FnMut(P::Color) -> C,
    {
        let palette = self.palette.map(|palette| {
            palette
                .iter()
                .map(|p| f(*p))
                .collect::<Vec<_>>()
                .into_boxed_slice()
        });

        Image {
            width: self.width,
            height: self.height,
            data: self
                .data
                .into_iter()
                .map(|p| {
                    U::from_raw_parts_paletted(
                        U::COLOR_TYPE,
                        U::BIT_DEPTH,
                        &[p.palette_index().into() as u8],
                        palette.as_deref(),
                    )
                    .expect("could not perform safe conversion of palette references")
                })
                .collect(),
            format: self.format,
            overlay: self.overlay,
            palette,
        }
    }

    /// Takes this image and flattens this paletted image into an unpaletted image. This is similar
    /// to [`Self::convert`] but the output type is automatically resolved.
    #[must_use = "the image is consumed by this method and returns a new image"]
    pub fn flatten_palette<'a>(self) -> Image<P::Color>
    where
        Self: 'a,
        P: Paletted<'a>,
    {
        self.map_pixels(|pixel| pixel.color())
    }

    /// Quantizes this image using its colors and turns it into its paletted counterpart.
    /// This currently only works with 8-bit palettes.
    ///
    /// This is similar to [`Self::convert`] but the output type is automatically resolved.
    /// This is also the inverse conversion of [`Self::flatten_palette`].
    ///
    /// # Errors
    /// * The palette could not be created.
    ///
    /// # See Also
    /// * [`Quantizer`] - Implementation of the core quantizer. Use this for more fine-grained
    /// control over the quantization process, such as adjusting the quantization speed.
    #[must_use = "the image is consumed by this method and returns a new image"]
    pub fn quantize<'p, T>(self, palette_size: u8) -> Image<T>
    where
        Self: 'p,
        P: TrueColor,
        T: Pixel<Color = P> + Paletted<'p, Subpixel = u8>,
    {
        let width = self.width();
        let (palette, pixels) = crate::quantize::Quantizer::new()
            .with_palette_size(palette_size as usize)
            .quantize(self.data)
            .expect("unable to quantize image");

        Image::from_paletted_pixels(width, palette, pixels)
    }
}

impl Image<Rgba> {
    /// Splits this image into an `Rgb` image and an `L` image, where the `Rgb` image contains the
    /// red, green, and blue color channels and the `L` image contains the alpha channel.
    ///
    /// There is a more optimized method available, [`Self::map_rgb_pixels`], if you only need to perform
    /// operations on individual RGB pixels. If you can, you should use that instead.
    ///
    /// # Example
    /// Rotating the image by 90 degrees but keeping the alpha channel untouched:
    ///
    /// ```no_run
    /// use ril::prelude::*;
    ///
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::<Rgba>::open("image.png")?;
    /// let (rgb, alpha) = image.split_rgb_and_alpha();
    /// let inverted = Image::from_rgb_and_alpha(rgb.rotated(90), alpha);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// * [`Self::from_rgb_and_alpha`] - The inverse of this method.
    /// * [`Self::map_rgb_pixels`] - A more optimized method for performing operations on individual RGB
    /// pixels.
    #[must_use]
    pub fn split_rgb_and_alpha(self) -> (Image<Rgb>, Image<L>) {
        let (r, g, b, a) = self.bands();
        (Image::from_bands((r, g, b)), a)
    }

    /// Creates an `Rgba` image from an `Rgb` image and an `L` image, where the `Rgb` image contains
    /// the red, green, and blue color channels and the `L` image contains the alpha channel.
    ///
    /// # Panics
    /// * The dimensions of the two images do not match.
    ///
    /// # See Also
    /// * [`Self::split_rgb_and_alpha`] - The inverse of this method.
    #[must_use]
    pub fn from_rgb_and_alpha(rgb: Image<Rgb>, alpha: Image<L>) -> Self {
        debug_assert_eq!(
            rgb.dimensions(),
            alpha.dimensions(),
            "dimensions of RGB and alpha images do not match",
        );
        rgb.map_data(|data| {
            data.into_iter()
                .zip(alpha.data)
                .map(|(Rgb { r, g, b }, L(a))| Rgba { r, g, b, a })
                .collect()
        })
    }

    /// Performs the given operation `f` on every pixel in this image, ignoring the alpha channel.
    /// The alpha channel is left untouched.
    ///
    /// # Example
    /// Inverting the image but keeping the alpha channel untouched:
    ///
    /// ```no_run
    /// use ril::prelude::*;
    ///
    /// # fn main() -> ril::Result<()> {
    /// let image = Image::<Rgba>::open("image.png")?;
    /// let inverted = image.map_rgb_pixels(|rgb| rgb.inverted());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # See Also
    /// * [`Self::map_alpha_pixels`] - Performs the given operation on every pixel in the alpha
    /// channel.
    /// * [`Self::split_rgb_and_alpha`] - If you need to operate on the entire `Image<Rgb>`
    /// (and `Image<L>`).
    #[must_use]
    pub fn map_rgb_pixels(self, mut f: impl FnMut(Rgb) -> Rgb) -> Self {
        self.map_pixels(|Rgba { r, g, b, a }| {
            let Rgb { r, g, b } = f(Rgb { r, g, b });
            Rgba { r, g, b, a }
        })
    }

    /// Performs the given operation `f` on every pixel in the alpha channel of this image.
    /// The RGB channels are left untouched.
    ///
    /// # See Also
    /// * [`Self::map_rgb_pixels`] - Performs the given operation on every pixel in the RGB channels.
    /// * [`Self::split_rgb_and_alpha`] - If you need to operate on the entire `Image<L>`
    /// (and `Image<Rgb>`).
    #[must_use]
    pub fn map_alpha_pixels(self, mut f: impl FnMut(L) -> L) -> Self {
        self.map_pixels(|Rgba { r, g, b, a }| Rgba {
            r,
            g,
            b,
            a: f(L(a)).value(),
        })
    }
}

impl<'a> From<Image<PalettedRgb<'a>>> for Image<PalettedRgba<'a>> {
    fn from(image: Image<PalettedRgb<'a>>) -> Self {
        image.map_palette(Into::into)
    }
}

impl<'a> From<Image<PalettedRgba<'a>>> for Image<PalettedRgb<'a>> {
    fn from(image: Image<PalettedRgba<'a>>) -> Self {
        image.map_palette(Into::into)
    }
}

macro_rules! impl_cast_quantize {
    ($t:ty: $p:ty) => {
        impl From<Image<$t>> for Image<$p> {
            fn from(image: Image<$t>) -> Self {
                let width = image.width();
                let (palette, pixels) = crate::quantize::Quantizer::new()
                    .with_palette_size(image.data.len())
                    .quantize(image.data)
                    .expect("unable to quantize image");

                Image::from_paletted_pixels(width, palette, pixels)
            }
        }
    };
}

impl_cast_quantize!(Rgb: PalettedRgb<'_>);
impl_cast_quantize!(Rgba: PalettedRgba<'_>);

macro_rules! impl_cast {
    ($t:ty: $($f:ty)+) => {
        $(
            impl From<Image<$f>> for Image<$t> {
                fn from(f: Image<$f>) -> Self {
                    f.map_pixels(<$t>::from)
                }
            }
        )+
    };
}

impl_cast!(BitPixel: L Rgb Rgba Dynamic PalettedRgb<'_> PalettedRgba<'_>);
impl_cast!(L: BitPixel Rgb Rgba Dynamic PalettedRgb<'_> PalettedRgba<'_>);
impl_cast!(Rgb: BitPixel L Rgba Dynamic PalettedRgb<'_> PalettedRgba<'_>);
impl_cast!(Rgba: BitPixel L Rgb Dynamic PalettedRgb<'_> PalettedRgba<'_>);
impl_cast!(Dynamic: BitPixel L Rgb Rgba PalettedRgb<'_> PalettedRgba<'_>);

/// Represents an image with multiple channels, called bands.
///
/// Each band should be represented as a separate [`Image`] with [`L`] or [`BitPixel`] pixels.
pub trait Banded<T> {
    /// Takes this image and returns its bands.
    fn bands(&self) -> T;

    /// Creates a new image from the given bands.
    fn from_bands(bands: T) -> Self;
}

type Band = Image<L>;

macro_rules! map_idx {
    ($image:expr, $idx:expr) => {{
        use $crate::{Image, L};

        Image {
            width: $image.width,
            height: $image.height,
            data: $image.data.iter().map(|p| L(p.as_bytes()[$idx])).collect(),
            format: $image.format,
            overlay: $image.overlay,
            palette: None,
        }
    }};
}

macro_rules! extract_bands {
    ($image:expr; $($idx:literal)+) => {{
        ($(map_idx!($image, $idx)),+)
    }};
}

macro_rules! validate_dimensions {
    ($target:ident, $($others:ident),+) => {{
        $(
            assert_eq!(
                $target.dimensions(),
                $others.dimensions(),
                "bands have different dimensions: {} has dimensions of {:?}, which is different \
                from {} which has dimensions of {:?}",
                stringify!($target),
                $target.dimensions(),
                stringify!($others),
                $others.dimensions()
            );
        )+
    }};
}

impl Banded<(Band, Band, Band)> for Image<Rgb> {
    fn bands(&self) -> (Band, Band, Band) {
        extract_bands!(self; 0 1 2)
    }

    fn from_bands((r, g, b): (Band, Band, Band)) -> Self {
        validate_dimensions!(r, g, b);

        r.map_data(|data| {
            data.into_iter()
                .zip(g.data.into_iter())
                .zip(b.data.into_iter())
                .map(|((L(r), L(g)), L(b))| Rgb::new(r, g, b))
                .collect()
        })
    }
}

impl Banded<(Band, Band, Band, Band)> for Image<Rgba> {
    fn bands(&self) -> (Band, Band, Band, Band) {
        extract_bands!(self; 0 1 2 3)
    }

    fn from_bands((r, g, b, a): (Band, Band, Band, Band)) -> Self {
        validate_dimensions!(r, g, b, a);

        r.map_data(|data| {
            data.into_iter()
                .zip(g.data.into_iter())
                .zip(b.data.into_iter())
                .zip(a.data.into_iter())
                .map(|(((L(r), L(g)), L(b)), L(a))| Rgba::new(r, g, b, a))
                .collect()
        })
    }
}

/// Represents the underlying encoding format of an image.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// No known encoding is known for the image.
    ///
    /// This is usually because the image was created manually. See [`Image::set_format`]
    /// to manually set the encoding format.
    Unknown,

    /// The image is encoded in the PNG format.
    Png,

    /// The image is encoded in the JPEG format.
    Jpeg,

    /// The image is encoded in the GIF format.
    Gif,

    /// The image is encoded in the BMP format.
    Bmp,

    /// The image is encoded in the TIFF format.
    Tiff,

    /// The image is encoded in the WebP format.
    WebP,

    /// The image is encoded in the QOI format.
    Qoi,
}

impl Default for ImageFormat {
    fn default() -> Self {
        Self::Unknown
    }
}

impl ImageFormat {
    /// Returns whether the extension is unknown.
    #[inline]
    #[must_use]
    pub fn is_unknown(&self) -> bool {
        self == &Self::Unknown
    }

    /// Parses the given extension and returns the corresponding image format.
    ///
    /// If the extension is an unknown extension, Ok([`ImageFormat::unknown`]) is returned.
    ///
    /// If the extension is completely invalid and fails to be converted into a `&str`,
    /// the [`InvalidExtension`] error is returned.
    ///
    /// # Errors
    /// * The extension is completely invalid and failed to be converted into a `&str`.
    pub fn from_extension(ext: impl AsRef<OsStr>) -> Result<Self> {
        let extension = ext.as_ref().to_str();

        Ok(
            match extension
                .ok_or_else(|| InvalidExtension(ext.as_ref().to_os_string()))?
                .to_ascii_lowercase()
                .as_str()
            {
                "png" | "apng" => Self::Png,
                "jpg" | "jpeg" => Self::Jpeg,
                "gif" => Self::Gif,
                "bmp" => Self::Bmp,
                "tiff" => Self::Tiff,
                "webp" => Self::WebP,
                "qoi" => Self::Qoi,
                _ => Self::Unknown,
            },
        )
    }

    /// Returns the format specified by the given path.
    ///
    /// This uses [`ImageFormat::from_extension`] to parse the extension.
    ///
    /// This resolves via the extension of the path. See [`ImageFormat::infer_encoding`] for an
    /// implementation that can resolve the format from the data.
    ///
    /// # Errors
    /// * No extension can be resolved from the path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        path.as_ref()
            .extension()
            .ok_or_else(|| InvalidExtension(path.as_ref().into()))
            .and_then(Self::from_extension)
    }

    /// Returns the format specified by the given MIME type.
    pub fn from_mime_type(mime: impl AsRef<str>) -> Self {
        let mime = mime.as_ref();

        match mime {
            "image/png" => Self::Png,
            "image/jpeg" => Self::Jpeg,
            "image/gif" => Self::Gif,
            "image/bmp" => Self::Bmp,
            "image/tiff" => Self::Tiff,
            "image/webp" => Self::WebP,
            // Unofficial, but in the specification
            "image/qoi" => Self::Qoi,
            _ => Self::Unknown,
        }
    }

    /// Infers the encoding format from the given data via a byte stream.
    #[must_use]
    pub fn infer_encoding(sample: &[u8]) -> Self {
        if sample.starts_with(b"\x89PNG\x0D\x0A\x1A\x0A") {
            Self::Png
        } else if sample.starts_with(b"\xFF\xD8\xFF") {
            Self::Jpeg
        } else if sample.starts_with(b"GIF") {
            Self::Gif
        } else if sample.starts_with(b"BM") {
            Self::Bmp
        } else if sample.len() > 11 && &sample[8..12] == b"WEBP" {
            Self::WebP
        } else if (sample.starts_with(b"\x49\x49\x2A\0") || sample.starts_with(b"\x4D\x4D\0\x2A"))
            && sample[8] != 0x43
            && sample[9] != 0x52
        {
            Self::Tiff
        } else if sample.starts_with(b"qoif") {
            Self::Qoi
        } else {
            Self::Unknown
        }
    }

    /// Encodes the `Image` into raw bytes.
    ///
    /// # Errors
    /// * An error occurred while encoding.
    ///
    /// # Panics
    /// * No encoder implementation is found for this image encoding.
    #[cfg_attr(
        not(any(
            feature = "png",
            feature = "gif",
            feature = "jpeg",
            feature = "webp",
            feature = "qoi"
        )),
        allow(unused_variables, unreachable_code)
    )]
    pub fn run_encoder<P: Pixel>(&self, image: &Image<P>, dest: &mut impl Write) -> Result<()> {
        match self {
            #[cfg(feature = "png")]
            Self::Png => png::PngEncoder::new().encode(image, dest),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegEncoder::new().encode(image, dest),
            #[cfg(feature = "gif")]
            Self::Gif => gif::GifEncoder::new().encode(image, dest),
            #[cfg(feature = "webp")]
            Self::WebP => webp::WebPEncoder::default().encode(image, dest),
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiEncoder::default().encode(image, dest),
            _ => panic!(
                "No encoder implementation is found for this image format. \
                 Did you forget to enable the feature?"
            ),
        }
    }

    /// Encodes the `ImageSequence` into raw bytes. If the encoding does not supported image
    /// sequences (or multi-frame images), it will only encode the first frame.
    ///
    /// # Errors
    /// * An error occurred while encoding.
    ///
    /// # Panics
    /// * No encoder implementation is found for this image encoding.
    #[cfg_attr(
        not(any(
            feature = "png",
            feature = "gif",
            feature = "jpeg",
            feature = "webp",
            feature = "qoi"
        )),
        allow(unused_variables, unreachable_code)
    )]
    pub fn run_sequence_encoder<P: Pixel>(
        &self,
        seq: &crate::ImageSequence<P>,
        dest: &mut impl Write,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "png")]
            Self::Png => png::PngEncoder::new().encode_sequence(seq, dest),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegEncoder::new().encode_sequence(seq, dest),
            #[cfg(feature = "gif")]
            Self::Gif => gif::GifEncoder::new().encode_sequence(seq, dest),
            #[cfg(feature = "webp")]
            Self::WebP => webp::WebPEncoder::default().encode_sequence(seq, dest),
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiEncoder::default().encode_sequence(seq, dest),
            _ => panic!(
                "No encoder implementation is found for this image format. \
                 Did you forget to enable the feature?"
            ),
        }
    }

    /// Decodes the image data from into an image.
    ///
    /// # Errors
    /// * An error occurred while decoding.
    ///
    /// # Panics
    /// * No decoder implementation is found for this image encoding.
    #[cfg_attr(
        not(any(
            feature = "png",
            feature = "gif",
            feature = "jpeg",
            feature = "webp",
            feature = "qoi"
        )),
        allow(unused_variables, unreachable_code)
    )]
    #[allow(clippy::needless_pass_by_value)] // would require a major refactor
    pub fn run_decoder<P: Pixel>(&self, stream: impl Read) -> Result<Image<P>> {
        match self {
            #[cfg(feature = "png")]
            Self::Png => png::PngDecoder::new().decode(stream),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegDecoder::new().decode(stream),
            #[cfg(feature = "gif")]
            Self::Gif => gif::GifDecoder::new().decode(stream),
            #[cfg(feature = "webp")]
            Self::WebP => webp::WebPDecoder::default().decode(stream),
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiDecoder::new().decode(stream),
            _ => panic!(
                "No encoder implementation is found for this image format. \
                 Did you forget to enable the feature?"
            ),
        }
    }

    /// Decodes the image sequence data into an image sequence.
    ///
    /// # Errors
    /// * An error occurred while decoding.
    ///
    /// # Panics
    /// * No decoder implementation is found for this image encoding.
    #[cfg_attr(
        not(any(
            feature = "png",
            feature = "gif",
            feature = "jpeg",
            feature = "webp",
            feature = "qoi"
        )),
        allow(unused_variables, unreachable_code)
    )]
    #[allow(clippy::needless_pass_by_value)] // would require a major refactor
    pub fn run_sequence_decoder<P: Pixel, R: Read>(
        &self,
        stream: R,
    ) -> Result<DynamicFrameIterator<P, R>> {
        Ok(match self {
            #[cfg(feature = "png")]
            Self::Png => DynamicFrameIterator::Png(png::PngDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegDecoder::new().decode_sequence(stream)?,
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiDecoder::new().decode_sequence(stream)?,
            #[cfg(feature = "gif")]
            Self::Gif => DynamicFrameIterator::Gif(gif::GifDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "webp")]
            Self::WebP => {
                DynamicFrameIterator::WebP(webp::WebPDecoder::new().decode_sequence(stream)?)
            }
            _ => panic!(
                "No encoder implementation is found for this image format. \
                 Did you forget to enable the feature?"
            ),
        })
    }
}

impl Display for ImageFormat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Png => "png",
                Self::Jpeg => "jpeg",
                Self::Gif => "gif",
                Self::Bmp => "bmp",
                Self::Tiff => "tiff",
                Self::WebP => "webp",
                Self::Qoi => "qoi",
                Self::Unknown => "",
            }
        )
    }
}
