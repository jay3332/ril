use crate::{
    encode, ColorType, Decoder, Encoder, Frame, FrameIterator, Image, ImageFormat, OverlayMode,
    Pixel,
};
use std::{
    io::{Cursor, Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    result::Result,
    time::Duration,
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
        let encoder = image_webp::WebPEncoder::new(self.writer.by_ref());

        encoder
            .encode(
                &data,
                frame.image().width.into(),
                frame.image().height.into(),
                match self.native_color_type {
                    ColorType::L => image_webp::ColorType::L8,
                    ColorType::LA => image_webp::ColorType::La8,
                    ColorType::Rgb => image_webp::ColorType::Rgb8,
                    ColorType::Rgba => image_webp::ColorType::Rgba8,
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
    type Sequence = WebPSequenceDecoder<P>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut decoder = image_webp::WebPDecoder::new(Cursor::new(
            stream.bytes().collect::<Result<Vec<u8>, _>>()?,
        ))
        .map_err(|e| crate::Error::DecodingError(e.to_string()))?;

        let mut image_buf: Vec<u8> = create_image_buffer(&decoder);
        decoder
            .read_image(&mut image_buf)
            .map_err(|e| crate::Error::DecodingError(e.to_string()))?;

        let (width, height) = decoder.dimensions();

        let data = image_buf_to_pixeldata(&decoder, image_buf).unwrap();

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
        let decoder = image_webp::WebPDecoder::new(Cursor::new(
            stream.bytes().collect::<Result<Vec<u8>, _>>()?,
        ))
        .map_err(|e| crate::Error::DecodingError(e.to_string()))?;

        Ok(WebPSequenceDecoder::<P> {
            marker: PhantomData,
            decoder,
        })
    }
}

pub struct WebPSequenceDecoder<P: Pixel> {
    marker: PhantomData<P>,
    decoder: image_webp::WebPDecoder<Cursor<Vec<u8>>>,
}

impl<P: Pixel> FrameIterator<P> for WebPSequenceDecoder<P> {
    fn len(&self) -> u32 {
        image_webp::WebPDecoder::num_frames(&self.decoder)
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn loop_count(&self) -> crate::LoopCount {
        match image_webp::WebPDecoder::loop_count(&self.decoder) {
            image_webp::LoopCount::Forever => crate::LoopCount::Infinite,
            image_webp::LoopCount::Times(n) => {
                crate::LoopCount::Exactly((Into::<u16>::into(n)) as u32)
            }
        }
    }

    fn into_sequence(self) -> crate::Result<crate::ImageSequence<P>>
    where
        Self: Sized,
    {
        let loop_count = self.loop_count();
        let frames = self.collect::<crate::Result<Vec<_>>>()?;

        Ok(crate::ImageSequence::from_frames(frames).with_loop_count(loop_count))
    }
}

impl<P: Pixel> Iterator for WebPSequenceDecoder<P> {
    type Item = crate::Result<crate::Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        let mut image_buf: Vec<u8> = create_image_buffer(&self.decoder);
        let (width, height) = self.decoder.dimensions();

        let frame = self.decoder.read_frame(&mut image_buf);

        match frame {
            Err(image_webp::DecodingError::NoMoreFrames) => return None,
            Err(_) | Ok(_) => (),
        }

        let data = image_buf_to_pixeldata(&self.decoder, image_buf).unwrap();

        let frame_duration = self.decoder.loop_duration() / self.decoder.num_frames() as u64;

        let frame = Frame::from_image(Image {
            width: NonZeroU32::new(width as _).unwrap(),
            height: NonZeroU32::new(height as _).unwrap(),
            data,
            format: ImageFormat::WebP,
            overlay: OverlayMode::default(),
            palette: None,
        })
        .with_delay(Duration::from_millis(frame_duration))
        .with_disposal(crate::DisposalMethod::Background);
        Some(Ok(frame))
    }
}

/// Creates a preallocated [Vec<u8>] for the decoder to write to.
fn create_image_buffer(decoder: &image_webp::WebPDecoder<Cursor<Vec<u8>>>) -> Vec<u8> {
    let image_buf_len = decoder
        .output_buffer_size()
        .ok_or(crate::Error::DecodingError(
            "Failed to preallocate buffer for image data".to_string(),
        ))
        .unwrap();
    vec![0; image_buf_len]
}

/// Converts the imagebuf from [create_image_buffer()] into a [Result<Vec<P: Pixel>>].
fn image_buf_to_pixeldata<P: Pixel>(
    decoder: &image_webp::WebPDecoder<Cursor<Vec<u8>>>,
    image_buf: Vec<u8>,
) -> crate::Result<Vec<P>> {
    let (color_type, pixel_bytes) = if decoder.has_alpha() {
        (ColorType::Rgba, 4)
    } else {
        (ColorType::Rgb, 3)
    };

    image_buf
        .as_slice()
        .chunks_exact(pixel_bytes)
        .map(|chunk| P::from_raw_parts(color_type, 8, chunk))
        .collect::<crate::Result<Vec<_>>>()
}
