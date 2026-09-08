# Working on MacPane with agents

This repo is set up so that most work items on the project board can be picked up by a Claude Code session with a specialised agent. The definitions live in `.claude/agents/`. Each one is a Markdown file with frontmatter naming the agent, the model, and the tools it may use, followed by its brief.

## The agents

| Agent | Owns | Typical issue labels |
| --- | --- | --- |
| `rfb-protocol` | `crates/macpane-rfb`: handshake, security, encodings, framebuffer. | `area:protocol`, `area:crypto`, `area:encoding` |
| `viewer-ui` | `crates/macpane`: screens, input mapping, texture upload, session thread. | `area:ui`, `area:input` |
| `discovery` | `crates/macpane-discovery` and network/LAN concerns. | `area:discovery`, `area:network` |
| `mac-compat` | Everything that needs a real Mac: runs the checklist, documents quirks, files bugs. | `needs-mac` |
| `release` | CI, packaging, winget, versioning. | `area:release`, `area:ci` |
| `reviewer` | Reads a PR for correctness and safety against untrusted server input. Never edits. | any PR |

## How a work item flows

1. An issue on the board describes the change, names the agent in its body, and links the relevant doc section.
2. A session invokes the agent with the issue number. The agent reads the issue, the linked docs, and the code it owns.
3. The agent works on a branch named `<area>/<short-slug>`, keeps commits small, and runs `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` before opening a PR.
4. The PR body says what changed, how it was tested, and, for protocol work, which items on `MAC-COMPAT-CHECKLIST.md` still need a live run.
5. `reviewer` reads the PR. Its findings go back to the authoring agent.
6. A human merges. Agents do not merge to `main` unless the human has said so for that session.

## Ground rules every agent follows

- No `unsafe` without an ADR.
- Every new `Error` variant gets a test that produces it.
- The LAN gate (`macpane_rfb::lan`) is never bypassed, even in examples or tests that hit the network.
- No new outbound network destinations. Ever.
- Prefer a scripted-bytes unit test over a live test. Live tests are documented on the checklist, not automated.
- Update `docs/APPLE-SCREEN-SHARING.md` whenever a new Mac quirk is learned.
