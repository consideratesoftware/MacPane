//! Raw encoding (0): width*height pixels in the negotiated pixel format.

use std::io::Read;

use crate::error::Result;
use crate::framebuffer::Framebuffer;
use crate::io::ReadExt;
use crate::messages::Rect;
use crate::pixel_format::PixelFormat;

pub fn decode<R: Read>(
    r: &mut R,
    pf: &PixelFormat,
    rect: Rect,
    fb: &mut Framebuffer,
) -> Result<()> {
    let bpp = pf.bytes_per_pixel();
    let n = rect.width as usize * rect.height as usize;
    let data = r.read_bytes(n * bpp)?;
    if *pf == PixelFormat::RGBA32 {
        fb.blit_rgba(rect, &data);
    } else {
        let mut rgba = Vec::with_capacity(n * 4);
        for px in data.chunks_exact(bpp) {
            rgba.extend_from_slice(&pf.pixel_to_rgba(px));
        }
        fb.blit_rgba(rect, &rgba);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_path_copies_bytes_verbatim() {
        let mut fb = Framebuffer::new(2, 2);
        let rect = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 1,
        };
        let data = [1, 2, 3, 4, 5, 6, 7, 8];
        decode(&mut data.as_slice(), &PixelFormat::RGBA32, rect, &mut fb).unwrap();
        assert_eq!(fb.pixel(0, 0), [1, 2, 3, 4]);
        assert_eq!(fb.pixel(1, 0), [5, 6, 7, 8]);
    }

    #[test]
    fn slow_path_converts_format() {
        let pf = PixelFormat {
            big_endian: true,
            red_shift: 16,
            green_shift: 8,
            blue_shift: 0,
            ..PixelFormat::RGBA32
        };
        let mut fb = Framebuffer::new(1, 1);
        let rect = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        decode(&mut [0x00, 0xaa, 0xbb, 0xcc].as_slice(), &pf, rect, &mut fb).unwrap();
        assert_eq!(fb.pixel(0, 0), [0xaa, 0xbb, 0xcc, 0xff]);
    }

    #[test]
    fn short_read_is_an_error() {
        let mut fb = Framebuffer::new(2, 2);
        let rect = Rect {
            x: 0,
            y: 0,
            width: 2,
            height: 2,
        };
        assert!(decode(
            &mut [0u8; 3].as_slice(),
            &PixelFormat::RGBA32,
            rect,
            &mut fb
        )
        .is_err());
    }
}
