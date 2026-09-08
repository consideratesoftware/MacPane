//! The client-side copy of the remote screen, stored as tightly packed RGBA8.

use crate::messages::Rect;

#[derive(Debug, Clone)]
pub struct Framebuffer {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u16, height: u16) -> Self {
        let (w, h) = (width as usize, height as usize);
        Self {
            width: w,
            height: h,
            pixels: vec![0; w * h * 4],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    /// RGBA8 bytes, row-major, no padding.
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    /// Resize, discarding contents (DesktopSize pseudo-encoding).
    pub fn resize(&mut self, width: u16, height: u16) {
        *self = Self::new(width, height);
    }

    /// Returns true if `rect` lies fully inside the framebuffer.
    pub fn contains(&self, rect: Rect) -> bool {
        (rect.x as usize + rect.width as usize) <= self.width
            && (rect.y as usize + rect.height as usize) <= self.height
    }

    /// Copy `rect.width * rect.height` RGBA pixels from `src` into `rect`.
    /// `src` must be exactly `w*h*4` bytes. Callers must check `contains`.
    pub fn blit_rgba(&mut self, rect: Rect, src: &[u8]) {
        let w = rect.width as usize;
        let row_bytes = w * 4;
        debug_assert_eq!(src.len(), row_bytes * rect.height as usize);
        for row in 0..rect.height as usize {
            let dst_start = ((rect.y as usize + row) * self.width + rect.x as usize) * 4;
            let src_start = row * row_bytes;
            self.pixels[dst_start..dst_start + row_bytes]
                .copy_from_slice(&src[src_start..src_start + row_bytes]);
        }
    }

    /// CopyRect: move a rectangle from (src_x, src_y) to `dst`. Handles
    /// overlapping regions by copying through a temporary buffer.
    pub fn copy_rect(&mut self, src_x: u16, src_y: u16, dst: Rect) {
        let src = Rect {
            x: src_x,
            y: src_y,
            width: dst.width,
            height: dst.height,
        };
        let tmp = self.extract(src);
        self.blit_rgba(dst, &tmp);
    }

    /// Copy a rectangle out as a contiguous RGBA buffer.
    pub fn extract(&self, rect: Rect) -> Vec<u8> {
        let w = rect.width as usize;
        let row_bytes = w * 4;
        let mut out = Vec::with_capacity(row_bytes * rect.height as usize);
        for row in 0..rect.height as usize {
            let start = ((rect.y as usize + row) * self.width + rect.x as usize) * 4;
            out.extend_from_slice(&self.pixels[start..start + row_bytes]);
        }
        out
    }

    /// Fill a rectangle with one RGBA colour (used by RRE/Hextile/ZRLE solid tiles).
    pub fn fill(&mut self, rect: Rect, rgba: [u8; 4]) {
        for row in 0..rect.height as usize {
            let start = ((rect.y as usize + row) * self.width + rect.x as usize) * 4;
            for px in self.pixels[start..start + rect.width as usize * 4].chunks_exact_mut(4) {
                px.copy_from_slice(&rgba);
            }
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> [u8; 4] {
        let i = (y * self.width + x) * 4;
        [
            self.pixels[i],
            self.pixels[i + 1],
            self.pixels[i + 2],
            self.pixels[i + 3],
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blit_and_read_back() {
        let mut fb = Framebuffer::new(4, 4);
        let rect = Rect {
            x: 1,
            y: 1,
            width: 2,
            height: 2,
        };
        let src: Vec<u8> = (0..16).collect();
        fb.blit_rgba(rect, &src);
        assert_eq!(fb.pixel(1, 1), [0, 1, 2, 3]);
        assert_eq!(fb.pixel(2, 1), [4, 5, 6, 7]);
        assert_eq!(fb.pixel(1, 2), [8, 9, 10, 11]);
        assert_eq!(fb.pixel(2, 2), [12, 13, 14, 15]);
        assert_eq!(fb.pixel(0, 0), [0, 0, 0, 0]);
    }

    #[test]
    fn copy_rect_overlapping() {
        let mut fb = Framebuffer::new(4, 1);
        fb.blit_rgba(
            Rect {
                x: 0,
                y: 0,
                width: 4,
                height: 1,
            },
            &[1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4],
        );
        // Shift pixels 0..3 right by one.
        fb.copy_rect(
            0,
            0,
            Rect {
                x: 1,
                y: 0,
                width: 3,
                height: 1,
            },
        );
        assert_eq!(fb.pixel(1, 0), [1, 1, 1, 1]);
        assert_eq!(fb.pixel(2, 0), [2, 2, 2, 2]);
        assert_eq!(fb.pixel(3, 0), [3, 3, 3, 3]);
    }

    #[test]
    fn contains_rejects_out_of_bounds() {
        let fb = Framebuffer::new(10, 10);
        assert!(fb.contains(Rect {
            x: 0,
            y: 0,
            width: 10,
            height: 10
        }));
        assert!(!fb.contains(Rect {
            x: 1,
            y: 0,
            width: 10,
            height: 10
        }));
        assert!(!fb.contains(Rect {
            x: 0,
            y: 5,
            width: 1,
            height: 6
        }));
    }

    #[test]
    fn fill_sets_colour() {
        let mut fb = Framebuffer::new(3, 3);
        fb.fill(
            Rect {
                x: 1,
                y: 1,
                width: 2,
                height: 2,
            },
            [9, 8, 7, 255],
        );
        assert_eq!(fb.pixel(2, 2), [9, 8, 7, 255]);
        assert_eq!(fb.pixel(0, 0), [0, 0, 0, 0]);
    }
}
