use std::{
    ffi::CString,
    io::{self, BufWriter, Seek, Write},
    u32,
};

/// An extension for [std::io::Write]
pub trait WriteExt: Write {
    /// Read a little endian u32
    fn write_u32_le(&mut self, value: u32) -> io::Result<usize> {
        self.write(&u32::to_le_bytes(value))
    }

    fn write_zeroes(&mut self, n: usize) -> io::Result<usize> {
        self.write(&vec![0; n])
    }
}

impl<T: Write> WriteExt for T {}
