use super::ColorType;
use crate::{
    encode::{Decoder, Encoder, FrameIterator},
    DisposalMethod, Dynamic, Error, Frame, Image, ImageFormat, ImageSequence, LoopCount,
    OverlayMode, Pixel, Rgb, Rgba,
};

use crate::pixel::assume_pixel_from_palette;
pub use png::{AdaptiveFilterType, Compression, FilterType};
use std::borrow::Cow;
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

    fn prepare<'a, W: Write>(
        &mut self,
        width: u32,
        height: u32,
        color_type: ColorType,
        bit_depth: u8,
        dest: &'a mut W,
    ) -> png::Encoder<&'a mut W> {
        let mut encoder = png::Encoder::new(dest, width, height);
        encoder.set_adaptive_filter(self.adaptive_filter);
        encoder.set_filter(self.filter);
        encoder.set_compression(self.compression);
        encoder.set_color(get_png_color_type(color_type));
        encoder.set_depth(png::BitDepth::from_u8(bit_depth).unwrap());

        encoder
    }
}

impl Encoder for PngEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        let data = image.data.iter().flat_map(P::as_bytes).collect::<Vec<_>>();
        let color_type = image.data[0].color_type();

        let mut encoder = self.prepare(
            image.width(),
            image.height(),
            color_type,
            P::BIT_DEPTH,
            dest,
        );

        match color_type {
            ColorType::PaletteRgb => {
                let pal = image.palette().expect("no palette for paletted image?");
                encoder.set_palette(pal.iter().flat_map(Pixel::as_bytes).collect::<Cow<_>>());
            }
            ColorType::PaletteRgba => {
                let pal = image.palette().expect("no palette for paletted image?");
                encoder.set_palette(
                    pal.iter()
                        .map(|p| p.force_into_rgb())
                        .flat_map(|p| p.as_bytes())
                        .collect::<Cow<_>>(),
                );
                encoder.set_trns(
                    pal.iter()
                        .map(|p| p.force_into_rgba().a)
                        .collect::<Cow<_>>(),
                );
            }
            _ => (),
        }

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
        let sample = sequence
            .first_frame()
            .ok_or(Error::EmptyImageError)?
            .image();
        let pixel = &sample.data[0];

        let mut encoder = self.prepare(
            sample.width(),
            sample.height(),
            pixel.color_type(),
            P::BIT_DEPTH,
            dest,
        );
        encoder.set_animated(sequence.len() as u32, sequence.loop_count().count_or_zero())?;

        let mut writer = encoder.write_header()?;

        for frame in sequence.iter() {
            let data = frame
                .image()
                .data
                .iter()
                .flat_map(P::as_bytes)
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
