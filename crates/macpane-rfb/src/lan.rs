//! MacPane's one hard security promise: it only ever connects to hosts on
//! the local network. This module is the gate.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// Returns true for addresses that can only be reached over a local link or
/// a private network: RFC 1918, link-local, loopback, unique-local IPv6,
/// and IPv4-mapped forms of those.
pub fn is_local(addr: IpAddr) -> bool {
    match addr {
        IpAddr::V4(v4) => is_local_v4(v4),
        IpAddr::V6(v6) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_local_v4(v4);
            }
            is_local_v6(v6)
        }
    }
}

fn is_local_v4(a: Ipv4Addr) -> bool {
    a.is_private() || a.is_link_local() || a.is_loopback()
}

fn is_local_v6(a: Ipv6Addr) -> bool {
    if a.is_loopback() {
        return true;
    }
    let seg0 = a.segments()[0];
    // fe80::/10 link-local, fc00::/7 unique-local.
    (seg0 & 0xffc0) == 0xfe80 || (seg0 & 0xfe00) == 0xfc00
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn private_v4_ranges_are_local() {
        for s in [
            "10.0.0.1",
            "172.16.0.1",
            "172.31.255.254",
            "192.168.1.20",
            "169.254.9.9",
            "127.0.0.1",
        ] {
            assert!(is_local(ip(s)), "{s}");
        }
    }

    #[test]
    fn public_v4_is_not_local() {
        for s in [
            "8.8.8.8",
            "1.1.1.1",
            "172.32.0.1",
            "192.169.0.1",
            "100.64.0.1",
        ] {
            assert!(!is_local(ip(s)), "{s}");
        }
    }

    #[test]
    fn v6_cases() {
        assert!(is_local(ip("::1")));
        assert!(is_local(ip("fe80::1")));
        assert!(is_local(ip("fd12:3456::1")));
        assert!(is_local(ip("::ffff:192.168.0.2")));
        assert!(!is_local(ip("2001:4860:4860::8888")));
        assert!(!is_local(ip("::ffff:8.8.8.8")));
    }
}
