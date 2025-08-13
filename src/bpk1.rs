use std::{
    error::Error,
    fmt::Display,
    io::{BufRead, Cursor, Read, Seek, SeekFrom, Write},
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "tsify")]
use tsify::Tsify;

use crate::{
    error::GenericResult,
    lzss::decompress_from_slice,
    read::{BufReadSeekExt, ReadExt},
    write::WriteExt,
};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi, from_wasm_abi))]
pub struct BPK1Block {
    pub name: String,
    #[cfg_attr(feature = "tsify", tsify(type = "Uint8Array"), serde(with = "serde_bytes"))]
    pub data: Vec<u8>,
}

const BPK1_CRC32_ALG: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::Algorithm {
    width: 32,
    poly: 0x04c11db7,
    init: 0x04c11db7,
    refin: false,
    refout: false,
    xorout: 0x0,
    check: 0x0,
    residue: 0x0,
});

fn has_bpk1_magic(reader: &[u8]) -> bool {
    reader.get(0..4).is_some_and(|magic| magic == *b"BPK1")
}

#[derive(Debug, Clone, Copy)]
pub enum BPK1Error {
    BadMagic,
    ChecksumMismatched,
}

impl Display for BPK1Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BPK1Error::*;
        match self {
            BadMagic => write!(f, "Bad BPK1 magic"),
            ChecksumMismatched => write!(f, "Incorrect CRC32 checksum"),
        }
    }
}

impl Error for BPK1Error {}

pub fn calc_bpk1_checksum(data: &[u8]) -> u32 {
    let mut digest = BPK1_CRC32_ALG.digest();
    digest.update(data);
    digest.finalize()
}

pub trait BPK1File
where
    Self: Sized,
{
    fn new_from_bpk1_bytes(data: &[u8]) -> GenericResult<Self> {
        let mut reader: Box<dyn CursorTrait> = if has_bpk1_magic(data) {
            Box::new(Cursor::new(data))
        } else {
            let decompressed = decompress_from_slice(data)?;
            if !has_bpk1_magic(&decompressed) {
                Err(BPK1Error::BadMagic)?;
            }
            Box::new(Cursor::new(decompressed))
        };

        reader.seek_relative(4)?;
        let num_blocks = reader.read_u32_le()?;
        reader.seek_relative(0x38)?;

        struct BlockHeader {
            offset: u32,
            size: u32,
            checksum: u32,
            name: String,
        }

        let mut blocks = Vec::with_capacity(num_blocks as usize);

        for _ in 0..num_blocks {
            blocks.push(BlockHeader {
                offset: reader.read_u32_le()?,
                size: reader.read_u32_le()?,
                checksum: reader.read_u32_le()?,
                name: reader.read_null_padded_string(8)?,
            })
        }

        // Turn the headers into contentful blocks
        // Doing this *after* reading the headers since this involves seeking
        let blocks = blocks
            .into_iter()
            .map(|head| {
                let BlockHeader {
                    offset,
                    size,
                    checksum,
                    name,
                } = head;

                reader.seek(SeekFrom::Start(offset as u64))?;

                let data = reader.read_num_of_bytes(size as usize)?;

                if checksum != calc_bpk1_checksum(&data) {
                    Err(BPK1Error::ChecksumMismatched)?;
                }

                Ok(BPK1Block { name, data: data })
            })
            .collect::<GenericResult<Vec<BPK1Block>>>()?; // Collect into a Result<Vec> from an Iterator<Item = Result> to short circuit

        Self::new_from_bpk1_blocks(blocks)
    }

    fn bytes_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Vec<u8>> {
        let mut result = Vec::<u8>::new();
        let mut writer = Cursor::new(&mut result);

        writer.write(b"BPK1")?;
        writer.write_u32_le(blocks.len() as u32)?;
        writer.write_u32_le(7)?;

        let file_size_pos = writer.position();
        writer.write_u32_le(0)?;

        let header_size_pos = writer.position();
        writer.write_u32_le(0)?;

        writer.write(&[0; 0x2c])?; // padding

        let header_start_pos = writer.position();

        const BLOCK_HEADER_SIZE: u8 = 0x4 + 0x4 + 0x4 + 0x8;

        for block in &blocks {
            writer.write_u32_le(0)?; // will be offset
            writer.write_u32_le(block.data.len() as u32)?;
            writer.write_u32_le(calc_bpk1_checksum(&block.data))?;
            writer.write(&string_to_bpk1_bytes(&block.name))?;
        }

        let header_size = writer.position() - header_start_pos;

        writer.write_zeroes(0x10 - (header_size % 0x10) as usize)?; // padding

        let data_start_pos = writer.position();
        writer.set_position(header_size_pos);
        writer.write_u32_le((data_start_pos - header_start_pos) as u32)?;
        writer.set_position(data_start_pos);

        let mut index: u8 = 0;
        for block in &blocks {
            let start_position = writer.position();
            writer.write(&block.data)?;
            let end_position = writer.position();
            writer.set_position(header_start_pos + (index * BLOCK_HEADER_SIZE) as u64);
            writer.write_u32_le(start_position as u32)?;
            writer.set_position(end_position);
            index += 1;
        }

        let file_size = writer.position();
        writer.set_position(file_size_pos);
        writer.write_u32_le(file_size as u32)?;

        Ok(result)
    }

    fn new_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Self>;
}

fn string_to_bpk1_bytes(string: &String) -> [u8; 8] {
    let mut result = [0; 8];
    let mut index: usize = 0;
    for b in string.as_bytes() {
        result[index] = b.clone();
        index += 1;
        if index == 8 {
            break;
        }
    }
    result
}

trait CursorTrait: BufRead + Seek {}
impl<T: AsRef<[u8]>> CursorTrait for Cursor<T> {}

pub type BPK1Blocks = Vec<BPK1Block>;

impl BPK1File for BPK1Blocks {
    fn new_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Self> {
        Ok(blocks)
    }
}
