use pyo3::{
    exceptions::{PyIOError, PyRuntimeError, PyTypeError, PyValueError},
    prelude::*,
};
use ril::Error as RilError;

pub enum Error {
    Ril(RilError),
    UnexpectedFormat(String, String), // (Expected, Got)
}

impl From<Error> for PyErr {
    fn from(err: Error) -> Self {
        match err {
            Error::Ril(err) => match err {
                RilError::InvalidHexCode(_) => PyValueError::new_err(format!("{}", err)),
                RilError::InvalidExtension(_) => PyValueError::new_err(format!("{}", err)),
                RilError::DecodingError(_) => PyRuntimeError::new_err(format!("{}", err)),
                RilError::UnknownEncodingFormat => PyRuntimeError::new_err(format!("{}", err)),
                RilError::UnsupportedColorType => PyValueError::new_err(format!("{}", err)),
                RilError::IncompatibleImageData { .. } => {
                    PyRuntimeError::new_err(format!("{}", err))
                }
                RilError::IOError(_) => PyIOError::new_err(format!("{}", err)),
                RilError::EmptyImageError => PyRuntimeError::new_err(
                    "Cannot encode an empty image, or an image without data.",
                ),
            },
            Error::UnexpectedFormat(expected, got) => PyTypeError::new_err(format!(
                "Invalid Image format, expected `{}`, got `{}`",
                expected, got
            )),
        }
    }
}

impl From<RilError> for Error {
    fn from(err: RilError) -> Self {
        Self::Ril(err)
    }
}
