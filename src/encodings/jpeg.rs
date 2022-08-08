use crate::{
    DynamicFrameIterator,
    Image,
    ImageFormat,
    OverlayMode,
    Pixel,
    Result,
    encode::Decoder,
};
use super::{ColorType, PixelData};

use std::{io::Read, marker::PhantomData, num::NonZeroU32};

impl From<ColorType> for jpeg_decoder::PixelFormat {
    fn from(ty: ColorType) -> Self {
        type C = jpeg_decoder::PixelFormat;

        match ty {
            ColorType::L | ColorType::LA => C::L8,
            ColorType::Rgb | ColorType::Rgba => C::RGB24,
            ColorType::Palette => panic!("Palette color type is not supported"),
        }
    }
}

/// A JPEG decoder interface over [`zune_jpeg::Decoder`].
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

    fn decode(&mut self, stream: R) -> Result<Image<P>> {
        let mut decoder = jpeg_decoder::Decoder::new(stream);
        let data = decoder.decode()?;

        let info = decoder.info().unwrap();
        let (color_type, bit_depth) = match info.pixel_format {
            jpeg_decoder::PixelFormat::L8 => (ColorType::L, 8),
            jpeg_decoder::PixelFormat::L16 => (ColorType::L, 16),
            jpeg_decoder::PixelFormat::RGB24 => (ColorType::Rgb, 8),
            // Perform a conversion later
            jpeg_decoder::PixelFormat::CMYK32 => (ColorType::Rgb, 8),
        };
        let perform_conversion = info.pixel_format == jpeg_decoder::PixelFormat::CMYK32;

        let inst = std::time::Instant::now();
        let data = data.as_slice()
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

                PixelData::from_raw(color_type, bit_depth, chunk).and_then(P::from_pixel_data)
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
