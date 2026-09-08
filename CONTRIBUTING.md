# Contributing

Thanks for looking. MacPane is small and opinionated; the fastest way to help is to pick an issue from the project board.

## Setup

- Stable Rust (`rustup`), plus the MSVC C++ build tools on Windows.
- `cargo test --workspace` must pass.
- `cargo clippy --workspace --all-targets -- -D warnings` must be clean.
- `cargo fmt --all` before committing.

## Branches and PRs

- Branch from `main`, name it `<area>/<slug>` (for example `protocol/zrle`).
- One logical change per PR. Small PRs get reviewed and merged quickly.
- Describe how you tested. If the change touches the protocol, say which lines of `docs/MAC-COMPAT-CHECKLIST.md` you ran against a real Mac, or say that you could not.
- CI runs fmt, clippy, tests and a release build on Windows and Linux.

## Rules that will not be relaxed

- The client never connects to a non-local address. `macpane_rfb::lan::is_local` is the single gate.
- No outbound connections other than to the Mac the user picked.
- No `unsafe` without an ADR in `docs/adr/`.
- Every byte from the server is untrusted. Bounds-check before indexing; return `Error::Protocol` instead of panicking.

## Reporting a Mac incompatibility

Open an issue with the `needs-mac` label. Include macOS version, the output of the `probe` example, and what you expected. If you can, capture the first few hundred bytes of the server's handshake with Wireshark; it is unencrypted before auth and safe to share.
