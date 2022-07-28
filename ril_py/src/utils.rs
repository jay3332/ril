use crate::pixels::{BitPixel, Rgb, Rgba, L};
use pyo3::{exceptions::PyValueError, prelude::*};
use ril::{Dynamic, OverlayMode};

pub fn cast_pixel_to_pyobject(py: Python<'_>, pixel: &Dynamic) -> PyObject {
    match pixel {
        &Dynamic::BitPixel(v) => BitPixel::from(v).into_py(py),
        &Dynamic::L(v) => L::from(v).into_py(py),
        &Dynamic::Rgb(v) => Rgb::from(v).into_py(py),
        &Dynamic::Rgba(v) => Rgba::from(v).into_py(py),
    }
}

pub fn cast_overlay(overlay: &str) -> PyResult<OverlayMode> {
    match overlay {
        "merge" => Ok(OverlayMode::Merge),
        "replace" => Ok(OverlayMode::Replace),
        _ => Err(PyValueError::new_err(format!(
            "Expected one of `merge` or `replace`, got `{}`",
            overlay
        ))),
    }
}
