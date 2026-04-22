use std::{
    borrow::Borrow,
    error::Error,
    ffi::CString,
    fmt::Display,
    io::{BufRead, Cursor, Seek, SeekFrom, Write},
};

use serde::{Deserialize, Serialize};

#[cfg(feature = "tsify")]
use tsify::Tsify;

#[cfg(feature = "tsify")]
use crate::cstring;

use crate::{
    error::GenericResult,
    lzss::decompress_from_slice,
    read::{BufReadSeekExt, ReadExt},
    write::WriteExt,
};

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "tsify", derive(Tsify), tsify(into_wasm_abi, from_wasm_abi))]
pub struct BPK1Block {
    #[cfg_attr(feature = "tsify", serde(with = "cstring"))]
    pub name: CString,
    #[cfg_attr(
        feature = "tsify",
        tsify(type = "Uint8Array"),
        serde(with = "serde_bytes")
    )]
    pub data: Vec<u8>,
}

/** The custom CRC32 algorithm used in BPK1. */
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
        let block_name_len = reader.read_u32_le()? as usize;
        reader.seek_relative(0x34)?;

        struct BlockHeader {
            offset: u32,
            size: u32,
            checksum: u32,
            name: CString,
        }

        let mut blocks = Vec::with_capacity(num_blocks as usize);

        for _ in 0..num_blocks {
            blocks.push(BlockHeader {
                offset: reader.read_u32_le().unwrap(),
                size: reader.read_u32_le().unwrap(),
                checksum: reader.read_u32_le().unwrap(),
                name: reader.read_null_padded_cstring(block_name_len).unwrap(),
            });
            reader.seek_relative_to_nearest_multiple(0x4).unwrap();
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

                Ok(BPK1Block { name, data })
            })
            .collect::<GenericResult<Vec<BPK1Block>>>()?; // Collect into a Result<Vec> from an Iterator<Item = Result> to short circuit

        Self::new_from_bpk1_blocks(blocks)
    }

    fn bytes_from_bpk1_blocks<P: Borrow<BPK1Block>>(blocks: &[P]) -> GenericResult<Vec<u8>> {
        let mut result = Vec::<u8>::new();
        let mut writer = Cursor::new(&mut result);

        let max_name_len = blocks
            .iter()
            .map(|block| block.borrow().name.count_bytes())
            .max()
            .unwrap_or(0);

        writer.write_all(b"BPK1")?;
        writer.write_u32_le(blocks.len() as u32)?;
        writer.write_u32_le(max_name_len as u32)?;

        let file_size_pos = writer.position();
        writer.write_u32_le(0)?; // file size
        writer.write_u32_le(0)?; // header size

        writer.write_all(&[0; 0x2c])?; // padding

        let mut positions = vec![0; blocks.len()];

        for (index, block) in blocks.iter().enumerate() {
            positions[index] = writer.position();
            writer.write_u32_le(0)?; // will be offset
            writer.write_u32_le(block.borrow().data.len() as u32)?;
            writer.write_u32_le(calc_bpk1_checksum(&block.borrow().data))?;
            writer.write_all(&cstring_to_bpk1_bytes(&block.borrow().name, max_name_len))?;
            writer.seek_relative_to_nearest_multiple(0x4)?;
        }

        writer.seek_relative_to_nearest_multiple(0x10)?;

        let data_start_pos = writer.position();

        for (index, block) in blocks.iter().enumerate() {
            let start_position = writer.position();
            writer.write_all(&block.borrow().data)?;
            writer.seek_relative_to_nearest_multiple(0x4)?;
            let end_position = writer.position();
            writer.set_position(positions[index]);
            writer.write_u32_le(start_position as u32)?;
            writer.set_position(end_position);
        }

        let file_size = writer.position();
        writer.set_position(file_size_pos);
        writer.write_u32_le(file_size as u32)?;
        writer.write_u32_le(data_start_pos as u32)?;

        Ok(result)
    }

    fn new_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Self>;
}

fn cstring_to_bpk1_bytes(string: &CString, length: usize) -> Vec<u8> {
    let mut bytes: Vec<u8> = vec![0; length];
    _ = bytes.as_mut_slice().write(string.to_bytes());
    bytes
}

trait CursorTrait: BufRead + Seek {}
impl<T: AsRef<[u8]>> CursorTrait for Cursor<T> {}

pub type BPK1Blocks = Vec<BPK1Block>;

impl BPK1File for BPK1Blocks {
    fn new_from_bpk1_blocks(blocks: Vec<BPK1Block>) -> GenericResult<Self> {
        Ok(blocks)
    }
}

#[cfg(test)]
pub mod tests {

    use std::fs::read;
    use std::fs::write;

    use crate::lzss;

    use super::*;

    #[test]
    fn test_seri_deseri() {
        // using read instead of include_bytes so it fails at runtime if the test case isn't present instead of not compiling

        let file = &read("test_cases/letter.bpk").unwrap();

        let decompressed = lzss::decompress_from_slice(file).unwrap();
        write(
            "test_cases/test-seri-deseri-decompressed.bpk",
            &decompressed,
        )
        .unwrap();

        let blocks = BPK1Blocks::new_from_bpk1_bytes(&decompressed).unwrap();
        let rebuilt = BPK1Blocks::bytes_from_bpk1_blocks(&blocks).unwrap();
        write("test_cases/test-seri-deseri-rebuilt.bpk", &rebuilt).unwrap();

        if decompressed != rebuilt {
            panic!("files do not match");
        }
    }
}
