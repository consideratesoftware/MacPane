# Live compatibility checklist

CI has no Mac. Run this by hand whenever the protocol layer changes, and record the result in the PR.

Record: macOS version, Mac model, Windows version, network (wired/Wi-Fi).

## Handshake

- [ ] `probe` connects with ARD auth (username + password, VNC password option OFF).
- [ ] `probe` connects with VNC auth (VNC password option ON, no username).
- [ ] Wrong password produces a readable error, not a hang.
- [ ] Server name in output matches the Mac's computer name.

## Framebuffer

- [ ] `frame.ppm` shows the desktop with correct colours (red is red).
- [ ] Changing the Mac's resolution while connected resizes the viewer without disconnecting.
- [ ] Dragging a window on the Mac updates smoothly in the viewer.

## Input

- [ ] Click, right-click, middle-click land where the cursor is drawn.
- [ ] Scroll wheel scrolls; direction matches Windows expectation.
- [ ] Typing in TextEdit produces the typed text, including shifted symbols.
- [ ] Win+C / Win+V copy and paste on the Mac.
- [ ] Win+Tab opens the Mac app switcher (Command+Tab).
- [ ] Alt+letter produces Option characters (Alt+8 gives a bullet).
- [ ] Enter, Backspace, Delete, arrows, Home/End, Esc all work.

## Resilience

- [ ] Put the Mac to sleep and wake it: viewer reports disconnect cleanly (auto-reconnect is M1).
- [ ] Pull the network cable: viewer reports disconnect within ~10s, no freeze.
- [ ] Connect two viewers at once (shared flag): both keep working.

## Discovery

- [ ] Mac appears in "Nearby Macs" within 5 seconds of launching the app.
- [ ] Name displays with spaces and apostrophes correct.
- [ ] Turning off Screen Sharing removes it from the list.
