//! Filters that can be applied on images.

use crate::{BitPixel, Image, Pixel};
use num_traits::{bounds::UpperBounded, AsPrimitive, FromPrimitive, Zero};
use std::marker::PhantomData;

/// An image filter than can be lazily applied to an image or a filtered image.
///
/// Filters make the following guarantees:
/// * They preserve the image dimensions.
/// * They modify single pixels at a time. They can use surrounding pixels for context, but they
///   cannot modify them.
pub trait Filter {
    /// The pixel type of the input image.
    type Input: Pixel;
    /// The pixel type of the output image.
    type Output: Pixel;

    /// Applies the filter to the given pixel.
    fn apply_pixel(
        &self,
        image: &Image<Self::Input>,
        x: u32,
        y: u32,
        pixel: Self::Input,
    ) -> Self::Output;

    /// Applies the filter to the given image.
    fn apply_image(&self, image: Image<Self::Input>) -> Image<Self::Output> {
        image.map_image_with_coords(|image, x, y, pixel| self.apply_pixel(image, x, y, pixel))
    }
}

/// A brightness filter.
pub struct Brightness<P: Pixel> {
    /// The brightness adjustment factor, between -1.0 and 1.0.
    pub factor: f64,
    _marker: PhantomData<P>,
}

impl<P: Pixel> Brightness<P>
where
    P::Subpixel: AsPrimitive<f64> + FromPrimitive + Ord + UpperBounded + Zero,
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

impl<P: Pixel> Filter for Brightness<P>
where
    P::Subpixel: AsPrimitive<f64> + FromPrimitive + Ord + UpperBounded + Zero,
{
    type Input = P;
    type Output = P;

    fn apply_pixel(
        &self,
        _image: &Image<Self::Input>,
        _x: u32,
        _y: u32,
        pixel: Self::Input,
    ) -> Self::Output {
        pixel.map_subpixels(
            |c| {
                let max = P::Subpixel::max_value();
                let c = self.factor.mul_add(max.as_(), c.as_());
                P::Subpixel::from_f64(c)
                    .expect("out of bounds")
                    .clamp(P::Subpixel::zero(), max)
            },
            |alpha| alpha,
        )
    }
}

/// A filter which applies the given filter only to the given mask.
pub struct Mask<F: Filter> {
    /// The filter to apply.
    pub filter: F,
    /// The mask to apply the filter to.
    pub mask: Image<BitPixel>,
}
