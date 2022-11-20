use crate::{ColorType, Encoder, Error, Image, ImageSequence, Pixel};
use libwebp_sys as libwebp;
use std::io::Write;

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
            let encoder = libwebp::WebPAnimEncoderNew(
                first_frame.width() as _,
                first_frame.height() as _,
                std::ptr::null(),
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

                config.lossless = matches!(self.quality, WebPQuality::Lossless) as _;
                if let WebPQuality::Lossy(q) = self.quality {
                    config.quality = q as _;
                }

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
