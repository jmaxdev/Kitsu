# Kitsu Documentation

> **Kitsu** — A modern version control system written in Rust.

Kitsu is a from-scratch VCS that reimagines version control with its own terminology, object model, and workflow. It uses **content-addressable storage** backed by SHA-256 hashing and zlib compression, **Ed25519 cryptographic signatures** for checkpoint integrity, and supports both **self-hosted SSH/SFTP registries** and a **Git bridge** for GitHub/GitLab integration.

**Current Version:** `0.0.1-alpha` · **License:** [UPL 1.0](../LICENSE.md)

---

## Table of Contents

| Document | Description |
|----------|-------------|
| [Architecture](architecture.md) | High-level system architecture and data flow diagrams |
| [CLI Reference](cli-reference.md) | Complete reference for all commands, flags, and subcommands |
| [Object Model](object-model.md) | The Chunk → Map → Checkpoint data model |
| [Storage Engine](storage-engine.md) | Content-addressable storage, hashing, and compression |
| [Identity & Cryptography](identity-and-crypto.md) | Ed25519 personas, key management, and checkpoint signing |
| [Networking](networking.md) | SSH/SFTP Sovereign Registry and Git bridge protocols |
| [Repository Internals](repository-internals.md) | The `.kitsu/` directory structure and file formats |
| [Modules Reference](modules.md) | Detailed reference for each Rust source module |
| [CI/CD Pipelines](ci-cd.md) | Test and production release workflows |
| [Contributing](contributing.md) | How to set up, build, test, and contribute |
| [Glossary](glossary.md) | Kitsu-specific terminology and Git equivalents |

---

## Installation

You can download the pre-compiled binaries for your operating system from the **[Releases page](https://github.com/jmaxdev/Kitsu/releases)**.

### Linux (Ubuntu/Debian)
```bash
wget https://github.com/jmaxdev/Kitsu/releases/latest/download/kitsu-x86_64-unknown-linux-gnu.tar.gz
tar -xzf kitsu-x86_64-unknown-linux-gnu.tar.gz
sudo mv kitsu /usr/local/bin/
```

### macOS
```bash
# For Apple Silicon (M1/M2/M3)
wget https://github.com/jmaxdev/Kitsu/releases/latest/download/kitsu-aarch64-apple-darwin.tar.gz
tar -xzf kitsu-aarch64-apple-darwin.tar.gz
sudo mv kitsu-m1 /usr/local/bin/kitsu

# For Intel Macs
wget https://github.com/jmaxdev/Kitsu/releases/latest/download/kitsu-x86_64-apple-darwin.tar.gz
tar -xzf kitsu-x86_64-apple-darwin.tar.gz
sudo mv kitsu-intel /usr/local/bin/kitsu
```

### Windows
1. Download `kitsu-x86_64-pc-windows-msvc.zip` from the Releases page.
2. Extract the `.zip` file.
3. Move `kitsu.exe` to a folder and add that folder to your system's `PATH` environment variable.

---

## Quickstart

### 1. Build from source

```bash
# Prerequisites: Rust stable toolchain (edition 2024)
# Linux/macOS: libssh2, openssl, pkg-config
# Windows: no extra system deps needed

git clone https://github.com/jmaxdev/Kitsu.git
cd Kitsu
cargo build --release

# Binary is at target/release/kitsu (or kitsu.exe on Windows)
```

### 2. Initialize a repository

```bash
kitsu ignite
```

The **Ignite Assistant** will walk you through initial setup, including optional remote registry configuration (GitHub or custom SSH server).

### 3. Track files

```bash
kitsu track src/main.rs src/lib.rs
```

This stages files for the next checkpoint by hashing their contents and recording them in the binary stage file.

### 4. Create a checkpoint

```bash
kitsu freeze -m "Initial checkpoint"

# With Ed25519 signature:
kitsu freeze -m "Signed checkpoint" -S
```

### 5. View the timeline

```bash
kitsu timeline
```

### 6. Push to a remote registry

```bash
# SSH/SFTP (Sovereign Registry)
kitsu push origin main

# GitHub/GitLab (Git Bridge)
kitsu push origin main
```

---

## System Requirements

| Requirement | Details |
|-------------|---------|
| **Rust** | Stable toolchain, edition 2024 |
| **Linux** | `libssh2-1-dev`, `libssl-dev`, `pkg-config` |
| **macOS** | `libssh2`, `openssl`, `pkg-config` (via Homebrew) |
| **Windows** | No additional system dependencies |

---

## Project Status

Kitsu is in **alpha** (`0.0.1-alpha`). The following features are functional:

- ✅ Repository initialization with interactive wizard
- ✅ Content-addressable object storage (Chunk, Map, Checkpoint)
- ✅ File tracking and staging
- ✅ Checkpoint creation with optional Ed25519 signing
- ✅ Timeline history traversal
- ✅ Diff between checkpoints (line-level, colorized)
- ✅ Stream (branch) management
- ✅ Seal (tag) management with semantic versioning
- ✅ Export/Import via tar.gz archives
- ✅ Push/Pull via SSH/SFTP
- ✅ Git bridge for GitHub/GitLab push
- ✅ Multi-persona identity management
- ✅ Repository info, stats, verification
- ⚠️ `state` command (WIP)
- ⚠️ Git pull support (WIP)
- ⚠️ Merge/conflict resolution (not yet implemented)
