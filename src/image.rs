use crate::pixel::Pixel;

/// A high-level image representation.
///
/// This represents a static, single-frame image.
/// See [`ImageSequence`] for information on opening animated or multi-frame images.
#[derive(Clone)]
pub struct Image<P: Pixel> {
    width: u32,
    height: u32,
    data: Vec<P>,
}

impl<P: Pixel> Image<P> {
    fn resolve_coordinate(&self, x: u32, y: u32) -> usize {
        (y * self.width + x) as usize
    }

    fn calculate_coordinate(&self, pos: u32) -> (u32, u32) {
        (pos % self.width, pos / self.width)
    }

    /// Returns the width of the image.
    pub fn width(&self) -> u32 {
        self.width
    }

    /// Returns the height of the image.
    pub fn height(&self) -> u32 {
        self.height
    }

    /// Returns the dimensions of the image.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Returns the amount of pixels in the image.
    pub fn len(&self) -> u32 {
        self.width * self.height
    }

    /// Returns a reference of the pixel at the given coordinates.
    pub fn pixel(&self, x: u32, y: u32) -> &P {
        &self.data[self.resolve_coordinate(x, y)]
    }

    /// Returns a mutable reference to the pixel at the given coordinates.
    pub fn pixel_mut(&mut self, x: u32, y: u32) -> &mut P {
        &mut self.data[self.resolve_coordinate(x, y)]
    }

    /// Sets the pixel at the given coordinates to the given pixel.
    pub fn set_pixel(&mut self, x: u32, y: u32, pixel: P) {
        self.data[self.resolve_coordinate(x, y)] = pixel
    }

    /// Takes this image and inverts it.
    pub fn inverted(self) -> Self {
        self.map_pixels(|pixel| pixel.inverted())
    }

    /// Returns the image with the each pixel in the image mapped to the given function.
    ///
    /// The function should take the x and y coordinates followed by the pixel and return the new
    /// pixel.
    pub fn map_pixels(self, f: impl Fn(u32, u32, P) -> P) -> Self {
        Self {
            width: self.width,
            height: self.height,
            data: self.data
                .into_iter()
                .enumerate()
                .map(|(i, p)| {
                    let (x, y) = self.calculate_coordinate(i as u32);

                    f(x, y, p)
                })
                .collect(),
        }
    }

    /// Returns the image with each row of pixels represented as a Vec mapped to the given function.
    ///
    /// The function should take the y coordinate followed by the row of pixels
    /// (represented as a Vec) and return the new row of pixels, also represented as a Vec.
    pub fn map_rows(self, f: impl Fn(u32, Vec<P>) -> Vec<P>) -> Self {
        Self {
            width: self.width,
            height: self.height,
            data: self.data
                .chunks(self.width as usize)
                .into_iter()
                .enumerate()
                .map(|(y, row)| f(y as u32, row.to_vec()))
                .flatten()
                .collect(),
        }
    }
}
