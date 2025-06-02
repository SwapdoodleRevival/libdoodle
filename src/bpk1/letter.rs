use std::fmt::Display;
use std::io::{self, Cursor, Seek};

use serde::Serialize;

use super::{BPK1Block, BPK1File, BlocksHashMap, stationery::Stationery};
use crate::error::GenericError;
use crate::read::{self, *};
use crate::{color::Colors, error::GenericResult, mii_data::MiiData, read::ReadExt, sheet::Sheet};

#[derive(Debug, Serialize)]
pub struct Letter {
    pub thumbnails: Vec<Vec<u8>>,
    pub sender_mii: Option<MiiData>,
    pub stationery: Option<Stationery>,
    pub sheets: Vec<Sheet>,
    pub colors: Option<Colors>,
    pub blocks: BlocksHashMap,
    pub common: CommonInfo,
}

impl BPK1File for Letter {
    fn new_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Self> {
        let mut thumbnails = vec![];
        let mut sender_mii = None;
        let mut stationery = None;
        let mut colors = None;
        let mut sheets = vec![];
        let mut common: Option<CommonInfo> = None;

        for block in &blocks {
            // Apparently you can't cleanly match against CString; so I'll just use a byte string. Essentially identical
            match block.name.to_bytes() {
                b"THUMB2" => {
                    thumbnails.push(block.data.to_owned());
                }
                b"MIISTD1" => {
                    let mut slice: &[u8] = &block.data;
                    sender_mii = Some(MiiData::from_bytes(slice.read_const_num_of_bytes()?)?)
                }
                b"COLSLT1" => {
                    colors = Some(Colors::from_bytes(&block.data)?);
                }
                b"STATIN1" => stationery = Some(Stationery::new_from_bpk1_bytes(&block.data)?),
                b"SHEET1" => {
                    sheets.push(Sheet::from_bytes(&block.data).unwrap());
                }
                b"COMMON1" => {
                    common = Some(CommonInfo::from_bytes(&block.data)?)
                }
                _ => {}
            }
        }

        Ok(Letter {
            thumbnails,
            sender_mii,
            stationery,
            colors,
            sheets,
            blocks: BlocksHashMap::new_from_bpk1_blocks(blocks)?,
            common: common.ok_or("COMMON1 header is missing")?
        })
    }
}

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

#[cfg(test)]
pub mod tests {
    use std::fs::read;

    use chrono::{DateTime, Utc};

    use super::*;

    #[test]
    fn test_de() {
        // using read instead of include_bytes so it fails at runtime if the test case isn't present instead of not compiling
        let letter =
            dbg!(Letter::new_from_bpk1_bytes(&read("test_cases/letter.bpk").unwrap()).unwrap());
        let mii = letter.sender_mii.unwrap();
        println!("Mii: {:#?}", mii);
        let datetime: DateTime<Utc> = mii.mii_creation_date.into();
        println!("Creation date: {} UTC", datetime.format("%d/%m/%Y %T"));
        println!("{}", mii.get_mii_studio_url());
        println!("{:#?}", letter.sheets);
    }
}
