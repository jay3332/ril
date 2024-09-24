//! An interfacing layer between `fast_image_resize` and this crate.

use crate::{encodings::ColorType, Pixel};
use fast_image_resize::{
    images::{Image as ImageOut, ImageRef},
    FilterType as ResizeFilterType, PixelType as ResizePixelType, ResizeAlg, ResizeOptions,
    Resizer,
};

/// A filtering algorithm that is used to resize an image.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FilterType {
    /// A simple nearest neighbor algorithm. Although the fastest, this gives the lowest quality
    /// resizings.
    ///
    /// When upscaling this is good if you want a "pixelated" effect with no aliasing.
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
    /// Lanczos filter on all pixels. This can give antialiasing effects.
    Lanczos3,
    /// If upscaling, repeats the image in a tiling fashion to fill the desired size.
    /// If downscaling, this just crops the image.
    Tile,
}

impl Default for FilterType {
    fn default() -> Self {
        Self::Nearest
    }
}

impl From<FilterType> for ResizeAlg {
    fn from(f: FilterType) -> Self {
        type F = ResizeFilterType;

        Self::Convolution(match f {
            FilterType::Nearest => return Self::Nearest,
            FilterType::Box => F::Box,
            FilterType::Bilinear => F::Bilinear,
            FilterType::Hamming => F::Hamming,
            FilterType::Bicubic => F::CatmullRom,
            FilterType::Mitchell => F::Mitchell,
            FilterType::Lanczos3 => F::Lanczos3,
            FilterType::Tile => unimplemented!("tile filter is implemented separately"),
        })
    }
}

impl FilterType {
    /// Performs a resize operation on the given data.
    ///
    /// # Panics
    /// * The given data is empty.
    /// * Unsupported bit depth.
    pub fn resize<P: Pixel>(
        &self,
        data: &[P],
        src_width: u32,
        src_height: u32,
        dst_width: u32,
        dst_height: u32,
    ) -> Vec<P> {
        let color_type = data[0].color_type();
        let pixel_type = match P::BIT_DEPTH {
            1 | 2 | 4 | 8 => match color_type {
                ColorType::L | ColorType::PaletteRgb | ColorType::PaletteRgba => {
                    ResizePixelType::U8
                }
                ColorType::LA => ResizePixelType::U8x2,
                ColorType::Rgb => ResizePixelType::U8x3,
                ColorType::Rgba => ResizePixelType::U8x4,
                ColorType::Dynamic => unreachable!(),
            },
            16 => match color_type {
                ColorType::L | ColorType::PaletteRgb | ColorType::PaletteRgba => {
                    ResizePixelType::U16
                }
                ColorType::LA => ResizePixelType::U16x2,
                ColorType::Rgb => ResizePixelType::U16x3,
                ColorType::Rgba => ResizePixelType::U16x4,
                ColorType::Dynamic => unreachable!(),
            },
            _ => panic!("Unsupported bit depth"),
        };

        let buffer = data.iter().flat_map(P::as_bytes).collect::<Vec<_>>();
        // We are able to unwrap here since we validated the buffer throughout the creation of the image.
        let src =
            ImageRef::new(src_width, src_height, &buffer, pixel_type).expect("Invalid buffer size");
        let mut dest = ImageOut::new(dst_width, dst_height, pixel_type);

        let mut resizer = Resizer::new();
        let options = ResizeOptions::new().resize_alg(ResizeAlg::from(*self));

        // The pixel type is the same, we can unwrap here
        resizer.resize(&src, &mut dest, Some(&options)).unwrap();

        let bpp = color_type.channels() * ((P::BIT_DEPTH as usize + 7) >> 3);
        dest.into_vec()
            .chunks_exact(bpp)
            .map(P::from_bytes)
            .collect()
    }
}

fn resize_tiled<P: Pixel>(data: &[P], src_width: u32, dst_width: u32, dst_height: u32) -> Vec<P> {
    let chunks = data.chunks_exact(src_width as _);

    chunks
        .flat_map(|chunk| chunk.iter().cycle().take(dst_width as _))
        .cycle()
        .take((dst_width * dst_height) as _)
        .copied()
        .collect()
}

/// Performs a resize operation on the given data.
///
/// # Panics
/// * The given data is empty.
/// * Unsupported bit depth.
pub fn resize<P: Pixel>(
    data: &[P],
    src_width: u32,
    src_height: u32,
    dst_width: u32,
    dst_height: u32,
    filter: FilterType,
) -> Vec<P> {
    match filter {
        FilterType::Tile => resize_tiled(data, src_width, dst_width, dst_height),
        _ => filter.resize(data, src_width, src_height, dst_width, dst_height),
    }
}
