# Security

MacPane's threat model, in short: the Mac you connect to might be hostile (or compromised), and the network might carry traffic you did not intend.

## What MacPane promises

- It will not connect to a public IP address. The gate is `macpane_rfb::lan::is_local` and applies to every code path that opens a socket.
- It makes no other network connections: no update checks, telemetry, or relays.
- Every server message is bounds-checked before touching memory. Malformed input yields an error and a disconnect, not a panic or an out-of-bounds write.
- Passwords live in memory for the session only. Nothing is written to disk unless you opt into saved credentials (planned, via Windows Credential Manager).

## What it does not promise

- RFB itself is unencrypted after authentication. Anyone on your LAN can see your screen traffic. That is inherent to macOS Screen Sharing over VNC and the reason MacPane is LAN-only. If your LAN is untrusted, use a VPN or SSH tunnel and connect to `127.0.0.1`.
- VNC password auth (type 2) is DES with an 8-character key. It resists casual sniffing, nothing more. Prefer Apple's type 30 auth, which does a Diffie-Hellman exchange.

## Reporting

Open a GitHub issue, or if the problem is sensitive, email the maintainer listed on the GitHub organisation page. Please include a way to reproduce.
