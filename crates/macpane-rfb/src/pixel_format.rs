use std::io::{Read, Write};

use crate::error::{Error, Result};
use crate::io::{ReadExt, WriteExt};

/// The 16-byte PIXEL_FORMAT structure from RFC 6143 section 7.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelFormat {
    pub bits_per_pixel: u8,
    pub depth: u8,
    pub big_endian: bool,
    pub true_colour: bool,
    pub red_max: u16,
    pub green_max: u16,
    pub blue_max: u16,
    pub red_shift: u8,
    pub green_shift: u8,
    pub blue_shift: u8,
}

impl PixelFormat {
    /// The format MacPane always negotiates: 32 bpp, little-endian, with
    /// red in the lowest byte. Raw pixel bytes then arrive as `R G B x`,
    /// which is exactly the RGBA layout the framebuffer and the GUI use, so
    /// no per-pixel conversion is needed on the hot path.
    pub const RGBA32: PixelFormat = PixelFormat {
        bits_per_pixel: 32,
        depth: 24,
        big_endian: false,
        true_colour: true,
        red_max: 255,
        green_max: 255,
        blue_max: 255,
        red_shift: 0,
        green_shift: 8,
        blue_shift: 16,
    };

    pub fn bytes_per_pixel(&self) -> usize {
        (self.bits_per_pixel as usize) / 8
    }

    pub fn read_from<R: Read>(r: &mut R) -> Result<Self> {
        let bits_per_pixel = r.read_u8()?;
        let depth = r.read_u8()?;
        let big_endian = r.read_u8()? != 0;
        let true_colour = r.read_u8()? != 0;
        let red_max = r.read_u16()?;
        let green_max = r.read_u16()?;
        let blue_max = r.read_u16()?;
        let red_shift = r.read_u8()?;
        let green_shift = r.read_u8()?;
        let blue_shift = r.read_u8()?;
        r.skip(3)?;
        if !matches!(bits_per_pixel, 8 | 16 | 32) {
            return Err(Error::Protocol(format!(
                "bad bits-per-pixel {bits_per_pixel}"
            )));
        }
        Ok(Self {
            bits_per_pixel,
            depth,
            big_endian,
            true_colour,
            red_max,
            green_max,
            blue_max,
            red_shift,
            green_shift,
            blue_shift,
        })
    }

    pub fn write_to<W: Write>(&self, w: &mut W) -> Result<()> {
        w.write_u8(self.bits_per_pixel)?;
        w.write_u8(self.depth)?;
        w.write_u8(self.big_endian as u8)?;
        w.write_u8(self.true_colour as u8)?;
        w.write_u16(self.red_max)?;
        w.write_u16(self.green_max)?;
        w.write_u16(self.blue_max)?;
        w.write_u8(self.red_shift)?;
        w.write_u8(self.green_shift)?;
        w.write_u8(self.blue_shift)?;
        w.write_all(&[0, 0, 0])?;
        Ok(())
    }

    /// Convert one pixel in this format into RGBA bytes. Only used for
    /// formats other than [`PixelFormat::RGBA32`], e.g. when a server
    /// ignores our SetPixelFormat. Slow path.
    pub fn pixel_to_rgba(&self, px: &[u8]) -> [u8; 4] {
        let raw: u32 = match (self.bits_per_pixel, self.big_endian) {
            (8, _) => px[0] as u32,
            (16, true) => u16::from_be_bytes([px[0], px[1]]) as u32,
            (16, false) => u16::from_le_bytes([px[0], px[1]]) as u32,
            (32, true) => u32::from_be_bytes([px[0], px[1], px[2], px[3]]),
            (32, false) => u32::from_le_bytes([px[0], px[1], px[2], px[3]]),
            _ => 0,
        };
        let scale = |v: u32, max: u16| -> u8 {
            if max == 0 {
                0
            } else {
                ((v * 255) / max as u32) as u8
            }
        };
        let r = scale((raw >> self.red_shift) & self.red_max as u32, self.red_max);
        let g = scale(
            (raw >> self.green_shift) & self.green_max as u32,
            self.green_max,
        );
        let b = scale(
            (raw >> self.blue_shift) & self.blue_max as u32,
            self.blue_max,
        );
        [r, g, b, 255]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let mut buf = Vec::new();
        PixelFormat::RGBA32.write_to(&mut buf).unwrap();
        assert_eq!(buf.len(), 16);
        let back = PixelFormat::read_from(&mut buf.as_slice()).unwrap();
        assert_eq!(back, PixelFormat::RGBA32);
    }

    #[test]
    fn rejects_garbage_bpp() {
        let mut buf = vec![7u8; 16];
        buf[0] = 12;
        assert!(PixelFormat::read_from(&mut buf.as_slice()).is_err());
    }

    #[test]
    fn converts_bgra_big_endian() {
        // A typical server default: 32bpp big-endian, r shift 16, g 8, b 0.
        let pf = PixelFormat {
            big_endian: true,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
            ..PixelFormat::RGBA32
        };
        assert_eq!(
            pf.pixel_to_rgba(&[0x00, 0x11, 0x22, 0x33]),
            [0x11, 0x22, 0x33, 0xff]
        );
    }

    #[test]
    fn converts_rgb565() {
        let pf = PixelFormat {
            bits_per_pixel: 16,
            depth: 16,
            big_endian: false,
            true_colour: true,
            red_max: 31,
            green_max: 63,
            blue_max: 31,
            red_shift: 11,
            green_shift: 5,
            blue_shift: 0,
        };
        // Pure red in RGB565 little-endian = 0xF800 -> bytes 00 F8.
        assert_eq!(pf.pixel_to_rgba(&[0x00, 0xF8]), [255, 0, 0, 255]);
    }
}
