use pyo3::prelude::*;

/// Represents a single-bit pixel that represents either a pixel that is on or off.
#[pyclass]
pub struct BitPixel {
    #[pyo3(get)]
    value: bool
}

#[pyclass]
pub struct L {
    #[pyo3(get)]
    value: u8
}

#[pyclass]
pub struct Rgb {
    #[pyo3(get)]
    r: u8,
    #[pyo3(get)]
    g: u8,
    #[pyo3(get)]
    b: u8
}

#[pyclass]
pub struct Rgba {
    #[pyo3(get)]
    r: u8,
    #[pyo3(get)]
    g: u8,
    #[pyo3(get)]
    b: u8,
    #[pyo3(get)]
    a: u8
}

impl From<ril::BitPixel> for BitPixel {
    fn from(pixel: ril::BitPixel) -> Self {
        Self { value: pixel.value() }
    }
}

impl From<ril::L> for L {
    fn from(pixel: ril::L) -> Self {
        Self { value: pixel.value() }
    }
}

impl From<ril::Rgb> for Rgb {
    fn from(pixel: ril::Rgb) -> Self {
        Self { r: pixel.r, g: pixel.g, b: pixel.b }
    }
}

impl From<ril::Rgba> for Rgba {
    fn from(pixel: ril::Rgba) -> Self {
        Self { r: pixel.r, g: pixel.g, b: pixel.b, a: pixel.a }
    }
}
