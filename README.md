# Familiar

> *A desktop pet that reacts to your world.*

[简体中文](README_zh.md)

Familiar is a local-first desktop companion for coding agents. It turns Hook events into the actions, task bubbles, and status changes of a pixel-art desktop pet—without adding another dashboard to watch.

## Features

- **Reactive desktop pet** — Pixel-art actions follow agent activity without interrupting your flow.
- **Agent integrations** — Connect Claude Code, Codex CLI, DeepSeek Harness, Qoder, and Google Antigravity through their Hook APIs.
- **Remote connections** — Run the Agent and Familiar server remotely while the local desktop client subscribes to pet state.
- **System dashboard** — Show CPU, memory, and disk usage beside the pet.
- **Sprite packs** — Import more companions with the `.fpack` format.
- **Lightweight** — Pause unnecessary polling while UI surfaces are hidden.
- **Privacy-first** — Process events locally by default, with no telemetry and minimal operational logs.

## Supported integrations

| Integration | Transport | Status |
| --- | --- | --- |
| Claude Code | Hooks | Available |
| Codex CLI | Hooks | Available |
| DeepSeek Harness | Hooks | Available |
| Qoder | Hooks | Available |
| Google Antigravity | Hooks | Available |

Familiar only manages its own Hook entries. Installation creates a backup and preserves existing agent configuration.

## Install

Download the latest release from [GitHub Releases](https://github.com/Monster12138/familiar/releases).

Current release targets:

- macOS arm64 and x86_64
- Windows x86_64
- Linux x86_64 (`.deb` and AppImage)

macOS packages are not notarized with an Apple Developer ID, and Windows packages are not code-signed. Gatekeeper or SmartScreen may require manual approval on first launch.

## Build from source

Requirements: Rust 1.88+, Node.js 18+, npm, and the desktop prerequisites for your platform.

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd app
npm ci
npm run build
npm run tauri dev
```

## Remote mode

Local mode remains the default. In remote mode, the Agent and `familiar-cli serve` run on the same remote machine, while the desktop app subscribes to render-state updates over WebSocket.

Remote mode is not a Hook relay: the desktop client cannot forward local Agent events or modify Hooks on the server. See [Remote Deployment](docs/REMOTE_DEPLOYMENT.md) for TLS, authentication, and deployment instructions.

## Privacy

Familiar may read recent task descriptions, command text, tool names, file paths, and referenced Antigravity transcripts to render the current state. It does not persist full transcripts or Agent event history, and operational logs exclude raw prompts, commands, paths, and Hook payloads.

Remote mode sends bounded render-state summaries to the configured Familiar server/client connection. It does not expose a remote query API for full prompts, transcripts, file contents, Hook payloads, or command output.

See [Privacy & Data Handling](docs/PRIVACY.md) for details.

## Documentation

- [Privacy & Data Handling](docs/PRIVACY.md)
- [Remote Deployment](docs/REMOTE_DEPLOYMENT.md)
- [Design Notes](docs/DESIGN.md)
- [Sprite Pack Guide](docs/SPRITE_PACK_CREATION_GUIDE.md)
- [Development Workflow](CONTRIBUTING.md)
- [Release Process](docs/RELEASE_PROCESS.md)

## Contributing

Issues and pull requests are welcome. Please read [CONTRIBUTING.md](CONTRIBUTING.md), [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md), and [SECURITY.md](SECURITY.md) first.

Keep Familiar privacy-first: do not add telemetry or persist captured prompts, transcripts, file contents, or Hook payloads without an explicit product decision.

## License

Familiar is dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE). Built-in sprites, sprite archives, and application icons use the same project license; see [ASSETS.md](ASSETS.md).
