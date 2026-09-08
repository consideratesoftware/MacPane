//! RFB security types (RFC 6143 section 7.2) plus Apple's type 30.

pub mod ard;
pub mod vnc_auth;

/// Security type identifiers as sent on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SecurityType {
    /// No authentication.
    None = 1,
    /// Classic DES challenge-response ("VNC viewers may control screen with password").
    VncAuth = 2,
    /// Apple Remote Desktop: Diffie-Hellman + MD5 + AES-128-ECB, with a
    /// macOS username and password. Works without enabling the VNC
    /// password checkbox on the Mac.
    AppleRemoteDesktop = 30,
}

impl SecurityType {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            1 => Some(SecurityType::None),
            2 => Some(SecurityType::VncAuth),
            30 => Some(SecurityType::AppleRemoteDesktop),
            _ => None,
        }
    }
}

/// Pick the security type to use. Order of preference: no auth, then the
/// simple VNC password, then Apple's scheme, subject to what credentials
/// the caller supplied.
pub fn choose(offered: &[u8], has_password: bool, has_username: bool) -> Option<SecurityType> {
    let offers = |t: SecurityType| offered.contains(&(t as u8));
    if offers(SecurityType::None) {
        return Some(SecurityType::None);
    }
    if has_password && offers(SecurityType::VncAuth) {
        return Some(SecurityType::VncAuth);
    }
    if has_password && has_username && offers(SecurityType::AppleRemoteDesktop) {
        return Some(SecurityType::AppleRemoteDesktop);
    }
    // Fall back to whatever we can speak, so the caller can report which
    // credential is missing rather than "no common type".
    if offers(SecurityType::VncAuth) {
        return Some(SecurityType::VncAuth);
    }
    if offers(SecurityType::AppleRemoteDesktop) {
        return Some(SecurityType::AppleRemoteDesktop);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefers_none_when_offered() {
        assert_eq!(choose(&[30, 2, 1], true, true), Some(SecurityType::None));
    }

    #[test]
    fn prefers_vnc_auth_over_ard_when_password_given() {
        // macOS with the VNC password checkbox enabled offers 30, 33, 35, 2.
        assert_eq!(
            choose(&[30, 33, 35, 2], true, false),
            Some(SecurityType::VncAuth)
        );
    }

    #[test]
    fn uses_ard_when_only_ard_offered() {
        // macOS default: no VNC password, only Apple auth.
        assert_eq!(
            choose(&[30, 33, 35], true, true),
            Some(SecurityType::AppleRemoteDesktop)
        );
    }

    #[test]
    fn falls_back_so_caller_can_report_missing_credentials() {
        assert_eq!(
            choose(&[30], false, false),
            Some(SecurityType::AppleRemoteDesktop)
        );
        assert_eq!(choose(&[2], false, false), Some(SecurityType::VncAuth));
    }

    #[test]
    fn nothing_in_common() {
        assert_eq!(choose(&[33, 35, 16], true, true), None);
    }
}
