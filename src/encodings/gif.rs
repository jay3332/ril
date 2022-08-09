// TODO: paletted images. GIF has native support for this.

use crate::{
    encodings::ColorType, Decoder, DisposalMethod, Dynamic, Encoder, Error, Frame, FrameIterator,
    Image, ImageFormat, ImageSequence, LoopCount, OverlayMode, Pixel, Rgba,
};
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    time::Duration,
};

/// A GIF encoder interface over [`gif::Encoder`].
pub struct GifEncoder {
    speed: u8,
}

impl GifEncoder {
    /// Creates a new encoder with default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self { speed: 10 }
    }

    /// Sets the speed of the encoder. Higher speeds come at the cost of lower image quality.
    /// Must be between 1 and 30.
    ///
    /// # Panics
    /// * Speed is not between 1 and 30.
    #[must_use]
    pub fn with_speed(self, speed: u8) -> Self {
        assert!(speed > 0 && speed <= 30, "speed must be between 1 and 30");

        unsafe { self.with_speed_unchecked(speed) }
    }

    /// Sets the speed of the encoder. Higher speeds come at the cost of lower image quality.
    /// Should be between 1 and 30, but this doesn't check for that.
    ///
    /// # Safety
    /// Make sure `speed` is between 1 and 30.
    ///
    /// # See Also
    /// * [`with_speed`] for the safe, checked version of this method.
    #[must_use]
    pub const unsafe fn with_speed_unchecked(mut self, speed: u8) -> Self {
        self.speed = speed;
        self
    }
}

impl Encoder for GifEncoder {
    #[allow(clippy::cast_lossless)]
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        let mut encoder =
            gif::Encoder::new(dest, image.width() as u16, image.height() as u16, &[])?;
        let sample = image.data[0].as_pixel_data().type_data();

        macro_rules! data {
            ($t:ty) => {{
                image
                    .data
                    .iter()
                    .flat_map(|p| <$t>::from(Dynamic::from_pixel(*p).unwrap()).as_bytes())
                    .collect::<Vec<_>>()
            }};
            () => {{
                image
                    .data
                    .iter()
                    .flat_map(|p| p.as_bytes())
                    .collect::<Vec<_>>()
            }};
        }

        macro_rules! rgb {
            ($data:expr) => {{
                let pixels = $data;

                gif::Frame::from_rgb_speed(
                    image.width() as u16,
                    image.height() as u16,
                    &pixels,
                    self.speed as i32,
                )
            }};
        }

        macro_rules! rgba {
            ($data:expr) => {{
                let mut pixels = $data;

                gif::Frame::from_rgba_speed(
                    image.width() as u16,
                    image.height() as u16,
                    &mut pixels,
                    self.speed as i32,
                )
            }};
        }

        // TODO: paletted images
        let frame = match sample {
            (ColorType::Rgb, 8) => rgb!(data!()),
            (ColorType::Rgba, 8) => rgba!(data!()),
            (ColorType::L, 1 | 8) => rgb!(data!(crate::Rgb)),
            (ColorType::LA, 1 | 8) => rgba!(data!(crate::Rgba)),
            _ => return Err(Error::UnsupportedColorType),
        };

        encoder.write_frame(&frame)?;
        Ok(())
    }

    #[allow(clippy::cast_lossless, clippy::cast_precision_loss)]
    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        let image = sequence.first_frame().image();
        let mut encoder =
            gif::Encoder::new(dest, image.width() as u16, image.height() as u16, &[])?;

        encoder.set_repeat(match sequence.loop_count() {
            LoopCount::Exactly(n) => gif::Repeat::Finite(n as u16),
            LoopCount::Infinite => gif::Repeat::Infinite,
        })?;

        macro_rules! data {
            ($data:expr, $t:ty) => {{
                ($data)
                    .iter()
                    .flat_map(|p| <$t>::from(Dynamic::from_pixel(*p).unwrap()).as_bytes())
                    .collect::<Vec<_>>()
            }};
            ($data:expr) => {{
                ($data)
                    .iter()
                    .flat_map(|p| p.as_bytes())
                    .collect::<Vec<_>>()
            }};
        }

        macro_rules! rgb {
            ($data:expr) => {{
                let pixels = $data;

                gif::Frame::from_rgb_speed(
                    image.width() as u16,
                    image.height() as u16,
                    &pixels,
                    self.speed as i32,
                )
            }};
        }

        macro_rules! rgba {
            ($data:expr) => {{
                let mut pixels = $data;

                gif::Frame::from_rgba_speed(
                    image.width() as u16,
                    image.height() as u16,
                    &mut pixels,
                    self.speed as i32,
                )
            }};
        }

        let sample = image.data[0].as_pixel_data().type_data();

        for frame in sequence.iter() {
            let image = frame.image();

            // TODO: paletted images
            let mut out = match sample {
                (ColorType::Rgb, 8) => rgb!(data!(image.data)),
                (ColorType::Rgba, 8) => rgba!(data!(image.data)),
                (ColorType::L, 1 | 8) => rgb!(data!(image.data, crate::Rgb)),
                (ColorType::LA, 1 | 8) => rgba!(data!(image.data, crate::Rgba)),
                _ => return Err(Error::UnsupportedColorType),
            };
            out.delay = (frame.delay().as_millis() as f64 / 10.).round() as u16;
            out.dispose = match frame.disposal() {
                DisposalMethod::None => gif::DisposalMethod::Keep,
                DisposalMethod::Background => gif::DisposalMethod::Background,
                DisposalMethod::Previous => gif::DisposalMethod::Previous,
            };

            encoder.write_frame(&out)?;
        }

        Ok(())
    }
}

impl Default for GifEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// A decoder for GIF images.
pub struct GifDecoder<P: Pixel, R: Read> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> GifDecoder<P, R> {
    /// Creates a new decoder that decodes from the given reader.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Default for GifDecoder<P, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for GifDecoder<P, R> {
    type Sequence = GifFrameIterator<P, R>;

    #[allow(clippy::cast_lossless)]
    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut decoder = gif::DecodeOptions::new();
        // TODO: paletted images
        decoder.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = decoder.read_info(stream)?;

        let first = decoder.read_next_frame()?.ok_or(Error::EmptyImageError)?;
        let data = first
            .buffer
            .chunks(4)
            .map(|p| {
                P::from_dynamic(Dynamic::Rgba(Rgba {
                    r: p[0],
                    g: p[1],
                    b: p[2],
                    a: p[3],
                }))
            })
            .collect::<Vec<_>>();

        Ok(Image {
            width: NonZeroU32::new(decoder.width() as u32).unwrap(),
            height: NonZeroU32::new(decoder.height() as u32).unwrap(),
            data,
            format: ImageFormat::Gif,
            overlay: OverlayMode::default(),
        })
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        let mut decoder = gif::DecodeOptions::new();
        // TODO: paletted images
        decoder.set_color_output(gif::ColorOutput::RGBA);

        Ok(GifFrameIterator {
            decoder: decoder.read_info(stream)?,
            _marker: PhantomData,
        })
    }
}

pub struct GifFrameIterator<P: Pixel, R: Read> {
    decoder: gif::Decoder<R>,
    _marker: PhantomData<P>,
}

impl<P: Pixel, R: Read> FrameIterator<P> for GifFrameIterator<P, R> {
    fn len(&self) -> u32 {
        // Decoder also appears to not provide us with this either
        0
    }

    fn loop_count(&self) -> LoopCount {
        // Currently the decoder does not provide us with this info
        LoopCount::Infinite
    }
}

impl<P: Pixel, R: Read> Iterator for GifFrameIterator<P, R> {
    type Item = crate::Result<Frame<P>>;

    #[allow(clippy::cast_lossless)]
    fn next(&mut self) -> Option<Self::Item> {
        let width = self.decoder.width() as u32;
        let height = self.decoder.height() as u32;

        let frame = match self.decoder.read_next_frame() {
            Ok(Some(frame)) => frame,
            Ok(None) => return None,
            Err(e) => return Some(Err(e.into())),
        };

        let data = frame
            .buffer
            .chunks(4)
            .map(|p| {
                P::from_dynamic(Dynamic::Rgba(Rgba {
                    r: p[0],
                    g: p[1],
                    b: p[2],
                    a: p[3],
                }))
            })
            .collect::<Vec<_>>();

        let image = Image {
            width: NonZeroU32::new(width).unwrap(),
            height: NonZeroU32::new(height).unwrap(),
            data,
            format: ImageFormat::Gif,
            overlay: OverlayMode::default(),
        };

        Some(Ok(Frame::from_image(image)
            .with_delay(Duration::from_millis(frame.delay as u64 * 10))
            .with_disposal(match frame.dispose {
                gif::DisposalMethod::Keep | gif::DisposalMethod::Any => DisposalMethod::None,
                gif::DisposalMethod::Background => DisposalMethod::Background,
                gif::DisposalMethod::Previous => DisposalMethod::Previous,
            })))
    }
}
