# MacPane

LAN-only viewer for macOS Screen Sharing, for Windows. Rust workspace, three crates. Read `docs/ARCHITECTURE.md` before changing structure and `docs/APPLE-SCREEN-SHARING.md` before touching the protocol.

## Commands

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all
cargo build --release            # target/release/macpane.exe
cargo run -p macpane-rfb --example probe -- HOST --user U --pass P --out frame.ppm
```

On Windows the shell may need `%USERPROFILE%\.cargo\bin` on PATH.

## Conventions

- Protocol code is transport-agnostic (`impl Read`/`impl Write`) and tested with scripted byte vectors. Look at `client::tests::scripted_server` for the pattern.
- Bytes from the server are untrusted. Bounds-check, return `Error::Protocol`, never index blindly, never panic.
- `macpane_rfb::lan::is_local` is the LAN gate. Do not add a code path that opens a socket without it.
- No `unsafe` without an ADR in `docs/adr/`.
- No new outbound network destinations.
- Every new `Error` variant gets a test that produces it.
- Framebuffer is RGBA8; the client always negotiates `PixelFormat::RGBA32` so raw pixels copy straight through.
- Windows key = Command (Super_L), Alt = Option. See `crates/macpane/src/input.rs`.

## Work tracking

Issues and the project board on GitHub are the source of truth. Milestones M1..M3 match `docs/PLAN.md`. Agent briefs for each area are in `.claude/agents/`; `docs/AGENTS.md` explains how they are used.

## When you learn a new Mac quirk

Write it down in `docs/APPLE-SCREEN-SHARING.md` in the same PR.
