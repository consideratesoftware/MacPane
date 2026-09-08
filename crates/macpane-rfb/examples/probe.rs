//! Headless smoke test against a real server. Connects, prints what the
//! server said, pulls one full frame, and optionally writes it as a PPM.
//!
//! ```text
//! cargo run -p macpane-rfb --example probe -- 192.168.1.50 [--user NAME] [--pass PW] [--out frame.ppm]
//! ```

use std::io::Write;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use macpane_rfb::{connect, lan, Credentials, Framebuffer, Rect, ServerEvent, DEFAULT_PORT};

fn main() {
    let mut args = std::env::args().skip(1);
    let host = args.next().unwrap_or_else(|| {
        eprintln!("usage: probe HOST[:PORT] [--user NAME] [--pass PW] [--out frame.ppm]");
        std::process::exit(2);
    });
    let mut creds = Credentials::default();
    let mut out: Option<String> = None;
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--user" => creds.username = args.next(),
            "--pass" => creds.password = args.next(),
            "--out" => out = args.next(),
            other => {
                eprintln!("unknown flag {other}");
                std::process::exit(2);
            }
        }
    }

    let target = if host.contains(':') {
        host.clone()
    } else {
        format!("{host}:{DEFAULT_PORT}")
    };
    let addr = target
        .to_socket_addrs()
        .expect("resolve")
        .find(|a| lan::is_local(a.ip()))
        .unwrap_or_else(|| {
            eprintln!("{target} did not resolve to a local-network address; refusing");
            std::process::exit(1);
        });

    let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).expect("tcp connect");
    stream.set_nodelay(true).ok();
    let writer = stream.try_clone().expect("clone");

    let (mut reader, writer, info) = connect(stream, writer, &creds, &Default::default())
        .unwrap_or_else(|e| {
            eprintln!("handshake failed: {e}");
            std::process::exit(1);
        });

    println!("server:   {}", info.name);
    println!("version:  {}", info.server_version);
    println!("security: {:?}", info.security_type);
    println!("size:     {}x{}", info.width, info.height);
    println!("native:   {:?}", info.native_pixel_format);

    let writer = Arc::new(Mutex::new(writer));
    let mut fb = Framebuffer::new(info.width, info.height);
    writer
        .lock()
        .unwrap()
        .request_update(
            false,
            Rect {
                x: 0,
                y: 0,
                width: info.width,
                height: info.height,
            },
        )
        .expect("request");

    match reader.read_event(&mut fb) {
        Ok(ServerEvent::FramebufferUpdated { damaged }) => {
            println!("received {} rectangle(s)", damaged.len());
        }
        Ok(other) => println!("first message: {other:?}"),
        Err(e) => {
            eprintln!("read failed: {e}");
            std::process::exit(1);
        }
    }

    if let Some(path) = out {
        let mut f = std::fs::File::create(&path).expect("create");
        write!(f, "P6\n{} {}\n255\n", fb.width(), fb.height()).unwrap();
        let mut rgb = Vec::with_capacity(fb.width() * fb.height() * 3);
        for px in fb.pixels().chunks_exact(4) {
            rgb.extend_from_slice(&px[..3]);
        }
        f.write_all(&rgb).unwrap();
        println!("wrote {path}");
    }
}
