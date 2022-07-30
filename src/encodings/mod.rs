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
    #[must_use]
    pub fn channels(&self) -> usize {
        match self {
            Self::L => 1,
            Self::LA => 2,
            Self::Rgb => 3,
            Self::Rgba => 4,
            Self::Palette => 1,
        }
    }
}

impl From<png::ColorType> for ColorType {
    fn from(value: png::ColorType) -> Self {
        use png::ColorType::{Palette, L, LA, RGB, RGBA};

        match value {
            L => Self::L,
            LA => Self::LA,
            RGB => Self::Rgb,
            RGBA => Self::Rgba,
            Palette => Self::Palette,
        }
    }
}

impl From<ColorType> for png::ColorType {
    fn from(value: ColorType) -> Self {
        use ColorType::{Palette, Rgb, Rgba, L, LA};

        match value {
            L => Self::L,
            LA => Self::LA,
            Rgb => Self::RGB,
            Rgba => Self::RGBA,
            Palette => Self::Palette,
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
    pub fn type_data(&self) -> (ColorType, u8) {
        match self {
            Self::Bit(_) => (ColorType::L, 1),
            Self::L(_) => (ColorType::L, 8),
            Self::LA(..) => (ColorType::LA, 8),
            Self::Rgb(..) => (ColorType::Rgb, 8),
            Self::Rgba(..) => (ColorType::Rgba, 8),
            Self::Palette(_) => (ColorType::Palette, 8),
        }
    }

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
            (c, 1) if c.channels() == 1 => Self::Bit(data[0] != 0),
            (ColorType::L, _) => Self::L(data[0]),
            (ColorType::LA, _) => Self::LA(data[0], data[1]),
            (ColorType::Rgb, _) => Self::Rgb(data[0], data[1], data[2]),
            (ColorType::Rgba, _) => Self::Rgba(data[0], data[1], data[2], data[3]),
            (ColorType::Palette, _) => Self::Palette(data[0]),
        })
    }

    pub fn data(&self) -> Vec<u8> {
        match *self {
            Self::Bit(value) => vec![value.then_some(255).unwrap_or(0)],
            Self::L(l) => vec![l],
            Self::LA(l, a) => vec![l, a],
            Self::Rgb(r, g, b) => vec![r, g, b],
            Self::Rgba(r, g, b, a) => vec![r, g, b, a],
            Self::Palette(idx) => vec![idx],
        }
    }
}
