//! An interfacing layer between fast_image_resize and this crate.

use crate::Image;
use fast_image_resize::{Image as ResizeImage, ResizeAlg, FilterType as ResizeFilterType};

/// A filtering algorithm that is used to resize an image.
pub enum FilterType {
    /// A simple nearest neighbor algorithm. Although the fastest, this gives the lowest quality
    /// resizings.
    Nearest,
    /// A box filter algorithm. Equivalent to the [`Nearest`] filter if you are upscaling.
    Box,
    /// A bilinear filter. Calculates output pixel value using linear interpolation on all pixels.
    Bilinear,
    /// While having similar performance as the [`Bilinear`] filter, this produces a sharper and
    /// usually considered better quality image than the [`Bilinear`] filter, but **only** when
    /// downscaling. This may give worse results than bilinear when upscaling.
    Hamming,
    /// A Catmull-Rom bicubic filter, which is the most common bicubic filtering algorithm. Just
    /// like all cubic filters, it uses cubic interpolation on all pixels to calculate output
    /// pixels.
    Bicubic,
    /// A Mitchell-Netravali bicubic filter. Just like all cubic filters, it uses cubic
    /// interpolation on all pixels to calculate output pixels.
    Mitchell,
    /// A Lanczos filter with a window of 3. Calculates output pixel value using a high-quality
    /// Lanczos filter on all pixels.
    Lanczos3,
}

impl Default for FilterType {
    fn default() -> Self {
        FilterType::Bicubic
    }
}

impl From<FilterType> for ResizeAlg {
    fn from(f: FilterType) -> Self {
        type F = ResizeFilterType;

        ResizeAlg::Convolution(match f {
            FilterType::Nearest => return ResizeAlg::Nearest,
            FilterType::Box => F::Box,
            FilterType::Bilinear => F::Bilinear,
            FilterType::Hamming => F::Hamming,
            FilterType::Bicubic => F::CatmullRom,
            FilterType::Mitchell => F::Mitchell,
            FilterType::Lanczos3 => F::Lanczos3,
        })
    }
}
