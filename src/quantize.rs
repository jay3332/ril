//! Quantizes unpaletted pixel data to paletted data by quantizing the colors into a palette.

#![allow(dead_code)] // TODO

use crate::TrueColor;
use std::collections::HashMap;

/// Quantize an image with under 256 colors, panics otherwise.
///
/// This is optimized for GIFs since it also quantizes alpha channels to only 0 and 255 values.
///
/// Returns `(palette, color_count, image_data)`. A slice of the pallete can be retrieved by using
/// `&palette[..color_count]`.
pub fn quantize_simple<const COLORS: usize, P: TrueColor>(
    pixels: impl AsRef<[P]>,
) -> ([[u8; 4]; COLORS], usize, Vec<u8>) {
    let pixels = pixels.as_ref();
    let mut position = 0;
    let mut palette = [[0, 0, 0, 0]; COLORS];
    let mut lookup = HashMap::with_capacity(COLORS);

    let quantized = pixels
        .iter()
        .map(|pixel| {
            let key @ (r, g, b, a) = pixel.as_rgba_tuple();

            *lookup.entry(key).or_insert_with(|| {
                debug_assert!(
                    position < COLORS,
                    "received an image with more than {} colors, use a quantization algorithm to \
                     reduce the number of colors before using this function.",
                    COLORS,
                );
                palette[position] = [r, g, b, if a == 255 { 255 } else { 0 }];

                let index = position;
                position += 1;
                index
            }) as u8
        })
        .collect();

    (palette, position, quantized)
}

#[cfg(test)]
mod tests {
    use crate::quantize::quantize_simple;
    use crate::Rgba;

    #[test]
    fn mogus() {
        let sample = [
            Rgba::new(255, 255, 255, 255),
            Rgba::new(255, 255, 255, 255),
            Rgba::new(0, 255, 255, 255),
        ];

        let (palette, color_count, quantized) = quantize_simple::<256, _>(&sample);
        println!("{:?}", &palette[..color_count]);
        println!("{:?}", &quantized);
    }
}
