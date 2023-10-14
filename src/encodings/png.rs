use super::ColorType;
use crate::{
    encode::{self, Decoder, Encoder, FrameIterator},
    pixel::assume_pixel_from_palette,
    DisposalMethod, Dynamic, Frame, Image, ImageFormat, LoopCount, OverlayMode, Pixel, Rgb, Rgba,
};

pub use png::{AdaptiveFilterType, Compression, FilterType};
use std::{
    borrow::Cow,
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
            Indexed => Self::PaletteRgb,
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
        ColorType::PaletteRgb | ColorType::PaletteRgba => Indexed,
        ColorType::Dynamic => unreachable!(),
    }
}

/// PNG configuration options for [`PngEncoder`].
#[derive(Copy, Clone, Debug, Default)]
pub struct PngEncoderOptions {
    /// The adaptive filter type to use.
    pub adaptive_filter: AdaptiveFilterType,
    /// The filter type to use.
    pub filter: FilterType,
    /// The compression to use.
    pub compression: Compression,
}

impl PngEncoderOptions {
    /// Creates a new set of options with the default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            adaptive_filter: AdaptiveFilterType::NonAdaptive,
            filter: FilterType::Sub,
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
}

/// A PNG encoder interface around [`png::Encoder`].
///
/// # Note
/// You **must** anticipate the frame and loop counts of the sequence with
/// [`crate::EncoderMetadata::with_sequence`] before calling [`PngEncoder::add_frame`].
/// See [`Encoder#anticipating-frame-and-loop-counts`] for more information.
pub struct PngEncoder<P: Pixel, W: Write> {
    writer: png::Writer<W>,
    dimensions: (u32, u32),
    _marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> Encoder<P, W> for PngEncoder<P, W> {
    type Config = PngEncoderOptions;

    fn new(
        dest: W,
        metadata: impl encode::HasEncoderMetadata<Self::Config, P>,
    ) -> crate::Result<Self> {
        let mut encoder = png::Encoder::new(dest, metadata.width(), metadata.height());
        encoder.set_color(get_png_color_type(metadata.color_type()));
        encoder.set_depth(png::BitDepth::from_u8(metadata.bit_depth()).unwrap());

        match metadata.color_type() {
            ColorType::PaletteRgb => {
                let pal = metadata.palette().expect("no palette for paletted image?");
                encoder.set_palette(pal.iter().flat_map(Pixel::as_bytes).collect::<Cow<_>>());
            }
            ColorType::PaletteRgba => {
                let pal = metadata.palette().expect("no palette for paletted image?");
                encoder.set_palette(
                    pal.iter()
                        .map(Pixel::as_rgb)
                        .flat_map(|p| p.as_bytes())
                        .collect::<Cow<_>>(),
                );
                encoder.set_trns(pal.iter().map(|p| p.as_rgba().a).collect::<Cow<_>>());
            }
            _ => (),
        }

        if let Some((len, loops)) = metadata.sequence() {
            encoder.set_animated(len as _, loops.count_or_zero())?;
        }

        let dimensions = (metadata.width(), metadata.height());
        let config = metadata.config();
        encoder.set_adaptive_filter(config.adaptive_filter);
        encoder.set_filter(config.filter);
        encoder.set_compression(config.compression);

        Ok(Self {
            writer: encoder.write_header()?,
            dimensions,
            _marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl encode::FrameLike<P>) -> crate::Result<()> {
        if (frame.image().dimensions()) != self.dimensions {
            self.writer
                .set_frame_dimension(frame.image().width(), frame.image().height())?;
        }
        let data = frame
            .image()
            .data
            .iter()
            .flat_map(P::as_bytes)
            .collect::<Vec<_>>();

        if let Some(delay) = frame.delay() {
            self.writer
                .set_frame_delay(delay.as_millis() as u16, 1000)?;
        }
        if let Some(disposal) = frame.disposal() {
            self.writer.set_dispose_op(match disposal {
                DisposalMethod::None => png::DisposeOp::None,
                DisposalMethod::Background => png::DisposeOp::Background,
                DisposalMethod::Previous => png::DisposeOp::Previous,
            })?;
        }

        self.writer.write_image_data(&data)?;
        Ok(())
    }

    fn finish(self) -> crate::Result<()> {
        self.writer.finish()?;
        Ok(())
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

#[allow(clippy::type_complexity)]
fn read_data<P: Pixel>(
    buffer: &[u8],
    info: &png::Info,
) -> crate::Result<(Vec<P>, Option<Box<[P::Color]>>)> {
    let color_type: ColorType = info.color_type.into();
    let bit_depth = info.bit_depth as u8;

    let palette = info.palette.as_deref().map(|pal| {
        let pal = pal
            .chunks_exact(3)
            .map(|p| (p[0], p[1], p[2]))
            .collect::<Vec<_>>();

        let pal = if let Some(trns) = info.trns.as_deref() {
            trns.iter()
                .zip(pal)
                .map(|(a, (r, g, b))| PaletteRepr::Rgba(r, g, b, *a))
                .collect::<Vec<_>>()
        } else {
            pal.into_iter()
                .map(|(r, g, b)| PaletteRepr::Rgb(r, g, b))
                .collect()
        };

        pal.into_iter()
            .map(|p| match p {
                PaletteRepr::Rgb(r, g, b) => Dynamic::Rgb(Rgb::new(r, g, b)),
                PaletteRepr::Rgba(r, g, b, a) => Dynamic::Rgba(Rgba::new(r, g, b, a)),
            })
            .map(P::Color::from_dynamic)
            .collect::<Vec<_>>()
            .into_boxed_slice()
    });

    let chunks = buffer.chunks_exact(info.bytes_per_pixel());
    let data = if P::COLOR_TYPE.is_paletted() {
        let palette = palette.as_deref().expect("no palette for paletted image?");
        chunks
            // SAFETY: considered safe for unartificial types as the safety is upheld by the
            // crate. Otherwise, safety must be upheld by the user.
            .map(|idx| unsafe { assume_pixel_from_palette(palette, idx[0]) })
            .collect::<crate::Result<Vec<_>>>()?
    } else {
        chunks
            .map(|chunk| {
                P::from_raw_parts_paletted(color_type, bit_depth, chunk, palette.as_deref())
            })
            .collect::<crate::Result<Vec<_>>>()?
    };

    Ok((data, palette))
}

enum PaletteRepr {
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
}

impl<P: Pixel, R: Read> Decoder<P, R> for PngDecoder<P, R> {
    type Sequence = ApngFrameIterator<P, R>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut reader = Self::prepare(stream)?;

        // Here we are decoding a single image, so only capture the first frame:
        let buffer = &mut vec![0; reader.output_buffer_size()];
        reader.next_frame(buffer)?;

        let info = reader.info();
        let (data, palette) = read_data(buffer, info)?;

        Ok(Image {
            width: NonZeroU32::new(info.width).unwrap(),
            height: NonZeroU32::new(info.height).unwrap(),
            data,
            format: ImageFormat::Png,
            overlay: OverlayMode::default(),
            palette,
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

    #[allow(clippy::type_complexity)]
    fn next_frame(&mut self) -> crate::Result<(Vec<P>, Option<Box<[P::Color]>>, png::OutputInfo)> {
        let buffer = &mut vec![0; self.reader.output_buffer_size()];
        let info = self.reader.next_frame(buffer)?;
        let (data, palette) = read_data(buffer, self.info())?;

        Ok((data, palette, info))
    }
}

impl<P: Pixel, R: Read> FrameIterator<P> for ApngFrameIterator<P, R> {
    fn len(&self) -> u32 {
        self.info().animation_control.map_or(1, |a| a.num_frames)
    }

    fn loop_count(&self) -> LoopCount {
        match self.info().animation_control.map(|a| a.num_plays) {
            Some(0) | None => LoopCount::Infinite,
            Some(n) => LoopCount::Exactly(n),
        }
    }
}

impl<P: Pixel, R: Read> Iterator for ApngFrameIterator<P, R> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.seq >= self.len() {
            return None;
        }

        let (data, palette, output_info) = match self.next_frame() {
            Ok(o) => o,
            Err(e) => return Some(Err(e)),
        };

        let inner = Image {
            width: NonZeroU32::new(output_info.width).unwrap(),
            height: NonZeroU32::new(output_info.height).unwrap(),
            data,
            format: ImageFormat::Png,
            overlay: OverlayMode::default(),
            palette,
        };

        self.seq += 1;
        let fc = self.info().frame_control();

        Some(Ok(Frame::from_image(inner)
            .with_delay(fc.map_or_else(Duration::default, |f| {
                Duration::from_secs_f64(f64::from(f.delay_num) / f64::from(f.delay_den))
            }))
            .with_disposal(fc.map_or_else(
                DisposalMethod::default,
                |f| match f.dispose_op {
                    png::DisposeOp::None => DisposalMethod::None,
                    png::DisposeOp::Background => DisposalMethod::Background,
                    png::DisposeOp::Previous => DisposalMethod::Previous,
                },
            ))))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = (self.len() - self.seq) as usize;

        (remaining, Some(remaining))
    }
}
