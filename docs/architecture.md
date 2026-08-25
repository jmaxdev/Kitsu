# System Architecture

Kitsu is structured as a Cargo workspace monorepo, separating core version control engine logic from command-line interface presentation, interactive assistants, and self-update routines.

---

## 1. Monorepo Organization

The codebase is partitioned into two primary crates located in the `src/` directory:

```
Kitsu/
├── Cargo.toml                  # Workspace manifest
├── docs/                       # Comprehensive documentation
└── src/
    ├── core/                   # Crate: 'core' (library)
    │   ├── Cargo.toml
    │   ├── src/
    │   │   ├── lib.rs          # Re-exports and top-level public interface
    │   │   ├── error.rs        # Typed error definitions (KitsuError)
    │   │   ├── config.rs       # Constants and configuration layout
    │   │   ├── repository.rs   # Central orchestrator (Repository struct)
    │   │   ├── state.rs        # Working state computation engine
    │   │   ├── exclude.rs      # Unified file exclusion engine (.gitignore + .exclude)
    │   │   ├── update.rs       # Update checker & in-place binary self-updater
    │   │   ├── objects/        # Core object model (Chunk, Map, Checkpoint)
    │   │   │   ├── mod.rs
    │   │   │   ├── chunk.rs
    │   │   │   ├── map.rs
    │   │   │   └── checkpoint.rs
    │   │   ├── storage/        # Storage backend and index persistence
    │   │   │   ├── mod.rs
    │   │   │   ├── backend.rs
    │   │   │   └── index.rs
    │   │   ├── refs/           # Reference management (HEAD, Stream, Seal)
    │   │   │   ├── mod.rs
    │   │   │   ├── head.rs
    │   │   │   ├── stream.rs
    │   │   │   └── seal.rs
    │   │   ├── diff/           # Tree diffing and delta formatting
    │   │   │   ├── mod.rs
    │   │   │   └── engine.rs
    │   │   ├── identity/       # Personas and cryptographic operations (Ed25519)
    │   │   │   ├── mod.rs
    │   │   │   ├── crypto.rs
    │   │   │   └── persona.rs
    │   │   └── remote/         # Remote synchronization protocols
    │   │       ├── mod.rs
    │   │       ├── git_bridge.rs   # GitHub/GitLab bridge with custom branch
    │   │       ├── local_bridge.rs # Local filesystem transport
    │   │       ├── transport.rs    # Sovereign SSH/SFTP transport
    │   │       └── registry.rs     # Remote configuration manager
    │   └── tests/
    │       └── repo_integration.rs # End-to-end repository lifecycle integration tests
    │
    └── cli/                    # Crate: 'cli' (binary: 'kitsu')
        ├── Cargo.toml
        └── src/
            ├── main.rs         # Command-line entrypoint and update banner check
            ├── app.rs          # Clap CLI definition and schema
            └── commands/       # Decoupled command handlers (one module per subcommand)
                ├── ignite.rs
                ├── copy.rs
                ├── track.rs
                ├── freeze.rs
                ├── timeline.rs
                ├── diff.rs
                ├── rollback.rs
                ├── seal.rs
                ├── switch.rs
                ├── export.rs
                ├── import.rs
                ├── push.rs
                ├── pull.rs
                ├── contents.rs
                ├── hash.rs
                ├── repository.rs
                ├── persona.rs
                ├── burn.rs
                ├── state.rs
                ├── peek.rs
                └── update.rs
```

---

## 2. Layered Architecture

The system operates across three conceptual layers:

```
+-------------------------------------------------------------+
|                      User / CLI Layer                       |
|               (src/cli: Clap, Output, Prompts)              |
+-------------------------------------------------------------+
                              |
                              v
+-------------------------------------------------------------+
|                   Repository Orchestrator                   |
|          (src/core/repository.rs, state.rs, diff.rs)         |
+-------------------------------------------------------------+
         |                    |                    |
         v                    v                    v
+-----------------+  +-----------------+  +------------------+
|   Object Model  |  | Reference Engine|  | Networking Layer |
| (Chunk/Map/CP)  |  |  (HEAD/Streams) |  |(Git/Local/SSH)   |
+-----------------+  +-----------------+  +------------------+
         \                    |                    /
          +-------------------+-------------------+
                              |
                              v
+-------------------------------------------------------------+
|                    Storage Backend Layer                    |
|      (SHA-256 CAS, zlib compression, Binary Stage Index)    |
+-------------------------------------------------------------+
```

### Layer Responsibilities

1. **Storage Layer (`storage/`)**:
   - Manages content-addressable storage on the local filesystem.
   - Provides SHA-256 computation, loose-object directory fan-out (`objects/xx/yy...`), and zlib stream compression/decompression.
   - Manages the binary staging index (`.kitsu/stage`).

2. **Object Layer (`objects/`)**:
   - Defines the core immutable primitives: `Chunk` (files), `Map` (directories), and `Checkpoint` (snapshots).
   - Encapsulates binary and text serialization and deserialization formats.

3. **Reference Layer (`refs/`)**:
   - Resolves and updates mutable pointers: HEAD, Streams (branches), and Seals (semantic version tags).
   - Handles auto-increment version calculations including prereleases (`alpha`, `beta`, `rc`, `alpha.0`).
   - Resolves user-provided reference selectors (`~N`, `#N`, stream names, seal tags, or raw hashes).

4. **Remote & Networking Layer (`remote/`)**:
   - Provides three transport strategies: Git Bridge (`git2`) with customizable data branch, Local directory bridge (`LocalBridge`), and Sovereign SSH/SFTP (`ssh2`).

5. **Self-Update Engine (`update.rs`)**:
   - Periodically checks official GitHub Releases (`https://github.com/jmaxdev/Kitsu`) with local disk caching.
   - Performs atomic in-place binary self-replacement via `self-replace`.

6. **CLI Layer (`src/cli/`)**:
   - Parses command-line flags and parameters via Clap.
   - Handles interactive user prompts (via `dialoguer`) and terminal formatting (via `colored`).
   - Displays non-intrusive update banners.
   - Delegates all business logic to `core`.

---

## 3. Error Handling Model

All library errors in `src/core` are strongly typed using the `thiserror` crate (`KitsuError`), providing granular diagnostics without opaque string errors:

- `ObjectNotFound { hash }`
- `InvalidObjectFormat`
- `CorruptObject { hash }`
- `NoHead`
- `NoParent`
- `StreamNotFound { name }`
- `SealNotFound { name }`
- `AuthenticationFailed { user }`
- `RemoteError(String)`
- `IdentityNotFound { id }`
- `NoPrivateKey`
- `IndexOutOfBounds { index, max }`
- `RepositoryNotFound { path }`
- `RepositoryAlreadyExists { path }`
- `UnknownObjectType(String)`
