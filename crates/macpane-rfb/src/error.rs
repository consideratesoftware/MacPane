use std::fmt;

/// Errors produced while talking to an RFB server.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("server sent an unsupported protocol version: {0:?}")]
    UnsupportedVersion(String),

    #[error("server refused the connection: {0}")]
    ConnectionRefused(String),

    #[error("no mutually supported security type (server offered {0:?})")]
    NoCommonSecurityType(Vec<u8>),

    #[error("authentication failed: {0}")]
    AuthFailed(String),

    #[error("password required for security type {0}")]
    PasswordRequired(SecurityTypeName),

    #[error("username required for Apple Remote Desktop authentication")]
    UsernameRequired,

    #[error("server sent an unknown message type {0}")]
    UnknownMessage(u8),

    #[error("server used an encoding we did not request: {0}")]
    UnexpectedEncoding(i32),

    #[error("malformed data from server: {0}")]
    Protocol(String),

    #[error("refusing to connect to non-local address {0}")]
    NotLocal(std::net::IpAddr),
}

pub type Result<T> = std::result::Result<T, Error>;

/// Human-friendly name of a security type, for error messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecurityTypeName(pub u8);

impl fmt::Display for SecurityTypeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            1 => write!(f, "None"),
            2 => write!(f, "VncAuth"),
            30 => write!(f, "AppleRemoteDesktop"),
            other => write!(f, "type {other}"),
        }
    }
}
