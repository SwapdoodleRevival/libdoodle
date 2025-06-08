use std::error::Error;
use std::fmt::Display;

use serde::Serialize;

use super::{BPK1Block, BPK1File, BlocksHashMap, stationery::Stationery};
use crate::common_info::CommonInfo;
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
                b"COMMON1" => common = Some(CommonInfo::from_bytes(&block.data)?),
                _ => {}
            }
        }

        Ok(Letter {
            thumbnails,
            sender_mii,
            stationery,
            colors,
            sheets,
            common: common.ok_or(LetterParsingError::MissingCommonInfoBlock)?,
            blocks: BlocksHashMap::new_from_bpk1_blocks(blocks)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LetterParsingError {
    MissingCommonInfoBlock,
}

impl Display for LetterParsingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use LetterParsingError::*;
        write!(
            f,
            "{}",
            match self {
                MissingCommonInfoBlock => "Missing COMMON1 block",
            }
        )
    }
}

impl Error for LetterParsingError {}

#[cfg(test)]
pub mod tests {
    use std::ffi::CStr;
    use std::ffi::CString;
    use std::fs::read;
    use std::fs::write;
    use std::str::FromStr;

    use chrono::{DateTime, Utc};

    use crate::lzss;

    use super::*;

    #[test]
    fn test_seri_deseri() {
        // using read instead of include_bytes so it fails at runtime if the test case isn't present instead of not compiling
        let letter =
            dbg!(Letter::new_from_bpk1_bytes(&read("test_cases/letter.bpk").unwrap()).unwrap());
        let mii = letter.sender_mii.unwrap();
        println!("Mii: {:#?}", mii);
        let datetime: DateTime<Utc> = mii.mii_creation_date.into();
        println!("Creation date: {} UTC", datetime.format("%d/%m/%Y %T"));
        println!("{}", mii.get_mii_studio_url());
        println!("{:#?}", letter.sheets);

        let mut blocks: Vec<BPK1Block> = vec![];
        for (name, block_part) in letter.blocks {
            for block in block_part {
                blocks.push(BPK1Block { name: CString::new(name.clone()).unwrap(), data: block });
            }
        }

        let out = Letter::bytes_from_bpk1_blocks(blocks).unwrap();
        write("test_cases/output.bpk", &out);
    }
}
