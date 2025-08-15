use serde::Serialize;
use std::io::{Cursor, Error as IoError};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{
    bits::PickBit,
    read::{ReadExt, read_utf16_name},
};

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Colors {
    pub colors: Vec<Color>,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
pub struct Color {
    pub primary: RGBA,
    pub extra1: RGBA,
    pub extra2: RGBA,
    pub extra3: RGBA,
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
pub struct RGBA {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl RGBA {
    fn full_rgb_value(nibble: u8) -> u8 {
        nibble << 4 | nibble
    }

    pub fn from_bytes(bytes: [u8; 2]) -> Self {
        RGBA {
            r: Self::full_rgb_value(bytes[1].pick_bits(4..=7)),
            g: Self::full_rgb_value(bytes[1].pick_bits(0..=3)),
            b: Self::full_rgb_value(bytes[0].pick_bits(4..=7)),
            a: Self::full_rgb_value(bytes[0].pick_bits(0..=3)),
        }
    }
}

impl Colors {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IoError> {
        Self::try_from(bytes)
    }
}

impl Color {
    pub fn from_bytes(bytes: [u8; 0x4c]) -> Color {
        Color {
            primary: RGBA::from_bytes(bytes[4..=5].try_into().unwrap()),
            extra1: RGBA::from_bytes(bytes[6..=7].try_into().unwrap()),
            extra2: RGBA::from_bytes(bytes[8..=9].try_into().unwrap()),
            extra3: RGBA::from_bytes(bytes[10..=11].try_into().unwrap()),
            id: u32::from_le_bytes(bytes[0..=3].try_into().unwrap()),
            name: read_utf16_name::<0x40>(bytes[0xc..=0x4b].try_into().unwrap()),
        }
    }
}

impl TryFrom<&[u8]> for Colors {
    type Error = IoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut reader = Cursor::new(value);
        let num_colors = reader.read_u32_le()?;
        reader.read_u32_le()?;
        reader.set_position(0x10);

        let mut colors: Colors = Colors { colors: vec![] };

        for _ in 0..num_colors {
            colors
                .colors
                .push(Color::from_bytes(reader.read_const_num_of_bytes()?));
        }

        Ok(colors)
    }
}
