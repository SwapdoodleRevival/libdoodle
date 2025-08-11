use std::{
    ffi::CString,
    io::{self, BufWriter, Seek, Write},
    u32,
};

/// An extension for [std::io::Write]
pub trait WriteExt: Write {
    /// Read a little endian u32
    fn write_u32_le(&mut self, value: u32) -> io::Result<()> {
        self.write(&u32::to_le_bytes(value))?;
        Ok(())
    }

    fn write_zeroes(&mut self, N: usize) -> io::Result<()> {
        for _ in 0..N {
            self.write(&[0; 0x1]);
        }
        Ok(())
    }
}

impl<T: Write> WriteExt for T {}