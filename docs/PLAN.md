# Plan

Work is tracked on the GitHub project board; this file is the narrative version. Milestones map 1:1 to GitHub milestones.

## M0: Foundation (done in the initial scaffold)

- Cargo workspace, CI, docs, agent definitions.
- `macpane-rfb`: 3.7/3.8 handshake, None/VncAuth/ARD security, Raw + CopyRect, DesktopSize, framebuffer, LAN gate. 54 unit tests.
- `macpane-discovery`: Bonjour browse.
- `macpane`: connect screen with discovered hosts, viewer with pointer and keyboard.

## M1: "I can actually use it" (first dogfood)

Goal: replace RealVNC for daily use on one Windows PC and one Mac.

- Verify against a real Mac (Sonoma or Sequoia) using the probe example and the checklist. Fix whatever breaks. This is the first thing to do when a Mac is available.
- ZRLE encoding. Raw is fine on gigabit but ZRLE is what the Mac prefers and cuts bandwidth ~10x.
- Partial texture uploads (only damaged rects) instead of whole-frame upload.
- Cursor pseudo-encoding rendered locally, so the pointer does not lag.
- Auto-reconnect with backoff when the Mac sleeps and wakes.
- Remember last host and username (not password) in a config file.
- Basic keyboard completeness: Cmd+Tab, Cmd+Space, function keys, Caps Lock state.

## M2: Polish

- Save passwords in Windows Credential Manager, opt-in.
- Fullscreen toggle and "fit to window" vs "1:1" scaling.
- Retina handling: a 2x Mac framebuffer should look right on a 1x or 1.5x Windows display.
- Clipboard both directions with UTF-8 (RFB's Extended Clipboard pseudo-encoding).
- Hextile fallback for older servers.
- Connection quality indicator (bytes/sec, fps, round-trip).
- App icon, window title with Mac name, remember window size.

## M3: Distribution

- `cargo dist` or a GitHub Actions release workflow producing a signed-checksum `.zip` and `.msi`.
- winget manifest.
- A one-page website with the Mac setup steps.

## Later / ideas

- Tight encoding (JPEG) for wireless links.
- Multi-monitor selection.
- Linux build (everything except the installer is already portable).
- Audio: not possible over Apple's VNC; document that clearly.

## Non-goals

- Anything that touches the internet: relays, accounts, update checks, telemetry.
- Serving a Windows screen to a Mac. This is a viewer only.
- Supporting Apple's proprietary high-performance Screen Sharing mode. It is Mac-to-Mac only and undocumented.
