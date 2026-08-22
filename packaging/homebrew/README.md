# Homebrew packaging

Familiar is distributed through two Homebrew package types:

- `familiar-cli` is a Formula for the server and other headless deployments.
- `familiar` is a Cask for the macOS desktop application. The application
  bundle includes the same-version `familiar-cli`, and the Cask exposes it as
  the `familiar-cli` command.

The templates in this directory are intended to be copied into a tap such as
`Monster12138/homebrew-familiar`:

```text
homebrew-familiar/
├── Casks/
│   └── familiar.rb
└── Formula/
    └── familiar-cli.rb
```

The `.rb.template` files are deliberately not loadable Homebrew definitions.
Before updating a tap, replace the `__VERSION__`, `__SOURCE_SHA256__`,
`__ARM_SHA256__`, and `__INTEL_SHA256__` markers from the release assets.

## Install

```bash
brew tap Monster12138/familiar

# Server, remote deployment, or CLI-only installation
brew install familiar-cli

# macOS desktop application plus the embedded familiar-cli
brew install --cask familiar
```

The Formula builds the CLI from the immutable Git tag, so it works on
Homebrew-supported macOS and Linux hosts without requiring a desktop runtime.
The Cask downloads the signed/notarized macOS application bundle and links the
embedded CLI from `Familiar.app/Contents/Resources/bin/familiar-cli`.

The release workflow uploads standalone CLI archives for macOS, Linux, and
Windows in addition to the Tauri desktop bundles. Those archives are useful
for server installations outside Homebrew; the Formula itself builds from
source to keep the tap reproducible.
