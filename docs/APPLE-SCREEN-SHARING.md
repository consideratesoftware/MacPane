# macOS Screen Sharing: what MacPane talks to

Everything MacPane knows about Apple's VNC server, collected in one place.

## Enabling it

System Settings > General > Sharing > Screen Sharing. That's the whole install. The server is `screensharingd`, listening on TCP 5900 on all interfaces, and it registers `_rfb._tcp` with Bonjour using the computer name.

"Remote Management" is the Apple Remote Desktop variant of the same daemon. MacPane works with either, but plain Screen Sharing is enough and exposes less.

## Authentication modes

The Mac advertises different security types depending on settings:

| Setting | Types offered | What MacPane does |
| --- | --- | --- |
| Default | 30 (ARD), 33, 35 | Uses 30 with your Mac username and password. |
| "VNC viewers may control screen with password" enabled | 30, 33, 35, 2 | Uses 2 (VNC password). Username not needed. |

Types 33 and 35 are Apple-private and undocumented; MacPane ignores them. Type 30 is Diffie-Hellman + MD5 + AES-128-ECB and is documented in `crates/macpane-rfb/src/security/ard.rs`.

Failed logins are rate-limited by macOS; after several failures the server refuses connections for a while with "Too many security failures". MacPane surfaces that string verbatim.

## Protocol quirks

- Version string is `RFB 003.889\n`. Treat as 3.8.
- Default pixel format is 32bpp big-endian, red shift 16. MacPane immediately sends SetPixelFormat for little-endian RGBA and the server honours it.
- Supported encodings, roughly in the order the Mac prefers: ZRLE (16), Zlib (6), Hextile (5), CopyRect (1), Raw (0). Tight is not supported.
- Supports DesktopSize (-223) and Cursor (-239) pseudo-encodings. Resolution changes and Retina scale changes come through as DesktopSize.
- Framebuffer is the **native** resolution. A 14" MacBook Pro reports 3024x1964. Expect big frames.
- Sends ServerCutText for the Mac clipboard. Latin-1 only unless Extended Clipboard is negotiated.
- Keeps the connection open indefinitely, but the Mac itself will sleep unless "Wake for network access" is on and sleep is disabled while plugged in. Sleep is the number one cause of "VNC keeps disconnecting".
- Keyboard: Super_L/Super_R (0xffeb/0xffec) map to Command. Alt_L maps to Option. Meta keysyms also work but Super is the safe choice.

## Keeping it LAN-only on the Mac side

MacPane will not connect to a public address. To also make the Mac refuse connections from outside your subnet, even if a router misconfiguration forwards port 5900, add a pf rule:

```bash
sudo sh -c 'printf "block in proto tcp to any port 5900\npass in proto tcp from 192.168.1.0/24 to any port 5900\n" > /etc/pf.anchors/lanvnc && pfctl -f /etc/pf.anchors/lanvnc -e'
```

Replace `192.168.1.0/24` with your subnet. This does not persist across reboots by default; a launchd plist that re-applies it is a reasonable follow-up.

## Verifying with the probe

```bash
cargo run -p macpane-rfb --example probe -- your-mac.local --user NAME --pass PW --out frame.ppm
```

Expected output names the Mac, reports `security: AppleRemoteDesktop`, the native size, and one received rectangle set. Open `frame.ppm` in any image viewer to confirm the pixels are right way round and the right colour.
