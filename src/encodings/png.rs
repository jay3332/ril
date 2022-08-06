use super::{ColorType, PixelData};
use crate::{
    encode::{Decoder, Encoder},
    Image, ImageFormat, Pixel,
};

pub use png::{AdaptiveFilterType, Compression, FilterType};
use std::{
    io::{Read, Write},
    num::NonZeroU32,
};

impl From<png::ColorType> for ColorType {
    fn from(value: png::ColorType) -> Self {
        use png::ColorType::*;

        match value {
            Grayscale => Self::L,
            GrayscaleAlpha => Self::LA,
            Rgb => Self::Rgb,
            Rgba => Self::Rgba,
            Indexed => Self::Palette,
        }
    }
}

fn get_png_color_type(src: ColorType) -> png::ColorType {
    use png::ColorType::*;

    match src {
        ColorType::L => Grayscale,
        ColorType::LA => GrayscaleAlpha,
        ColorType::Rgb => Rgb,
        ColorType::Rgba => Rgba,
        ColorType::Palette => Indexed,
    }
}

/// A PNG encoder interface around [`png::Encoder`].
pub struct PngEncoder {
    /// The adaptive filter type to use.
    pub adaptive_filter: AdaptiveFilterType,
    /// The filter type to use.
    pub filter: FilterType,
    /// The compression to use.
    pub compression: Compression,
}

impl PngEncoder {
    /// Creates a new encoder with the default settings.
    #[must_use]
    pub fn new() -> Self {
        Self {
            adaptive_filter: AdaptiveFilterType::NonAdaptive,
            filter: FilterType::NoFilter,
            compression: Compression::Default,
        }
    }

    /// Sets the adaptive filter type to use.
    pub fn with_adaptive_filter(mut self, value: AdaptiveFilterType) -> Self {
        self.adaptive_filter = value;
        self
    }

    /// Sets the filter type to use.
    pub fn with_filter(mut self, value: FilterType) -> Self {
        self.filter = value;
        self
    }

    /// Sets the compression level to use.
    pub fn with_compression(mut self, value: Compression) -> Self {
        self.compression = value;
        self
    }
}

impl Encoder for PngEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        let mut encoder = png::Encoder::new(dest, image.width(), image.height());

        encoder.set_adaptive_filter(self.adaptive_filter);
        encoder.set_filter(self.filter);
        encoder.set_compression(self.compression);

        let sample = image.pixel(0, 0);
        let (color_type, bit_depth) = sample.as_pixel_data().type_data();

        encoder.set_color(get_png_color_type(color_type));
        encoder.set_depth(png::BitDepth::from_u8(bit_depth).unwrap());

        let data = image
            .data
            .iter()
            .flat_map(|pixel| pixel.as_pixel_data().data())
            .collect::<Vec<_>>();

        let mut writer = encoder.write_header()?;
        writer.write_image_data(&data)?;
        writer.finish()?;

        Ok(())
    }
}

/// A PNG decoder interface around [`png::Decoder`].
pub struct PngDecoder;

impl Decoder for PngDecoder {
    fn decode<P: Pixel>(&mut self, stream: impl Read) -> crate::Result<Image<P>> {
        let decoder = png::Decoder::new(stream);
        let mut reader = decoder.read_info()?;

        // Here we are decoding a single image, so only capture the first frame:
        let buffer = &mut vec![0; reader.output_buffer_size()];
        reader.next_frame(buffer)?;

        let info = reader.info();
        let color_type: ColorType = info.color_type.into();
        let bit_depth = info.bit_depth as u8;

        let data = buffer
            .chunks_exact(info.bytes_per_pixel())
            .map(|chunk| {
                PixelData::from_raw(color_type, bit_depth, chunk).and_then(P::from_pixel_data)
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(Image {
            width: NonZeroU32::new(info.width).unwrap(),
            height: NonZeroU32::new(info.height).unwrap(),
            data,
            format: ImageFormat::Png,
            overlay: Default::default(),
        })
    }
}
