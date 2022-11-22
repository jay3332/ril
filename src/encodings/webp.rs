use crate::{
    ColorType, Decoder, DisposalMethod, Encoder, Error, Frame, FrameIterator, Image, ImageFormat,
    ImageSequence, LoopCount, OverlayMode, Pixel,
};
use libwebp_sys as libwebp;
use libwebp_sys::WebPMuxAnimParams;
use std::io::{Read, Write};
use std::marker::PhantomData;
use std::num::NonZeroU32;
use std::time::Duration;

/// A WebP image encoder.
pub struct WebPEncoder {
    /// Image quality to encode at. Larger values produce higher quality images at
    /// the expense of larger file sizes. Valid values are in the range [0, 100]. For lossless
    /// encoding, higher values will produce better compression at the expense of more computation
    /// and time.
    pub quality: f32,
    /// Whether to use lossless encoding.
    pub lossless: bool,
}

impl Default for WebPEncoder {
    fn default() -> Self {
        Self {
            quality: 75.0,
            lossless: false,
        }
    }
}

impl WebPEncoder {
    /// Creates a new WebP encoder that uses lossy encoding with a quality of 75.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the quality to encode at.
    #[must_use]
    pub const fn with_quality(mut self, quality: f32) -> Self {
        self.quality = quality;
        self
    }

    /// Sets whether to use lossless encoding.
    #[must_use]
    pub const fn with_lossless(mut self, lossless: bool) -> Self {
        self.lossless = lossless;
        self
    }

    #[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]
    fn encode_image<P: Pixel>(&self, image: &Image<P>) -> crate::Result<libwebp::WebPData> {
        unsafe {
            let mut picture = std::mem::zeroed::<libwebp::WebPPicture>();
            picture.width = image.width() as _;
            picture.height = image.height() as _;
            picture.use_argb = 1;

            if libwebp::WebPPictureAlloc(std::ptr::addr_of_mut!(picture)) == 0 {
                return Err(Error::EncodingError("WebP memory error".to_string()));
            }

            let mut writer = std::mem::zeroed::<libwebp::WebPMemoryWriter>();
            libwebp::WebPMemoryWriterInit(std::ptr::addr_of_mut!(writer));

            picture.writer = Some(_wrapped);
            picture.custom_ptr = std::ptr::addr_of_mut!(writer).cast();

            macro_rules! import_libwebp_picture {
                ($func:ident, $stride:literal, $force_into:ident) => {{
                    libwebp::$func(
                        std::ptr::addr_of_mut!(picture),
                        image
                            .data
                            .iter()
                            .flat_map(|p| p.$force_into().as_bytes())
                            .collect::<Vec<_>>()
                            .as_ptr(),
                        (image.width() * $stride) as _,
                    )
                }};
            }
            let sample = image.data[0].color_type();
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

            config.lossless = self.lossless as _;
            config.quality = self.quality;

            let res =
                libwebp::WebPEncode(std::ptr::addr_of!(config), std::ptr::addr_of_mut!(picture));

            let mut free = || libwebp::WebPPictureFree(std::ptr::addr_of_mut!(picture));
            if res == 0 {
                free();
                return Err(Error::EncodingError("WebP encoding error".to_string()));
            }

            let data = libwebp::WebPData {
                bytes: writer.mem,
                size: writer.size,
            };

            free();
            Ok(data)
        }
    }
}

extern "C" fn _wrapped(
    data: *const u8,
    size: usize,
    picture: *const libwebp::WebPPicture,
) -> std::ffi::c_int {
    unsafe { libwebp::WebPMemoryWrite(data, size, picture) }
}

#[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]
impl Encoder for WebPEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> crate::Result<()> {
        let data = self.encode_image(image)?;
        dest.write_all(unsafe { std::slice::from_raw_parts(data.bytes, data.size as _) })?;

        Ok(())
    }

    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        let sample = sequence.first_frame();

        unsafe {
            let mux = libwebp::WebPMuxNew();
            let params = WebPMuxAnimParams {
                bgcolor: 0,
                loop_count: sequence.loop_count().count_or_zero() as _,
            };
            libwebp::WebPMuxSetCanvasSize(mux, sample.width() as _, sample.height() as _);
            libwebp::WebPMuxSetAnimationParams(mux, std::ptr::addr_of!(params));

            for frame in sequence.iter() {
                let frame = libwebp::WebPMuxFrameInfo {
                    bitstream: self.encode_image(frame)?,
                    duration: frame.delay().as_millis() as _,
                    id: libwebp::WEBP_CHUNK_ANMF,
                    dispose_method: match frame.disposal() {
                        DisposalMethod::None => libwebp::WEBP_MUX_DISPOSE_NONE,
                        _ => libwebp::WEBP_MUX_DISPOSE_BACKGROUND,
                    },
                    ..std::mem::zeroed() // TODO: blend method could be configurable
                };

                libwebp::WebPMuxPushFrame(mux, std::ptr::addr_of!(frame), 0);
            }

            let mut data = std::mem::zeroed::<libwebp::WebPData>();
            match libwebp::WebPMuxAssemble(mux, std::ptr::addr_of_mut!(data)) {
                libwebp::WEBP_MUX_NOT_FOUND => return Err(Error::EmptyImageError),
                libwebp::WEBP_MUX_INVALID_ARGUMENT => {
                    return Err(Error::EncodingError(
                        "WebP mux invalid argument".to_string(),
                    ))
                }
                libwebp::WEBP_MUX_BAD_DATA => {
                    return Err(Error::EncodingError("WebP mux bad data".to_string()))
                }
                libwebp::WEBP_MUX_MEMORY_ERROR => {
                    return Err(Error::EncodingError("WebP mux memory error".to_string()))
                }
                libwebp::WEBP_MUX_NOT_ENOUGH_DATA => {
                    return Err(Error::EncodingError("WebP mux not enough data".to_string()))
                }
                _ => (),
            };

            let out = std::slice::from_raw_parts(data.bytes, data.size as _);
            dest.write_all(out)?;

            libwebp::WebPDataClear(std::ptr::addr_of_mut!(data));
            libwebp::WebPMuxDelete(mux);
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
