use crate::{
    draw::Draw,
    encode::{Decoder, Encoder},
    encodings::png, error::{
        Error::{self, InvalidExtension},
        Result,
    },
    pixel::Pixel,
    Dynamic,
    ResizeAlgorithm,
    DynamicFrameIterator,
};

use std::{
    ffi::OsStr,
    fmt::{self, Display},
    fs::File,
    io::{Read, Write},
    num::NonZeroU32,
    path::Path,
};

/// The behavior to use when overlaying images on top of each other.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum OverlayMode {
    /// Replace alpha values with the alpha values of the overlay image. This is the default
    /// behavior.
    #[default]
    Replace,
    /// Merge the alpha values of overlay image with the alpha values of the base image.
    Merge,
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
    pub(crate) background: P,
}

impl<P: Pixel> Image<P> {
    /// Creates a new image with the given width and height, with all pixels being set
    /// intially to `fill`.
    ///
    /// Both the width and height must be non-zero.
    #[must_use]
    pub fn new(width: u32, height: u32, fill: P) -> Self {
        Self {
            width: NonZeroU32::new(width).unwrap(),
            height: NonZeroU32::new(height).unwrap(),
            data: vec![fill; (width * height) as usize],
            format: ImageFormat::default(),
            overlay: OverlayMode::default(),
            background: P::default(),
        }
    }

    /// Creates a new image with the given width and height. The pixels are then resolved through
    /// then given callback function which takes two parameters - the x and y coordinates of
    /// a pixel - and returns a pixel.
    #[must_use]
    pub fn from_fn(width: u32, height: u32, f: impl Fn(u32, u32) -> P) -> Self {
        Self::new(width, height, P::default()).map_pixels_with_coords(|x, y, _| f(x, y))
    }

    /// Creates a new image shaped with the given width and a 1-dimensional sequence of pixels
    /// which will be shaped according to the width.
    ///
    /// # Panics
    /// * The length of the pixels is not a multiple of the width.
    #[must_use]
    pub fn from_pixels(width: u32, pixels: impl AsRef<[P]>) -> Self {
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
            background: P::default(),
        }
    }

    /// Decodes an image with the explicitly given image encoding from the raw byte stream.
    ///
    /// # Errors
    /// * `DecodingError`: The image could not be decoded, maybe it is corrupt.
    pub fn decode_from_bytes(format: ImageFormat, bytes: impl Read) -> Result<Self> {
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
    pub fn decode_inferred_from_bytes(mut bytes: impl Read) -> Result<Self> {
        let buf = &mut [0; 12];
        let n = bytes.read(buf)?;

        match ImageFormat::infer_encoding(buf) {
            ImageFormat::Unknown => Err(Error::UnknownEncodingFormat),
            format => format.run_decoder((&buf[..n]).chain(bytes)),
        }
    }

    /// Opens a file from the given path and decodes it into an image.
    ///
    /// The encoding of the image is automatically inferred. You can explicitly pass in an encoding
    /// by using the [`decode_from_bytes`] method.
    ///
    /// # Errors
    /// todo!()
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
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    pub fn encode(&self, encoding: ImageFormat, dest: &mut impl Write) -> Result<()> {
        encoding.run_encoder(self, dest)
    }

    /// Saves the image with the given encoding to the given path.
    /// You can try saving to a memory buffer by using the [`encode`] method.
    ///
    /// # Errors
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
    pub fn save(&self, encoding: ImageFormat, path: impl AsRef<Path>) -> Result<()> {
        let mut file = File::create(path).map_err(Error::IOError)?;
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
    /// * An error occured during encoding.
    ///
    /// # Panics
    /// * No encoder implementation for the given encoding format.
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
        (y * self.width() + x) as usize
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

    /// Returns a Vec of slices representing the pixels of the image.
    /// Each slice in the Vec is a row. The returned slice should be of ``Vec<&[P; width]>``.
    #[inline]
    #[must_use]
    pub fn pixels(&self) -> Vec<&[P]> {
        self.data.chunks_exact(self.width() as usize).collect()
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

    /// Returns the background color of the image.
    ///
    /// This can be thought of as the default color for this image when a pixel is missing a color.
    #[inline]
    #[must_use]
    pub const fn background_color(&self) -> P {
        self.background
    }

    /// Returns the same image with its background color set to the given value.
    #[must_use]
    pub const fn with_background_color(mut self, color: P) -> Self {
        self.background = color;
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
        self.overlay_pixel_with_mode(x, y, pixel, self.overlay)
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

    /// Inverts this image in place.
    pub fn invert(&mut self) {
        self.data.iter_mut().for_each(|p| *p = p.inverted());
    }

    /// Takes this image and inverts it. Useful for method chaining.
    #[must_use]
    pub fn inverted(self) -> Self {
        self.map_pixels(|pixel| pixel.inverted())
    }

    /// Returns the image replaced with the given data. It is up to you to make sure
    /// the data is the correct size.
    ///
    /// The function should take the current image data and return the new data.
    ///
    /// # Note
    /// This resets the background color back to the default.
    pub fn map_data<T: Pixel>(self, f: impl FnOnce(Vec<P>) -> Vec<T>) -> Image<T> {
        Image {
            width: self.width,
            height: self.height,
            data: f(self.data),
            format: self.format,
            overlay: self.overlay,
            background: T::default(),
        }
    }

    /// Sets the data of this image to the new data. This is used a lot internally,
    /// but should rarely be used by you.
    ///
    /// # Panics
    /// * Panics if the data is misinformed.
    pub fn set_data(&mut self, data: Vec<P>) {
        assert_eq!(
            self.width() * self.height(),
            data.len() as u32,
            "misformed data"
        );

        self.data = data;
    }

    /// Returns the image with each pixel in the image mapped to the given function.
    ///
    /// The function should take the pixel and return another pixel.
    pub fn map_pixels<T: Pixel>(self, f: impl Fn(P) -> T) -> Image<T> {
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
                .into_iter()
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
    pub fn crop(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        self.width = NonZeroU32::new(x2 - x1).unwrap();
        self.height = NonZeroU32::new(y2 - y1).unwrap();
        self.data = self
            .pixels()
            .into_iter()
            .skip(y1 as usize)
            .zip(y1..y2)
            .flat_map(|(row, _)| &row[x1 as usize..x2 as usize])
            .copied()
            .collect();
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
        let chunks = self
            .data
            .chunks_exact(self.width() as usize)
            .collect::<Vec<_>>();

        let flipped = (0..self.width() as usize)
            .map(|i| chunks.iter().map(|c| c[i]).rev().collect::<Vec<_>>())
            .collect::<Vec<_>>();

        self.data = (0..self.height() as usize)
            .flat_map(|i| flipped.iter().map(|c| c[i]).collect::<Vec<_>>())
            .collect()
    }

    /// Takes this image and flips it vertically, or about the x-axis. Useful for method chaining.
    #[must_use]
    pub fn flipped(mut self) -> Self {
        self.flip();
        self
    }

    /// Resizes this image in place to the given dimensions using the given resizing algorithm
    /// in place.
    pub fn resize(&mut self, width: u32, height: u32, algorithm: ResizeAlgorithm) {
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
    #[must_use]
    pub fn resized(mut self, width: u32, height: u32, algorithm: ResizeAlgorithm) -> Self {
        self.resize(width, height, algorithm);
        self
    }

    /// Draws an object or shape onto this image.
    pub fn draw(&mut self, entity: &impl Draw<P>) {
        entity.draw(self);
    }

    /// Takes this image, draws the given object or shape onto it, and returns it.
    /// Useful for method chaining and drawing multiple objects at once.
    #[must_use]
    pub fn with(mut self, entity: &impl Draw<P>) -> Self {
        self.draw(entity);
        self
    }

    /// Pastes the given image onto this image at the given x and y coordinates.
    ///
    /// This is a shorthand for using the [`draw`] method with [`Paste`].
    pub fn paste(&mut self, x: u32, y: u32, image: Image<P>) {
        self.draw(&crate::Paste::new(image).with_position(x, y));
    }

    /// Pastes the given image onto this image at the given x and y coordinates,
    /// masked with the given masking image.
    ///
    /// Currently, only [`BitPixel`] images are supported for the masking image.
    ///
    /// This is a shorthand for using the [`draw`] method with [`Paste`].
    pub fn paste_with_mask(
        &mut self,
        x: u32,
        y: u32,
        image: Image<P>,
        mask: Image<crate::BitPixel>,
    ) {
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
    pub fn mask_alpha(&mut self, mask: &Image<crate::L>)
    where
        P: crate::Alpha,
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
}

/// Represents an image with multiple channels, called bands.
///
/// Each band should be represented as a separate [`Image`] with [`L`] or [`BitPixel`] pixels.
pub trait Banded<T> {
    /// Takes this image and returns its bands.
    fn bands(&self) -> T;

    /// Creates a new image from the given bands.
    fn from_bands(bands: T) -> Self;
}

type Band = Image<crate::L>;

macro_rules! map_idx {
    ($image:expr, $idx:expr) => {{
        use $crate::{Image, L};

        Image {
            width: $image.width,
            height: $image.height,
            data: $image
                .data
                .iter()
                .map(|p| L(p.as_pixel_data().data()[$idx]))
                .collect(),
            format: $image.format,
            overlay: $image.overlay,
            background: Default::default(),
        }
    }};
}

macro_rules! extract_bands {
    ($image:expr; $($idx:expr),+) => {{
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

impl Banded<(Band, Band, Band)> for Image<crate::Rgb> {
    fn bands(&self) -> (Band, Band, Band) {
        extract_bands!(self; 0, 1, 2)
    }

    fn from_bands((r, g, b): (Band, Band, Band)) -> Self {
        use crate::L;

        validate_dimensions!(r, g, b);

        r.map_data(|data| {
            data.into_iter()
                .zip(g.data.into_iter())
                .zip(b.data.into_iter())
                .map(|((L(r), L(g)), L(b))| crate::Rgb::new(r, g, b))
                .collect()
        })
    }
}

impl Banded<(Band, Band, Band, Band)> for Image<crate::Rgba> {
    fn bands(&self) -> (Band, Band, Band, Band) {
        extract_bands!(self; 0, 1, 2, 3)
    }

    fn from_bands((r, g, b, a): (Band, Band, Band, Band)) -> Self {
        use crate::L;

        validate_dimensions!(r, g, b, a);

        r.map_data(|data| {
            data.into_iter()
                .zip(g.data.into_iter())
                .zip(b.data.into_iter())
                .zip(a.data.into_iter())
                .map(|(((L(r), L(g)), L(b)), L(a))| crate::Rgba::new(r, g, b, a))
                .collect()
        })
    }
}

/// Represents the underlying encoding format of an image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ImageFormat {
    /// No known encoding is known for the image.
    ///
    /// This is usually because the image was created manually. See [`Image::set_format`]
    /// to manually set the encoding format.
    #[default]
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
    /// the [`Error::InvalidExtension`] error is returned.
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
        } else {
            Self::Unknown
        }
    }

    /// Encodes the `Image` into raw bytes.
    ///
    /// # Errors
    /// * An error occured while encoding.
    ///
    /// # Panics
    /// * No encoder implementation is found for this image encoding.
    pub fn run_encoder<P: Pixel>(&self, image: &Image<P>, dest: &mut impl Write) -> Result<()> {
        match self {
            Self::Png => png::PngEncoder::new().encode(image, dest),
            _ => panic!("No encoder implementation is found for this image format"),
        }
    }

    /// Encodes the `ImageSequence1 into raw bytes. If the encoding does not supported image
    /// sequences (or multi-frame images), it will only encode the first frame.
    ///
    /// # Errors
    /// * An error occured while encoding.
    ///
    /// # Panics
    /// * No encoder implementation is found for this image encoding.
    pub fn run_sequence_encoder<P: Pixel>(&self, seq: &crate::ImageSequence<P>, dest: &mut impl Write) -> Result<()> {
        match self {
            Self::Png => png::PngEncoder::new().encode_sequence(seq, dest),
            _ => panic!("No encoder implementation is found for this image format"),
        }
    }

    /// Decodes the image data from into an image.
    ///
    /// # Errors
    /// * An error occured while decoding.
    ///
    /// # Panics
    /// * No decoder implementation is found for this image encoding.
    pub fn run_decoder<P: Pixel>(&self, stream: impl Read) -> Result<Image<P>> {
        match self {
            Self::Png => png::PngDecoder::new().decode(stream),
            _ => panic!("No decoder implementation for this image format"),
        }
    }

    /// Decodes the image sequence data into an image sequence.
    ///
    /// # Errors
    /// * An error occured while decoding.
    ///
    /// # Panics
    /// * No decoder implementation is found for this image encoding.
    pub fn run_sequence_decoder<P: Pixel, R: Read>(&self, stream: R) -> Result<DynamicFrameIterator<P, R>> {
        Ok(match self {
            Self::Png => DynamicFrameIterator::Png(png::PngDecoder::new().decode_sequence(stream)?),
            _ => panic!("No decoder implementation for this image format"),
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
                Self::Unknown => "",
            }
        )
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use crate::prelude::*;

    #[test]
    fn test_encoding() {
        let image = Image::open("/Users/jay3332/Downloads/jay3332.png").unwrap();
        let mask = Image::new(image.width(), image.height(), BitPixel(false)).with(
            &Ellipse::from_bounding_box(0, 0, image.width(), image.height())
                .with_fill(BitPixel(true)),
        );
        let mut background = Image::new(image.width(), image.height(), Rgba::transparent());

        background.paste_with_mask(0, 0, image, mask);
        background.save_inferred("test.png").unwrap();
    }

    #[test]
    fn test_sequence() {
        let seq = ImageSequence::<Rgba>::new()
            .with_frame(
                Frame::from_image(
                    Image::open("/Users/jay3332/Downloads/jay3332.png")
                        .unwrap()
                        .resized(256, 256, ResizeAlgorithm::Nearest),
                )
                .with_delay(Duration::from_millis(500)),
            )
            .with_frame(
                Frame::from_image(
                    Image::open("/Users/jay3332/Downloads/jay3332.png")
                        .unwrap()
                        .inverted()
                        .resized(256, 256, ResizeAlgorithm::Nearest),
                )
                .with_delay(Duration::from_millis(500)),
            );

        seq.save_inferred("test.png").unwrap();
    }
}
