# MacPane

**A LAN-only viewer for macOS Screen Sharing, built for Windows.**

MacPane connects to the VNC server that is already built into every Mac. Nothing to install on the Mac, no account, no cloud relay, and it refuses to connect to anything outside your local network.

```
Windows PC  --(RFB over your LAN only)-->  Mac  (System Settings > Sharing > Screen Sharing)
```

## Why

Existing VNC viewers on Windows are either general-purpose and clunky, or cloud-first and want an account. MacPane is opinionated about one thing: looking at a Mac from a Windows machine on the same network, and doing that well.

- **Finds your Mac by name** via Bonjour, no IP addresses to remember.
- **Speaks Apple's native auth** (security type 30), so you can log in with your Mac username and password without enabling the legacy "VNC password" checkbox.
- **Mac-shaped keyboard**: Windows key is Command, Alt is Option.
- **LAN-only by construction**: the client will not open a socket to a public address, ever.
- **Single small `.exe`**, no runtime to install.

## Status

Pre-alpha. The protocol core and a minimal viewer exist; see [docs/PLAN.md](docs/PLAN.md) for the roadmap and the [project board](https://github.com/orgs/consideratesoftware/projects) for live status.

## Setting up the Mac

1. System Settings > General > Sharing > turn on **Screen Sharing**.
2. Click the (i) > "Allow access for" > Only these users > add yourself.
3. Optional: Computer Settings > "VNC viewers may control screen with password". Only needed if you want to log in without a Mac username.
4. Under Battery/Energy: enable **Wake for network access** and prevent sleep while plugged in. A sleeping Mac is the most common cause of dropped sessions.
5. Do **not** forward port 5900 on your router. MacPane cannot reach it from outside anyway, and nothing else should either.

More detail, including a `pf` firewall rule that pins the Mac's port 5900 to your subnet, is in [docs/APPLE-SCREEN-SHARING.md](docs/APPLE-SCREEN-SHARING.md).

## Building

Requires a stable Rust toolchain and, on Windows, the MSVC build tools (C++ workload).

```bash
cargo build --release
```

The binary is `target/release/macpane.exe`.

```bash
cargo test --workspace
```

To sanity-check the protocol against a real Mac without the GUI:

```bash
cargo run -p macpane-rfb --example probe -- 192.168.1.50 --user sunny --pass '...' --out frame.ppm
```

## Layout

| Crate | Purpose |
| --- | --- |
| `crates/macpane-rfb` | Pure-Rust RFB client: handshake, VNC and Apple auth, encodings, framebuffer. No GUI dependencies. |
| `crates/macpane-discovery` | Bonjour browse for `_rfb._tcp`. |
| `crates/macpane` | The Windows app (egui/eframe). |

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for how the pieces fit together.

## Contributing

Issues and PRs welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) first. This repo is set up for AI-assisted development; see [docs/AGENTS.md](docs/AGENTS.md).

## License

MIT. See [LICENSE](LICENSE).

MacPane implements the RFB protocol from RFC 6143 and the publicly documented Apple Remote Desktop authentication exchange. "Mac", "macOS" and "Apple" are trademarks of Apple Inc. MacPane is not affiliated with or endorsed by Apple.
