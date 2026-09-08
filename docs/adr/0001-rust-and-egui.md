# ADR 0001: Rust with egui for the client

Date: 2026-09-08. Status: accepted.

## Context

MacPane is a VNC viewer for Windows that talks to macOS Screen Sharing. The candidates were C# with WPF and Rust with a lightweight GUI toolkit. A .NET scaffold was created first and then replaced.

## Decision

Rust, with `egui`/`eframe` for the window.

## Reasons

- **Single small static binary.** No runtime to install, which matches the "nothing to set up" pitch. A release build is a few MB.
- **Untrusted input.** The server streams binary data that we parse and index into buffers with. Rust's bounds checking without a garbage collector fits, and the protocol crate is `#![forbid(unsafe_code)]`-eligible.
- **The GUI is trivial.** A VNC viewer is one textured rectangle plus keyboard and mouse. egui provides that, DPI awareness, and a decent form for the connect screen with almost no code. WPF's strengths (data binding, rich controls) would go unused.
- **Crate ecosystem covers the hard parts.** `des`, `aes`, `md-5`, `num-bigint` for auth; `flate2` for ZRLE; `mdns-sd` for Bonjour. All pure Rust.
- **Portability.** The same code builds on Linux and macOS, which makes a future Linux client cheap and lets contributors without Windows work on the protocol.

## Consequences

- Native Windows integration (tray icon, jump lists, credential manager) needs the `windows` crate rather than coming for free. These are roadmap items, not blockers.
- Build requires the MSVC C++ build tools on Windows.
- Text input for non-Latin scripts through egui is weaker than WPF. Acceptable for now; revisit if it bites.

## Alternatives considered

- **C# / WPF**: best native feel, but a runtime dependency (or a 70 MB self-contained build) and no protocol libraries worth using.
- **Tauri + noVNC**: needs a TCP-to-WebSocket bridge and a WebView; more moving parts for no gain.
- **Wrapping TigerVNC/libvncclient**: forces GPL and a C build; the protocol is small enough to own.
