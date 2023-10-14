use crate::encode::{FrameLike, HasEncoderMetadata};
use crate::{
    pixel::Dynamic, ColorType, Decoder, Encoder, Image, Pixel, SingleFrameIterator,
};
use core::marker::PhantomData;
use qoi::{Channels, ColorSpace, Decoder as QDecoder, Encoder as QEncoder};
use std::io::{Read, Write};

impl From<Channels> for ColorType {
    fn from(value: Channels) -> ColorType {
        match value {
            Channels::Rgb => ColorType::Rgb,
            Channels::Rgba => ColorType::Rgba,
        }
    }
}

/// A QOI encoder interface over [`qoi::Encoder`].
pub struct QoiEncoder<P, W> {
    config: ColorSpace,
    writer: W,
    _marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> Encoder<P, W> for QoiEncoder<P, W> {
    type Config = ColorSpace;

    fn new(writer: W, metadata: impl HasEncoderMetadata<Self::Config, P>) -> crate::Result<Self> {
        Ok(Self {
            config: metadata.config(),
            writer,
            _marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl FrameLike<P>) -> crate::Result<()> {
        let image = frame.image();
        let data = image.data.iter();
        // Convert the pixels to RGB or RGBA, then to bytes
        let data: Box<[u8]> = if P::COLOR_TYPE.has_alpha() {
            data.map(P::as_rgba).flat_map(|p| p.as_bytes()).collect()
        } else {
            data.map(P::as_rgb).flat_map(|p| p.as_bytes()).collect()
        };
        // Write to stream
        QEncoder::new(&data, image.width(), image.height())?
            .with_colorspace(self.config)
            .encode_to_stream(&mut self.writer)?;
        Ok(())
    }

    fn finish(self) -> crate::Result<()> {
        Ok(())
    }
}

pub struct QoiDecoder<P, R> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> QoiDecoder<P, R> {
    /// Create a new decoder that decodes into the given pixel type.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for QoiDecoder<P, R> {
    type Sequence = SingleFrameIterator<P>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut decoder = QDecoder::from_stream(stream)?;
        // Decode the header
        let header = decoder.header().to_owned();

        // Convert the pixels
        let pixels = decoder
            .decode_to_vec()?
            // Since qoi::Channels is #[repr(u8)], this works
            .chunks(header.channels as usize)
            .map(|chunk| P::from_dynamic(Dynamic::from_bytes(chunk)))
            .collect::<Vec<P>>();

        Ok(Image::from_pixels(header.width, pixels))
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        self.decode(stream).map(SingleFrameIterator::new)
    }
}
