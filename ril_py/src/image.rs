use std::path::PathBuf;

use crate::draw::DrawEntity;
use crate::error::Error;
use crate::pixels::{BitPixel, Pixel, Rgb, Rgba, L};
use crate::utils::cast_pixel_to_pyobject;
use pyo3::{prelude::*, types::PyType};
use ril::{Dynamic, Image as RilImage, ImageFormat};

/// Python representation of `ril::Image`
#[pyclass]
#[derive(Clone)]
pub struct Image {
    inner: RilImage<Dynamic>,
}

#[pymethods]
impl Image {
    /// Creates a new image with the given width and height, with all pixels being set intially to `fill`.
    #[classmethod]
    fn new(_: &PyType, width: u32, height: u32, fill: Pixel) -> Self {
        Self {
            inner: RilImage::new(width, height, fill.inner),
        }
    }

    /// Decodes an image with the explicitly given image encoding from the raw bytes.
    ///
    /// if `format` is not provided then it will try to infer its encoding.
    #[classmethod]
    fn from_bytes(_: &PyType, bytes: &[u8], format: Option<&str>) -> Result<Self, Error> {
        Ok(if let Some(format) = format {
            Self {
                inner: RilImage::decode_from_bytes(ImageFormat::from_extension(format)?, bytes)?,
            }
        } else {
            Self {
                inner: RilImage::decode_inferred_from_bytes(bytes)?,
            }
        })
    }

    /// Creates a new image shaped with the given width
    /// and a 1-dimensional sequence of pixels which will be shaped according to the width.
    #[classmethod]
    fn from_pixels(_: &PyType, width: u32, pixels: Vec<Pixel>) -> Self {
        Self {
            inner: RilImage::from_pixels(
                width,
                pixels
                    .into_iter()
                    .map(|p| p.inner)
                    .collect::<Vec<Dynamic>>(),
            ),
        }
    }

    /// Opens a file from the given path and decodes it into an image.
    ///
    /// The encoding of the image is automatically inferred.
    /// You can explicitly pass in an encoding by using the [from_bytes] method.
    #[classmethod]
    fn open(_: &PyType, path: PathBuf) -> Result<Self, Error> {
        Ok(Self {
            inner: RilImage::open(path)?,
        })
    }

    /// Returns the overlay mode of the image.
    #[getter]
    fn overlay_mode(&self) -> String {
        format!("{}", self.inner.overlay_mode())
    }

    /// Returns the mode of the image.
    #[getter]
    fn mode(&self) -> &str {
        match self.inner.pixel(0, 0) {
            Dynamic::BitPixel(_) => "bitpixel",
            Dynamic::L(_) => "L",
            Dynamic::Rgb(_) => "RGB",
            Dynamic::Rgba(_) => "RGBA",
        }
    }

    /// Returns the width of the image.
    #[getter]
    fn width(&self) -> u32 {
        self.inner.width()
    }

    /// Returns the height of the image.
    #[getter]
    fn height(&self) -> u32 {
        self.inner.height()
    }

    /// Crops this image in place to the given bounding box.
    fn crop(&mut self, x1: u32, y1: u32, x2: u32, y2: u32) {
        self.inner.crop(x1, y1, x2, y2);
    }

    /// Draws an object or shape onto this image.
    fn draw(&mut self, entity: DrawEntity) {
        entity.0.draw(&mut self.inner);
    }

    /// Encodes the image with the given encoding and returns `bytes`.
    fn encode(&self, encoding: &str) -> Result<Vec<u8>, Error> {
        let encoding = ImageFormat::from_extension(encoding)?;

        let mut buf = Vec::new();
        self.inner.encode(encoding, &mut buf)?;
        
        Ok(buf)
    }

    /// Saves the image with the given encoding to the given path. You can try saving to a memory buffer by using the encode method.
    fn save(&self, encoding: &str, path: PathBuf) -> Result<(), Error> {
        let encoding = ImageFormat::from_extension(encoding)?;
        self.inner.save(encoding, path)?;

        Ok(())
    }

    /// Returns a list of list representing the pixels of the image. Each list in the list is a row.
    ///
    /// For example:
    ///
    /// [[Pixel, Pixel, Pixel], [Pixel, Pixel, Pixel]]
    ///
    /// where the width of the inner list is determined by the width of the image.
    ///
    /// # Warning
    ///
    /// This function requires multiple iterations, so it is a heavy operation for larger image.
    fn pixels(&self, py: Python<'_>) -> Vec<Vec<PyObject>> {
        self.inner
            .pixels()
            .into_iter()
            .map(|p| {
                p.into_iter()
                    .map(|p| cast_pixel_to_pyobject(py, p))
                    .collect::<Vec<PyObject>>()
            })
            .collect::<Vec<Vec<PyObject>>>()
    }

    fn paste(&mut self, x: u32, y: u32, image: Self, mask: Option<Self>) -> Result<(), Error> {
        if let Some(mask) = mask {
            if &mask.format() != "bitpixel" {
                return Err(Error::UnexpectedFormat("bitpixel".to_string(), mask.format()));
            }
    
            self.inner.paste_with_mask(x, y, image.inner, mask.inner.convert::<ril::BitPixel>());
        } else {
            self.inner.paste(x, y, image.inner);
        }

        Ok(())
    }

    fn mask_alpha(&mut self, mask: Self) -> Result<(), Error> {
        if &mask.format() != "L" {
            return Err(Error::UnexpectedFormat("L".to_string(), mask.format()));
        }
        
        self.inner.mask_alpha(&mask.inner.convert::<ril::L>());

        Ok(())
    }

    fn mirror(&mut self) {
        self.inner.mirror();
    }

    fn flip(&mut self) {
        self.inner.flip();
    }

    /// Returns the encoding format of the image.
    /// This is nothing more but metadata about the image.
    /// When saving the image, you will still have to explicitly specify the encoding format.
    #[getter]
    fn format(&self) -> String {
        format!("{}", self.inner.format())
    }

    /// Returns the dimensions of the image.
    #[getter]
    fn dimensions(&self) -> (u32, u32) {
        self.inner.dimensions()
    }

    /// Returns the pixel at the given coordinates.
    fn get_pixel(&self, py: Python<'_>, x: u32, y: u32) -> PyObject {
        match self.inner.pixel(x, y) {
            &Dynamic::BitPixel(v) => BitPixel::from(v).into_py(py),
            &Dynamic::L(v) => L::from(v).into_py(py),
            &Dynamic::Rgb(v) => Rgb::from(v).into_py(py),
            &Dynamic::Rgba(v) => Rgba::from(v).into_py(py),
        }
    }

    /// Sets the pixel at the given coordinates to the given pixel.
    fn set_pixel(&mut self, x: u32, y: u32, pixel: Pixel) {
        self.inner.set_pixel(x, y, pixel.inner)
    }

    /// Inverts the image in-place.
    fn invert(&mut self) {
        self.inner.invert()
    }

    fn __len__(&self) -> usize {
        self.inner.len() as usize
    }

    fn __repr__(&self) -> String {
        format!(
            "<Image mode={} width={} height={} format={} dimensions=({}, {})>",
            self.mode(),
            self.width(),
            self.height(),
            self.format(),
            self.dimensions().0,
            self.dimensions().1
        )
    }

    fn __bool__(&self) -> bool {
        !self.inner.is_empty()
    }
}
