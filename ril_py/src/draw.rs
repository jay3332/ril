use std::{fmt::Display, marker::PhantomData};

use pyo3::{
    exceptions::{PyRuntimeError, PyValueError},
    prelude::*, types::PyType,
};
use ril::{
    draw::{Border as RilBorder, BorderPosition as RilBorderPosition, Rectangle as RilRectangle},
    Dynamic, OverlayMode, Draw,
};

use crate::{
    pixels::Pixel,
    utils::{cast_overlay, cast_pixel_to_pyobject},
    Xy,
};

fn get_border_position(position: &str) -> PyResult<RilBorderPosition> {
    match position {
        "inset" => Ok(RilBorderPosition::Inset),
        "center" => Ok(RilBorderPosition::Center),
        "outset" => Ok(RilBorderPosition::Outset),
        _ => Err(PyValueError::new_err(
            "position provided is not valid, it must be one of `inset`, `center`, or `outset`"
                .to_string(),
        )),
    }
}

fn from_border_position(position: RilBorderPosition) -> String {
    match position {
        RilBorderPosition::Inset => "inset".to_string(),
        RilBorderPosition::Center => "center".to_string(),
        RilBorderPosition::Outset => "outset".to_string(),
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Border {
    pub inner: RilBorder<Dynamic>,
}

#[pymethods]
impl Border {
    #[new]
    fn new(color: Pixel, thickness: u32, position: &str) -> PyResult<Self> {
        let position = get_border_position(position)?;

        Ok(Self {
            inner: RilBorder {
                color: color.inner,
                thickness,
                position,
            },
        })
    }

    #[getter]
    fn get_color(&self) -> Pixel {
        self.inner.color.into()
    }

    #[getter]
    fn get_thickness(&self) -> u32 {
        self.inner.thickness
    }

    #[getter]
    fn get_border_position(&self) -> String {
        from_border_position(self.inner.position)
    }

    #[setter]
    fn set_color(&mut self, pixel: Pixel) {
        self.inner.color = pixel.inner;
    }

    #[setter]
    fn set_thickness(&mut self, thickness: u32) {
        self.inner.thickness = thickness;
    }

    #[setter]
    fn set_border_position(&mut self, position: &str) -> PyResult<()> {
        self.inner.position = get_border_position(position)?;

        Ok(())
    }

    fn __repr__(&self) -> String {
        format!(
            "<Border color={} thickness={} position={}>",
            self.get_color(),
            self.get_thickness(),
            self.get_border_position()
        )
    }
}

impl Display for Border {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.__repr__())
    }
}

#[pyclass]
#[derive(Clone)]
pub struct Rectangle {
    pub inner: RilRectangle<Dynamic>,
}

#[pymethods]
impl Rectangle {
    #[new]
    fn new(
        position: Xy,
        size: Xy,
        border: Option<Border>,
        fill: Option<Pixel>,
        overlay: Option<&str>,
    ) -> PyResult<Self> {
        let overlay = if let Some(overlay) = overlay {
            Some(match overlay {
                "merge" => Ok::<OverlayMode, PyErr>(OverlayMode::Merge),
                "replace" => Ok(OverlayMode::Replace),
                _ => {
                    return Err(PyValueError::new_err(format!(
                        "Expected `merge` or `replace`, got `{}`",
                        overlay
                    )))
                }
            }?)
        } else {
            None
        };

        Ok(Self {
            inner: RilRectangle {
                position,
                size,
                border: border.map(|b| b.inner),
                fill: fill.map(|f| f.inner),
                overlay,
            },
        })
    }

    #[classmethod]
    fn from_bounding_box(_: &PyType, x1: u32, y1: u32, x2: u32, y2: u32) -> Self {
        Self { inner: RilRectangle::from_bounding_box(x1, y1, x2, y2) }
    }

    #[getter]
    fn get_position(&self) -> Xy {
        self.inner.position
    }

    #[getter]
    fn get_size(&self) -> Xy {
        self.inner.size
    }

    #[getter]
    fn get_border(&self) -> Option<Border> {
        self.inner
            .border
            .as_ref()
            .map(|b| Border { inner: b.clone() })
    }

    #[getter]
    fn get_fill(&self, py: Python<'_>) -> Option<PyObject> {
        if let Some(fill) = self.inner.fill {
            Some(cast_pixel_to_pyobject(py, &fill))
        } else {
            None
        }
    }

    #[getter]
    fn get_overlay(&self) -> Option<String> {
        self.inner.overlay.map(|o| format!("{}", o))
    }

    #[setter]
    fn set_position(&mut self, position: Xy) {
        self.inner.position = position;
    }

    #[setter]
    fn set_size(&mut self, size: Xy) {
        self.inner.size = size;
    }

    #[setter]
    fn set_border(&mut self, border: Option<Border>) {
        self.inner.border = border.map(|b| b.inner);
    }

    #[setter]
    fn set_fill(&mut self, fill: Option<Pixel>) {
        self.inner.fill = fill.map(|f| f.inner);
    }

    #[setter]
    fn set_overlay(&mut self, overlay: &str) -> PyResult<()> {
        self.inner.overlay = Some(cast_overlay(overlay)?);

        Ok(())
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "<Rectangle position=({}, {}) size=({}, {}) border={} fill={} overlay={}>",
            self.get_position().0,
            self.get_position().1,
            self.get_size().0,
            self.get_size().1,
            self.get_border()
                .map_or("None".to_string(), |f| format!("{}", f)),
            self.get_fill(py)
                .map_or("None".to_string(), |f| format!("{}", f)),
            self.get_overlay()
                .map_or("None".to_string(), |f| format!("{}", f))
        )
    }
}

macro_rules! impl_draw_entities {
    ( $obj:expr, $( $class:ty ),* ) => {{
        $(
            match $obj.extract::<$class>() {
                Ok(r) => return Ok(Self(Box::new(r.inner), PhantomData)),
                Err(_) => ()
            }
        )*

        Err(PyRuntimeError::new_err(
            "Invalid argument for draw".to_string(),
        ))
    }};
}

pub struct DrawEntity<'a>(pub Box<dyn Draw<Dynamic>>, PhantomData<&'a ()>);

impl<'a> FromPyObject<'a> for DrawEntity<'a> {
    fn extract(obj: &'a PyAny) -> PyResult<Self> {
        impl_draw_entities!(obj, Rectangle)
    }
}
