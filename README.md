# Kitsu

A modern version control system written in Rust.

[![Rust Edition](https://img.shields.io/badge/rust-edition%202024-blue.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE.md)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen.svg)]()

---

## Overview

Kitsu is a lightweight, content-addressable version control system designed as a modern, sovereign alternative to Git. Built from scratch in Rust, Kitsu provides snapshot tracking, cryptographic verification, branch/tag primitives, local/offline repository modes, and multi-mode remote synchronization.

Kitsu maintains full interoperability with Git hosting platforms (such as GitHub and GitLab) via an integrated Git bridge with configurable data branches, while also supporting sovereign self-hosted registries over SSH/SFTP and 100% offline local filesystem repositories.

---

## Key Features

- **Content-Addressable Storage**: SHA-256 hashing and zlib compression for immutable, deduplicated data storage.
- **Cryptographic Checkpoint Signing**: Native Ed25519 signature generation and verification for every checkpoint.
- **Monorepo Architecture**: Structured as a modular Cargo workspace comprising a core engine library (`core`) and a decoupled command-line interface (`cli`).
- **Multi-Mode Networking**: Seamlessly push to and pull from Git remotes (with configurable data branch), local directory remotes (for offline backups and USB drives), and sovereign SSH/SFTP servers.
- **100% Offline / Local Operation**: Initialize and run repositories entirely offline without any external network dependency.
- **Self-Updating Engine**: Non-intrusive update notification checks and atomic in-place binary self-updating (`kitsu update`) via GitHub Releases.
- **Flexible Exclusion Engine**: Native support for both `.gitignore` and `.exclude` rule definitions.
- **Persona Identity Management**: Configurable local and global identity personas with automatic keypair generation.
- **Portability**: Complete repository export and import capabilities using compressed tar archives.

---

## Terminology Mapping (Kitsu vs. Git)

| Kitsu Concept | Git Equivalent | Description |
|---|---|---|
| **Chunk** | Blob | Raw file content addressed by SHA-256 |
| **Map** | Tree | Directory structure and file mode hierarchy |
| **Checkpoint** | Commit | Snapshot metadata, author info, parent link, and optional signature |
| **Stream** | Branch | Movable reference to a checkpoint lineage |
| **Seal** | Tag / Release | Named semantic version pointer with auto-increment and prerelease capabilities |
| **Stage** | Index | Staging index tracking file metadata and content hashes |
| **Ignite** | `git init` | Repository creation assistant (offline or remote) |
| **Track** | `git add` | Stage file content for the next checkpoint |
| **Freeze** | `git commit` | Seal staged modifications into an immutable checkpoint |
| **Timeline** | `git log` | Chronological checkpoint history |
| **Rollback** | `git reset --hard` | Restore working tree and HEAD to a specified checkpoint |
| **State** | `git status` | Working tree state inspection |
| **Update** | `rustup update / brew upgrade` | Self-updates the Kitsu binary in-place from official GitHub releases |

---

## Workspace Structure

The project is structured as a Cargo workspace with clean separation between core domain logic and presentation layers:

```
Kitsu/
├── Cargo.toml                  # Workspace definition and dependency manifest
├── docs/                       # Complete system documentation
└── src/
    ├── core/                   # Core VCS library (core crate)
    │   ├── Cargo.toml
    │   └── src/
    │       ├── lib.rs          # Public interface and module declarations
    │       ├── error.rs        # Typed error definitions (KitsuError)
    │       ├── config.rs       # Application configuration and constants
    │       ├── repository.rs   # Central repository orchestrator
    │       ├── state.rs        # Working state computation engine
    │       ├── exclude.rs      # .gitignore and .exclude rule engine
    │       ├── update.rs       # Update checker and in-place self-updater
    │       ├── objects/        # Chunk, Map, and Checkpoint primitives
    │       ├── storage/        # Storage backend and binary staging index
    │       ├── refs/           # HEAD, Stream, and Seal management
    │       ├── diff/           # Map diffing and textual delta engine
    │       ├── identity/       # Personas and Ed25519 cryptography
    │       └── remote/         # GitBridge, LocalBridge, SshTransport, and registry
    │
    └── cli/                    # Command-line binary (cli crate / kitsu binary)
        ├── Cargo.toml
        └── src/
            ├── main.rs         # Command dispatcher and update notification banner
            ├── app.rs          # Command-line interface definitions (Clap)
            └── commands/       # Individual subcommand implementations
```

---

## Installation

### Automated Install (Recommended)

#### Linux & macOS
Run the installation script in your terminal:
```bash
curl -fsSL https://raw.githubusercontent.com/jmaxdev/Kitsu/dev/install.sh | bash
```
Or using `wget`:
```bash
wget -qO- https://raw.githubusercontent.com/jmaxdev/Kitsu/dev/install.sh | bash
```

#### Windows (PowerShell)
Run the PowerShell installer in your terminal:
```powershell
irm https://raw.githubusercontent.com/jmaxdev/Kitsu/dev/install.ps1 | iex
```

---

### Pre-built Binaries (GitHub Releases)

Download pre-compiled binaries directly from [GitHub Releases](https://github.com/jmaxdev/Kitsu/releases):

| Platform | Target Architecture | Archive |
|---|---|---|
| **Linux** | `x86_64` (GNU/Linux) | `kitsu-x86_64-unknown-linux-gnu.tar.gz` |
| **macOS** | `aarch64` (Apple Silicon) | `kitsu-aarch64-apple-darwin.tar.gz` |
| **macOS** | `x86_64` (Intel) | `kitsu-x86_64-apple-darwin.tar.gz` |
| **Windows** | `x86_64` (MSVC) | `kitsu-x86_64-pc-windows-msvc.zip` |

---

### Build from Source

#### Prerequisites
- **Rust**: Version 1.85+ (Edition 2024)
- **C Compiler & Build Tools**:
  - Linux: `libssh2-1-dev`, `libssl-dev`, `pkg-config`
  - macOS: `libssh2`, `openssl`, `pkg-config` (via Homebrew)
  - Windows: Visual Studio C++ Build Tools

```bash
# Clone the repository
git clone https://github.com/jmaxdev/Kitsu.git
cd Kitsu

# Compile the workspace in release mode
cargo build --release

# The compiled binary will be located at target/release/kitsu
./target/release/kitsu --help
```

---

## Quick Start

```bash
# 1. Initialize a new repository (choose offline or remote in assistant)
kitsu ignite

# 2. Stage files for tracking
kitsu track src/main.rs Cargo.toml

# 3. Create a signed checkpoint
kitsu freeze -m "feat: initial system setup" --sign

# 4. View history
kitsu timeline

# 5. Inspect working tree status
kitsu state

# 6. Create a semantic version seal (supports major, minor, patch, alpha, beta, rc)
kitsu seal --bump patch

# 7. Push to a configured remote (Git, local folder, or SSH)
kitsu push

# 8. Check for updates and self-update
kitsu update --check
```

---

## Command Reference

| Command | Arguments / Flags | Description |
|---|---|---|
| `ignite` | None | Initializes a new repository (offline mode or remote configuration assistant) |
| `copy` | `<URL_OR_PATH> [DIR]` | Clones a repository from a Git remote, local folder, or SSH server |
| `track` | `<FILES...>` | Stages files for the next checkpoint |
| `freeze` | `-m <MSG> [-S/--sign]` | Creates a new checkpoint from staged modifications |
| `timeline` | None | Displays chronological checkpoint history |
| `diff` | `[OLD] [NEW]` | Displays differences between checkpoints or working tree |
| `rollback` | `[TARGET]` | Restores working directory and HEAD to a target checkpoint |
| `seal` | `[VERSION] [-b <BUMP>] [-l/--list]` | Creates or lists version seals (`major`, `minor`, `patch`, `alpha`, `beta`, `rc`) |
| `switch` | `<TARGET>` | Switches working tree and HEAD to a stream, seal, or hash |
| `export` | `<TARGET> <OUTPUT>` | Exports reachable objects into a compressed tar archive |
| `import` | `<INPUT>` | Imports objects from a tar archive into local storage |
| `push` | `[REMOTE] [TARGET] [-b <BRANCH>]` | Pushes objects and refs to a remote registry (Git, local, or SSH) |
| `pull` | `[REMOTE] [TARGET] [-b <BRANCH>]` | Pulls objects and refs from a remote registry (Git, local, or SSH) |
| `contents` | `[TARGET]` | Lists all tracked files and hashes inside a checkpoint |
| `hash` | `<FILE>` | Computes the SHA-256 chunk hash of a file |
| `state` | None | Displays staged, unstaged, and untracked file status |
| `peek` | `<HASH>` | Displays raw uncompressed content of an object |
| `burn` | `[HASH] [-a/--aggressive]` | Deletes an object from the object store |
| `repository` | `info \| stats \| verify \| vacuum \| remote \| stream` | Repository maintenance and inspection subcommands |
| `persona` | `add \| list \| use \| edit \| github \| keys` | Identity and cryptographic keypair management |
| `update` | `[-t/--tag <TAG>] [-c/--check]` | Checks GitHub for updates and self-updates the binary in-place |

---

## Documentation

Exhaustive technical documentation is available in the [`docs/`](docs/README.md) directory:

- [Architecture & Workspace Structure](docs/architecture.md)
- [Object Model & Data Structures](docs/object-model.md)
- [Storage Engine & Staging Index](docs/storage-engine.md)
- [CLI Reference Manual](docs/cli-reference.md)
- [Remote Protocols & Git Bridge](docs/remote-and-networking.md)
- [Identity & Ed25519 Cryptography](docs/identity-and-crypto.md)
- [Git Compatibility & Interoperability](docs/git-compatibility.md)
- [Terminology Glossary](docs/glossary.md)

---

## License

This project is licensed under the **[UnSetSoft Public License 1.0](LICENSE.md)**.
