mod draw;
mod error;
mod image;
mod pixels;
mod utils;

use draw::{Border, Rectangle};
use image::Image;
use pixels::*;
use pyo3::prelude::*;

type Xy = (u32, u32);

macro_rules! add_classes {
    ( $m:expr, $( $class:ty ),* ) => {{
        $(
            $m.add_class::<$class>()?;
        )*
    }};
}

#[pymodule]
fn ril(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    add_classes!(m, BitPixel, Image, L, Pixel, Rgb, Rgba, Border, Rectangle);

    Ok(())
}
