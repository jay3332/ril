use crate::{
    ColorType, Decoder, DisposalMethod, Encoder, Error, Frame, FrameIterator, Image, ImageFormat,
    ImageSequence, LoopCount, OverlayMode, Pixel,
};
use libwebp_sys as libwebp;
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    time::Duration,
};

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
            let free = |mut picture| {
                libwebp::WebPPictureFree(std::ptr::addr_of_mut!(picture));
            };

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
                    import_libwebp_picture!(WebPPictureImportRGBA, 4, as_rgba)
                }
                _ => import_libwebp_picture!(WebPPictureImportRGB, 3, as_rgb),
            } == 0
            {
                free(picture);
                return Err(Error::EncodingError("WebP encoding error".to_string()));
            }

            let mut config = std::mem::zeroed::<libwebp::WebPConfig>();
            if libwebp::WebPConfigInit(std::ptr::addr_of_mut!(config)) == 0 {
                free(picture);
                return Err(Error::EncodingError("WebP version error".to_string()));
            }

            config.lossless = self.lossless as _;
            config.quality = self.quality;

            let res =
                libwebp::WebPEncode(std::ptr::addr_of!(config), std::ptr::addr_of_mut!(picture));
            if res == 0 {
                free(picture);
                return Err(Error::EncodingError("WebP encoding error".to_string()));
            }

            let data = libwebp::WebPData {
                bytes: writer.mem,
                size: writer.size,
            };

            free(picture);
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
        let mut data = self.encode_image(image)?;
        unsafe {
            let result = dest.write_all(std::slice::from_raw_parts(data.bytes, data.size));
            libwebp::WebPDataClear(std::ptr::addr_of_mut!(data));
            result?
        }
        Ok(())
    }

    fn encode_sequence<P: Pixel>(
        &mut self,
        sequence: &ImageSequence<P>,
        dest: &mut impl Write,
    ) -> crate::Result<()> {
        let sample = sequence.first_frame().ok_or(Error::EmptyImageError)?;

        unsafe {
            let mux = libwebp::WebPMuxNew();
            let params = libwebp::WebPMuxAnimParams {
                bgcolor: 0,
                loop_count: sequence.loop_count().count_or_zero() as _,
            };
            libwebp::WebPMuxSetCanvasSize(mux, sample.width() as _, sample.height() as _);
            libwebp::WebPMuxSetAnimationParams(mux, std::ptr::addr_of!(params));

            let mut final_image = std::mem::zeroed::<libwebp::WebPData>();
            let mut encoded_frames = Vec::new();

            let free = |mut final_image: libwebp::WebPData,
                        encoded_frames: Vec<libwebp::WebPData>,
                        mux: *mut libwebp::WebPMux| {
                libwebp::WebPDataClear(std::ptr::addr_of_mut!(final_image));
                for mut f in encoded_frames {
                    libwebp::WebPDataClear(std::ptr::addr_of_mut!(f));
                }
                libwebp::WebPMuxDelete(mux);
            };

            for frame in sequence.iter() {
                let encoded_frame = match self.encode_image(frame) {
                    Ok(d) => d,
                    Err(e) => {
                        free(final_image, encoded_frames, mux);
                        return Err(e);
                    }
                };
                encoded_frames.push(encoded_frame);

                let frame_info = libwebp::WebPMuxFrameInfo {
                    bitstream: encoded_frame,
                    duration: frame.delay().as_millis() as _,
                    id: libwebp::WEBP_CHUNK_ANMF,
                    dispose_method: match frame.disposal() {
                        DisposalMethod::None => libwebp::WEBP_MUX_DISPOSE_NONE,
                        _ => libwebp::WEBP_MUX_DISPOSE_BACKGROUND,
                    },
                    ..std::mem::zeroed() // TODO: blend method could be configurable
                };

                libwebp::WebPMuxPushFrame(mux, std::ptr::addr_of!(frame_info), 0);
            }

            let mux_error = libwebp::WebPMuxAssemble(mux, std::ptr::addr_of_mut!(final_image));
            match mux_error {
                libwebp::WEBP_MUX_OK => {}
                libwebp::WEBP_MUX_NOT_FOUND => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EmptyImageError);
                }
                libwebp::WEBP_MUX_INVALID_ARGUMENT => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EncodingError(
                        "WebP mux invalid argument".to_string(),
                    ));
                }
                libwebp::WEBP_MUX_BAD_DATA => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EncodingError("WebP mux bad data".to_string()));
                }
                libwebp::WEBP_MUX_MEMORY_ERROR => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EncodingError("WebP mux memory error".to_string()));
                }
                libwebp::WEBP_MUX_NOT_ENOUGH_DATA => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EncodingError("WebP mux not enough data".to_string()));
                }
                i32::MIN..=-5_i32 | 2_i32..=i32::MAX => {
                    free(final_image, encoded_frames, mux);
                    return Err(Error::EncodingError(format!(
                        "WebP mux error {}",
                        mux_error
                    )));
                }
            };

            let data = std::slice::from_raw_parts(final_image.bytes, final_image.size);
            let result = dest.write_all(data);
            free(final_image, encoded_frames, mux);
            result?;
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
