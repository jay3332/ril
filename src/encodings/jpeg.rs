use super::ColorType;
use crate::{
    encode::{self, Decoder, Encoder},
    Error, Image, ImageFormat, OverlayMode, Pixel, Result, SingleFrameIterator,
};

use jpeg_decoder::PixelFormat as DecoderPixelFormat;
use jpeg_encoder::ColorType as EncoderColorType;
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
};

/// JPEG encoder options.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct JpegEncoderOptions {
    quality: u8,
}

impl Default for JpegEncoderOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl JpegEncoderOptions {
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

enum JpegSpecialCase {
    L1,
    LA1,
    None,
}

/// A JPEG encoder interface over [`jpeg_encoder::Encoder`].
pub struct JpegEncoder<P: Pixel, W: Write> {
    native_color_type: ColorType,
    color_type: EncoderColorType,
    special_case: JpegSpecialCase,
    quality: u8,
    writer: Option<W>,
    _marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> Encoder<P, W> for JpegEncoder<P, W> {
    type Config = JpegEncoderOptions;

    fn new(dest: W, metadata: impl encode::HasEncoderMetadata<Self::Config, P>) -> Result<Self> {
        let sample = (metadata.color_type(), metadata.bit_depth());
        let color_type = match sample {
            (ColorType::Rgb | ColorType::PaletteRgb, 8) => EncoderColorType::Rgb,
            (ColorType::Rgba | ColorType::PaletteRgba, 8) => EncoderColorType::Rgba,
            // Just like how Rgba strips into Rgb, perform the same thing here manually
            (ColorType::Luma, 1 | 8) | (ColorType::LumaA, 8) => EncoderColorType::Luma,
            _ => return Err(Error::UnsupportedColorType),
        };
        let special_case = match sample {
            (ColorType::Luma, 1) => JpegSpecialCase::L1,
            (ColorType::LumaA, 1) => JpegSpecialCase::LA1,
            _ => JpegSpecialCase::None,
        };

        Ok(Self {
            native_color_type: metadata.color_type(),
            color_type,
            special_case,
            quality: metadata.config().quality,
            writer: Some(dest),
            _marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl encode::FrameLike<P>) -> Result<()> {
        let data = frame.image().data.iter();
        let mut data = match self.native_color_type {
            ColorType::PaletteRgb => data.flat_map(|p| p.as_rgb().as_bytes()).collect::<Vec<_>>(),
            ColorType::PaletteRgba => data
                .flat_map(|p| p.as_rgba().as_bytes())
                .collect::<Vec<_>>(),
            _ => data.flat_map(P::as_bytes).collect::<Vec<_>>(),
        };

        match self.special_case {
            JpegSpecialCase::L1 => data
                .iter_mut()
                .for_each(|p| *p = if *p > 0 { 255 } else { 0 }),
            JpegSpecialCase::LA1 => data = data.into_iter().step_by(2).collect(),
            JpegSpecialCase::None => (),
        }

        let encoder = jpeg_encoder::Encoder::new(
            self.writer
                .take()
                .expect("jpeg cannot encode multiple frames"),
            self.quality,
        );
        encoder.encode(
            &data,
            frame.image().width() as u16,
            frame.image().height() as u16,
            self.color_type,
        )?;
        Ok(())
    }

    // no-op
    fn finish(self) -> Result<()> {
        Ok(())
    }
}

/// A JPEG decoder interface over [`jpeg_decoder::Decoder`].
#[derive(Default)]
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
    type Sequence = SingleFrameIterator<P>;

    #[allow(clippy::cast_lossless)]
    fn decode(&mut self, stream: R) -> Result<Image<P>> {
        let mut decoder = jpeg_decoder::Decoder::new(stream);
        let data = decoder.decode()?;

        let info = decoder.info().unwrap();
        let (color_type, bit_depth) = match info.pixel_format {
            DecoderPixelFormat::L8 => (ColorType::Luma, 8),
            DecoderPixelFormat::L16 => (ColorType::Luma, 16),
            DecoderPixelFormat::RGB24 | DecoderPixelFormat::CMYK32 => (ColorType::Rgb, 8),
        };
        let perform_conversion = info.pixel_format == jpeg_decoder::PixelFormat::CMYK32;

        let data = data
            .as_slice()
            .chunks_exact(info.pixel_format.pixel_bytes())
            .map(|chunk| {
                if color_type == ColorType::Luma {
                    return P::from_raw_parts(ColorType::Luma, bit_depth, chunk);
                }

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
            palette: None,
        })
    }

    fn decode_sequence(&mut self, stream: R) -> Result<Self::Sequence> {
        let image = self.decode(stream)?;

        Ok(SingleFrameIterator::new(image))
    }
}
