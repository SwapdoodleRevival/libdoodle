use std::io::{Cursor, Read, Seek};

pub use nintendo_lz::CompressionLevel;

use crate::error::GenericResult;

pub fn compress_from_slice(slice: &[u8], level: CompressionLevel) -> GenericResult<Vec<u8>> {
    let mut out: Vec<u8> = vec![];
    nintendo_lz::compress(&slice, &mut out, level)?;
    Ok(out)
}

pub fn compress_lz10_from_slice(slice: &[u8]) -> GenericResult<Vec<u8>> {
    compress_from_slice(slice, CompressionLevel::LZ10)
}

pub fn compress_lz11_from_slice(slice: &[u8], max_repeat_size: u32) -> GenericResult<Vec<u8>> {
    compress_from_slice(slice, CompressionLevel::LZ11(max_repeat_size))
}

pub fn decompress_from_reader<R: Read + Seek>(reader: &mut R) -> GenericResult<Vec<u8>> {
    nintendo_lz::decompress(reader)
}

pub fn decompress_from_slice(slice: &[u8]) -> GenericResult<Vec<u8>> {
    let mut cursor = Cursor::new(slice);
    decompress_from_reader(&mut cursor)
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_decompress() {
        assert_eq!(
            decompress_from_slice(&[
                0x10, 0x14, 0x00, 0x00, 0x08, 0x61, 0x62, 0x63, 0x64, 0xD0, 0x03,
            ])
            .unwrap(),
            b"abcdabcdabcdabcdabcd"
        );
    }
}
