use std::io::{self, Cursor};

use serde::Serialize;

use crate::read::ReadExt;

#[derive(Debug, Serialize)]
pub struct CommonInfo {
    pub sender_pid: u32,
}

impl TryFrom<&[u8]> for CommonInfo {
    type Error = io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut reader = Cursor::new(value);

        reader.set_position(0x18);

        Ok(CommonInfo {
            sender_pid: reader.read_u32_le()?
        })
    }
}

impl CommonInfo {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        Self::try_from(bytes)
    }
}