use std::error::Error;
use std::fmt::Display;

use serde::Serialize;

use crate::blocks::{colslt1::Colors, miistd1::MiiData, sheet1::Sheet, common1::CommonInfo};
use crate::bpk1::{BPK1Block, BPK1Blocks, BPK1File};
use crate::error::GenericResult;
use crate::files::stationery::Stationery;
use crate::read::ReadExt;

#[derive(Debug, Serialize)]
pub struct Letter {
    pub thumbnails: Vec<Vec<u8>>,
    pub sender_mii: Option<MiiData>,
    pub stationery: Option<Stationery>,
    pub sheets: Vec<Sheet>,
    pub colors: Option<Colors>,
    pub blocks: BPK1Blocks,
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
            match block.name.as_str() {
                "THUMB2" => {
                    thumbnails.push(block.data.to_owned());
                }
                "MIISTD1" => {
                    let mut slice: &[u8] = &block.data;
                    sender_mii = Some(MiiData::from_bytes(slice.read_const_num_of_bytes()?)?)
                }
                "COLSLT1" => {
                    colors = Some(Colors::from_bytes(&block.data)?);
                }
                "STATIN1" => stationery = Some(Stationery::new_from_bpk1_bytes(&block.data)?),
                "SHEET1" => {
                    sheets.push(Sheet::from_bytes(&block.data).unwrap());
                }
                "COMMON1" => common = Some(CommonInfo::from_bytes(&block.data)?),
                _ => {}
            }
        }

        Ok(Letter {
            thumbnails,
            sender_mii,
            stationery,
            colors,
            sheets,
            blocks: BPK1Blocks::new_from_bpk1_blocks(blocks)?,
            common: common.ok_or(LetterParsingError::MissingCommonInfoBlock)?,
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
