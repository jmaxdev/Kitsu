# Kitsu Documentation

Welcome to the technical documentation for the Kitsu version control system.

This documentation suite covers system architecture, data models, storage mechanisms, CLI interfaces, networking protocols, and cryptographic workflows.

---

## Documentation Index

| Section | Description |
|---|---|
| [Architecture](architecture.md) | High-level system architecture, monorepo workspace design, and module boundaries |
| [Object Model](object-model.md) | Deep dive into `Chunk`, `Map`, and `Checkpoint` data structures and serialization |
| [Storage Engine](storage-engine.md) | Content-addressable storage (CAS), zlib compression, and binary staging index format |
| [CLI Reference](cli-reference.md) | Exhaustive command-line interface documentation for all subcommands |
| [Remote & Networking](remote-and-networking.md) | Dual-mode networking: Sovereign SSH/SFTP transport and Git Bridge protocol |
| [Identity & Cryptography](identity-and-crypto.md) | Persona management and Ed25519 digital signature generation/verification |
| [Git Compatibility](git-compatibility.md) | Interoperability guide for GitHub, GitLab, and `.gitignore` file resolution |
| [Glossary](glossary.md) | Complete terminology mapping between Kitsu and Git/VCS standards |

---

## Technical Specifications Summary

- **Implementation Language**: Rust (Edition 2024, stable toolchain)
- **Hashing Algorithm**: SHA-256 (64-character hexadecimal digest)
- **Compression**: Deflate / zlib format via `flate2`
- **Digital Signatures**: Ed25519 via `ed25519-dalek` with OS entropy (`OsRng`)
- **Metadata Root**: `.kitsu/` directory in the repository root
- **Default Stream**: `main`
- **Default Remote Branch**: `kitsu-data` (on Git-backed remotes)
- **Binary Encoding**: Big-endian fixed-width integer fields for staging index
