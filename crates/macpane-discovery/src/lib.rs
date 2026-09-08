//! Finds Macs advertising Screen Sharing over Bonjour (`_rfb._tcp.local.`).
//!
//! macOS registers the service with the computer name as the instance name,
//! so the UI can show "Sunny's MacBook Pro" instead of an IP address.

use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::mpsc::{self, Receiver};
use std::thread;

use mdns_sd::{ServiceDaemon, ServiceEvent};

pub const SERVICE_TYPE: &str = "_rfb._tcp.local.";

/// A host seen on the network.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Host {
    /// Human-friendly name, e.g. "Sunny's Mac mini".
    pub name: String,
    /// mDNS hostname, e.g. "sunnys-mac-mini.local.".
    pub hostname: String,
    pub addresses: Vec<IpAddr>,
    pub port: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiscoveryEvent {
    Found(Host),
    Lost { name: String },
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("mDNS daemon error: {0}")]
    Mdns(#[from] mdns_sd::Error),
}

/// Handle to a running browse. Dropping it stops discovery.
pub struct Discovery {
    daemon: ServiceDaemon,
    events: Receiver<DiscoveryEvent>,
}

impl Discovery {
    pub fn start() -> Result<Self, Error> {
        let daemon = ServiceDaemon::new()?;
        let receiver = daemon.browse(SERVICE_TYPE)?;
        let (tx, rx) = mpsc::channel();

        thread::Builder::new()
            .name("mdns-browse".into())
            .spawn(move || {
                while let Ok(event) = receiver.recv() {
                    let out = match event {
                        ServiceEvent::ServiceResolved(info) => {
                            let mut addresses: BTreeSet<IpAddr> =
                                info.get_addresses().iter().copied().collect();
                            // Prefer IPv4 first in the UI; keep both.
                            let addresses: Vec<IpAddr> = {
                                let v4: Vec<_> =
                                    addresses.iter().copied().filter(|a| a.is_ipv4()).collect();
                                let v6: Vec<_> =
                                    addresses.iter().copied().filter(|a| a.is_ipv6()).collect();
                                addresses.clear();
                                v4.into_iter().chain(v6).collect()
                            };
                            Some(DiscoveryEvent::Found(Host {
                                name: instance_name(info.get_fullname()),
                                hostname: info.get_hostname().to_string(),
                                addresses,
                                port: info.get_port(),
                            }))
                        }
                        ServiceEvent::ServiceRemoved(_, fullname) => Some(DiscoveryEvent::Lost {
                            name: instance_name(&fullname),
                        }),
                        _ => None,
                    };
                    if let Some(ev) = out {
                        if tx.send(ev).is_err() {
                            break;
                        }
                    }
                }
            })
            .expect("spawn mdns thread");

        Ok(Self { daemon, events: rx })
    }

    /// Non-blocking drain of pending events.
    pub fn poll(&self) -> Vec<DiscoveryEvent> {
        self.events.try_iter().collect()
    }
}

impl Drop for Discovery {
    fn drop(&mut self) {
        let _ = self.daemon.stop_browse(SERVICE_TYPE);
        let _ = self.daemon.shutdown();
    }
}

/// "Sunny's Mac._rfb._tcp.local." -> "Sunny's Mac", with mDNS escapes undone.
fn instance_name(fullname: &str) -> String {
    let raw = fullname.strip_suffix(SERVICE_TYPE).unwrap_or(fullname);
    let raw = raw.strip_suffix('.').unwrap_or(raw);
    unescape(raw)
}

/// Undo DNS-SD escaping: `\032` -> space, `\.` -> `.`, `\\` -> `\`.
fn unescape(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            if i + 3 < bytes.len() && bytes[i + 1..i + 4].iter().all(u8::is_ascii_digit) {
                let code: u32 = std::str::from_utf8(&bytes[i + 1..i + 4])
                    .unwrap()
                    .parse()
                    .unwrap();
                out.push(code as u8);
                i += 4;
                continue;
            }
            out.push(bytes[i + 1]);
            i += 2;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_service_suffix() {
        assert_eq!(instance_name("Sunny's Mac._rfb._tcp.local."), "Sunny's Mac");
    }

    #[test]
    fn unescapes_dns_sd() {
        assert_eq!(unescape("Sunny\\039s\\032Mac"), "Sunny's Mac");
        assert_eq!(unescape("a\\.b"), "a.b");
        assert_eq!(unescape("a\\\\b"), "a\\b");
        assert_eq!(unescape("plain"), "plain");
        assert_eq!(unescape("trailing\\"), "trailing\\");
    }
}
