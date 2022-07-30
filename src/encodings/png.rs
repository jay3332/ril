use super::{zlib::ZlibReader, PixelData};
use crate::{
    encode::{ByteStream, Decoder, Encoder},
    Error::{DecodingError, EmptyImageError, IncompatibleImageData},
    Image, Pixel, Result,
};

use crc32fast::hash as crc;
use miniz_oxide::deflate::compress_to_vec_zlib;
use std::io::Write;

pub const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ColorType {
    #[default]
    L = 0,
    RGB = 2,
    Palette = 3,
    LA = 4,
    RGBA = 6,
}

impl ColorType {
    #[must_use]
    pub const fn channels(&self) -> usize {
        match self {
            Self::L | Self::Palette => 1,
            Self::LA => 2,
            Self::RGB => 3,
            Self::RGBA => 4,
        }
    }
}

impl TryFrom<u8> for ColorType {
    type Error = crate::Error;

    fn try_from(value: u8) -> Result<Self> {
        match value {
            0 => Ok(Self::L),
            2 => Ok(Self::RGB),
            3 => Ok(Self::Palette),
            4 => Ok(Self::LA),
            6 => Ok(Self::RGBA),
            _ => Err(DecodingError("invalid color type")),
        }
    }
}

#[derive(Debug, Default)]
pub struct PngHeader {
    width: u32,
    height: u32,
    // Currently, only 8-bit grayscale (L) and RGB8/RGBA8 are supported.
    //
    // For bit-depths below 8-bits, the value is scaled up. 16-bit depths are planned to be
    // supported in the future. For now, they scale down.
    //
    // Grayscale with a bit-depth of 1 transforms is a special case; it doesn't scale but instead
    // is transformed into the special BitPixel type.
    bit_depth: u8,
    color_type: ColorType,
    // Only 0 is a valid value as defined the spec.
    compression_method: u8,
    // Only 0 is a valid value as defined in the spec.
    filter_method: u8,
    // Values 0 "no interlace" and 1 "Adam7 interlace" are allowed.
    interlace_method: u8,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum FilterType {
    #[default]
    None = 0,
    Sub = 1,
    Up = 2,
    Average = 3,
    Paeth = 4,
}

impl From<u8> for FilterType {
    fn from(member: u8) -> Self {
        match member {
            0 => Self::None,
            1 => Self::Sub,
            2 => Self::Up,
            3 => Self::Average,
            4 => Self::Paeth,
            _ => panic!("invalid filter index"),
        }
    }
}

impl FilterType {
    fn paeth(a: u8, b: u8, c: u8) -> u8 {
        let p = a + b - c;
        let pa = p.abs_diff(a);
        let pb = p.abs_diff(b);
        let pc = p.abs_diff(c);

        if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        }
    }

    pub fn filter(&self, x: u8, a: u8, b: u8, c: u8) -> u8 {
        x - match self {
            Self::None => 0,
            Self::Sub => a,
            Self::Up => b,
            Self::Average => (a + b) / 2,
            Self::Paeth => Self::paeth(a, b, c),
        }
    }

    pub fn reconstruct(
        &self,
        previous: &Option<Vec<Vec<u8>>>,
        current: &Vec<&[u8]>,
        i: usize,
        j: usize,
    ) -> u8 {
        let x = current[i][j];

        macro_rules! a {
            () => {{
                if i > 0 {
                    current[i - 1][j]
                } else {
                    x
                }
            }};
        }

        macro_rules! b {
            () => {{
                previous.as_ref().map(|p| p[i][j]).unwrap_or(x)
            }};
        }

        macro_rules! c {
            () => {{
                if i > 0 {
                    previous.as_ref().map(|p| p[i - 1][j]).unwrap_or(x)
                } else {
                    x
                }
            }};
        }

        x + match self {
            Self::None => 0,
            Self::Sub => a!(),
            Self::Up => b!(),
            Self::Average => (a!() + b!()) / 2,
            Self::Paeth => Self::paeth(a!(), b!(), c!()),
        }
    }
}

/// Decodes a PNG image into an image.
pub struct PngDecoder {
    inflater: ZlibReader,
    /// The decoded IHDR metadata about the image.
    pub ihdr: PngHeader,
    /// THe accumulative IDAT chunks containing pixel data about the image.
    pub idat: Vec<u8>,
}

impl PngDecoder {
    /// Creates a new PNG decoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inflater: ZlibReader::new(),
            ihdr: PngHeader::default(),
            idat: Vec::new(),
        }
    }

    fn parse_header(&mut self, data: &mut ByteStream) -> Result<()> {
        self.ihdr.width = data.read_u32()?;
        self.ihdr.height = data.read_u32()?;
        self.ihdr.bit_depth = match data.read_u8()? {
            16 => return Err(DecodingError("16-bit bit depth is not yet supported")),
            n @ (1 | 2 | 4 | 8) => n,
            _ => return Err(DecodingError("Invalid bit depth")),
        };
        self.ihdr.color_type = match data.read_u8()? {
            4 => return Err(DecodingError("LA color type is not supported yet")),
            3 => return Err(DecodingError("Palette color type is not supported yet")),
            n @ (0 | 2 | 6) => ColorType::try_from(n).unwrap(),
            _ => return Err(DecodingError("Invalid color type")),
        };
        self.ihdr.compression_method = data.read_u8()?;
        self.ihdr.filter_method = data.read_u8()?;
        self.ihdr.interlace_method = data.read_u8()?;

        Ok(())
    }
}

impl Decoder for PngDecoder {
    fn decode<P: Pixel>(&mut self, stream: &mut ByteStream) -> Result<Image<P>> {
        let signature = stream.read(8);

        if signature != PNG_SIGNATURE {
            return Err(DecodingError("Invalid PNG signature"));
        }

        let mut parsing_idat = false;

        while stream.remaining() >= 12 {
            {
                let length = stream.read_u32()?;
                let chunk_type = stream.read_to::<[u8; 4]>();
                let data = stream.read(length as usize);

                if parsing_idat && &chunk_type != b"IDAT" {
                    parsing_idat = false;
                    self.inflater.finish(&mut self.idat)?;
                }

                match &chunk_type {
                    b"IHDR" => self.parse_header(&mut ByteStream::new(data))?,
                    b"IDAT" => {
                        parsing_idat = true;
                        self.inflater.decompress(data, &mut self.idat)?;
                    }
                    b"IEND" => {
                        let pixels = self.idat.chunks_exact(self.ihdr.width as usize + 1);

                        let mut result = Vec::new();
                        let mut previous = None;

                        // Propgate uninformative panics in the future
                        if pixels.len() != (self.ihdr.width * self.ihdr.height) as usize {
                            return Err(IncompatibleImageData {
                                width: self.ihdr.width,
                                height: self.ihdr.height,
                                received: pixels.len(),
                            });
                        }

                        for scanline in pixels {
                            let filter_type = FilterType::from(scanline[0]);

                            let channels = self.ihdr.color_type.channels();
                            let pixels = self.idat[1..].chunks(channels).collect::<Vec<_>>();

                            let out = if filter_type != FilterType::None {
                                (0..self.ihdr.width as usize)
                                    .map(|i| {
                                        (0..channels)
                                            .map(|j| {
                                                filter_type.reconstruct(&previous, &pixels, i, j)
                                            })
                                            .collect::<Vec<_>>()
                                    })
                                    .collect::<Vec<_>>()
                            } else {
                                pixels.into_iter().map(|p| p.to_vec()).collect()
                            };

                            previous.replace(out.clone());
                            result.extend(out);
                        }

                        return Ok(Image {
                            width: self.ihdr.width,
                            height: self.ihdr.height,
                            data: result
                                .into_iter()
                                .map(|p| {
                                    PixelData::from_raw(
                                        super::ColorType::from(self.ihdr.color_type),
                                        self.ihdr.bit_depth,
                                        &p[..],
                                    )
                                    .and_then(|p| P::from_pixel_data(p))
                                })
                                .collect::<Result<Vec<_>>>()?,
                            format: crate::ImageFormat::Png,
                            overlay: crate::image::OverlayMode::default(),
                        });
                    }
                    // Ignore unknown chunks
                    _ => (),
                }
            }

            let _crc = stream.read_u32()?;
        }

        Err(DecodingError(
            "Unexpected end of file (expected IEND chunk)",
        ))
    }
}

impl Default for PngDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Encodes an image into a PNG image.
#[derive(Debug)]
pub struct PngEncoder {
    /// The compression level to compress IDAT pixel data. Must be between 0 and 9.
    ///
    /// The default level is 6. Lower values mean faster encoding with the cost of having a larger
    /// output. Quality will not be influenced since compression is lossless.
    pub compression_level: u8,
    /// The default filtering type to use per row.
    pub filter_type: FilterType,
}

impl PngEncoder {
    /// Creates a new PNG encoder.
    #[must_use]
    pub fn new() -> Self {
        Self {
            compression_level: 6,
            filter_type: FilterType::default(),
        }
    }

    /// Sets the compression level. Must be a value between 0 and 9.
    ///
    /// # Panics
    /// * The compression value is not between 0 and 9.
    pub fn with_compression_level(self, level: u8) -> Self {
        assert!(level <= 9, "Compression level must be between 0 and 9");

        // SAFETY: bounds are checked above
        unsafe { self.with_compression_level_unchecked(level) }
    }

    /// Sets the compression level. Should be a value between 0 and 9, but this does not check
    /// for that. Any other value may lead to unwanted or unknown behavior.
    pub unsafe fn with_compression_level_unchecked(mut self, level: u8) -> Self {
        self.compression_level = level;
        self
    }

    /// Sets the default filter type to use per row.
    pub fn with_filter_method(mut self, ty: FilterType) -> Self {
        self.filter_type = ty;
        self
    }

    fn write_chunk(
        &mut self,
        name: &'static str,
        data: &[u8],
        dest: &mut impl Write,
    ) -> Result<()> {
        dest.write(&(data.len() as u32).to_be_bytes())?;
        dest.write(name.as_bytes())?;
        dest.write(data)?;
        dest.write(&crc(&*[name.as_bytes(), data].concat()).to_be_bytes())?;

        Ok(())
    }
}

impl Encoder for PngEncoder {
    fn encode<P: Pixel>(&mut self, image: &Image<P>, dest: &mut impl Write) -> Result<()> {
        dest.write(&PNG_SIGNATURE)?;

        let first = image.data.get(0).ok_or(EmptyImageError)?;
        let (ty, depth) = first.as_pixel_data().type_data();

        let ihdr = [
            &image.width.to_be_bytes() as &[_],
            &image.height.to_be_bytes(),
            &[
                depth,
                ColorType::from(ty) as u8,
                0,
                0,
                // TODO: interlacing
                0,
            ],
        ]
        .concat();

        self.write_chunk("IHDR", &*ihdr, dest)?;

        // Use this instead of .flat_map due to Rust's borrow checker rules
        let mut idat = Vec::<u8>::with_capacity(image.len() as usize);

        for row in image.pixels() {
            idat.push(self.filter_type as u8);

            let row = row
                .into_iter()
                .map(P::as_pixel_data)
                .map(|p| p.data())
                .collect::<Vec<_>>();

            // todo
            assert_eq!(self.filter_type, FilterType::None);

            row.into_iter().for_each(|p| idat.extend(p));
        }

        let compressed = compress_to_vec_zlib(&*idat, self.compression_level);
        self.write_chunk("IDAT", &*compressed, dest)?;

        // IEND is a chunk with no data
        self.write_chunk("IEND", &[], dest)?;

        Ok(())
    }
}

impl Default for PngEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::PngDecoder;
    use crate::encode::{ByteStream, Decoder};
    use crate::Rgb;

    #[test]
    fn test_decode_basic() {
        // A non-filtered, non-interlaced PNG image of a single red pixel
        let data = &[
            137_u8, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0,
            1, 8, 2, 0, 0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 8, 215, 99, 248, 207,
            192, 0, 0, 3, 1, 1, 0, 24, 221, 141, 176, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
        ];

        let image = PngDecoder::new()
            .decode::<Rgb>(&mut ByteStream::new(data))
            .unwrap();

        assert_eq!(image.dimensions(), (1, 1));
        assert_eq!(image.pixels(), vec![&[Rgb::new(255, 0, 0)]]);
    }
}
