# Contributing to Familiar

Thank you for your interest in contributing to Familiar!

## Code of Conduct

All contributors are expected to adhere to our [Code of Conduct](CODE_OF_CONDUCT.md).

## Development Setup

### Prerequisites

- **Rust**: 1.88 or later (Cargo, rustfmt, clippy)
- **Node.js**: v18 or later and `npm`
- **Platform Dependencies**:
  - macOS: Xcode Command Line Tools
  - Windows: Visual Studio Build Tools (C++ Desktop Development)
  - Linux: GTK3 and webkit2gtk development packages

### Building & Running

1. **Clone the repository**:
   ```bash
   git clone https://github.com/Monster12138/familiar.git
   cd familiar
   ```

2. **Backend & Hook Verification**:
   ```bash
   cargo check --workspace
   cargo test --workspace
   cargo clippy --workspace --all-targets -- -D warnings
   ```

3. **Frontend & Tauri Desktop App**:
   ```bash
   cd app
   npm ci
   npm run build
   npm run tauri dev
   ```

4. **Install a local macOS build** (optional, macOS only):
   ```bash
   scripts/install-macos.sh --local
   ```
   Builds the release bundle and installs it to `/Applications`, replacing and
   relaunching any running Familiar. Without `--local` the script first pulls
   `origin main` and requires a clean working tree.

## Pull Request Guidelines

1. **Focused Scopes**: Keep PRs focused on a single feature, bug fix, or documentation update.
2. **Format & Quality Checks**:
   - Format Rust code with `rustfmt --edition 2021 <files>`.
   - Ensure `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass.
   - Run `cd app && npm run build` to verify frontend production compilation.
3. **Commit Style**: Use Conventional Commit format (e.g. `feat:`, `fix:`, `docs:`, `chore:`).
4. **Privacy First**: Do not introduce telemetry, remote network calls, or logging of confidential user payloads.
