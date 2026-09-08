# Architecture

MacPane is three crates in one Cargo workspace. Dependencies point one way: the app depends on the libraries, the libraries depend on nothing in the app.

```
crates/macpane            egui app: screens, input mapping, session thread
    |            \
    v             v
crates/macpane-rfb   crates/macpane-discovery
(protocol, crypto,   (Bonjour browse)
 framebuffer)
```

## `macpane-rfb`: the protocol core

Synchronous and transport-agnostic. Everything takes `impl Read` / `impl Write`, so unit tests script a fake server with a `Vec<u8>` and the GUI hands it a `TcpStream`.

| Module | Responsibility |
| --- | --- |
| `client` | `connect()` runs the full handshake and returns `(ServerReader, ClientWriter, ServerInfo)`. Reader and writer are separate so they can live on different threads. |
| `security` | Chooses a security type from what the server offers. `vnc_auth` is the DES challenge. `ard` is Apple's DH + MD5 + AES-128-ECB scheme (type 30). |
| `pixel_format` | The 16-byte PIXEL_FORMAT struct. MacPane always asks the server for `RGBA32` so raw pixels are already in the layout the framebuffer and GPU texture use. A slow conversion path exists for servers that ignore SetPixelFormat. |
| `encodings` | One module per encoding. Each reads exactly its bytes and writes into the `Framebuffer`. Currently `raw` and `copyrect`; `zrle` and `hextile` are the next two (see PLAN). |
| `framebuffer` | Client-side RGBA8 copy of the screen. Bounds checking lives here and in `client`; a malicious server cannot make us index out of range. |
| `messages` | Message type constants, `Encoding`, `Rect`, `ServerEvent`. |
| `keysym` | X11 keysym constants and the char-to-keysym rule. |
| `lan` | `is_local(IpAddr)`: the LAN-only gate. Private, link-local, loopback, ULA, and IPv4-mapped forms. |
| `io` | Big-endian helpers and Latin-1 codecs. |

### Threading model in the app

```
UI thread                          rfb-reader thread
---------                          -----------------
egui frame:                        loop {
  drain SessionEvents                fb.lock(); reader.read_event(&mut fb)
  if dirty: upload fb -> texture     send SessionEvent
  translate input -> writer.lock()   request_update(incremental=true)
}                                  }
```

- `Framebuffer` is behind `Arc<Mutex<_>>`. The reader holds the lock only while decoding one message; the UI holds it only while copying to a texture.
- `ClientWriter` is behind `Arc<Mutex<_>>`. Both threads write short messages; contention is negligible.
- The reader requests the next incremental update as soon as it finishes applying one, so the server always has a request outstanding. This is the standard VNC "keep one request in flight" pattern and is what makes updates feel continuous.
- Reconnect logic will sit above `session::open` (roadmap).

## `macpane-discovery`

Wraps `mdns-sd`. Browses `_rfb._tcp.local.`, resolves each service to name, hostname, addresses and port, and exposes a non-blocking `poll()` for the UI to drain each frame. DNS-SD escape sequences in instance names are decoded so "Sunny\032s\032Mac" shows as "Sunny's Mac".

## `macpane`: the app

- `session.rs`: resolves the host, applies the LAN gate, runs the handshake, spawns the reader thread. Returns a `Session` handle owning the framebuffer, writer and event channel.
- `input.rs`: pure functions from egui input to keysyms and button masks. The Mac-shaped mapping lives here so it is unit-testable.
- `ui.rs`: two screens. `Connect` shows discovered Macs and a form; `Viewer` uploads the framebuffer to a texture, letterboxes it, and forwards input.

The GUI is egui because a VNC viewer is a textured quad plus input, and egui gives us that cross-platform with DPI handling for free. Native Windows chrome (tray icon, jump lists, an installer) is additive and tracked in the plan. See [adr/0001-rust-and-egui.md](adr/0001-rust-and-egui.md).

## Security posture

1. **No outbound connections except to the chosen Mac.** No update checks, no telemetry, no relay.
2. **The LAN gate is in the library**, not the UI, so every caller (GUI, probe example, future CLI) gets it.
3. **Every rectangle from the server is bounds-checked** before it touches the framebuffer. Decoders return `Error::Protocol` rather than panicking.
4. **No `unsafe`.** If we ever need it (e.g. a native Windows API), it goes in one clearly named module with a comment explaining the invariant.
5. Credentials are held in memory for the session only. Saved connections (roadmap) will use Windows Credential Manager, never a plaintext file.

## Testing strategy

- Protocol: scripted byte streams in unit tests, covering the happy path, refusals, auth failures, and malformed input. Aim for every `Error` variant to have a test that produces it.
- Crypto: textbook DH values and round-trip decryption; see `security::ard::tests`.
- Input mapping: table tests.
- Live: the `probe` example against a real Mac. There is no Mac in CI, so live verification is a manual checklist in [MAC-COMPAT-CHECKLIST.md](MAC-COMPAT-CHECKLIST.md).
