//! Client-to-server and server-to-client message definitions
//! (RFC 6143 sections 7.5 and 7.6).

/// Encodings, as the i32 values sent in SetEncodings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum Encoding {
    Raw = 0,
    CopyRect = 1,
    Rre = 2,
    Hextile = 5,
    Zlib = 6,
    Tight = 7,
    Zrle = 16,
    // Pseudo-encodings.
    Cursor = -239,
    DesktopSize = -223,
    ContinuousUpdates = -313,
}

impl Encoding {
    pub fn from_i32(v: i32) -> Option<Self> {
        Some(match v {
            0 => Encoding::Raw,
            1 => Encoding::CopyRect,
            2 => Encoding::Rre,
            5 => Encoding::Hextile,
            6 => Encoding::Zlib,
            7 => Encoding::Tight,
            16 => Encoding::Zrle,
            -239 => Encoding::Cursor,
            -223 => Encoding::DesktopSize,
            -313 => Encoding::ContinuousUpdates,
            _ => return None,
        })
    }
}

/// Client-to-server message type bytes.
pub mod client_msg {
    pub const SET_PIXEL_FORMAT: u8 = 0;
    pub const SET_ENCODINGS: u8 = 2;
    pub const FRAMEBUFFER_UPDATE_REQUEST: u8 = 3;
    pub const KEY_EVENT: u8 = 4;
    pub const POINTER_EVENT: u8 = 5;
    pub const CLIENT_CUT_TEXT: u8 = 6;
}

/// Server-to-client message type bytes.
pub mod server_msg {
    pub const FRAMEBUFFER_UPDATE: u8 = 0;
    pub const SET_COLOUR_MAP_ENTRIES: u8 = 1;
    pub const BELL: u8 = 2;
    pub const SERVER_CUT_TEXT: u8 = 3;
}

/// A rectangle in framebuffer coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Something the server told us that the UI may care about. Framebuffer
/// pixel changes are applied directly to the shared [`crate::Framebuffer`];
/// the event only reports the damaged rectangles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServerEvent {
    /// One FramebufferUpdate message was fully applied. `damaged` lists the
    /// rectangles that changed so the UI can upload only those.
    FramebufferUpdated {
        damaged: Vec<Rect>,
    },
    /// The server changed its desktop size (DesktopSize pseudo-encoding).
    DesktopResized {
        width: u16,
        height: u16,
    },
    Bell,
    /// Server clipboard changed (Latin-1 text).
    CutText(String),
}

/// Pointer button mask bits for PointerEvent.
pub mod buttons {
    pub const LEFT: u8 = 1 << 0;
    pub const MIDDLE: u8 = 1 << 1;
    pub const RIGHT: u8 = 1 << 2;
    pub const SCROLL_UP: u8 = 1 << 3;
    pub const SCROLL_DOWN: u8 = 1 << 4;
    pub const SCROLL_LEFT: u8 = 1 << 5;
    pub const SCROLL_RIGHT: u8 = 1 << 6;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_round_trip() {
        for e in [
            Encoding::Raw,
            Encoding::CopyRect,
            Encoding::Zrle,
            Encoding::Cursor,
            Encoding::DesktopSize,
            Encoding::ContinuousUpdates,
        ] {
            assert_eq!(Encoding::from_i32(e as i32), Some(e));
        }
        assert_eq!(Encoding::from_i32(12345), None);
    }
}
