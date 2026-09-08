//! Big-endian read/write helpers over any `Read`/`Write`. RFB is big-endian
//! throughout except where a pixel format says otherwise.

use std::io::{Read, Write};

use crate::error::Result;

pub trait ReadExt: Read {
    fn read_u8(&mut self) -> Result<u8> {
        let mut b = [0u8; 1];
        self.read_exact(&mut b)?;
        Ok(b[0])
    }

    fn read_u16(&mut self) -> Result<u16> {
        let mut b = [0u8; 2];
        self.read_exact(&mut b)?;
        Ok(u16::from_be_bytes(b))
    }

    fn read_i32(&mut self) -> Result<i32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b)?;
        Ok(i32::from_be_bytes(b))
    }

    fn read_u32(&mut self) -> Result<u32> {
        let mut b = [0u8; 4];
        self.read_exact(&mut b)?;
        Ok(u32::from_be_bytes(b))
    }

    fn read_bytes(&mut self, n: usize) -> Result<Vec<u8>> {
        let mut v = vec![0u8; n];
        self.read_exact(&mut v)?;
        Ok(v)
    }

    /// Reads a u32 length prefix followed by that many bytes, decoded as
    /// Latin-1 (RFB's default string encoding).
    fn read_string_u32(&mut self) -> Result<String> {
        let len = self.read_u32()? as usize;
        let bytes = self.read_bytes(len)?;
        Ok(latin1(&bytes))
    }

    fn skip(&mut self, n: usize) -> Result<()> {
        let mut sink = [0u8; 64];
        let mut remaining = n;
        while remaining > 0 {
            let take = remaining.min(sink.len());
            self.read_exact(&mut sink[..take])?;
            remaining -= take;
        }
        Ok(())
    }
}

impl<R: Read + ?Sized> ReadExt for R {}

pub trait WriteExt: Write {
    fn write_u8(&mut self, v: u8) -> Result<()> {
        self.write_all(&[v])?;
        Ok(())
    }

    fn write_u16(&mut self, v: u16) -> Result<()> {
        self.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn write_i32(&mut self, v: i32) -> Result<()> {
        self.write_all(&v.to_be_bytes())?;
        Ok(())
    }

    fn write_u32(&mut self, v: u32) -> Result<()> {
        self.write_all(&v.to_be_bytes())?;
        Ok(())
    }
}

impl<W: Write + ?Sized> WriteExt for W {}

/// Decode bytes as ISO-8859-1.
pub fn latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

/// Encode a string as ISO-8859-1, replacing unencodable chars with `?`.
pub fn to_latin1(s: &str) -> Vec<u8> {
    s.chars()
        .map(|c| if (c as u32) < 256 { c as u8 } else { b'?' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_big_endian() {
        let data = [0x01, 0x02, 0x03, 0x04, 0xff, 0xff, 0xff, 0xfe];
        let mut r = data.as_slice();
        assert_eq!(r.read_u16().unwrap(), 0x0102);
        assert_eq!(r.read_u16().unwrap(), 0x0304);
        assert_eq!(r.read_i32().unwrap(), -2);
    }

    #[test]
    fn writes_big_endian() {
        let mut w = Vec::new();
        w.write_u16(0xabcd).unwrap();
        w.write_i32(-223).unwrap();
        assert_eq!(w, vec![0xab, 0xcd, 0xff, 0xff, 0xff, 0x21]);
    }

    #[test]
    fn skip_handles_lengths_larger_than_buffer() {
        let data = vec![9u8; 200];
        let mut r = data.as_slice();
        r.skip(150).unwrap();
        assert_eq!(r.len(), 50);
    }

    #[test]
    fn latin1_round_trip() {
        assert_eq!(latin1(&to_latin1("caf\u{e9}")), "caf\u{e9}");
        assert_eq!(to_latin1("\u{1F600}"), b"?");
    }
}
