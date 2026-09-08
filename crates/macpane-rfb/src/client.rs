//! Handshake and the two halves of a live session: [`ServerReader`] and
//! [`ClientWriter`].

use std::io::{Read, Write};

use crate::encodings;
use crate::error::{Error, Result, SecurityTypeName};
use crate::framebuffer::Framebuffer;
use crate::io::{to_latin1, ReadExt, WriteExt};
use crate::messages::{client_msg, server_msg, Encoding, Rect, ServerEvent};
use crate::pixel_format::PixelFormat;
use crate::security::{self, SecurityType};

/// Credentials offered to the server. Which ones are needed depends on the
/// security types the Mac advertises; see [`security::choose`].
#[derive(Debug, Clone, Default)]
pub struct Credentials {
    pub username: Option<String>,
    pub password: Option<String>,
}

/// Facts learned during the handshake.
#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub width: u16,
    pub height: u16,
    pub name: String,
    pub server_version: String,
    pub security_type: SecurityType,
    /// The pixel format the server initially offered (informational; the
    /// session immediately switches to [`PixelFormat::RGBA32`]).
    pub native_pixel_format: PixelFormat,
}

/// Options for [`connect`].
#[derive(Debug, Clone)]
pub struct ConnectOptions {
    /// Allow other clients to stay connected (ClientInit shared flag).
    pub shared: bool,
    pub encodings: Vec<Encoding>,
}

impl Default for ConnectOptions {
    fn default() -> Self {
        Self {
            shared: true,
            encodings: encodings::SUPPORTED.to_vec(),
        }
    }
}

/// Perform the full RFB handshake over separate read and write halves of a
/// connection (use `TcpStream::try_clone` to get two handles). On success
/// the pixel format has been set to RGBA32, encodings have been sent, and
/// the connection is ready for `request_update`.
pub fn connect<R: Read, W: Write>(
    mut reader: R,
    mut writer: W,
    creds: &Credentials,
    options: &ConnectOptions,
) -> Result<(ServerReader<R>, ClientWriter<W>, ServerInfo)> {
    // --- ProtocolVersion -------------------------------------------------
    let version_bytes = reader.read_bytes(12)?;
    let server_version = String::from_utf8_lossy(&version_bytes)
        .trim_end()
        .to_string();
    let (major, minor) = parse_version(&version_bytes)
        .ok_or_else(|| Error::UnsupportedVersion(server_version.clone()))?;
    if major != 3 || minor < 7 {
        return Err(Error::UnsupportedVersion(server_version));
    }
    // macOS reports 3.889; anything >= 3.8 is treated as 3.8.
    let negotiated_minor = if minor >= 8 { 8 } else { 7 };
    writer.write_all(format!("RFB 003.00{negotiated_minor}\n").as_bytes())?;
    writer.flush()?;

    // --- Security handshake ----------------------------------------------
    let count = reader.read_u8()?;
    if count == 0 {
        let reason = reader.read_string_u32()?;
        return Err(Error::ConnectionRefused(reason));
    }
    let offered = reader.read_bytes(count as usize)?;
    let has_password = creds.password.as_deref().is_some_and(|p| !p.is_empty());
    let has_username = creds.username.as_deref().is_some_and(|u| !u.is_empty());
    let chosen = security::choose(&offered, has_password, has_username)
        .ok_or_else(|| Error::NoCommonSecurityType(offered.clone()))?;
    writer.write_u8(chosen as u8)?;
    writer.flush()?;

    match chosen {
        SecurityType::None => {}
        SecurityType::VncAuth => {
            let password = creds
                .password
                .as_deref()
                .filter(|p| !p.is_empty())
                .ok_or(Error::PasswordRequired(SecurityTypeName(chosen as u8)))?;
            let mut challenge = [0u8; 16];
            reader.read_exact(&mut challenge)?;
            let response = security::vnc_auth::respond(&challenge, password);
            writer.write_all(&response)?;
            writer.flush()?;
        }
        SecurityType::AppleRemoteDesktop => {
            let password = creds
                .password
                .as_deref()
                .filter(|p| !p.is_empty())
                .ok_or(Error::PasswordRequired(SecurityTypeName(chosen as u8)))?;
            let username = creds
                .username
                .as_deref()
                .filter(|u| !u.is_empty())
                .ok_or(Error::UsernameRequired)?;
            let mut head = [0u8; 4];
            reader.read_exact(&mut head)?;
            let key_len = u16::from_be_bytes([head[2], head[3]]) as usize;
            let mut rest = reader.read_bytes(2 * key_len)?;
            let mut all = head.to_vec();
            all.append(&mut rest);
            let params = security::ard::ServerParams::parse(&all)?;
            let resp = security::ard::respond(&params, username, password)?;
            writer.write_all(&resp.ciphertext)?;
            writer.write_all(&resp.client_public)?;
            writer.flush()?;
        }
    }

    // SecurityResult: always present in 3.8; absent for None in 3.7.
    if !(chosen == SecurityType::None && negotiated_minor == 7) {
        let result = reader.read_u32()?;
        if result != 0 {
            let reason = if negotiated_minor >= 8 {
                reader
                    .read_string_u32()
                    .unwrap_or_else(|_| "unknown reason".into())
            } else {
                "unknown reason".into()
            };
            return Err(Error::AuthFailed(reason));
        }
    }

    // --- ClientInit / ServerInit -----------------------------------------
    writer.write_u8(options.shared as u8)?;
    writer.flush()?;

    let width = reader.read_u16()?;
    let height = reader.read_u16()?;
    let native_pixel_format = PixelFormat::read_from(&mut reader)?;
    let name = reader.read_string_u32()?;

    let info = ServerInfo {
        width,
        height,
        name,
        server_version,
        security_type: chosen,
        native_pixel_format,
    };

    let mut client = ClientWriter { w: writer };
    client.set_pixel_format(&PixelFormat::RGBA32)?;
    client.set_encodings(&options.encodings)?;

    let server = ServerReader {
        r: reader,
        pf: PixelFormat::RGBA32,
    };
    Ok((server, client, info))
}

/// Parse `RFB xxx.yyy\n`.
fn parse_version(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() != 12 || &bytes[0..4] != b"RFB " || bytes[7] != b'.' || bytes[11] != b'\n' {
        return None;
    }
    let major = std::str::from_utf8(&bytes[4..7]).ok()?.parse().ok()?;
    let minor = std::str::from_utf8(&bytes[8..11]).ok()?.parse().ok()?;
    Some((major, minor))
}

/// The receiving half of a session. Call [`ServerReader::read_event`] in a
/// loop on a dedicated thread.
pub struct ServerReader<R: Read> {
    r: R,
    pf: PixelFormat,
}

impl<R: Read> ServerReader<R> {
    /// Block until one server message arrives, apply it to `fb`, and return
    /// a summary. Pixel data goes straight into `fb`.
    pub fn read_event(&mut self, fb: &mut Framebuffer) -> Result<ServerEvent> {
        let msg_type = self.r.read_u8()?;
        match msg_type {
            server_msg::FRAMEBUFFER_UPDATE => self.read_framebuffer_update(fb),
            server_msg::SET_COLOUR_MAP_ENTRIES => {
                self.r.skip(1)?;
                let _first = self.r.read_u16()?;
                let n = self.r.read_u16()? as usize;
                self.r.skip(n * 6)?;
                // We never negotiate a colour-mapped format, so nothing to do.
                Ok(ServerEvent::FramebufferUpdated {
                    damaged: Vec::new(),
                })
            }
            server_msg::BELL => Ok(ServerEvent::Bell),
            server_msg::SERVER_CUT_TEXT => {
                self.r.skip(3)?;
                let text = self.r.read_string_u32()?;
                Ok(ServerEvent::CutText(text))
            }
            other => Err(Error::UnknownMessage(other)),
        }
    }

    fn read_framebuffer_update(&mut self, fb: &mut Framebuffer) -> Result<ServerEvent> {
        self.r.skip(1)?;
        let count = self.r.read_u16()?;
        let mut damaged = Vec::with_capacity(count as usize);
        let mut resized = None;

        for _ in 0..count {
            let rect = Rect {
                x: self.r.read_u16()?,
                y: self.r.read_u16()?,
                width: self.r.read_u16()?,
                height: self.r.read_u16()?,
            };
            let enc_raw = self.r.read_i32()?;
            let enc = Encoding::from_i32(enc_raw).ok_or(Error::UnexpectedEncoding(enc_raw))?;

            match enc {
                Encoding::DesktopSize => {
                    fb.resize(rect.width, rect.height);
                    resized = Some((rect.width, rect.height));
                    continue;
                }
                Encoding::Cursor => {
                    // Cursor shape: w*h pixels + bitmask rows. Ignored for now.
                    let bytes =
                        rect.width as usize * rect.height as usize * self.pf.bytes_per_pixel();
                    let mask = (rect.width as usize).div_ceil(8) * rect.height as usize;
                    self.r.skip(bytes + mask)?;
                    continue;
                }
                _ => {}
            }

            if !fb.contains(rect) {
                return Err(Error::Protocol(format!(
                    "rectangle {rect:?} outside {}x{} framebuffer",
                    fb.width(),
                    fb.height()
                )));
            }

            match enc {
                Encoding::Raw => encodings::raw::decode(&mut self.r, &self.pf, rect, fb)?,
                Encoding::CopyRect => encodings::copyrect::decode(&mut self.r, rect, fb)?,
                other => return Err(Error::UnexpectedEncoding(other as i32)),
            }
            damaged.push(rect);
        }

        if let Some((width, height)) = resized {
            if damaged.is_empty() {
                return Ok(ServerEvent::DesktopResized { width, height });
            }
        }
        Ok(ServerEvent::FramebufferUpdated { damaged })
    }
}

/// The sending half of a session. Cheap to wrap in a `Mutex` and share
/// between the UI thread and the reader thread.
pub struct ClientWriter<W: Write> {
    w: W,
}

impl<W: Write> ClientWriter<W> {
    pub fn set_pixel_format(&mut self, pf: &PixelFormat) -> Result<()> {
        self.w.write_u8(client_msg::SET_PIXEL_FORMAT)?;
        self.w.write_all(&[0, 0, 0])?;
        pf.write_to(&mut self.w)?;
        self.w.flush()?;
        Ok(())
    }

    pub fn set_encodings(&mut self, encodings: &[Encoding]) -> Result<()> {
        self.w.write_u8(client_msg::SET_ENCODINGS)?;
        self.w.write_u8(0)?;
        self.w.write_u16(encodings.len() as u16)?;
        for e in encodings {
            self.w.write_i32(*e as i32)?;
        }
        self.w.flush()?;
        Ok(())
    }

    /// Ask for the given region. `incremental = true` means "only what
    /// changed since last time", which is what a viewer wants in steady state.
    pub fn request_update(&mut self, incremental: bool, rect: Rect) -> Result<()> {
        self.w.write_u8(client_msg::FRAMEBUFFER_UPDATE_REQUEST)?;
        self.w.write_u8(incremental as u8)?;
        self.w.write_u16(rect.x)?;
        self.w.write_u16(rect.y)?;
        self.w.write_u16(rect.width)?;
        self.w.write_u16(rect.height)?;
        self.w.flush()?;
        Ok(())
    }

    pub fn key_event(&mut self, keysym: u32, down: bool) -> Result<()> {
        self.w.write_u8(client_msg::KEY_EVENT)?;
        self.w.write_u8(down as u8)?;
        self.w.write_all(&[0, 0])?;
        self.w.write_u32(keysym)?;
        self.w.flush()?;
        Ok(())
    }

    pub fn pointer_event(&mut self, button_mask: u8, x: u16, y: u16) -> Result<()> {
        self.w.write_u8(client_msg::POINTER_EVENT)?;
        self.w.write_u8(button_mask)?;
        self.w.write_u16(x)?;
        self.w.write_u16(y)?;
        self.w.flush()?;
        Ok(())
    }

    pub fn cut_text(&mut self, text: &str) -> Result<()> {
        let bytes = to_latin1(text);
        self.w.write_u8(client_msg::CLIENT_CUT_TEXT)?;
        self.w.write_all(&[0, 0, 0])?;
        self.w.write_u32(bytes.len() as u32)?;
        self.w.write_all(&bytes)?;
        self.w.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_versions() {
        assert_eq!(parse_version(b"RFB 003.008\n"), Some((3, 8)));
        assert_eq!(parse_version(b"RFB 003.889\n"), Some((3, 889)));
        assert_eq!(parse_version(b"RFB 003.003\n"), Some((3, 3)));
        assert_eq!(parse_version(b"HTTP/1.1 200"), None);
    }

    /// Build the bytes a server would send for a full 3.8 handshake with
    /// security type None and a 2x1 desktop, then one raw update.
    fn scripted_server(minor: &str, security: &[u8], with_result: bool) -> Vec<u8> {
        let mut s = Vec::new();
        s.extend_from_slice(format!("RFB 003.{minor}\n").as_bytes());
        s.push(security.len() as u8);
        s.extend_from_slice(security);
        if with_result {
            s.extend_from_slice(&0u32.to_be_bytes());
        }
        // ServerInit
        s.extend_from_slice(&2u16.to_be_bytes());
        s.extend_from_slice(&1u16.to_be_bytes());
        let mut pf = Vec::new();
        PixelFormat::RGBA32.write_to(&mut pf).unwrap();
        s.extend_from_slice(&pf);
        let name = b"Sunny's Mac";
        s.extend_from_slice(&(name.len() as u32).to_be_bytes());
        s.extend_from_slice(name);
        s
    }

    #[test]
    fn handshake_none_38() {
        let server = scripted_server("008", &[1], true);
        let mut sent = Vec::new();
        let (_r, _w, info) = connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .unwrap();
        assert_eq!(info.width, 2);
        assert_eq!(info.height, 1);
        assert_eq!(info.name, "Sunny's Mac");
        assert_eq!(info.security_type, SecurityType::None);
        assert_eq!(info.server_version, "RFB 003.008");
        // Client should have written: version, security choice, ClientInit,
        // SetPixelFormat (20 bytes), SetEncodings (4 + 4*n).
        assert!(sent.starts_with(b"RFB 003.008\n\x01\x01"));
        let n = encodings::SUPPORTED.len();
        assert_eq!(sent.len(), 12 + 1 + 1 + 20 + 4 + 4 * n);
    }

    #[test]
    fn apple_version_is_treated_as_38() {
        let server = scripted_server("889", &[1], true);
        let mut sent = Vec::new();
        let (_r, _w, info) = connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .unwrap();
        assert_eq!(info.server_version, "RFB 003.889");
        assert!(sent.starts_with(b"RFB 003.008\n"));
    }

    #[test]
    fn handshake_none_37_has_no_security_result() {
        let server = scripted_server("007", &[1], false);
        let mut sent = Vec::new();
        connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .unwrap();
        assert!(sent.starts_with(b"RFB 003.007\n"));
    }

    #[test]
    fn rejects_33() {
        let server = b"RFB 003.003\n".to_vec();
        let mut sent = Vec::new();
        let err = connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, Error::UnsupportedVersion(_)));
    }

    #[test]
    fn reports_refusal_reason() {
        let mut server = b"RFB 003.008\n\x00".to_vec();
        let reason = b"Too many security failures";
        server.extend_from_slice(&(reason.len() as u32).to_be_bytes());
        server.extend_from_slice(reason);
        let mut sent = Vec::new();
        let err = connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, Error::ConnectionRefused(r) if r == "Too many security failures"));
    }

    #[test]
    fn vnc_auth_requires_password() {
        let server = scripted_server("008", &[2], true);
        let mut sent = Vec::new();
        let err = connect(
            server.as_slice(),
            &mut sent,
            &Credentials::default(),
            &ConnectOptions::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, Error::PasswordRequired(_)));
    }

    #[test]
    fn ard_requires_username() {
        let server = scripted_server("008", &[30], true);
        let mut sent = Vec::new();
        let creds = Credentials {
            username: None,
            password: Some("pw".into()),
        };
        let err = connect(
            server.as_slice(),
            &mut sent,
            &creds,
            &ConnectOptions::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, Error::UsernameRequired));
    }

    #[test]
    fn vnc_auth_failure_reports_reason() {
        let mut server = b"RFB 003.008\n\x01\x02".to_vec();
        server.extend_from_slice(&[0u8; 16]); // challenge
        server.extend_from_slice(&1u32.to_be_bytes()); // failed
        let reason = b"Authentication failure";
        server.extend_from_slice(&(reason.len() as u32).to_be_bytes());
        server.extend_from_slice(reason);
        let mut sent = Vec::new();
        let creds = Credentials {
            username: None,
            password: Some("pw".into()),
        };
        let err = connect(
            server.as_slice(),
            &mut sent,
            &creds,
            &ConnectOptions::default(),
        )
        .err()
        .unwrap();
        assert!(matches!(err, Error::AuthFailed(r) if r == "Authentication failure"));
        // 12 version + 1 choice + 16 response
        assert_eq!(sent.len(), 12 + 1 + 16);
    }

    #[test]
    fn reads_raw_update_and_reports_damage() {
        let mut fb = Framebuffer::new(2, 1);
        let mut msg = vec![server_msg::FRAMEBUFFER_UPDATE, 0];
        msg.extend_from_slice(&1u16.to_be_bytes());
        for v in [0u16, 0, 2, 1] {
            msg.extend_from_slice(&v.to_be_bytes());
        }
        msg.extend_from_slice(&0i32.to_be_bytes());
        msg.extend_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
        let mut reader = ServerReader {
            r: msg.as_slice(),
            pf: PixelFormat::RGBA32,
        };
        let ev = reader.read_event(&mut fb).unwrap();
        assert_eq!(
            ev,
            ServerEvent::FramebufferUpdated {
                damaged: vec![Rect {
                    x: 0,
                    y: 0,
                    width: 2,
                    height: 1
                }]
            }
        );
        assert_eq!(fb.pixel(1, 0), [5, 6, 7, 8]);
    }

    #[test]
    fn desktop_size_resizes() {
        let mut fb = Framebuffer::new(2, 1);
        let mut msg = vec![server_msg::FRAMEBUFFER_UPDATE, 0];
        msg.extend_from_slice(&1u16.to_be_bytes());
        for v in [0u16, 0, 10, 20] {
            msg.extend_from_slice(&v.to_be_bytes());
        }
        msg.extend_from_slice(&(-223i32).to_be_bytes());
        let mut reader = ServerReader {
            r: msg.as_slice(),
            pf: PixelFormat::RGBA32,
        };
        let ev = reader.read_event(&mut fb).unwrap();
        assert_eq!(
            ev,
            ServerEvent::DesktopResized {
                width: 10,
                height: 20
            }
        );
        assert_eq!(fb.width(), 10);
        assert_eq!(fb.height(), 20);
    }

    #[test]
    fn out_of_bounds_rect_is_rejected_not_panicked() {
        let mut fb = Framebuffer::new(2, 1);
        let mut msg = vec![server_msg::FRAMEBUFFER_UPDATE, 0];
        msg.extend_from_slice(&1u16.to_be_bytes());
        for v in [1u16, 0, 2, 1] {
            msg.extend_from_slice(&v.to_be_bytes());
        }
        msg.extend_from_slice(&0i32.to_be_bytes());
        msg.extend_from_slice(&[0u8; 8]);
        let mut reader = ServerReader {
            r: msg.as_slice(),
            pf: PixelFormat::RGBA32,
        };
        assert!(matches!(
            reader.read_event(&mut fb),
            Err(Error::Protocol(_))
        ));
    }

    #[test]
    fn cut_text_and_bell() {
        let mut fb = Framebuffer::new(1, 1);
        let mut msg = vec![server_msg::BELL, server_msg::SERVER_CUT_TEXT, 0, 0, 0];
        msg.extend_from_slice(&2u32.to_be_bytes());
        msg.extend_from_slice(b"hi");
        let mut reader = ServerReader {
            r: msg.as_slice(),
            pf: PixelFormat::RGBA32,
        };
        assert_eq!(reader.read_event(&mut fb).unwrap(), ServerEvent::Bell);
        assert_eq!(
            reader.read_event(&mut fb).unwrap(),
            ServerEvent::CutText("hi".into())
        );
    }

    #[test]
    fn writer_message_layouts() {
        let mut buf = Vec::new();
        let mut w = ClientWriter { w: &mut buf };
        w.request_update(
            true,
            Rect {
                x: 1,
                y: 2,
                width: 3,
                height: 4,
            },
        )
        .unwrap();
        w.key_event(0x61, true).unwrap();
        w.pointer_event(1, 10, 20).unwrap();
        w.cut_text("ab").unwrap();
        let expected: Vec<u8> = vec![
            3, 1, 0, 1, 0, 2, 0, 3, 0, 4, // update request
            4, 1, 0, 0, 0, 0, 0, 0x61, // key event
            5, 1, 0, 10, 0, 20, // pointer
            6, 0, 0, 0, 0, 0, 0, 2, b'a', b'b', // cut text
        ];
        assert_eq!(buf, expected);
    }
}
