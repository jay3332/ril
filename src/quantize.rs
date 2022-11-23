//! Quantizes unpaletted pixel data to paletted data by quantizing the colors into a palette.

#[cfg_attr(not(feature = "quantize"), allow(unused_imports))]
use crate::{Pixel, TrueColor};
use std::collections::HashMap;

#[cfg(feature = "quantize")]
use color_quant::NeuQuant;

/// Configuration options regarding behavior of quantization.
#[derive(Clone, Debug)]
pub struct Quantizer {
    /// The maximum number of colors in the palette. Defaults to `256`.
    pub palette_size: usize,
    /// Whether to optimize the palette for GIF images.
    pub gif_optimization: bool,
    /// The quality of the output colors. If you plan on using a lossless quantizer like
    /// [`quantize_simple`], this value will be disregarded. Otherwise, lower values will produce
    /// lower quality colors in a faster time, and higher values will produce higher quality colors
    /// in a slower amount of time.
    ///
    /// The quality can be any value between 1 and 30. Defaults to 20.
    pub quality: u8,
    /// When attempting lossy quantization and this value is `true`, the quantizer will check if
    /// the amount of unique colors in the image is less than the desired palette size. If it is,
    /// the quantizer will fallback to a lossless quantization algorithm.
    ///
    /// This does incur a `O(n)` time complexity when calculating the amount of unique colors in the
    /// image, where `n` is the amount of pixels in the image. This isn't very significant, but
    /// it's worth noting.
    pub fallback_to_lossless: bool,
}

impl Default for Quantizer {
    fn default() -> Self {
        Self {
            palette_size: 256,
            gif_optimization: false,
            quality: 20,
            fallback_to_lossless: true,
        }
    }
}

impl Quantizer {
    /// Creates a new [`QuantizerConfig`] with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the maximum number of colors in the palette.
    #[must_use]
    pub const fn with_palette_size(mut self, palette_size: usize) -> Self {
        self.palette_size = palette_size;
        self
    }

    /// Sets whether to optimize the palette for GIF images.
    #[must_use]
    pub const fn with_gif_optimization(mut self, gif_optimization: bool) -> Self {
        self.gif_optimization = gif_optimization;
        self
    }

    /// Sets the quality of the output colors. If you plan on using a lossless quantizer like
    /// [`quantize_simple`], this value will be disregarded. Otherwise, lower values will produce
    /// lower quality colors in a faster time, and higher values will produce higher quality colors
    /// in a slower amount of time.
    ///
    /// The quality can be any value between 1 and 30.
    #[must_use]
    pub const fn with_quality(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }

    /// Sets whether to fallback to a lossless quantization algorithm when attempting lossy
    /// quantization and the amount of unique colors in the image is less than the desired palette
    /// size.
    ///
    /// This does incur a `O(n)` time complexity when calculating the amount of unique colors in the
    /// image, where `n` is the amount of pixels in the image. This isn't very significant, but
    /// it's worth noting.
    #[must_use]
    pub const fn with_fallback_to_lossless(mut self, fallback_to_lossless: bool) -> Self {
        self.fallback_to_lossless = fallback_to_lossless;
        self
    }

    /// Quantizes the given pixels to a palette of the given size. Returns `(palette, image_data)`.
    ///
    /// # Behavior
    /// * If the `quantize` feature is disabled (enabled by default), this function will simply
    /// run the [`quantize_simple`] function (lossless) and return `Err` if there are more unique
    /// colors than the palette size.
    /// * Otherwise, this function will always favor a lossy quantization algorithm unless the
    /// `fallback_to_lossless` option is set to `true` (which is the default) and the amount of
    /// unique colors in the image is less than the desired palette size.
    ///
    /// # Errors
    /// * There are more unique colors than the palette size, and the `quantize` feature is disabled
    pub fn quantize<P: TrueColor>(
        &self,
        pixels: impl AsRef<[P]>,
    ) -> crate::Result<(Vec<P>, Vec<u8>)> {
        #[cfg(feature = "quantize")]
        {
            quantize_lossy(pixels, self)
        }
        #[cfg(not(feature = "quantize"))]
        {
            quantize_simple(pixels, self)
        }
    }
}

/// Quantize an image with under 256 colors, panics otherwise. Returns `(palette, image_data)`.
///
/// # Errors
/// * There are more unique colors than the palette size
pub fn quantize_simple<P: TrueColor>(
    pixels: impl AsRef<[P]>,
    config: &Quantizer,
) -> crate::Result<(Vec<P>, Vec<u8>)> {
    let pixels = pixels.as_ref();
    let mut palette = Vec::with_capacity(config.palette_size);
    let mut lookup = HashMap::with_capacity(config.palette_size);

    let quantized = pixels
        .iter()
        .map(|pixel| {
            let key @ (r, g, b, mut a) = pixel.as_rgba_tuple();
            let mut maybe_err = None;

            let result = *lookup.entry(key).or_insert_with(|| {
                if palette.len() >= config.palette_size {
                    maybe_err = Some(crate::Error::QuantizationOverflow {
                        unique_colors: palette.len(),
                        palette_size: config.palette_size,
                    });
                    return 0;
                }
                if config.gif_optimization {
                    // GIFs only accept one transparent color, for which this color is fully
                    // transparent, otherwise the color is fully opaque. Only account for fully
                    // transparent pixels. So this information is not lost during encoding.
                    a = if a == 0 { 0 } else { 255 };
                }
                palette.push(P::from_rgba_tuple((r, g, b, a)));
                palette.len() - 1
            }) as u8;

            maybe_err.map_or(Ok(result), Err)
        })
        .collect::<crate::Result<_>>()?;

    Ok((palette, quantized))
}

/// Quantizes an image using the NeuQuant algorithm.
///
/// Returns `(palette, color_count, image_data)`. A slice of the pallete can be retrieved by using
/// `&palette[..color_count]`.
///
/// # Errors
/// * An error occurred while quantizing the image
#[cfg(feature = "quantize")]
pub fn quantize_lossy<P: TrueColor>(
    pixels: impl AsRef<[P]>,
    config: &Quantizer,
) -> crate::Result<(Vec<P>, Vec<u8>)> {
    let pixels = pixels.as_ref();
    if config.fallback_to_lossless {
        let count = pixels.windows(2).filter(|win| win[0] != win[1]).count() + 1;
        if count <= config.palette_size {
            return quantize_simple::<P>(pixels, config);
        }
    }

    let pixels = pixels
        .iter()
        .flat_map(|p| p.into_rgba().as_bytes())
        .collect::<Vec<_>>();

    #[allow(clippy::cast_lossless)]
    let quantizer = NeuQuant::new(31 - config.quality as i32, config.palette_size, &pixels);
    let palette = quantizer
        .color_map_rgba()
        .chunks_exact(4)
        .map(|chunk| P::from_rgba_tuple((chunk[0], chunk[1], chunk[2], chunk[3])))
        .collect();

    Ok((
        palette,
        (0..pixels.len())
            .step_by(4)
            .map(|i| quantizer.index_of(&pixels[i..i + 4]) as u8)
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use crate::quantize::Quantizer;

    #[test]
    fn test_quantization() {
        let sample = [
            Rgba::new(255, 255, 255, 255),
            Rgba::new(255, 255, 255, 255),
            Rgba::new(0, 255, 255, 255),
        ];

        let (palette, pixels) = Quantizer::new().quantize(&sample).unwrap();
        assert_eq!(
            &palette[..],
            &[Rgba::new(255, 255, 255, 255), Rgba::new(0, 255, 255, 255)]
        );
        assert_eq!(pixels, &[0, 0, 1]);
    }
}
