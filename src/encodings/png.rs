use super::{ColorType, PixelData};
use crate::{
    encode::{Decoder, Encoder, FrameIterator},
    DisposalMethod, Frame, Image, ImageFormat, ImageSequence, LoopCount, OverlayMode, Pixel,
};

pub use png::{AdaptiveFilterType, Compression, FilterType};
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    time::Duration,
};

impl From<png::ColorType> for ColorType {
    #[allow(clippy::enum_glob_use)]
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

#[allow(clippy::enum_glob_use)]
const fn get_png_color_type(src: ColorType) -> png::ColorType {
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
    pub const fn new() -> Self {
        Self {
            adaptive_filter: AdaptiveFilterType::NonAdaptive,
            filter: FilterType::NoFilter,
            compression: Compression::Default,
        }
    }

    /// Sets the adaptive filter type to use.
    #[must_use]
    pub const fn with_adaptive_filter(mut self, value: AdaptiveFilterType) -> Self {
        self.adaptive_filter = value;
        self
    }

    /// Sets the filter type to use.
    #[must_use]
    pub const fn with_filter(mut self, value: FilterType) -> Self {
        self.filter = value;
        self
    }

    /// Sets the compression level to use.
    #[must_use]
    pub const fn with_compression(mut self, value: Compression) -> Self {
        self.compression = value;
        self
    }

    fn prepare<'a, P: Pixel, W: Write>(
        &mut self,
        width: u32,
        height: u32,
        sample: &P,
        dest: &'a mut W,
    ) -> png::Encoder<&'a mut W> {
        let mut encoder = png::Encoder::new(dest, width, height);

        encoder.set_adaptive_filter(self.adaptive_filter);
        encoder.set_filter(self.filter);
        encoder.set_compression(self.compression);

        let (color_type, bit_depth) = sample.as_pixel_data().type_data();

        encoder.set_color(get_png_color_type(color_type));
        encoder.set_depth(png::BitDepth::from_u8(bit_depth).unwrap());

        encoder
    }
}

impl Encoder for PngEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        let data = image
            .data
            .iter()
            .flat_map(|pixel| pixel.as_pixel_data().data())
            .collect::<Vec<_>>();

        let encoder = self.prepare(image.width(), image.height(), &image.data[0], dest);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&data)?;
        writer.finish()?;

        Ok(())
    }

    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        let sample = sequence.first_frame().image();
        let pixel = &sample.data[0];

        let mut encoder = self.prepare(sample.width(), sample.height(), pixel, dest);
        encoder.set_animated(sequence.len() as u32, sequence.loop_count().count_or_zero())?;

        let mut writer = encoder.write_header()?;

        for frame in sequence.iter() {
            let data = frame
                .image()
                .data
                .iter()
                .flat_map(|pixel| pixel.as_pixel_data().data())
                .collect::<Vec<_>>();

            writer.set_frame_delay(frame.delay().as_millis() as u16, 1000)?;
            writer.set_dispose_op(match frame.disposal() {
                DisposalMethod::None => png::DisposeOp::None,
                DisposalMethod::Background => png::DisposeOp::Background,
                DisposalMethod::Previous => png::DisposeOp::Previous,
            })?;
            writer.write_image_data(&data)?;
        }

        writer.finish()?;
        Ok(())
    }
}

impl Default for PngEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// A PNG decoder interface around [`png::Decoder`].
pub struct PngDecoder<P: Pixel, R: Read> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> PngDecoder<P, R> {
    /// Creates a new decoder with the default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    fn prepare(stream: R) -> crate::Result<png::Reader<R>> {
        let decoder = png::Decoder::new(stream);
        decoder.read_info().map_err(Into::into)
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for PngDecoder<P, R> {
    type Sequence = ApngFrameIterator<P, R>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut reader = Self::prepare(stream)?;

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
            overlay: OverlayMode::default(),
            background: P::default(),
        })
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        let reader = Self::prepare(stream)?;

        Ok(ApngFrameIterator {
            seq: 0,
            reader,
            _marker: PhantomData,
        })
    }
}

impl<P: Pixel, R: Read> Default for PngDecoder<P, R> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ApngFrameIterator<P: Pixel, R: Read> {
    seq: u32,
    reader: png::Reader<R>,
    _marker: PhantomData<P>,
}

impl<P: Pixel, R: Read> ApngFrameIterator<P, R> {
    fn info(&self) -> &png::Info {
        self.reader.info()
    }

    fn next_frame(&mut self) -> crate::Result<(Vec<P>, png::OutputInfo)> {
        let buffer = &mut vec![0; self.reader.output_buffer_size()];
        let info = self.reader.next_frame(buffer)?;

        let (color_type, bit_depth, bpp) = {
            let info = self.info();
            let color_type: ColorType = info.color_type.into();
            let bit_depth = info.bit_depth as u8;

            (color_type, bit_depth, info.bytes_per_pixel())
        };

        let data = buffer
            .chunks_exact(bpp)
            .map(|chunk| {
                PixelData::from_raw(color_type, bit_depth, chunk).and_then(P::from_pixel_data)
            })
            .collect::<crate::Result<Vec<_>>>()?;

        Ok((data, info))
    }
}

impl<P: Pixel, R: Read> FrameIterator<P> for ApngFrameIterator<P, R> {
    fn len(&self) -> u32 {
        self.info()
            .animation_control
            .map_or(1, |a| a.num_frames)
    }

    fn loop_count(&self) -> LoopCount {
        match self.info().animation_control.map(|a| a.num_plays) {
            Some(0) | None => LoopCount::Infinite,
            Some(n) => LoopCount::Exactly(n),
        }
    }

    fn into_sequence(self) -> crate::Result<ImageSequence<P>> {
        let loop_count = self.loop_count();
        let frames = self.collect::<crate::Result<Vec<_>>>()?;

        Ok(ImageSequence::from_frames(frames).with_loop_count(loop_count))
    }
}

impl<P: Pixel, R: Read> Iterator for ApngFrameIterator<P, R> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.seq >= self.len() {
            return None;
        }

        let (data, output_info) = match self.next_frame() {
            Ok(o) => o,
            Err(e) => return Some(Err(e)),
        };

        let inner = Image {
            width: NonZeroU32::new(output_info.width).unwrap(),
            height: NonZeroU32::new(output_info.height).unwrap(),
            data,
            format: ImageFormat::Png,
            overlay: OverlayMode::default(),
            background: P::default(),
        };

        self.seq += 1;
        let fc = self.info().frame_control();

        Some(Ok(Frame::from_image(inner)
            .with_delay(
                fc.map_or_else(Duration::default, |f| Duration::from_secs_f64(f64::from(f.delay_num) / f64::from(f.delay_den))),
            )
            .with_disposal(
                fc.map_or_else(DisposalMethod::default, |f| match f.dispose_op {
                    png::DisposeOp::None => DisposalMethod::None,
                    png::DisposeOp::Background => DisposalMethod::Background,
                    png::DisposeOp::Previous => DisposalMethod::Previous,
                }),
            )))
    }
}
