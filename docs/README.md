# Kitsu Documentation

Welcome to the technical documentation for the Kitsu version control system.

This documentation suite covers system architecture, data models, storage mechanisms, CLI interfaces, networking protocols, update mechanisms, and cryptographic workflows.

---

## Documentation Index

| Section | Description |
|---|---|
| [Architecture](architecture.md) | High-level system architecture, monorepo workspace design, error model, and module boundaries |
| [Object Model](object-model.md) | Deep dive into `Chunk`, `Map`, and `Checkpoint` data structures and serialization |
| [Storage Engine](storage-engine.md) | Content-addressable storage (CAS), zlib compression, and binary staging index format |
| [CLI Reference](cli-reference.md) | Exhaustive command-line interface documentation for all subcommands (including `update` and `seal`) |
| [Remote & Networking](remote-and-networking.md) | Triple-mode networking: Git Bridge (customizable data branch), Local filesystem remotes, and Sovereign SSH/SFTP |
| [Identity & Cryptography](identity-and-crypto.md) | Persona management and Ed25519 digital signature generation/verification |
| [Git Compatibility](git-compatibility.md) | Interoperability guide for GitHub, GitLab, custom data branches, and dual `.gitignore` / `.exclude` resolution |
| [Glossary](glossary.md) | Complete terminology mapping between Kitsu and Git/VCS standards |

---

## Technical Specifications Summary

- **Implementation Language**: Rust (Edition 2024, stable toolchain)
- **Workspace Architecture**: Cargo monorepo partitioned into core library (`core`) and binary interface (`cli`)
- **Hashing Algorithm**: SHA-256 (64-character hexadecimal digest)
- **Compression**: Deflate / zlib format via `flate2`
- **Digital Signatures**: Ed25519 via `ed25519-dalek` with OS entropy (`OsRng`)
- **Metadata Root**: `.kitsu/` directory in the repository root
- **Default Stream**: `main`
- **Default Remote Branch**: `kitsu-data` (configurable per remote or command)
- **Transport Backends**: Git Bridge (`git2`), Local Filesystem (`LocalBridge`), Sovereign SSH/SFTP (`ssh2`)
- **Self-Update**: Atomic in-place executable replacement via `self-replace` from official GitHub Releases
- **Binary Encoding**: Big-endian fixed-width integer fields for staging index
