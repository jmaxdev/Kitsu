# Contributing to Kitsu

Thank you for your interest in contributing to Kitsu! This guide covers everything you need to set up your development environment and start contributing.

> **Important:** Kitsu is licensed under the [UnSetSoft Public License (UPL) 1.0](../LICENSE.md). By contributing, you agree that your changes are **contributive modifications** towards the original project — the only type of modification permitted under this license.

---

## Prerequisites

### Rust Toolchain

Kitsu requires the **Rust stable toolchain** with edition 2024 support:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Ensure you have the latest stable toolchain
rustup update stable

# Install required components
rustup component add clippy rustfmt
```

### System Dependencies

#### Linux (Ubuntu/Debian)

```bash
sudo apt-get update
sudo apt-get install -y libssh2-1-dev libssl-dev pkg-config
```

#### macOS

```bash
brew update
brew install libssh2 openssl pkg-config
```

#### Windows

No additional system dependencies are required. The `ssh2` crate uses bundled libraries on Windows.

---

## Getting Started

### Clone the Repository

```bash
git clone https://github.com/jmaxdev/Kitsu.git
cd Kitsu
```

### Build

```bash
# Debug build (faster compilation)
cargo build

# Release build (optimized)
cargo build --release
```

### Run Tests

```bash
cargo test --verbose
```

### Run the Binary

```bash
# Debug
cargo run -- ignite

# Release
cargo run --release -- ignite

# Or directly after building
./target/debug/kitsu ignite
./target/release/kitsu ignite
```

---

## Project Structure

```
kitsu/
├── Cargo.toml          ← Package manifest and dependencies
├── Cargo.lock          ← Locked dependency versions
├── build.rs            ← Build script (extracts compile-time constants)
├── LICENSE.md          ← UnSetSoft Public License (UPL) 1.0
├── .gitignore          ← Git ignore rules
├── .github/
│   └── workflows/
│       ├── test.yml        ← CI: build, test, lint on push to dev
│       └── production.yml  ← CD: build releases on tag push
├── src/
│   ├── main.rs         ← CLI entry point and command dispatcher (1116 LOC)
│   ├── config.rs       ← Build-time constants and AppConfig (42 LOC)
│   ├── objects.rs      ← Object model: Chunk, Map, Checkpoint (175 LOC)
│   ├── storage.rs      ← Content-addressable storage engine (134 LOC)
│   ├── index.rs        ← Staging area and tree construction (131 LOC)
│   ├── diff.rs         ← Recursive Map diff with colored output (83 LOC)
│   ├── identity.rs     ← Ed25519 persona management (116 LOC)
│   ├── remote.rs       ← SSH/SFTP remote operations (123 LOC)
│   └── exclude.rs      ← Gitignore-compatible exclusion patterns (28 LOC)
└── doc/                ← Documentation
```

---

## Code Quality Standards

### Formatting

All code must pass `rustfmt` formatting:

```bash
# Check formatting (same as CI)
cargo fmt --all -- --check

# Auto-fix formatting
cargo fmt --all
```

### Linting

All code must pass Clippy with zero warnings:

```bash
# Check with strict mode (same as CI)
cargo clippy -- -D warnings
```

If Clippy reports a false positive, you can suppress it with an attribute:

```rust
#[allow(clippy::some_lint_name)]
```

However, please include a comment explaining why the suppression is necessary.

### Testing

Run the full test suite before submitting:

```bash
cargo test --verbose
```

#### Writing Tests

Tests live inside their respective modules using `#[cfg(test)]` blocks:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_my_feature() {
        // Use tempfile for filesystem tests
        let dir = tempfile::tempdir().unwrap();
        // ...
    }
}
```

For tests requiring a temporary directory, use the `tempfile` crate (already in `[dev-dependencies]`).

---

## Build System

### `build.rs`

The build script extracts configuration from `Cargo.toml` and exposes it as compile-time environment variables:

| Variable | Source | Description |
|----------|--------|-------------|
| `APP_NAME` | `package.name` | Binary name for CLI help |
| `DIR_NAME` | `package.metadata.kitsu.dir_name` | Repository directory name |
| `ABOUT` | `package.description` | CLI description text |

These are accessed in code via `env!("APP_NAME")` etc.

The build script also sets `cargo:rerun-if-changed=Cargo.toml` to rebuild when the manifest changes.

### `Cargo.toml` Metadata

Custom Kitsu configuration lives under `[package.metadata.kitsu]`:

```toml
[package.metadata.kitsu]
dir_name = ".kitsu"
```

---

## Branching Strategy

| Branch | Purpose | CI |
|--------|---------|-----|
| `dev` | Main development branch | Test pipeline (build + test + fmt + clippy) |
| `v*` tags | Release versions | Production pipeline (multi-platform build + release) |

### Workflow

1. Create a feature branch from `dev`
2. Make your changes
3. Ensure `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test` pass
4. Open a PR targeting `dev`
5. CI will run the test pipeline automatically

---

## Key Dependencies

| Crate | Purpose | Docs |
|-------|---------|------|
| `clap` | CLI argument parsing with derive macros | [docs.rs](https://docs.rs/clap) |
| `sha2` | SHA-256 hashing | [docs.rs](https://docs.rs/sha2) |
| `flate2` | Zlib compression | [docs.rs](https://docs.rs/flate2) |
| `serde` + `toml` | TOML serialization for config and identity | [docs.rs](https://docs.rs/serde) |
| `ssh2` | SSH/SFTP client | [docs.rs](https://docs.rs/ssh2) |
| `ed25519-dalek` | Ed25519 signatures | [docs.rs](https://docs.rs/ed25519-dalek) |
| `git2` | libgit2 bindings for Git bridge | [docs.rs](https://docs.rs/git2) |
| `similar` | Text diff algorithm | [docs.rs](https://docs.rs/similar) |
| `colored` | ANSI terminal colors | [docs.rs](https://docs.rs/colored) |
| `chrono` | UTC timestamps | [docs.rs](https://docs.rs/chrono) |
| `semver` | Semantic version parsing | [docs.rs](https://docs.rs/semver) |
| `ignore` | Gitignore pattern matching | [docs.rs](https://docs.rs/ignore) |
| `dialoguer` | Interactive terminal prompts | [docs.rs](https://docs.rs/dialoguer) |
| `walkdir` | Recursive directory walking | [docs.rs](https://docs.rs/walkdir) |
| `tar` | Tar archive creation | [docs.rs](https://docs.rs/tar) |
| `dirs` | Home directory resolution | [docs.rs](https://docs.rs/dirs) |
| `rpassword` | Hidden password input | [docs.rs](https://docs.rs/rpassword) |
| `hex` | Hex encoding/decoding | [docs.rs](https://docs.rs/hex) |
| `rand` + `rand_core` | Cryptographic RNG | [docs.rs](https://docs.rs/rand) |
| `anyhow` | Ergonomic error handling | [docs.rs](https://docs.rs/anyhow) |

### Dev Dependencies

| Crate | Purpose |
|-------|---------|
| `tempfile` | Temporary directories for tests |

---

## Areas for Contribution

### Known WIP Features

- **`kitsu state`** — Working tree status comparison (currently a placeholder)
- **Git pull support** — Pulling from GitHub/GitLab is not yet implemented
- **Merge/conflict resolution** — No merge strategy exists yet
- **Partial staging** — `kitsu track .` for recursive add

### Improvement Opportunities

- **Error handling** — Replace `unwrap()` calls with proper error propagation
- **Unit tests** — Expand test coverage beyond `objects.rs` and `storage.rs`
- **Integration tests** — End-to-end tests for CLI commands
- **Performance** — Parallel object pushing/pulling
- **Documentation** — Inline rustdoc comments on public APIs
- **Cross-platform** — Test and fix path handling for Windows

---

## License

Kitsu is released under the **[UnSetSoft Public License (UPL) 1.0](../LICENSE.md)**.

### What this means for contributors

| Allowed | Not Allowed |
|---------|-------------|
| ✅ Use parts of the code with attribution | ❌ Distribute original or modified versions |
| ✅ Contribute modifications back to the project | ❌ Fork or create derivative works |
| ✅ Use for personal, non-commercial projects | ❌ Use for commercial purposes |
| | ❌ Use Kitsu's brand or trademarks |

**Key rules:**

1. **Contributive-only modifications** — You may only modify the source code for the purpose of contributing back to this repository. Private forks for redistribution are not permitted.
2. **Source disclosure** — If you use parts of Kitsu's code in another project, you must include a link to the original repository.
3. **License retention** — All redistributed files must retain the UPL license and copyright notice.
4. **No warranty** — The software is provided "as is" with no warranties.

By submitting a pull request, you confirm that your contribution complies with these terms.
