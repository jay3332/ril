use crate::{
    error::{Error::InvalidExtension, Result},
    FrameIterator, Image, Pixel,
};
use std::{
    ffi::OsStr,
    fmt,
    fmt::Display,
    io::{Read, Write},
    path::Path,
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
#[cfg(any(
    feature = "png",
    feature = "gif",
    feature = "jpeg",
    feature = "webp",
    feature = "qoi"
))]
use crate::{Decoder, Encoder};

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
            // Not official, but in the spec
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
    /// * An error occured while encoding.
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
    pub fn run_encoder<P: Pixel>(&self, image: &Image<P>, dest: impl Write) -> Result<()> {
        match self {
            #[cfg(feature = "png")]
            Self::Png => png::PngEncoder::encode_static(image, dest),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegEncoder::encode_static(image, dest),
            #[cfg(feature = "gif")]
            Self::Gif => gif::GifEncoder::encode_static(image, dest),
            #[cfg(feature = "webp")]
            Self::WebP => webp::WebPStaticEncoder::encode_static(image, dest),
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiEncoder::encode_static(image, dest),
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
    /// * An error occured while encoding.
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
        dest: impl Write,
    ) -> Result<()> {
        match self {
            #[cfg(feature = "png")]
            Self::Png => png::PngEncoder::encode_sequence(seq, dest),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => jpeg::JpegEncoder::encode_sequence(seq, dest),
            #[cfg(feature = "gif")]
            Self::Gif => gif::GifEncoder::encode_sequence(seq, dest),
            #[cfg(feature = "webp")]
            Self::WebP => webp::WebPMuxEncoder::encode_sequence(seq, dest),
            #[cfg(feature = "qoi")]
            Self::Qoi => qoi::QoiEncoder::encode_sequence(seq, dest),
            _ => panic!(
                "No encoder implementation is found for this image format. \
                 Did you forget to enable the feature?"
            ),
        }
    }

    /// Decodes the image data from into an image.
    ///
    /// # Errors
    /// * An error occured while decoding.
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
            Self::WebP => webp::WebPDecoder::new().decode(stream),
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
    /// * An error occured while decoding.
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
    pub fn run_sequence_decoder<'a, P: Pixel + 'a, R: Read + 'a>(
        &self,
        stream: R,
    ) -> Result<Box<dyn FrameIterator<P> + 'a>> {
        Ok(match self {
            #[cfg(feature = "png")]
            Self::Png => Box::new(png::PngDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "jpeg")]
            Self::Jpeg => Box::new(jpeg::JpegDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "gif")]
            Self::Gif => Box::new(gif::GifDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "webp")]
            Self::WebP => Box::new(webp::WebPDecoder::new().decode_sequence(stream)?),
            #[cfg(feature = "qoi")]
            Self::Qoi => Box::new(qoi::QoiDecoder::new().decode_sequence(stream)?),
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
