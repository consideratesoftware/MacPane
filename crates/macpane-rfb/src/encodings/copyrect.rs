//! CopyRect encoding (1): the server asks us to copy an existing region.

use std::io::Read;

use crate::error::{Error, Result};
use crate::framebuffer::Framebuffer;
use crate::io::ReadExt;
use crate::messages::Rect;

pub fn decode<R: Read>(r: &mut R, rect: Rect, fb: &mut Framebuffer) -> Result<()> {
    let src_x = r.read_u16()?;
    let src_y = r.read_u16()?;
    let src = Rect {
        x: src_x,
        y: src_y,
        width: rect.width,
        height: rect.height,
    };
    if !fb.contains(src) {
        return Err(Error::Protocol(format!(
            "CopyRect source {src:?} out of bounds"
        )));
    }
    fb.copy_rect(src_x, src_y, rect);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copies_region() {
        let mut fb = Framebuffer::new(2, 1);
        fb.blit_rgba(
            Rect {
                x: 0,
                y: 0,
                width: 1,
                height: 1,
            },
            &[9, 9, 9, 9],
        );
        let dst = Rect {
            x: 1,
            y: 0,
            width: 1,
            height: 1,
        };
        decode(&mut [0, 0, 0, 0].as_slice(), dst, &mut fb).unwrap();
        assert_eq!(fb.pixel(1, 0), [9, 9, 9, 9]);
    }

    #[test]
    fn rejects_out_of_bounds_source() {
        let mut fb = Framebuffer::new(2, 1);
        let dst = Rect {
            x: 0,
            y: 0,
            width: 1,
            height: 1,
        };
        assert!(decode(&mut [0, 5, 0, 0].as_slice(), dst, &mut fb).is_err());
    }
}
