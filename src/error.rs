//! Common error types.

use std::ffi::OsString;
use std::fmt;

/// A shortcut type equivalent to `Result<T, ril::Error>`.
pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    /// An invalid hex code was provided when trying to parse a hex value.
    InvalidHexCode(String),

    /// Received an invalid palette index.
    InvalidPaletteIndex,

    /// An invalid extension was provided when trying to resolve an image's encoding format
    /// from a file extension.
    ///
    /// # Note
    /// This is **not** an error that occurs when the file extension is not recognized, or
    /// is an unknown image extension. This occurs if the OsStr fails conversion to a native
    /// &str. In the case of this, [`ImageFormat::Unknown`] is used instead.
    InvalidExtension(OsString),

    /// Failed to encode an image.
    EncodingError(String),

    /// Invalid data was encountered when an image, usually because it is corrupted.
    ///
    /// Errors can differ across encodings, so the inner ``&'static str`` here is nothing more than
    /// an error message.
    DecodingError(String),

    /// An error occured while trying to render or rasterize a font.
    #[cfg(feature = "text")]
    FontError(&'static str),

    /// No encoding format could be inferred for the given image.
    UnknownEncodingFormat,

    /// An image received data incompatible with the image's dimensions.
    IncompatibleImageData {
        width: u32,
        height: u32,
        received: usize,
    },

    /// Received an unsupported color type when trying to create a pixel from raw data.
    ///
    /// This occurs when the color type is not supported by the pixel type. This is almost
    /// always fixed by switching the pixel type to [`Dynamic`] then using [`Image::convert`]
    /// to convert the image into your desired type.
    UnsupportedColorType,

    /// An error occured when trying to read a file or when trying to write to a file.
    IoError(std::io::Error),

    /// Tried to encode an empty image, or an image without data. This is also raised when trying
    /// to encode an image sequence with no frames.
    EmptyImageError,

    /// Attempted lossless quantization, but there are more unique colors than the desired palette
    /// size.
    QuantizationOverflow {
        /// The amount of unique colors in the image.
        unique_colors: usize,
        /// The desired palette size.
        palette_size: usize,
    },
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidHexCode(hex_code) => write!(f, "Invalid hex code: {hex_code}"),
            Self::InvalidPaletteIndex => write!(f, "Invalid palette index"),
            Self::InvalidExtension(ext) => {
                write!(f, "Invalid extension: {}", ext.to_string_lossy())
            }
            Self::EncodingError(msg) => write!(f, "Encoding error: {msg}"),
            Self::DecodingError(msg) => write!(f, "Decoding error: {msg}"),
            #[cfg(feature = "text")]
            Self::FontError(msg) => write!(f, "Font error: {msg}"),
            Self::UnknownEncodingFormat => write!(f, "Could not infer encoding format"),
            Self::UnsupportedColorType => write!(
                f,
                "Unsupported color type. Try using the `Dynamic` pixel type instead."
            ),
            Self::IncompatibleImageData {
                width,
                height,
                received,
            } => write!(
                f,
                "An image with dimensions {width}x{height} should have {} pixels, received {received} instead",
                width * height,
            ),
            Self::IoError(error) => write!(f, "IO error: {error}"),
            Self::EmptyImageError => write!(f, "Tried encoding an empty image"),
            Self::QuantizationOverflow {
                unique_colors,
                palette_size,
            } => write!(
                f,
                "received an image with more unique colors ({unique_colors}) than the desired \
                maximum ({palette_size}), use a lossy quantization algorithm (i.e. enable the \
                `quantize` cargo feature) to reduce the number of colors before using this \
                function.",
            ),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

#[cfg(feature = "png")]
impl From<png::EncodingError> for Error {
    fn from(err: png::EncodingError) -> Self {
        match err {
            png::EncodingError::IoError(err) => Self::IoError(err),
            png::EncodingError::Format(err) => Self::EncodingError(err.to_string()),
            png::EncodingError::LimitsExceeded => {
                Self::EncodingError("limits exceeded".to_string())
            }
            png::EncodingError::Parameter(err) => Self::EncodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "png")]
impl From<png::DecodingError> for Error {
    fn from(err: png::DecodingError) -> Self {
        match err {
            png::DecodingError::IoError(err) => Self::IoError(err),
            png::DecodingError::Format(err) => Self::DecodingError(err.to_string()),
            png::DecodingError::LimitsExceeded => {
                Self::DecodingError("limits exceeded".to_string())
            }
            png::DecodingError::Parameter(err) => Self::DecodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "jpeg")]
impl From<jpeg_decoder::Error> for Error {
    fn from(err: jpeg_decoder::Error) -> Self {
        match err {
            jpeg_decoder::Error::Io(err) => Self::IoError(err),
            err => Self::DecodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "jpeg")]
impl From<jpeg_encoder::EncodingError> for Error {
    fn from(err: jpeg_encoder::EncodingError) -> Self {
        match err {
            jpeg_encoder::EncodingError::IoError(err) => Self::IoError(err),
            err => Self::EncodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "gif")]
impl From<gif::EncodingError> for Error {
    fn from(err: gif::EncodingError) -> Self {
        match err {
            gif::EncodingError::Io(err) => Self::IoError(err),
            gif::EncodingError::Format(err) => Self::EncodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "gif")]
impl From<gif::DecodingError> for Error {
    fn from(err: gif::DecodingError) -> Self {
        match err {
            gif::DecodingError::Io(err) => Self::IoError(err),
            gif::DecodingError::Format(err) => Self::DecodingError(err.to_string()),
        }
    }
}

#[cfg(feature = "qoi")]
impl From<qoi::Error> for Error {
    fn from(value: qoi::Error) -> Self {
        use crate::Error::*;
        use qoi::Error::*;
        match value {
            InvalidMagic { .. } => DecodingError("invalid magic number".to_string()),
            InvalidChannels { channels } => EncodingError(format!(
                "qoi only supports either 3 or 4 channels, got {channels}"
            )),
            InvalidColorSpace { .. } => {
                DecodingError("colorspace of image is malformed".to_string())
            }
            InvalidImageDimensions { width, height } => {
                if width.min(height) == 0 {
                    EmptyImageError
                } else {
                    EncodingError(format!(
                        "image dimensions of {} by {} are not valid, must be below 400Mp",
                        width, height
                    ))
                }
            }
            InvalidImageLength {
                size,
                width,
                height,
            } => IncompatibleImageData {
                width,
                height,
                received: size,
            },
            OutputBufferTooSmall { size, required } => EncodingError(format!(
                "buffer of size {} is too small to hold image of size {}",
                size, required
            )),
            UnexpectedBufferEnd => {
                DecodingError("buffer reached end before decoding was finished".to_string())
            }
            InvalidPadding => DecodingError(
                "incorrectly placed stream end marker encountered during decoding".to_string(),
            ),
            qoi::Error::IoError(error) => Error::IoError(error),
        }
    }
}