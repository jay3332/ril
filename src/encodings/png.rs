use super::{zlib::ZlibReader, PixelData};
use crate::{
    encode::{ByteStream, Decoder},
    Error::{DecodingError, IncompatibleImageData},
    Image, Pixel, Result,
};

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

pub struct PngDecoder {
    inflater: ZlibReader,
    pub ihdr: PngHeader,
    pub idat: Vec<u8>,
}

impl PngDecoder {
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
                        let pixels = self.idat[1..]
                            .chunks(self.ihdr.color_type.channels())
                            .collect::<Vec<_>>();

                        if pixels.len() != (self.ihdr.width * self.ihdr.height) as usize {
                            return Err(IncompatibleImageData {
                                width: self.ihdr.width,
                                height: self.ihdr.height,
                                received: pixels.len(),
                            });
                        }

                        return Ok(Image {
                            width: self.ihdr.width,
                            height: self.ihdr.height,
                            data: pixels
                                .into_iter()
                                .map(|p| {
                                    PixelData::from_raw(
                                        super::ColorType::from(self.ihdr.color_type),
                                        self.ihdr.bit_depth,
                                        p,
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
