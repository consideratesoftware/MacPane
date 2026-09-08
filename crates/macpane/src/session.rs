//! Owns the TCP connection and the reader thread. The UI talks to it
//! through `Session`: a shared framebuffer, a shared writer, and an event
//! channel.

use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use macpane_rfb::{
    connect, lan, ClientWriter, ConnectOptions, Credentials, Error, Framebuffer, Rect, ServerEvent,
    ServerInfo, DEFAULT_PORT,
};

pub type SharedWriter = Arc<Mutex<ClientWriter<TcpStream>>>;

/// What the reader thread reports to the UI.
#[derive(Debug)]
pub enum SessionEvent {
    Server(ServerEvent),
    Disconnected(String),
}

pub struct Session {
    pub info: ServerInfo,
    pub framebuffer: Arc<Mutex<Framebuffer>>,
    pub writer: SharedWriter,
    pub events: Receiver<SessionEvent>,
    pub peer: SocketAddr,
}

/// Resolve `host[:port]`, refuse anything off the local network, and run
/// the handshake. Blocking; call from a worker thread.
pub fn open(
    host: &str,
    creds: &Credentials,
    repaint: impl Fn() + Send + 'static,
) -> Result<Session, String> {
    let target = if host.contains(':') && !host.starts_with('[') && host.matches(':').count() == 1 {
        host.to_string()
    } else {
        format!("{host}:{DEFAULT_PORT}")
    };
    let mut candidates: Vec<SocketAddr> = target
        .to_socket_addrs()
        .map_err(|e| format!("could not resolve {host}: {e}"))?
        .collect();
    if candidates.is_empty() {
        return Err(format!("{host} did not resolve to any address"));
    }
    // Prefer IPv4, then filter to local-only.
    candidates.sort_by_key(|a| !a.is_ipv4());
    let peer = match candidates.iter().copied().find(|a| lan::is_local(a.ip())) {
        Some(a) => a,
        None => return Err(Error::NotLocal(candidates[0].ip()).to_string()),
    };

    let stream = TcpStream::connect_timeout(&peer, Duration::from_secs(5))
        .map_err(|e| format!("could not connect to {peer}: {e}"))?;
    stream.set_nodelay(true).ok();
    let write_half = stream.try_clone().map_err(|e| e.to_string())?;

    let (mut reader, mut writer, info) =
        connect(stream, write_half, creds, &ConnectOptions::default())
            .map_err(|e| e.to_string())?;

    let framebuffer = Arc::new(Mutex::new(Framebuffer::new(info.width, info.height)));
    writer
        .request_update(
            false,
            Rect {
                x: 0,
                y: 0,
                width: info.width,
                height: info.height,
            },
        )
        .map_err(|e| e.to_string())?;
    let writer: SharedWriter = Arc::new(Mutex::new(writer));

    let (tx, rx) = mpsc::channel();
    {
        let fb = Arc::clone(&framebuffer);
        let writer = Arc::clone(&writer);
        thread::Builder::new()
            .name("rfb-reader".into())
            .spawn(move || loop {
                let result = {
                    let mut fb = fb.lock().unwrap();
                    reader.read_event(&mut fb)
                };
                match result {
                    Ok(ev) => {
                        let (w, h) = {
                            let fb = fb.lock().unwrap();
                            (fb.width() as u16, fb.height() as u16)
                        };
                        let is_update = matches!(
                            ev,
                            ServerEvent::FramebufferUpdated { .. }
                                | ServerEvent::DesktopResized { .. }
                        );
                        if tx.send(SessionEvent::Server(ev)).is_err() {
                            break;
                        }
                        repaint();
                        if is_update {
                            // Keep the pipeline full: ask for the next delta immediately.
                            let mut w_lock = writer.lock().unwrap();
                            if w_lock
                                .request_update(
                                    true,
                                    Rect {
                                        x: 0,
                                        y: 0,
                                        width: w,
                                        height: h,
                                    },
                                )
                                .is_err()
                            {
                                let _ = tx.send(SessionEvent::Disconnected("write failed".into()));
                                repaint();
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(SessionEvent::Disconnected(e.to_string()));
                        repaint();
                        break;
                    }
                }
            })
            .map_err(|e| e.to_string())?;
    }

    Ok(Session {
        info,
        framebuffer,
        writer,
        events: rx,
        peer,
    })
}
