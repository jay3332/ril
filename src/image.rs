use crate::error::{Error::InvalidExtension, Result};
use crate::pixel::Pixel;

use crate::Dynamic;
use std::ffi::OsStr;
use std::fmt;
use std::path::Path;

/// A high-level image representation.
///
/// This represents a static, single-frame image.
/// See [`ImageSequence`] for information on opening animated or multi-frame images.
#[derive(Clone)]
pub struct Image<P: Pixel = Dynamic> {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) data: Vec<P>,
    pub(crate) format: ImageFormat,
}

impl<P: Pixel> Image<P> {
    #[inline]
    #[must_use]
    const fn resolve_coordinate(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    /// Returns the width of the image.
    #[inline]
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the image.
    #[inline]
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Returns a Vec of slices representing the pixels of the image.
    /// Each slice in the Vec is a row. The returned slice should be of ``Vec<&[P; width]>``.
    #[inline]
    #[must_use]
    pub fn pixels(&self) -> Vec<&[P]> {
        self.data.chunks_exact(self.width as usize).collect()
    }

    /// Returns the encoding format of the image. This is nothing more but metadata about the image.
    /// When saving the image, you will still have to explicitly specify the encoding format.
    #[inline]
    #[must_use]
    pub const fn format(&self) -> ImageFormat {
        self.format
    }

    /// Returns the dimensions of the image.
    #[inline]
    #[must_use]
    pub const fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the amount of pixels in the image.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.width * self.height
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

    /// Takes this image and inverts it.
    #[must_use]
    pub fn inverted(self) -> Self {
        self.map_pixels(|pixel| pixel.inverted())
    }

    /// Returns the image replaced with the given data. It is up to you to make sure
    /// the data is the correct size.
    ///
    /// The function should take the current image data and return the new data.
    pub fn map_data<T: Pixel>(self, f: impl FnOnce(Vec<P>) -> Vec<T>) -> Image<T> {
        Image {
            width: self.width,
            height: self.height,
            data: f(self.data),
            format: self.format,
        }
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

    /// Returns the image with each row of pixels represented as a slice mapped to the given
    /// function.
    ///
    /// The function should take the y coordinate followed by the row of pixels
    /// (represented as a slice) and return an Iterator of pixels.
    pub fn map_rows<I, T: Pixel>(self, f: impl Fn(u32, &[P]) -> I) -> Image<T>
    where
        I: IntoIterator<Item = T>,
    {
        let width = self.width;

        self.map_data(|data| {
            data.chunks(width as usize)
                .into_iter()
                .enumerate()
                .flat_map(|(y, row)| f(y as u32, row))
                .collect()
        })
    }

    /// Converts the image into an image with the given pixel type.
    pub fn convert<T: Pixel + From<P>>(self) -> Image<T> {
        self.map_pixels(T::from)
    }

    /// Sets the encoding format of this image. Note that when saving the file,
    /// an encoding format will still have to be explicitly specified.
    /// This is more or less image metadata.
    pub fn set_format(&mut self, format: ImageFormat) {
        self.format = format;
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
    pub fn from_extension(ext: impl AsRef<OsStr>) -> Result<Self> {
        let extension = ext.as_ref().to_str();

        Ok(
            match extension
                .ok_or_else(|| InvalidExtension(ext.as_ref().to_os_string()))?
                .to_ascii_lowercase()
                .as_str()
            {
                "png" => Self::Png,
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
    pub fn infer_encoding() -> Self {
        todo!()
    }
}

impl fmt::Display for ImageFormat {
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
