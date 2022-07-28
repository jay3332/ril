use pyo3::{
    exceptions::{PyIOError, PyRuntimeError, PyValueError},
    prelude::*,
};
use ril::Error as RilError;

pub struct Error(RilError);

impl From<Error> for PyErr {
    fn from(err: Error) -> Self {
        let err = err.0;

        match err {
            RilError::InvalidHexCode(_) => PyValueError::new_err(format!("{}", err)),
            RilError::InvalidExtension(_) => PyValueError::new_err(format!("{}", err)),
            RilError::DecodingError(_) => PyRuntimeError::new_err(format!("{}", err)),
            RilError::UnknownEncodingFormat => PyRuntimeError::new_err(format!("{}", err)),
            RilError::UnsupportedColorType => PyValueError::new_err(format!("{}", err)),
            RilError::IncompatibleImageData { .. } => PyRuntimeError::new_err(format!("{}", err)),
            RilError::IOError(_) => PyIOError::new_err(format!("{}", err)),
        }
    }
}

impl From<RilError> for Error {
    fn from(err: RilError) -> Self {
        Self(err)
    }
}
