use serde::Serialize;
use std::io::{Cursor, Error as IoError};
#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{bits::PickBit, error::GenericResult, read::ReadExt};

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Sheet {
    pub events: Vec<SheetEvent>,
    pub secret_page: bool,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi))]
pub struct Sticker {
    index: u8,
    rotation: u8,
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(namespace, into_wasm_abi))]
pub enum SheetEventData {
    PaintEvent {
        continue_to_next: bool,
        thick_pen: bool,
        color_index: u8,
    },
    GameIconEvent {
        sticker_data: Sticker,
    },
    BadgeEvent {
        sticker_data: Sticker,
    },
    MiiEvent {
        sticker_data: Sticker,
        facial_expression: u8,
    },
    PhotoEvent,
    Unknown {
        stroke_type: u8,
    },
}

impl SheetEventData {
    fn from_bytes(bytes: [u8; 4]) -> Self {
        return match (bytes[0].pick_bits(0..=3)) {
            0 => Self::PaintEvent {
                continue_to_next: bytes[2].pick_bit(6),
                color_index: bytes[3].pick_bits(0..=2),
                thick_pen: bytes[3].pick_bit(3),
            },
            12 => Self::GameIconEvent {
                sticker_data: Sticker {
                    index: (bytes[2].pick_bits(7..=7) << 1) | bytes[2].pick_bits(6..=6),
                    rotation: bytes[3].pick_bits(4..=7),
                },
            },
            13 => Self::BadgeEvent {
                sticker_data: Sticker {
                    index: (bytes[2].pick_bits(7..=7) << 1) | bytes[2].pick_bits(6..=6),
                    rotation: bytes[3].pick_bits(4..=7),
                },
            },
            14 => Self::MiiEvent {
                sticker_data: Sticker {
                    index: bytes[2].pick_bits(6..=7),
                    rotation: bytes[3].pick_bits(4..=7),
                },
                facial_expression: bytes[3].pick_bits(0..=3),
            },
            9 => Self::PhotoEvent,
            unknown_kind => Self::Unknown {
                stroke_type: unknown_kind,
            },
        };
    }
}

#[derive(Debug, Serialize)]
#[cfg_attr(feature = "tsify", derive(Tsify))]
pub struct SheetEvent {
    pub x: u8,
    pub y: u8,
    pub style_3d: bool,
    pub data: SheetEventData,
}

impl Sheet {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, IoError> {
        Self::try_from(bytes)
    }
}

impl SheetEvent {
    pub fn from_bytes(bytes: [u8; 4]) -> SheetEvent {
        SheetEvent {
            x: (bytes[2].pick_bits(0..=3)) << 4 | bytes[1].pick_bits(4..=7),
            y: (bytes[1].pick_bits(0..=3)) << 4 | bytes[0].pick_bits(4..=7),
            style_3d: bytes[2].pick_bit(5),
            data: SheetEventData::from_bytes(bytes),
        }
    }
}

impl TryFrom<&[u8]> for Sheet {
    type Error = IoError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut reader = Cursor::new(value);
        reader.read_u32_le()?; // seems to be constant
        let num_blocks = reader.read_u32_le()?;

        let secret_page: bool = reader.read_const_num_of_bytes::<1>()?[0] != 0;

        reader.set_position(0x40);

        Ok(Sheet {
            events: (0..num_blocks)
                .map(|_| reader.read_const_num_of_bytes().map(SheetEvent::from_bytes))
                .collect::<Result<Vec<_>, _>>()?,
            secret_page: secret_page,
        })
    }
}
