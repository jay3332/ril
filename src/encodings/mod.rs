pub mod png;
pub(crate) mod zlib;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum ColorType {
    L,
    LA,
    Rgb,
    Rgba,
    Palette,
}

impl ColorType {
    pub fn channels(&self) -> usize {
        match self {
            ColorType::L => 1,
            ColorType::LA => 2,
            ColorType::Rgb => 3,
            ColorType::Rgba => 4,
            ColorType::Palette => 1,
        }
    }
}

impl From<png::ColorType> for ColorType {
    fn from(value: png::ColorType) -> Self {
        use png::ColorType::*;

        match value {
            L => ColorType::L,
            LA => ColorType::LA,
            RGB => ColorType::Rgb,
            RGBA => ColorType::Rgba,
            Palette => ColorType::Palette,
        }
    }
}

impl From<ColorType> for png::ColorType {
    fn from(value: ColorType) -> Self {
        use ColorType::*;

        match value {
            L => png::ColorType::L,
            LA => png::ColorType::LA,
            Rgb => png::ColorType::RGB,
            Rgba => png::ColorType::RGBA,
            Palette => png::ColorType::Palette,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum PixelData {
    Bit(bool),
    L(u8),
    LA(u8, u8),
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    Palette(u8),
}

impl PixelData {
    pub fn from_raw(color_type: ColorType, bit_depth: u8, data: &[u8]) -> crate::Result<Self> {
        // TODO: support 16-bit bit depths. right now, it scales down
        if !bit_depth.is_power_of_two() {
            return Err(crate::Error::DecodingError(
                "bit depth must be a power of two",
            ));
        }

        // Scale down to 8-bit
        let data = if bit_depth > 8 {
            let factor = (bit_depth / 8) as usize;
            let mut scaled = Vec::with_capacity(data.len() / factor);

            for chunk in data.chunks(factor) {
                let sum = chunk
                    .iter()
                    .rev()
                    .enumerate()
                    .map(|(i, &x)| (x as usize) << (8 * i))
                    .sum::<usize>();
                scaled.push((sum / factor) as u8);
            }

            scaled
        }
        // Scale up to 8-bit
        else if bit_depth == 2 || bit_depth == 4 {
            let factor = 8 / bit_depth;

            data.iter().map(|&x| x * factor).collect::<Vec<_>>()
        } else {
            data.to_vec()
        };

        Ok(match (color_type, bit_depth) {
            (c, 1) if c.channels() == 1 => PixelData::Bit(data[0] != 0),
            (ColorType::L, _) => PixelData::L(data[0]),
            (ColorType::LA, _) => PixelData::LA(data[0], data[1]),
            (ColorType::Rgb, _) => PixelData::Rgb(data[0], data[1], data[2]),
            (ColorType::Rgba, _) => PixelData::Rgba(data[0], data[1], data[2], data[3]),
            (ColorType::Palette, _) => PixelData::Palette(data[0]),
        })
    }
}
