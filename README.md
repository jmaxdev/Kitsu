<p align="center">
  <h1 align="center">🦊 Kitsu</h1>
  <p align="center">A modern version control system written in Rust</p>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/version-0.0.1--alpha-orange" alt="Version">
  <img src="https://img.shields.io/badge/rust-edition%202024-blue" alt="Rust Edition">
  <a href="LICENSE.md"><img src="https://img.shields.io/badge/license-UPL%201.0-green" alt="License"></a>
</p>

---

**Kitsu** is a from-scratch version control system that reimagines how developers track, snapshot, and share their code. Built entirely in Rust, it features content-addressable storage with SHA-256 hashing, Ed25519 cryptographic checkpoint signing, and dual-mode remote support (self-hosted SSH/SFTP or GitHub/GitLab bridge).

## ✨ Features

- 🔥 **Ignite** — Interactive repository initialization wizard
- 📦 **Content-Addressable Storage** — SHA-256 + zlib for deduplication and integrity
- 🔐 **Ed25519 Signatures** — Cryptographically sign checkpoints
- 🌊 **Streams** — Lightweight branch management
- 🔖 **Seals** — Semantic versioning with auto-bump support
- 🌐 **Sovereign Registry** — Host your own remote on any SSH server
- 🔗 **Git Bridge** — Push to GitHub/GitLab via a dedicated branch
- 👤 **Personas** — Multiple identity profiles with per-project or global scope
- 📤 **Export/Import** — Archive and transfer repositories as tar.gz

## 📦 Installation

Download the pre-compiled binaries for your operating system from the **[Releases page](https://github.com/jmaxdev/Kitsu/releases)**.

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

## 🚀 Quick Start

```bash
# Build from source
git clone https://github.com/jmaxdev/Kitsu.git
cd Kitsu
cargo build --release

# Initialize a repository
kitsu ignite

# Track and snapshot files
kitsu track src/main.rs README.md
kitsu freeze -m "Initial checkpoint"

# View history
kitsu timeline

# Create a version seal
kitsu seal -b patch
```

## 📖 Documentation

Full documentation is available in the [`doc/`](doc/README.md) directory:

| Document | Description |
|----------|-------------|
| [Architecture](doc/architecture.md) | System architecture and data flow |
| [CLI Reference](doc/cli-reference.md) | Complete command reference |
| [Object Model](doc/object-model.md) | Chunk, Map, Checkpoint internals |
| [Storage Engine](doc/storage-engine.md) | CAS, hashing, compression |
| [Identity & Crypto](doc/identity-and-crypto.md) | Ed25519 personas and signing |
| [Networking](doc/networking.md) | SSH/SFTP and Git bridge |
| [Repository Internals](doc/repository-internals.md) | `.kitsu/` directory structure |
| [Modules](doc/modules.md) | Rust module reference |
| [CI/CD](doc/ci-cd.md) | Test and release pipelines |
| [Contributing](doc/contributing.md) | How to contribute |
| [Glossary](doc/glossary.md) | Kitsu terminology |
| [License](LICENSE.md) | UnSetSoft Public License (UPL) 1.0 |

## 🛠️ System Requirements

| Platform | Dependencies |
|----------|-------------|
| **Linux** | `libssh2-1-dev`, `libssl-dev`, `pkg-config` |
| **macOS** | `libssh2`, `openssl`, `pkg-config` (Homebrew) |
| **Windows** | None (bundled libraries) |

Requires **Rust stable** toolchain (edition 2024).

## 📋 Status

Kitsu is in **alpha** (`0.0.1-alpha`). Core VCS operations are functional. Merge/conflict resolution and some advanced features are in development.

## 📄 License

Kitsu is licensed under the **[UnSetSoft Public License (UPL) 1.0](LICENSE.md)**.

- ✅ You may use **parts** of the code in other projects with proper attribution
- ❌ You may **not** distribute the original or modified versions
- ❌ You may **not** use it for commercial purposes
- ❌ You may **not** modify the code except for contributive purposes towards the original project

See [LICENSE.md](LICENSE.md) for the full terms.

## 🤝 Contributing

See [doc/contributing.md](doc/contributing.md) for setup instructions, code standards, and areas where help is needed.
