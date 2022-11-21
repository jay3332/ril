use crate::{
    ColorType, Decoder, DisposalMethod, Encoder, Error, Frame, FrameIterator, Image, ImageFormat,
    ImageSequence, LoopCount, OverlayMode, Pixel,
};
use libwebp_sys as libwebp;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::time::Duration;

/// Represents the encoding quality of a WebP image.
#[derive(Copy, Clone, Debug)]
pub enum WebPQuality {
    /// Lossless encoding.
    Lossless,
    /// Lossy encoding with the given quality factor. Larger values produce higher quality images at
    /// the expense of larger file sizes. Valid values are in the range [0, 100]. For lossless
    /// encoding, higher values will produce better compression at the expense of more computation
    /// and time.
    Lossy(u8),
}

impl Default for WebPQuality {
    fn default() -> Self {
        Self::Lossy(75)
    }
}

/// A WebP image encoder.
#[derive(Default)]
pub struct WebPEncoder {
    /// Image quality to encode at.
    pub quality: WebPQuality,
}

#[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]
impl Encoder for WebPEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        unsafe {
            let mut out = std::ptr::null_mut::<u8>();

            macro_rules! call_libwebp {
                ($func:ident, $stride:literal, $force_into:ident $(, $quality:expr)?) => {{
                    libwebp::$func(
                        image
                            .data
                            .iter()
                            .flat_map(|p| p.$force_into().as_bytes())
                            .collect::<Vec<_>>()
                            .as_ptr(),
                        image.width() as _,
                        image.height() as _,
                        (image.width() * $stride) as _,
                        $($quality,)?
                        &mut out,
                    )
                }};
            }

            let sample = image.data[0].color_type();
            let len = match (sample, self.quality) {
                (
                    ColorType::Rgba | ColorType::PaletteRgba | ColorType::LA,
                    WebPQuality::Lossless,
                ) => {
                    call_libwebp!(WebPEncodeLosslessRGBA, 4, force_into_rgba)
                }
                (_, WebPQuality::Lossless) => {
                    call_libwebp!(WebPEncodeLosslessRGB, 3, force_into_rgb)
                }
                (
                    ColorType::Rgba | ColorType::PaletteRgba | ColorType::LA,
                    WebPQuality::Lossy(q),
                ) => {
                    call_libwebp!(WebPEncodeRGBA, 4, force_into_rgba, q as _)
                }
                (_, WebPQuality::Lossy(q)) => {
                    call_libwebp!(WebPEncodeRGB, 3, force_into_rgb, q as _)
                }
            };
            if len == 0 {
                return Err(Error::EncodingError("WebP encoding failed".to_string()));
            }

            let out = std::slice::from_raw_parts(out, len as _);
            dest.write_all(out)?;
        }

        Ok(())
    }

    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        let first_frame = sequence.first_frame().image();

        unsafe {
            let mut options = std::mem::zeroed::<libwebp::WebPAnimEncoderOptions>();
            libwebp::WebPAnimEncoderOptionsInit(&mut options);
            options.anim_params.loop_count = sequence.loop_count().count_or_zero() as _;

            let encoder = libwebp::WebPAnimEncoderNew(
                first_frame.width() as _,
                first_frame.height() as _,
                std::ptr::addr_of!(options),
            );

            let mut timestamp = 0_i32;
            for frame in sequence.iter() {
                let mut picture = std::mem::zeroed::<libwebp::WebPPicture>();
                picture.width = frame.width() as _;
                picture.height = frame.height() as _;
                picture.use_argb = 1;

                if libwebp::WebPPictureAlloc(std::ptr::addr_of_mut!(picture)) == 0 {
                    return Err(Error::EncodingError("WebP memory error".to_string()));
                }

                macro_rules! import_libwebp_picture {
                    ($func:ident, $stride:literal, $force_into:ident) => {{
                        libwebp::$func(
                            std::ptr::addr_of_mut!(picture),
                            frame
                                .data
                                .iter()
                                .flat_map(|p| p.$force_into().as_bytes())
                                .collect::<Vec<_>>()
                                .as_ptr(),
                            (frame.width() * $stride) as _,
                        )
                    }};
                }
                let sample = frame.data[0].color_type();
                if match sample {
                    ColorType::Rgba | ColorType::PaletteRgba | ColorType::LA => {
                        import_libwebp_picture!(WebPPictureImportRGBA, 4, force_into_rgba)
                    }
                    _ => import_libwebp_picture!(WebPPictureImportRGB, 3, force_into_rgb),
                } == 0
                {
                    return Err(Error::EncodingError("WebP encoding error".to_string()));
                }

                let mut config = std::mem::zeroed::<libwebp::WebPConfig>();
                if libwebp::WebPConfigInit(std::ptr::addr_of_mut!(config)) == 0 {
                    return Err(Error::EncodingError("WebP version error".to_string()));
                }

                // TODO: setting config.lossless to 1 causes weird results
                // config.lossless = matches!(self.quality, WebPQuality::Lossless) as _;
                config.quality = match self.quality {
                    WebPQuality::Lossless => 100.0,
                    WebPQuality::Lossy(q) => q as _,
                };

                if libwebp::WebPAnimEncoderAdd(
                    encoder,
                    std::ptr::addr_of_mut!(picture),
                    timestamp,
                    std::ptr::addr_of_mut!(config),
                ) == 0
                {
                    return Err(Error::EncodingError("WebP encoding error".to_string()));
                }

                libwebp::WebPPictureFree(std::ptr::addr_of_mut!(picture));
                timestamp += frame.delay().as_millis() as i32;
            }

            libwebp::WebPAnimEncoderAdd(
                encoder,
                std::ptr::null_mut(),
                timestamp,
                std::ptr::null_mut(),
            );

            let mut data = std::mem::zeroed::<libwebp::WebPData>();
            if libwebp::WebPAnimEncoderAssemble(encoder, std::ptr::addr_of_mut!(data)) == 0 {
                return Err(Error::EncodingError("WebP encoding error".to_string()));
            }
            libwebp::WebPAnimEncoderDelete(encoder);

            let out = std::slice::from_raw_parts(data.bytes, data.size as _);
            libwebp::WebPDataClear(std::ptr::addr_of_mut!(data));

            dest.write_all(out)?;
        }

        Ok(())
    }
}

/// An interface for decoding WebP images and animations.
pub struct WebPDecoder<P: Pixel, R: Read> {
    _marker: PhantomData<(P, R)>,
}

impl<P: Pixel, R: Read> WebPDecoder<P, R> {
    /// Create a new decoder that decodes into the given pixel type.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl<P: Pixel, R: Read> Default for WebPDecoder<P, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<P: Pixel, R: Read> Decoder<P, R> for WebPDecoder<P, R> {
    type Sequence = WebPSequenceDecoder<P>;

    fn decode(&mut self, stream: R) -> crate::Result<Image<P>> {
        unsafe {
            let data = stream.bytes().collect::<Result<Vec<_>, _>>()?;
            let mut width @ mut height = 0;

            let out_ptr = libwebp::WebPDecodeRGBA(
                data.as_ptr(),
                data.len() as _,
                std::ptr::addr_of_mut!(width),
                std::ptr::addr_of_mut!(height),
            );
            if out_ptr.is_null() {
                return Err(Error::DecodingError("WebP decoding failed".to_string()));
            }

            let out = std::slice::from_raw_parts(out_ptr, (width * height * 4) as _)
                .chunks_exact(4)
                .map(|p| P::from_raw_parts(ColorType::Rgba, 8, p))
                .collect::<crate::Result<Vec<_>>>();

            libwebp::WebPFree(out_ptr.cast());

            Ok(Image {
                width: NonZeroU32::new(width as _).unwrap(),
                height: NonZeroU32::new(height as _).unwrap(),
                data: out?,
                format: ImageFormat::WebP,
                overlay: OverlayMode::default(),
                palette: None,
            })
        }
    }

    fn decode_sequence(&mut self, stream: R) -> crate::Result<Self::Sequence> {
        unsafe {
            let bytes = stream.bytes().collect::<Result<Vec<_>, _>>()?;
            let data = libwebp::WebPData {
                bytes: bytes.as_ptr(),
                size: bytes.len() as _,
            };
            let demuxer = libwebp::WebPDemux(std::ptr::addr_of!(data));

            Ok(WebPSequenceDecoder {
                _marker: PhantomData,
                demuxer,
                demux_iter: std::ptr::null_mut(),
            })
        }
    }
}

pub struct WebPSequenceDecoder<P: Pixel> {
    _marker: PhantomData<P>,
    demuxer: *const libwebp::WebPDemuxer,
    demux_iter: *mut libwebp::WebPIterator,
}

impl<P: Pixel> FrameIterator<P> for WebPSequenceDecoder<P> {
    fn len(&self) -> u32 {
        unsafe { libwebp::WebPDemuxGetI(self.demuxer, libwebp::WEBP_FF_FRAME_COUNT) as _ }
    }

    fn loop_count(&self) -> LoopCount {
        match unsafe { libwebp::WebPDemuxGetI(self.demuxer, libwebp::WEBP_FF_LOOP_COUNT) as _ } {
            0 => LoopCount::Infinite,
            n => LoopCount::Exactly(n),
        }
    }
}

impl<P: Pixel> Iterator for WebPSequenceDecoder<P> {
    type Item = crate::Result<Frame<P>>;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            if libwebp::WebPDemuxNextFrame(self.demux_iter) == 0 {
                return None;
            }

            let fragment = (*self.demux_iter).fragment;
            let mut width @ mut height = 0;

            let out_ptr = libwebp::WebPDecodeRGBA(
                fragment.bytes,
                fragment.size,
                std::ptr::addr_of_mut!(width),
                std::ptr::addr_of_mut!(height),
            );
            if out_ptr.is_null() {
                return Some(Err(Error::DecodingError(
                    "WebP decoding failed".to_string(),
                )));
            }

            let out = std::slice::from_raw_parts(out_ptr, (width * height * 4) as _)
                .chunks_exact(4)
                .map(|p| P::from_raw_parts(ColorType::Rgba, 8, p))
                .collect::<crate::Result<Vec<_>>>();

            libwebp::WebPFree(out_ptr.cast());

            let out = match out {
                Ok(out) => out,
                Err(err) => return Some(Err(err)),
            };

            let frame = Frame::from_image(Image {
                width: NonZeroU32::new(width as _).unwrap(),
                height: NonZeroU32::new(height as _).unwrap(),
                data: out,
                format: ImageFormat::WebP,
                overlay: OverlayMode::default(),
                palette: None,
            })
            .with_delay(Duration::from_millis((*self.demux_iter).duration as _))
            .with_disposal(match (*self.demux_iter).dispose_method {
                libwebp::WEBP_MUX_DISPOSE_BACKGROUND => DisposalMethod::Background,
                _ => DisposalMethod::None,
            });

            Some(Ok(frame))
        }
    }
}

impl<P: Pixel> Drop for WebPSequenceDecoder<P> {
    fn drop(&mut self) {
        unsafe {
            libwebp::WebPDemuxReleaseIterator(self.demux_iter);
            libwebp::WebPDemuxDelete(self.demuxer as *mut _);
        }
    }
}
