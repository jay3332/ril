use std::ffi::OsString;
use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

/// Represents an error that occurs within the crate.
#[derive(Debug)]
pub enum Error {
    /// An invalid hex code was provided when trying to parse a hex value.
    InvalidHexCode(String),

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
    IOError(std::io::Error),

    /// Tried to encode an empty image, or an image without data.
    EmptyImageError,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::InvalidHexCode(hex_code) => write!(f, "Invalid hex code: {}", hex_code),
            Self::InvalidExtension(ext) => {
                write!(f, "Invalid extension: {}", ext.to_string_lossy())
            }
            Self::EncodingError(msg) => write!(f, "Encoding error: {}", msg),
            Self::DecodingError(message) => write!(f, "Decoding error: {}", message),
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
                "An image with dimensions {}x{} should have {} pixels, received {} instead",
                width,
                height,
                width * height,
                received,
            ),
            Self::IOError(error) => write!(f, "IO error: {}", error),
            Self::EmptyImageError => write!(f, "Tried encoding an empty image"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::IOError(err)
    }
}

impl From<png::EncodingError> for Error {
    fn from(err: png::EncodingError) -> Self {
        match err {
            png::EncodingError::IoError(err) => Self::IOError(err),
            png::EncodingError::Format(err) => Self::EncodingError(err.to_string()),
            png::EncodingError::LimitsExceeded => Self::EncodingError("limits exceeded".to_string()),
            png::EncodingError::Parameter(err) => Self::EncodingError(err.to_string()),
        }
    }
}

impl From<png::DecodingError> for Error {
    fn from(err: png::DecodingError) -> Self {
        match err {
            png::DecodingError::IoError(err) => Self::IOError(err),
            png::DecodingError::Format(err) => Self::DecodingError(err.to_string()),
            png::DecodingError::LimitsExceeded => Self::DecodingError("limits exceeded".to_string()),
            png::DecodingError::Parameter(err) => Self::DecodingError(err.to_string()),
        }
    }
}
