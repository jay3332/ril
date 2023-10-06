use qoi::{Channels, ColorSpace};
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::num::NonZeroU32;
// (Creating this in the GitHub website because I can't figure out how to open a pull request without initially adding something)
use super::ColorType;
use crate::{
    encode::{Decoder, Encoder},
    pixel, DynamicFrameIterator, Error, Image, ImageFormat, Pixel, Rgb, Rgba, TrueColor,
};

// Re-export this
use crate::Error::{EmptyImageError, IoError};
use qoi::ColorSpace::*;

/// A QOI encoder interface around [`qoi::Encoder`].
#[derive(Default)]
pub struct QoiEncoder {
    pub color_space: ColorSpace,
}

impl QoiEncoder {
    /// Creates a new QOI encoder that is in the SRGB color space.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the color space of the encoder.
    pub fn with_color_space(mut self, space: ColorSpace) -> Self {
        self.color_space = space;
        self
    }
}

impl From<qoi::Error> for Error {
    fn from(value: qoi::Error) -> Self {
        use crate::Error::*;
        use qoi::Error::*;
        match value {
            InvalidMagic { .. } => DecodingError("invalid magic number".to_string()),
            InvalidChannels { channels } => EncodingError(format!(
                "qoi only supports either 3 or 4 channels, got {channels}"
            )),
            InvalidColorSpace { .. } => {
                DecodingError("colorspace of image is malformed".to_string())
            }
            InvalidImageDimensions { width, height } => {
                if width.min(height) == 0 {
                    EmptyImageError
                } else {
                    EncodingError(format!(
                        "image dimensions of {} by {} are not valid, must be below 400Mp",
                        width, height
                    ))
                }
            }
            InvalidImageLength {
                size,
                width,
                height,
            } => IncompatibleImageData {
                width,
                height,
                received: size,
            },
            OutputBufferTooSmall { size, required } => EncodingError(format!(
                "buffer of size {} is too small to hold image of size {}",
                size, required
            )),
            UnexpectedBufferEnd => {
                DecodingError("buffer reached end before decoding was finished".to_string())
            }
            InvalidPadding => DecodingError(
                "incorrectly placed stream end marker encountered during decoding".to_string(),
            ),
            qoi::Error::IoError(error) => Error::IoError(error),
        }
    }
}

impl Encoder for QoiEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        // TODO: Do a second pass on this!
        //       There's some weird code that needs optimization!
        let map: &dyn Fn(_) -> Result<Vec<u8>, Error> = &if P::COLOR_TYPE.has_alpha() {
            |pixel: &P| {
                Ok(
                    Rgba::from_raw_parts(P::COLOR_TYPE, P::BIT_DEPTH, pixel.as_bytes().as_ref())?
                        .as_bytes()
                        .to_vec(),
                )
            }
        } else {
            |pixel: &P| {
                Ok(
                    Rgb::from_raw_parts(P::COLOR_TYPE, P::BIT_DEPTH, pixel.as_bytes().as_ref())?
                        .as_bytes()
                        .to_vec(),
                )
            }
        };
        // See https://stackoverflow.com/a/59852696
        let encoded = qoi::encode_to_vec(
            image
                .data
                .iter()
                .map(map)
                .flat_map(|result| match result {
                    Ok(vec) => vec.into_iter().map(|item| Ok(item)).collect(),
                    Err(err) => vec![Err(err)],
                })
                .collect::<Result<Vec<u8>, Error>>()?,
            image.width(),
            image.height(),
        )?;
        dest.write_all(encoded.as_slice()).map_err(|e| IoError(e))
    }
}

/// A QOI decoder interface over [`qoi::Decoder`].
pub struct QoiDecoder<P: Pixel, R: Read> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> QoiDecoder<P, R> {
    /// Creates a new decoder with the default settings.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for QoiDecoder<P, R> {
    type Sequence = DynamicFrameIterator<P, R>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        let mut decoder = qoi::Decoder::from_stream(stream)?;
        let info = decoder.header();
        let (width, height) = (info.width, info.height);
        if (width == 0) || (height == 0) {
            return Err(EmptyImageError);
        }
        let (channels, color_type) = match info.channels {
            Channels::Rgb => (3, ColorType::Rgb),
            Channels::Rgba => (4, ColorType::Rgba),
        };
        // TODO: If/when sRGB support is added, grab it from the decoder.
        let _color_mode = info.colorspace;

        let raw_data: Vec<u8> = decoder.decode_to_vec()?;

        let data = raw_data
            .chunks(channels)
            .map(|chunk| P::from_raw_parts(color_type, 8, chunk))
            .collect::<Result<Vec<_>, Error>>()?;

        return Ok(Image {
            width: unsafe { NonZeroU32::new_unchecked(width) },
            height: unsafe { NonZeroU32::new_unchecked(height) },
            data,
            format: ImageFormat::Qoi,
            overlay: Default::default(),
            palette: None,
        });
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        let image = self.decode(stream)?;
        Ok(DynamicFrameIterator::single(image))
    }
}
