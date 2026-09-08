//! X11 keysym values used by the RFB KeyEvent message, plus the subset of
//! mappings MacPane needs. The GUI layer decides which physical Windows key
//! maps to which keysym (for example Win -> Super so macOS sees Command).

pub const BACKSPACE: u32 = 0xff08;
pub const TAB: u32 = 0xff09;
pub const RETURN: u32 = 0xff0d;
pub const ESCAPE: u32 = 0xff1b;
pub const INSERT: u32 = 0xff63;
pub const DELETE: u32 = 0xffff;
pub const HOME: u32 = 0xff50;
pub const END: u32 = 0xff57;
pub const PAGE_UP: u32 = 0xff55;
pub const PAGE_DOWN: u32 = 0xff56;
pub const LEFT: u32 = 0xff51;
pub const UP: u32 = 0xff52;
pub const RIGHT: u32 = 0xff53;
pub const DOWN: u32 = 0xff54;
pub const F1: u32 = 0xffbe;
pub const SHIFT_L: u32 = 0xffe1;
pub const SHIFT_R: u32 = 0xffe2;
pub const CONTROL_L: u32 = 0xffe3;
pub const CONTROL_R: u32 = 0xffe4;
pub const META_L: u32 = 0xffe7;
pub const META_R: u32 = 0xffe8;
pub const ALT_L: u32 = 0xffe9;
pub const ALT_R: u32 = 0xffea;
pub const SUPER_L: u32 = 0xffeb;
pub const SUPER_R: u32 = 0xffec;
pub const CAPS_LOCK: u32 = 0xffe5;
pub const SPACE: u32 = 0x0020;

/// Function key N (1-based) keysym.
pub fn function_key(n: u8) -> u32 {
    F1 + (n.saturating_sub(1)) as u32
}

/// Keysym for a printable Unicode character. Latin-1 maps directly; other
/// code points use the 0x01000000 + codepoint convention.
pub fn from_char(c: char) -> u32 {
    let cp = c as u32;
    if (0x20..=0x7e).contains(&cp) || (0xa0..=0xff).contains(&cp) {
        cp
    } else {
        0x0100_0000 + cp
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_maps_directly() {
        assert_eq!(from_char('a'), 0x61);
        assert_eq!(from_char(' '), SPACE);
    }

    #[test]
    fn unicode_uses_offset() {
        assert_eq!(from_char('\u{20ac}'), 0x0100_20ac);
    }

    #[test]
    fn function_keys() {
        assert_eq!(function_key(1), 0xffbe);
        assert_eq!(function_key(12), 0xffc9);
    }
}
