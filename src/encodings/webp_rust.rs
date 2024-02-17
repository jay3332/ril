use crate::{
    encode, ColorType, Decoder, Encoder, Image, ImageFormat, OverlayMode, Pixel,
    SingleFrameIterator,
};
use image_webp::{ColorType as WebpColorType, WebPDecoder as ImageWebPDecoder, WebPEncoder};
use std::{
    io::{Cursor, Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
};

#[derive(Copy, Clone, Debug, PartialEq, Default)]
pub struct WebPEncoderOptions {}

pub struct WebPStaticEncoder<P: Pixel, W: Write> {
    native_color_type: ColorType,
    writer: W,
    marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> Encoder<P, W> for WebPStaticEncoder<P, W> {
    type Config = WebPEncoderOptions;

    fn new(
        dest: W,
        metadata: impl encode::HasEncoderMetadata<Self::Config, P>,
    ) -> crate::Result<Self> {
        Ok(Self {
            native_color_type: metadata.color_type(),
            writer: dest,
            marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl encode::FrameLike<P>) -> crate::Result<()> {
        let data = frame
            .image()
            .data
            .iter()
            .flat_map(P::as_bytes)
            .collect::<Vec<_>>();
        let encoder = WebPEncoder::new(self.writer.by_ref());

        encoder
            .encode(
                &data,
                frame.image().width.into(),
                frame.image().height.into(),
                match self.native_color_type {
                    ColorType::L => WebpColorType::L8,
                    ColorType::LA => WebpColorType::La8,
                    ColorType::Rgb => WebpColorType::Rgb8,
                    ColorType::Rgba => WebpColorType::Rgba8,
                    _ => unreachable!(),
                },
            )
            .map_err(|e| crate::Error::EncodingError(e.to_string()))?;
        Ok(())
    }

    // no-op
    fn finish(self) -> crate::Result<()> {
        Ok(())
    }
}

pub struct WebPDecoder<P: Pixel, R: Read> {
    marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> Default for WebPDecoder<P, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Pixel, R: Read> WebPDecoder<P, R> {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for WebPDecoder<P, R> {
    type Sequence = SingleFrameIterator<P>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut decoder = ImageWebPDecoder::new(Cursor::new(
            stream
                .bytes()
                .collect::<std::result::Result<Vec<u8>, _>>()?,
        ))
        .map_err(|e| crate::Error::DecodingError(e.to_string()))?;

        let image_buf_len = decoder
            .output_buffer_size()
            .ok_or(crate::Error::DecodingError(
                "Failed to preallocate buffer for image data".to_string(),
            ))?;
        let mut image_buf: Vec<u8> = vec![0; image_buf_len];
        decoder
            .read_image(&mut image_buf)
            .map_err(|e| crate::Error::DecodingError(e.to_string()))?;

        let (width, height) = decoder.dimensions();
        let (color_type, pixel_bytes) = if decoder.has_alpha() {
            (ColorType::Rgba, 4)
        } else {
            (ColorType::Rgb, 3)
        };

        let data = image_buf
            .as_slice()
            .chunks_exact(pixel_bytes)
            .map(|chunk| P::from_raw_parts(color_type, 8, chunk))
            .collect::<crate::Result<Vec<_>>>()?;

        Ok(Image {
            width: NonZeroU32::new(width).unwrap(),
            height: NonZeroU32::new(height).unwrap(),
            data,
            format: ImageFormat::WebP,
            overlay: OverlayMode::default(),
            palette: None,
        })
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        let image = self.decode(stream)?;
        Ok(SingleFrameIterator::new(image))
    }
}
