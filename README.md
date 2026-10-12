<p align="center">
  <img src="app/src-tauri/icons/128x128.png" alt="Familiar" width="96" height="96">
</p>

<h1 align="center">Familiar</h1>

<p align="center">A desktop pet that reacts to your world.</p>

<p align="center">
  <a href="https://github.com/Monster12138/familiar/releases"><img src="https://img.shields.io/github/v/release/Monster12138/familiar?style=flat&color=blue" alt="Latest release"></a>
  <a href="https://github.com/Monster12138/familiar/actions/workflows/rust.yml"><img src="https://github.com/Monster12138/familiar/actions/workflows/rust.yml/badge.svg" alt="Rust CI"></a>
  <a href="#license"><img src="https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue" alt="License: MIT or Apache-2.0"></a>
  <img src="https://img.shields.io/badge/platforms-macOS%20%7C%20Windows%20%7C%20Linux-555" alt="Platforms: macOS, Windows, Linux">
</p>

<p align="center">
  <a href="README_zh.md">简体中文</a> · <a href="README.md">English</a> · <a href="https://github.com/Monster12138/familiar/releases">Download</a> · <a href="#documentation">Documentation</a> · <a href="https://github.com/Monster12138/familiar/issues">Issues</a>
</p>

Familiar is a local-first desktop companion for coding agents. It turns Hook events into the actions, task bubbles, and status changes of a pixel-art desktop pet—without adding another dashboard to watch.

<details>
<summary><strong>Contents</strong></summary>

- [Why Familiar?](#why-familiar)
- [Install](#install)
- [Quick start](#quick-start)
- [Features](#features)
- [Supported integrations](#supported-integrations)
- [Remote mode](#remote-mode)
- [Privacy](#privacy)
- [Build from source](#build-from-source)
- [Documentation](#documentation)
- [Contributing](#contributing)
- [License](#license)

</details>

---

## Why Familiar?

- **Multi-agent support**: Claude-code, ChatGpt, DeepSeek Harness... one desktop pet handles them all.
- **One less window to watch**: Follow agent activity through pet actions and task bubbles.
- **Fits your workflow**: Connect through agent Hook APIs while preserving existing configuration.
- **Your data, your choice**: Run locally by default; configure remote connections when needed.

## Install

### Homebrew

Install the desktop app on macOS:

```bash
brew install --cask Monster12138/familiar/familiar
```

Install the CLI on macOS or Linux:

```bash
brew install Monster12138/familiar/familiar-cli
```

### Download a release

Download the latest release from [GitHub Releases](https://github.com/Monster12138/familiar/releases).

| Platform | Architecture | Installation |
| --- | --- | --- |
| macOS | arm64 / x86_64 | Homebrew Cask or release download |
| Windows | x86_64 | Release download |
| Linux | x86_64 | `.deb` / AppImage |

> [!NOTE]
> macOS packages are not notarized with an Apple Developer ID, and Windows packages are not code-signed. Gatekeeper or SmartScreen may require manual approval on first launch.

<details>
<summary><strong>First launch on macOS: allow an unnotarized app</strong></summary>

Because Familiar is not yet notarized, macOS blocks it the first time you open
it. If you installed Familiar from this repository's Homebrew tap or GitHub
Releases, allow it manually:

1. Open Familiar. When macOS says it cannot verify the app, click **Done**.

   <img src="docs/images/macos-gatekeeper/01-unverified-app.jpg" alt="macOS cannot verify Familiar" width="300">

2. Open **System Settings → Privacy & Security**, scroll to **Security**, find
   the message that Familiar was blocked, and click **Open Anyway**.

   <img src="docs/images/macos-gatekeeper/02-open-anyway-setting.jpg" alt="Open Anyway in macOS Privacy and Security settings" width="480">

3. In the confirmation dialog, click **Open Anyway** again. Authenticate with
   your password or Touch ID if macOS asks. Familiar will open normally from
   then on.

   <img src="docs/images/macos-gatekeeper/03-confirm-open.jpg" alt="Confirm opening Familiar on macOS" width="300">

Only bypass this warning when you obtained Familiar from the official sources
linked above.

</details>

## Quick start

1. **Launch Familiar** and choose local mode in onboarding.
2. **Connect an agent**: Preview and confirm Hook injection in onboarding or **Settings → Hooks**.
3. **Start an agent task** and watch the pet actions and task bubbles respond to its activity.
4. **Make it yours**: Choose visible content, click-through, or fullscreen auto-hide in **Settings → Display Configuration**, and switch companions in **Pet Materials**.

> [!TIP]
> For agents running on another machine, use [remote mode](#remote-mode) and manage Hooks on the server.

## Features

- **Reactive desktop pet** — Pixel-art actions follow agent activity without interrupting your flow.
- **Agent integrations** — Connect Claude Code, Codex CLI, DeepSeek Harness, Qoder, and Google Antigravity through their Hook APIs.
- **Remote connections** — Run the Agent and Familiar server remotely while the local desktop client subscribes to pet state.
- **System dashboard** — Show CPU, memory, and disk usage beside the pet.
- **Sprite packs** — Import more companions with the `.fpack` format.
- **Lightweight** — Pause unnecessary polling while UI surfaces are hidden.
- **Privacy-first** — Process events locally by default, with no telemetry and minimal operational logs.
- **Fullscreen auto-hide** — Optionally hide the pet when the foreground app fills its screen on macOS / Windows, then restore it on exit. Off by default; unavailable on Linux.

## Supported integrations

| Integration | Transport | Status |
| --- | --- | --- |
| Claude Code | Hooks | Available |
| Codex CLI | Hooks | Available |
| DeepSeek Harness | Hooks | Available |
| Qoder | Hooks | Available |
| Google Antigravity | Hooks | Available |

Familiar only manages its own Hook entries. Installation creates a backup and preserves existing agent configuration.

## Remote mode

Local mode remains the default. In remote mode, the Agent and `familiar-cli serve` run on the same remote machine, while the desktop app subscribes to render-state updates over WebSocket.

Remote mode is not a Hook relay: the desktop client cannot forward local Agent events or modify Hooks on the server. See [Remote Deployment](docs/REMOTE_DEPLOYMENT.md) for TLS, authentication, and deployment instructions.

## Privacy

Familiar may read recent task descriptions, command text, tool names, file paths, and referenced Antigravity transcripts to render the current state. It does not persist full transcripts or Agent event history, and operational logs exclude raw prompts, commands, paths, and Hook payloads.

Remote mode sends bounded render-state summaries to the configured Familiar server/client connection. It does not expose a remote query API for full prompts, transcripts, file contents, Hook payloads, or command output.

See [Privacy & Data Handling](docs/PRIVACY.md) for details.

## Build from source

Requirements: Rust 1.88+, Node.js 18+, npm, and the desktop prerequisites for your platform.

```bash
# Build the CLI resource required by the desktop app
cargo build --release -p familiar-cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd app
npm ci
npm run build
npm run tauri dev
```

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
