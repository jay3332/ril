//! Filters that can be applied on images.

use crate::{Image, Pixel};
use std::marker::PhantomData;

/// An image filter than can be lazily applied to an image or a filtered image.
pub trait Filter {
    /// The pixel type of the input image.
    type Input: Pixel;
    /// The pixel type of the output image.
    type Output: Pixel;

    /// Applies the filter to the given pixel.
    fn apply_pixel(&self, x: u32, y: u32, pixel: &Self::Input) -> Self::Output;

    /// Applies the filter to the given image.
    fn apply_image(&self, image: Image<Self::Input>) -> Image<Self::Output> {
        image.map_pixels_with_coords(|x, y, pixel| self.apply_pixel(x, y, &pixel))
    }
}

/// A brightness filter.
pub struct BrightnessFilter<P: Pixel> {
    /// The brightness adjustment factor, between -1.0 and 1.0.
    pub factor: f64,
    _marker: PhantomData<P>,
}

impl<P: Pixel> BrightnessFilter<P>
where
    P::Subpixel:,
{
    /// Creates a new brightness filter.
    #[must_use]
    pub const fn new(factor: f64) -> Self {
        Self {
            factor,
            _marker: PhantomData,
        }
    }
}
//
// impl<P: Pixel> Filter for BrightnessFilter<P> {
//     type Input = P;
//     type Output = P;
//
//     fn apply_pixel(&self, _x: u32, _y: u32, pixel: &Self::Input) -> Self::Output {
//         pixel.map_subpixels(
//             |c| {
//                 let c = c as f64 + self.factor * 255.0;
//                 c.max(0.0).min(255.0) as u8
//             },
//             |alpha| alpha,
//         )
//     }
// }
