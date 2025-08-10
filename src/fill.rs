//! Includes the [`Fill`] and [`IntoFill`] traits and primitive fill types such as [`SolidFill`].
//! 
//! You should almost never need use a fill type directly since methods that take fills usually
//! allow anything that implements [`IntoFill`], which includes things such as [`Pixel`]s for solid
//! fills, [`gradient`][crate::gradient] types, and even [`Image`]s for image fills.

use crate::{Image, OverlayMode, Pixel};

pub type BoundingBox<T> = (T, T, T, T);

/// Any fill type that can be used to fill a shape, i.e. solid colors or gradients.
///
/// For solid colors, this trait is implemented for all [`Pixel`] types as [`SolidFill`].
pub trait IntoFill: Clone {
    /// The pixel type of the fill.
    type Pixel: Pixel;
    /// The fill type.
    type Fill: Fill<Self::Pixel>;

    /// Converts the fill into a fill type.
    fn into_fill(self) -> Self::Fill;
}

/// Handles the actual filling of a shape. See [`IntoFill`] for more information.
pub trait Fill<P: Pixel>: Clone {
    /// Sets the bounding box of the fill in place. This is used internally.
    fn set_bounding_box(&mut self, _bounding_box: BoundingBox<u32>) {}

    /// Sets the overlay mode of the fill. This is used internally.
    #[must_use = "this method consumes the fill and returns it back, it does not modify it in-place"]
    fn with_bounding_box(mut self, bounding_box: BoundingBox<u32>) -> Self
    where
        Self: Sized,
    {
        self.set_bounding_box(bounding_box);
        self
    }

    /// Gets the color of the fill at the given coordinates.
    fn get_pixel(&self, x: u32, y: u32) -> P;

    /// Plots the fill at the given coordinates on the given image.
    fn plot(&self, image: &mut Image<P>, x: u32, y: u32, mode: OverlayMode) {
        image.overlay_pixel_with_mode(x, y, self.get_pixel(x, y), mode);
    }

    /// Plots the fill at the given coordinates on the given image with a custom alpha value.
    fn plot_with_alpha(&self, image: &mut Image<P>, x: u32, y: u32, mode: OverlayMode, alpha: u8) {
        image.overlay_pixel_with_alpha(x, y, self.get_pixel(x, y), mode, alpha);
    }
}

impl<P: Pixel> IntoFill for P {
    type Pixel = P;
    type Fill = SolidFill<Self::Pixel>;

    fn into_fill(self) -> Self::Fill {
        SolidFill(self)
    }
}

impl<'a, P: Pixel> IntoFill for &'a Image<P> {
    type Pixel = P;
    type Fill = ImageFill<'a, Self::Pixel>;

    fn into_fill(self) -> Self::Fill {
        ImageFill(self)
    }
}

/// Represents a solid color fill.
#[derive(Copy, Clone, Debug)]
pub struct SolidFill<P: Pixel>(P);

impl<P: Pixel> SolidFill<P> {
    /// Creates a new solid fill.
    #[must_use]
    pub const fn new(color: P) -> Self {
        Self(color)
    }

    /// Returns a the color (represented as a [`Pixel`]) of the fill.
    #[must_use]
    pub const fn color(&self) -> P {
        self.0
    }
}

impl<P: Pixel> Fill<P> for SolidFill<P> {
    #[inline]
    fn get_pixel(&self, _: u32, _: u32) -> P {
        self.0
    }
}

/// Represents a fill that pulls pixels from an image.
///
/// # Warning
/// If pixels are unable to be pulled from this image (probably because it is too small),
/// **the pixel at `(0, 0)` will be used as the fallback pixel**. It is important to know this since
/// otherwise it can lead to confusing results.
///
/// As a rule of thumb, if you are unsure that the image in this fill matches or has higher
/// dimensions than the object being drawn using this fill, resize the image to fit.
#[derive(Copy, Clone)]
pub struct ImageFill<'a, P: Pixel>(&'a Image<P>);

impl<'a, P: Pixel> ImageFill<'a, P> {
    /// Creates a new image fill from the given image.
    #[must_use]
    pub const fn new(image: &'a Image<P>) -> Self {
        Self(image)
    }

    /// Returns a reference to the host image.
    #[must_use]
    pub const fn image(&self) -> &'a Image<P> {
        self.0
    }
}

impl<P: Pixel> Fill<P> for ImageFill<'_, P> {
    fn get_pixel(&self, x: u32, y: u32) -> P {
        self.0
            .get_pixel(x, y)
            .copied()
            .unwrap_or_else(|| self.0.data[0])
    }
}
