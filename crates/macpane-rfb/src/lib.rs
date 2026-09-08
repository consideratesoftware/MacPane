//! `macpane-rfb` is a pure-Rust client implementation of the RFB protocol
//! (RFC 6143) with the extensions needed to talk to macOS Screen Sharing:
//!
//! * RFB 3.8 handshake (Apple advertises `RFB 003.889`, which we treat as 3.8).
//! * Security types `None`, `VncAuth` (DES challenge) and
//!   `AppleRemoteDesktop` (type 30: Diffie-Hellman + MD5 + AES-128-ECB).
//! * `Raw` and `CopyRect` encodings today; `ZRLE` and `Hextile` are planned.
//!
//! The crate is transport-agnostic and synchronous: hand it anything that
//! implements [`std::io::Read`] and [`std::io::Write`]. The GUI runs the
//! reader on its own thread; see `crates/macpane`.

pub mod client;
pub mod encodings;
pub mod error;
pub mod framebuffer;
pub mod io;
pub mod keysym;
pub mod lan;
pub mod messages;
pub mod pixel_format;
pub mod security;

pub use client::{connect, ClientWriter, ConnectOptions, Credentials, ServerInfo, ServerReader};
pub use error::{Error, Result};
pub use framebuffer::Framebuffer;
pub use messages::{Encoding, Rect, ServerEvent};
pub use pixel_format::PixelFormat;

/// Default TCP port for RFB / macOS Screen Sharing.
pub const DEFAULT_PORT: u16 = 5900;
