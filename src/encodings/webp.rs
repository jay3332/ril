use crate::{
    encode, ColorType, Decoder, DisposalMethod, Encoder, Error, Frame, FrameIterator, Image,
    ImageFormat, LoopCount, OverlayMode, Pixel,
};
use libwebp_sys as libwebp;
use std::{
    io::{Read, Write},
    marker::PhantomData,
    num::NonZeroU32,
    ptr::{addr_of, addr_of_mut},
    time::Duration,
};

/// Options for the WebP image encoder.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct WebPEncoderOptions {
    /// Image quality to encode at. Larger values produce higher quality images at
    /// the expense of larger file sizes. Valid values are in the range [0, 100]. For lossless
    /// encoding, higher values will produce better compression at the expense of more computation
    /// and time.
    pub quality: f32,
    /// Whether to use lossless encoding.
    pub lossless: bool,
}

impl Default for WebPEncoderOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl WebPEncoderOptions {
    /// Creates a new WebP encoder that uses lossy encoding with a quality of 75.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            quality: 75.0,
            lossless: false,
        }
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
}

#[inline]
unsafe fn free_mux(
    encoded_frames: impl Iterator<Item = *mut libwebp::WebPData>,
    mux: *mut libwebp::WebPMux,
) {
    for f in encoded_frames {
        libwebp::WebPDataClear(f);
    }
    libwebp::WebPMuxDelete(mux);
}

#[inline]
unsafe fn free_picture(mut picture: libwebp::WebPPicture) {
    libwebp::WebPPictureFree(addr_of_mut!(picture));
}

#[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]
fn encode_image<P: Pixel>(
    options: &WebPEncoderOptions,
    image: &Image<P>,
) -> crate::Result<libwebp::WebPData> {
    unsafe {
        let mut picture = std::mem::zeroed::<libwebp::WebPPicture>();
        picture.width = image.width() as _;
        picture.height = image.height() as _;
        picture.use_argb = 1;

        if libwebp::WebPPictureAlloc(addr_of_mut!(picture)) == 0 {
            return Err(Error::EncodingError("WebP memory error".to_string()));
        }

        let mut writer = std::mem::zeroed::<libwebp::WebPMemoryWriter>();
        libwebp::WebPMemoryWriterInit(addr_of_mut!(writer));

        picture.writer = Some(_wrapped);
        picture.custom_ptr = addr_of_mut!(writer).cast();

        macro_rules! import_libwebp_picture {
            ($func:ident, $stride:literal, $force_into:ident) => {{
                libwebp::$func(
                    addr_of_mut!(picture),
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
            free_picture(picture);
            return Err(Error::EncodingError("WebP encoding error".to_string()));
        }

        let mut config = std::mem::zeroed::<libwebp::WebPConfig>();
        if libwebp::WebPConfigInit(addr_of_mut!(config)) == 0 {
            free_picture(picture);
            return Err(Error::EncodingError("WebP version error".to_string()));
        }

        config.lossless = options.lossless as _;
        config.quality = options.quality;

        let res = libwebp::WebPEncode(addr_of!(config), addr_of_mut!(picture));
        if res == 0 {
            free_picture(picture);
            return Err(Error::EncodingError("WebP encoding error".to_string()));
        }

        let data = libwebp::WebPData {
            bytes: writer.mem,
            size: writer.size,
        };

        free_picture(picture);
        Ok(data)
    }
}

extern "C" fn _wrapped(
    data: *const u8,
    size: usize,
    picture: *const libwebp::WebPPicture,
) -> std::ffi::c_int {
    unsafe { libwebp::WebPMemoryWrite(data, size, picture) }
}

/// An interface for encoding static WebP images.
///
/// # See Also
/// * [`WebPMuxEncoder`] for encoding WebP animations instead of just static images.
pub struct WebPStaticEncoder<P: Pixel, W: Write> {
    options: WebPEncoderOptions,
    writer: W,
    _marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> Encoder<P, W> for WebPStaticEncoder<P, W> {
    type Config = WebPEncoderOptions;

    fn new(
        dest: W,
        metadata: impl encode::HasEncoderMetadata<Self::Config, P>,
    ) -> crate::Result<Self> {
        Ok(Self {
            options: metadata.config(),
            writer: dest,
            _marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl encode::FrameLike<P>) -> crate::Result<()> {
        let mut data = encode_image(&self.options, frame.image())?;
        unsafe {
            let result = self
                .writer
                .write_all(std::slice::from_raw_parts(data.bytes, data.size));
            libwebp::WebPDataClear(addr_of_mut!(data));
            result?;
        }
        Ok(())
    }

    // no-op
    fn finish(self) -> crate::Result<()> {
        Ok(())
    }
}

/// An interface for encoding WebP animations.
///
/// # See Also
/// * [`WebPStaticEncoder`] for encoding static WebP images instead of animations.
pub struct WebPMuxEncoder<P: Pixel, W: Write> {
    options: WebPEncoderOptions,
    writer: W,
    mux: *mut libwebp::WebPMux,
    encoded_frames: Vec<libwebp::WebPData>, // drop later
    _marker: PhantomData<P>,
}

impl<P: Pixel, W: Write> WebPMuxEncoder<P, W> {
    #[inline]
    unsafe fn free(&mut self) {
        free_mux(
            self.encoded_frames.iter_mut().map(|f| f as *mut _),
            self.mux,
        );
    }
}

#[allow(clippy::cast_lossless, clippy::cast_possible_wrap)]
impl<P: Pixel, W: Write> Encoder<P, W> for WebPMuxEncoder<P, W> {
    type Config = WebPEncoderOptions;

    fn new(
        dest: W,
        metadata: impl encode::HasEncoderMetadata<Self::Config, P>,
    ) -> crate::Result<Self> {
        let mux = unsafe {
            let mux = libwebp::WebPMuxNew();
            libwebp::WebPMuxSetCanvasSize(mux, metadata.width() as _, metadata.height() as _);

            let params = libwebp::WebPMuxAnimParams {
                bgcolor: 0,
                loop_count: metadata.sequence().map_or(0, |(_, l)| l.count_or_zero()) as _,
            };
            libwebp::WebPMuxSetAnimationParams(mux, addr_of!(params));
            mux
        };

        let encoded_frames = match metadata.sequence() {
            Some((0, _)) | None => Vec::new(),
            Some((frame_count, _)) => Vec::with_capacity(frame_count as _),
        };

        Ok(Self {
            options: metadata.config(),
            writer: dest,
            mux,
            encoded_frames,
            _marker: PhantomData,
        })
    }

    fn add_frame(&mut self, frame: &impl encode::FrameLike<P>) -> crate::Result<()> {
        let encoded_frame = match encode_image(&self.options, frame.image()) {
            Ok(d) => d,
            Err(e) => {
                unsafe {
                    self.free();
                }
                return Err(e);
            }
        };
        self.encoded_frames.push(encoded_frame);

        let frame_info = libwebp::WebPMuxFrameInfo {
            bitstream: encoded_frame,
            duration: frame.delay().as_ref().map_or(0, Duration::as_millis) as _,
            id: libwebp::WEBP_CHUNK_ANMF,
            dispose_method: match frame.disposal() {
                Some(DisposalMethod::None) => libwebp::WEBP_MUX_DISPOSE_NONE,
                _ => libwebp::WEBP_MUX_DISPOSE_BACKGROUND,
            },
            ..unsafe { std::mem::zeroed() } // TODO: blend method could be configurable
        };

        unsafe {
            libwebp::WebPMuxPushFrame(self.mux, addr_of!(frame_info), 0);
        }
        Ok(())
    }

    fn finish(mut self) -> crate::Result<()> {
        let mut final_image = unsafe { std::mem::zeroed::<libwebp::WebPData>() };
        let mux_error = unsafe { libwebp::WebPMuxAssemble(self.mux, addr_of_mut!(final_image)) };
        {
            let mut free = || unsafe {
                self.free();
                libwebp::WebPDataClear(addr_of_mut!(final_image));
            };
            match mux_error {
                libwebp::WEBP_MUX_OK => {}
                libwebp::WEBP_MUX_NOT_FOUND => {
                    free();
                    return Err(Error::EmptyImageError);
                }
                libwebp::WEBP_MUX_INVALID_ARGUMENT => {
                    free();
                    return Err(Error::EncodingError(
                        "WebP mux invalid argument".to_string(),
                    ));
                }
                libwebp::WEBP_MUX_BAD_DATA => {
                    free();
                    return Err(Error::EncodingError("WebP mux bad data".to_string()));
                }
                libwebp::WEBP_MUX_MEMORY_ERROR => {
                    free();
                    return Err(Error::EncodingError("WebP mux memory error".to_string()));
                }
                libwebp::WEBP_MUX_NOT_ENOUGH_DATA => {
                    free();
                    return Err(Error::EncodingError("WebP mux not enough data".to_string()));
                }
                _ => {
                    free();
                    return Err(Error::EncodingError(format!("WebP mux error {mux_error}")));
                }
            };
        }

        let data = unsafe { std::slice::from_raw_parts(final_image.bytes, final_image.size) };
        let result = self.writer.write_all(data);

        unsafe {
            self.free();
            libwebp::WebPDataClear(addr_of_mut!(final_image));
        }
        result?;
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
                addr_of_mut!(width),
                addr_of_mut!(height),
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
            let demuxer = libwebp::WebPDemux(addr_of!(data));

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
                addr_of_mut!(width),
                addr_of_mut!(height),
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
