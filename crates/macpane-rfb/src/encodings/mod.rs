//! Framebuffer rectangle decoders. Each decoder reads exactly its own bytes
//! from the stream and writes into the shared [`Framebuffer`].

pub mod copyrect;
pub mod raw;

use crate::messages::Encoding;

/// Encodings the client advertises, in order of preference. The server
/// picks the first one it supports for each rectangle. Pseudo-encodings go
/// last.
pub const SUPPORTED: &[Encoding] = &[
    Encoding::CopyRect,
    Encoding::Raw,
    Encoding::DesktopSize,
    Encoding::Cursor,
];
