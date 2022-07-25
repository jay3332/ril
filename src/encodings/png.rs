use crate::{Pixel, encode::{ByteStream, Decoder}, Error::DecodingError, Image, Result};

use flate2::read::DeflateDecoder;
use std::io::Read;

pub const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
enum ColorType {
    #[default]
    L = 0,
    RGB = 2,
    Palette = 3,
    RGBA = 4,
    LA = 6,
}

#[derive(Default)]
struct PngHeader {
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

pub struct PngDecoder<P: Pixel> {
    pub ihdr: PngHeader,
    pub idat: Vec<u8>,
}

impl<P: Pixel> PngDecoder<P> {
    pub fn new() -> Self {
        Self {
            ihdr: PngHeader::default(),
            idat: Vec::new(),
        }
    }

    fn parse_header(&mut self, data: &mut ByteStream) -> Result<()> {
        self.ihdr.width = data.read_to();
        self.ihdr.height = data.read_to();
        self.ihdr.bit_depth = match data.read_to::<u8>() {
            16 => return Err(DecodingError("16-bit bit depth is not yet supported")),
            n @ (1 | 2 | 4 | 8) => n,
        };
        self.ihdr.color_type = match data.read_to::<u8>() {
            4 => return Err(DecodingError("LA color type is not supported yet")),
            3 => return Err(DecodingError("Palette color type is not supported yet")),
            n @ (0 | 2 | 6)  => ColorType::try_from(n).unwrap(),
            _ => return Err(DecodingError("Invalid color type")),
        };
        self.ihdr.compression_method = data.read_to();
        self.ihdr.filter_method = data.read_to();
        self.ihdr.interlace_method = data.read_to();

        Ok(())
    }
}

impl<P: Pixel> Decoder<P> for PngDecoder<P> {
    fn decode(&mut self, stream: &mut ByteStream) -> Result<Image<P>> {
        let signature = stream.read(8);
        assert_eq!(signature, PNG_SIGNATURE);

        while stream.remaining() > 12 {
            let length = stream.read_to::<u32>();
            let chunk_type = stream.read(4);
            let data = stream.read(length as usize);
            let _crc = stream.read(4);

            match chunk_type {
                b"IHDR" => self.parse_header(&mut ByteStream::new(data))?,
                b"IDAT" => {
                    let mut decoder = DeflateDecoder::new(data);
                    let mut buffer = Vec::new();

                    decoder.read_to_end(&mut buffer)?;
                    self.idat.extend_from_slice(&buffer);
                },
                b"IEND" => break,
                // Ignore unknown chunks
                _ => (),
            }
        }

        Ok(Image {

        })
    }
}
