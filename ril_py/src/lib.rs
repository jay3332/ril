mod pixels;

use pyo3::prelude::*;
use ril::{Image as RilImage, Dynamic};
use pixels::*;

/// Python representation of `ril::Image`
#[pyclass]
struct Image {
    inner: RilImage<Dynamic>,
}

#[pymethods]
impl Image {
    #[new]
    fn new() -> Self {
        todo!()
    }

    #[getter]
    fn mode(&self) -> &str {
        match self.inner.pixel(0, 0) {
            Dynamic::BitPixel(_) => "bitpixel",
            Dynamic::L(_) => "L",
            Dynamic::Rgb(_) => "RGB",
            Dynamic::Rgba(_) => "RGBA"
        }
    }

    #[getter]
    fn width(&self) -> u32 {
        self.inner.width()
    }

    #[getter]
    fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Returns a list of list representing the pixels of the image. Each list in the list is a row.
    /// 
    /// For example:
    /// 
    /// [[Pixel, Pixel, Pixel], [Pixel, Pixel, Pixel]]
    /// 
    /// where the width of the inner list is determined by the width of the image.
    fn pixels(&self, py: Python<'_>) -> Vec<Vec<PyObject>> {
        self.inner.pixels().into_iter().map(
            |p| p.into_iter().map(
                |p| match p {
                    &Dynamic::BitPixel(v) => BitPixel::from(v).into_py(py),
                    &Dynamic::L(v) => L::from(v).into_py(py),
                    &Dynamic::Rgb(v) => Rgb::from(v).into_py(py),
                    &Dynamic::Rgba(v) => Rgba::from(v).into_py(py)
                }
            ).collect::<Vec<PyObject>>()
        )
        .collect::<Vec<Vec<PyObject>>>()
    }

    fn format(&self) -> String {
        format!("{}", self.inner.format())
    }

    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    fn pixel(&self, py: Python<'_>, x: u32, y: u32) -> PyObject {
        match self.inner.pixel(x, y) {
            &Dynamic::BitPixel(v) => BitPixel::from(v).into_py(py),
            &Dynamic::L(v) => L::from(v).into_py(py),
            &Dynamic::Rgb(v) => Rgb::from(v).into_py(py),
            &Dynamic::Rgba(v) => Rgba::from(v).into_py(py)
        }
    }

    fn __len__(&self) -> usize {
        self.inner.len() as usize
    }
}

#[pymodule]
fn ril(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_class::<Image>()?;
    m.add_class::<BitPixel>()?;
    m.add_class::<L>()?;
    m.add_class::<Rgb>()?;
    m.add_class::<Rgba>()?;

    Ok(())
}
