# Familiar

> *“A desktop pet that reacts to your world.”* — A local-first companion that responds to the rhythm of your work, agents, and everyday workflows.

[简体中文](README_zh.md)

> [!NOTE]
> Familiar is in **Alpha**. The core Rust event pipeline, desktop companion, and hook integrations are usable; richer dashboards, persistence, and cross-platform release builds are still evolving.

## Why

Familiar starts with a desktop pet that reacts to your world. It stays quietly in your workspace and responds to the rhythm around you: an agent beginning a task, a workflow waiting for input, or a long stretch of focused work. Agent state linkage is one way it comes alive, turning local activity into subtle changes you can notice without opening another window.

The companion is not limited to coding tools. Familiar's hook layer can connect status from coding agents, other local agents, and non-programming workflows, so the pet can react to more of the way you work. Events are normalized locally, rendered by a Tauri desktop app, and never sent to a telemetry service by default.

## Features

- **Reactive desktop pet** — Pixel-art animations respond to activity without taking over your workspace.
- **Agent state linkage** — Connect Claude Code, Codex CLI, and Google Antigravity so the companion can reflect their activity.
- **Hook-based extensibility** — Adapt other agents and local workflows through the same event pipeline, including non-programming scenarios.
- **Modular architecture** — Keep hook parsing, state management, transport, and rendering in separate layers.
- **Local activity view** — Show the current CPU, memory, and disk state alongside the companion.
- **Sprite packs** — Import and package additional companions with the documented `.fpack` format.
- **Privacy-first by design** — Process hook data locally and minimize operational logs; no remote telemetry is included.

## Supported integrations

| Integration | Transport | Status |
| --- | --- | --- |
| Claude Code | Hook reporter over the local Familiar channel | Pending verification |
| Codex CLI | Hook reporter over the local Familiar channel | Available |
| Qoder | Hook reporter over the local Familiar channel | Available |
| Google Antigravity | Native hook adapter with transcript extraction | Available |

## Quick Start

Apple Silicon macOS builds are available from [GitHub Releases](https://github.com/Monster12138/familiar/releases). The current alpha builds are not notarized, so macOS may require explicit approval before first launch. Builds for other platforms can be created from source with Rust, Node.js, and the platform desktop prerequisites.

### Prerequisites

- **Rust** 1.88 or later, including Cargo, rustfmt, and Clippy
- **Node.js** 18 or later and npm
- **macOS** Xcode Command Line Tools
- **Windows** Visual Studio Build Tools with C++ desktop development
- **Linux** GTK3 and WebKitGTK development packages

### Build and run

From the repository root:

```bash
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Build the desktop application from `app/`:

```bash
cd app
npm ci
npm run build
npm run tauri dev
```

The settings window can install or remove Familiar-owned hooks. Existing agent configuration is backed up before it is changed.

## Architecture

| Layer | Responsibility |
| --- | --- |
| `familiar-hooks` | Parse, preview, install, and remove vendor hook integrations |
| `familiar-core` | Normalize events, manage state, load configuration, and provide sprite-pack abstractions |
| `familiar-api` | Local transport and experimental REST/WebSocket routes |
| `familiar-cli` | Lightweight hook reporter used by supported integrations |
| `app/` | Tauri composition, tray integration, desktop windows, and vanilla JavaScript rendering |

The primary local transport uses Unix domain sockets on Unix-like systems and loopback TCP where needed. Agent activity is currently held in process memory; the SQLite storage abstraction is experimental and is not wired into the desktop application.

## Privacy

Familiar may read the latest task description, command text, tool names, file paths, and related local hook data to render the current state. Antigravity transcript files may be read when a hook explicitly references them.

The current application does not persist a full transcript or agent event history, and it does not send captured data to a remote service. Operational logs contain only minimal metadata such as event kind, session identifier, and mood; raw prompts, commands, paths, and hook payloads are excluded.

See [docs/PRIVACY.md](docs/PRIVACY.md) for the complete data-flow, configuration, logging, and cleanup details.

## Documentation

- [Privacy & Data Handling](docs/PRIVACY.md) — What Familiar reads, keeps, and logs
- [Design Notes](docs/DESIGN.md) — Architecture and protocol background
- [Backend Workflow](docs/BACKEND_WORKFLOW.md) — Rust development workflow
- [Frontend Workflow](docs/FRONTEND_WORKFLOW.md) — UI development workflow
- [Sprite Pack Guide](docs/SPRITE_PACK_CREATION_GUIDE.md) — Create and package companions

## Project status

Implemented today:

- Local hook parsing and reporting for the supported integrations
- Event normalization and desktop state transitions
- Tauri desktop companion and sprite-pack loading
- Settings flows for hook installation and removal
- CPU, memory, and disk indicators

Planned or still experimental:

- Persistent agent history and retention management
- A complete statistics and activity dashboard
- Stable public API guarantees
- Signed and notarized macOS artifacts, plus verified Windows/Linux packaging

## Contributing

Issues and pull requests are welcome. Before contributing, please read:

- [CONTRIBUTING.md](CONTRIBUTING.md) — Development setup and contribution workflow
- [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md) — Community standards
- [SECURITY.md](SECURITY.md) — Private vulnerability reporting

Please keep the privacy-first behavior intact: do not add telemetry or persist captured prompts, transcripts, file contents, or hook payloads without an explicit product decision.

## License

Familiar is dual-licensed under the [MIT License](LICENSE-MIT) or the [Apache License, Version 2.0](LICENSE-APACHE), at your option.

Built-in sprites, sprite archives, and application icons are covered by the same project license. See [ASSETS.md](ASSETS.md) for the asset scope and third-party contribution requirements.
