use serde::Serialize;
use std::io;

use crate::read::ReadExt;

#[derive(Debug, Serialize)]
pub struct CommonInfo {
    pub note_id: u64,
    pub reply_to_note_id: u64,
    pub sender_pid: u32,
    pub sent: BasicDateTime,
}

#[derive(Debug, Serialize)]
pub struct BasicDateTime {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

impl TryFrom<&[u8]> for CommonInfo {
    type Error = io::Error;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        let mut value = value.get(0..0x40).ok_or_else(|| io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Common1 block too short",
        ))?;

        Ok(CommonInfo {
            note_id: value.read_u64_le()?,
            reply_to_note_id: value.read_u64_le()?,
            sent: BasicDateTime::from_common1_header(value.read_const_num_of_bytes()?),
            sender_pid: value.read_u32_le()?,
        })
    }
}

impl BasicDateTime {
    fn from_common1_header(data: [u8; 8]) -> Self {
        Self {
            year: data[0],
            month: data[1],
            day: data[2],
            hour: data[3],
            minute: data[4],
            second: data[5],
        }
    }
}

impl CommonInfo {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, io::Error> {
        Self::try_from(bytes)
    }
}
