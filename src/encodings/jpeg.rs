use super::ColorType;
use crate::{
    encode::{Decoder, Encoder},
    DynamicFrameIterator, Error, Image, ImageFormat, MaybePalettedImage, OverlayMode, Pixel,
    Result,
};

use jpeg_decoder::PixelFormat as DecoderPixelFormat;
use jpeg_encoder::ColorType as EncoderColorType;
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
};

/// A JPEG encoder interface over [`jpeg_encoder::Encoder`].
pub struct JpegEncoder {
    quality: u8,
}

impl JpegEncoder {
    /// Creates a new encoder with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self { quality: 90 }
    }

    /// Sets the quality of the encoded image. Must be between 0 and 100.
    ///
    /// # Panics
    /// * Quality is not between 0 and 100.
    #[must_use]
    pub fn with_quality(self, quality: u8) -> Self {
        assert!(quality <= 100, "quality must be between 0 and 100");

        unsafe { self.with_quality_unchecked(quality) }
    }

    /// Sets the quality of the encoded image. Should be between 0 and 100, but this doesn't
    /// check for that.
    ///
    /// # Safety
    /// Make sure `quality` is at most 100.
    ///
    /// # See Also
    /// * [`with_quality`] for the safe, checked version of this method.
    #[must_use]
    pub const unsafe fn with_quality_unchecked(mut self, quality: u8) -> Self {
        self.quality = quality;
        self
    }
}

impl Encoder for JpegEncoder {
    fn encode<P: Pixel>(
        &mut self,
        image: &Image<P>,
        dest: &mut impl Write,
    ) -> Result<()> {
        let sample @ (ct, _) = (image.data[0].color_type(), P::BIT_DEPTH);
        let color_type = match sample {
            (ColorType::Rgb | ColorType::PaletteRgb, 8) => EncoderColorType::Rgb,
            (ColorType::Rgba | ColorType::PaletteRgba, 8) => EncoderColorType::Rgba,
            // Just like how Rgba strips into Rgb, perform the same thing here manually
            (ColorType::L, 1 | 8) | (ColorType::LA, 8) => EncoderColorType::Luma,
            _ => return Err(Error::UnsupportedColorType),
        };

        let mut data = match ct {
            ColorType::PaletteRgb => image
                .data
                .iter()
                .map(|&p| p.force_into_rgb())
                .flat_map(|p| p.as_bytes())
                .collect::<Vec<_>>(),
            ColorType::PaletteRgba => image
                .data
                .iter()
                .map(|&p| p.force_into_rgba())
                .flat_map(|p| p.as_bytes())
                .collect::<Vec<_>>(),
            _ => image.data.iter().flat_map(P::as_bytes).collect::<Vec<_>>(),
        };

        if sample == (ColorType::L, 1) {
            data.iter_mut()
                .for_each(|p| *p = if *p > 0 { 255 } else { 0 });
        }
        if sample == (ColorType::LA, 1) {
            data = data.into_iter().step_by(2).collect();
        }

        let encoder = jpeg_encoder::Encoder::new(dest, self.quality);
        encoder.encode(
            &data,
            image.width() as u16,
            image.height() as u16,
            color_type,
        )?;

        Ok(())
    }
}

/// A JPEG decoder interface over [`jpeg_decoder::Decoder`].
pub struct JpegDecoder<P: Pixel, R: Read> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> JpegDecoder<P, R> {
    /// Creates a new decoder with the default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for JpegDecoder<P, R> {
    type Sequence = DynamicFrameIterator<P, R>;

    #[allow(clippy::cast_lossless)]
    fn decode(&mut self, stream: R) -> Result<Image<P>> {
        let mut decoder = jpeg_decoder::Decoder::new(stream);
        let data = decoder.decode()?;

        let info = decoder.info().unwrap();
        let (color_type, bit_depth) = match info.pixel_format {
            DecoderPixelFormat::L8 => (ColorType::L, 8),
            DecoderPixelFormat::L16 => (ColorType::L, 16),
            DecoderPixelFormat::RGB24 | DecoderPixelFormat::CMYK32 => (ColorType::Rgb, 8),
        };
        let perform_conversion = info.pixel_format == jpeg_decoder::PixelFormat::CMYK32;

        let data = data
            .as_slice()
            .chunks_exact(info.pixel_format.pixel_bytes())
            .map(|chunk| {
                let chunk = &if perform_conversion {
                    let c = chunk[0] as f32 / 255.0;
                    let y = chunk[1] as f32 / 255.0;
                    let m = chunk[2] as f32 / 255.0;
                    let k = chunk[3] as f32 / 255.0;

                    [
                        (255.0 * (1.0 - c) * (1.0 - k)).round() as u8,
                        (255.0 * (1.0 - y) * (1.0 - k)).round() as u8,
                        (255.0 * (1.0 - m) * (1.0 - k)).round() as u8,
                    ]
                } else {
                    [chunk[0], chunk[1], chunk[2]]
                };

                P::from_raw_parts(color_type, bit_depth, chunk)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(Image {
            width: NonZeroU32::new(info.width as u32).unwrap(),
            height: NonZeroU32::new(info.height as u32).unwrap(),
            data,
            format: ImageFormat::Jpeg,
            overlay: OverlayMode::default(),
        })
    }

    fn decode_sequence(&mut self, stream: R) -> Result<Self::Sequence> {
        let image = self.decode(stream)?;

        Ok(DynamicFrameIterator::single(image))
    }
}
