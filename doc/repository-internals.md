# Repository Internals

This document describes the on-disk structure of a Kitsu repository — the `.kitsu/` directory, its files, and their formats.

---

## Directory Layout

After running `kitsu ignite`, the following structure is created:

```
project/
├── .kitsu/                    ← Repository metadata directory
│   ├── CURRENT                ← HEAD pointer (active stream or detached hash)
│   ├── stage                  ← Binary staging area
│   ├── objects/               ← Content-addressable object store
│   │   └── <xx>/              ← 2-char hash prefix directories
│   │       └── <rest...>      ← Compressed object files
│   ├── streams/               ← Branch pointers
│   │   └── main               ← Default stream
│   ├── seals/                 ← Version tags
│   ├── remotes/               ← Remote registry URLs
│   │   └── origin             ← Default remote
│   ├── default_remote         ← Name of the default remote
│   ├── identity.toml          ← Local persona configuration
│   └── git_bridge/            ← Git helper repo (created on first Git push)
├── .exclude                   ← Kitsu ignore patterns (optional)
└── (project files)
```

---

## CURRENT File

The `CURRENT` file acts as Kitsu's **HEAD** pointer. It determines which checkpoint the working tree is based on.

### Attached Mode (Stream)

When working on a stream (branch), CURRENT contains:

```
stream: main
```

Format: `stream: <stream_name>\n`

In this mode, new checkpoints automatically update the stream pointer. This is the default state after `kitsu ignite`.

### Detached Mode

When pointing to a specific checkpoint (not a stream), CURRENT contains the raw hash:

```
a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef12345678
```

This happens when you `kitsu switch` to a seal or a direct hash.

### Resolution Logic

```rust
fn get_head_hash(current_dir: &Path, config: &AppConfig) -> Result<Option<String>> {
    let content = fs::read_to_string(current_path)?;
    if content.starts_with("stream: ") {
        // Read the hash from .kitsu/streams/<stream_name>
        let stream = content.trim_start_matches("stream: ").trim();
        let path = repo_dir.join(config.streams_dir).join(stream);
        // Return hash from stream file, or None if stream doesn't exist yet
    } else {
        // Detached mode: content IS the hash
        Ok(Some(content.trim().to_string()))
    }
}
```

---

## Stage (Staging Area)

The stage file is a **binary format** that tracks which files are ready for the next checkpoint.

**Location:** `.kitsu/stage`

### Binary Format

```
┌──────────────────────────────────────────────────┐
│ u32 (big-endian): entry count                    │
├──────────────────────────────────────────────────┤
│ For each entry:                                  │
│   u32 (big-endian): path length in bytes         │
│   [u8; path_len]:   UTF-8 path string            │
│   [u8; 64]:         SHA-256 hash as ASCII hex     │
│   u32 (big-endian): file mode                    │
│   u64 (big-endian): file size in bytes           │
├──────────────────────────────────────────────────┤
│ (repeat for each entry)                          │
└──────────────────────────────────────────────────┘
```

### Entry Fields

| Field | Size | Encoding | Description |
|-------|------|----------|-------------|
| Path length | 4 bytes | `u32` big-endian | Length of the path string |
| Path | Variable | UTF-8 | Relative path from project root |
| Hash | 64 bytes | ASCII hex | SHA-256 hash of the Chunk object |
| Mode | 4 bytes | `u32` big-endian | File mode: `0o100644` (file) or `0o40000` (dir) |
| Size | 8 bytes | `u64` big-endian | File size in bytes |

### Stage Operations

| Operation | Method | Description |
|-----------|--------|-------------|
| Load | `Stage::load()` | Reads and parses the binary stage file |
| Add | `Stage::add()` | Inserts or updates an entry in the BTreeMap |
| Save | `Stage::save()` | Serializes and writes the binary stage file |
| Build tree | `Stage::write_map()` | Converts flat entries into a hierarchical Map tree |

### Tree Building Algorithm

The `write_map()` method converts flat paths into a hierarchical Map structure:

```
Input (flat):
  "src/main.rs"     → hash1
  "src/config.rs"   → hash2
  "README.md"       → hash3

Step 1 - Group by first path component:
  Root entries: ["README.md"]
  "src" group: ["main.rs", "config.rs"]

Step 2 - Recursive Map creation:
  Map(src/) = hash_and_write(Map, [main.rs→hash1, config.rs→hash2])
  Map(root) = hash_and_write(Map, [README.md→hash3, src→Map(src/).hash])
```

---

## Objects Directory

The object store uses a **two-level sharding** scheme:

```
.kitsu/objects/
├── a1/
│   ├── b2c3d4e5f6...   ← Full path: a1b2c3d4e5f6...
│   └── f7e8d9c0b1...
├── 3f/
│   └── 2e1d0c9b8a...
└── ff/
    └── 0011223344...
```

- First 2 hex chars of hash → directory name
- Remaining 62 hex chars → file name
- Each file contains **zlib-compressed** object data
- Objects are write-once (never modified or overwritten)

See [Storage Engine](storage-engine.md) for detailed format documentation.

---

## Streams Directory

Streams are Kitsu's equivalent of **Git branches**. Each stream is a simple text file containing a checkpoint hash.

```
.kitsu/streams/
├── main             ← "a1b2c3d4e5f6...\n"
├── feature-auth     ← "9f8e7d6c5b4a...\n"
└── hotfix           ← "3a2b1c0d9e8f...\n"
```

When a new checkpoint is created while on a stream, the stream file is updated with the new hash. This is how Kitsu "advances" a branch.

### Stream Operations

| Command | Effect |
|---------|--------|
| `kitsu repository stream new <name>` | Creates a stream file pointing to HEAD |
| `kitsu repository stream list` | Lists all files in `streams/` |
| `kitsu repository stream rename <old> <new>` | Renames the file |
| `kitsu repository stream delete <name>` | Deletes the file |
| `kitsu switch <stream>` | Sets CURRENT to `stream: <name>` |

---

## Seals Directory

Seals are Kitsu's equivalent of **Git tags**, specifically for **semantic versions**.

```
.kitsu/seals/
├── 0.1.0           ← "a1b2c3d4e5f6...\n"
├── 1.0.0           ← "9f8e7d6c5b4a...\n"
└── 1.0.1           ← "3a2b1c0d9e8f...\n"
```

Each seal file is named with a semver string and contains the checkpoint hash it points to.

### Seal Operations

| Command | Effect |
|---------|--------|
| `kitsu seal <version>` | Creates a seal file pointing to HEAD |
| `kitsu seal -b <major\|minor\|patch>` | Auto-bumps from latest version |
| `kitsu seal -l` | Lists all seals sorted by version |

---

## Remotes Directory

Remote registries are stored as individual files:

```
.kitsu/remotes/
├── origin           ← "ssh://root@server.com/opt/vcontrol/repo"
└── github           ← "https://github.com/user/repo.git"
```

Each file's name is the remote identifier, and its content is the URL.

---

## Default Remote

```
.kitsu/default_remote    ← "origin"
```

A single-line text file containing the name of the default remote. If absent, `"origin"` is assumed.

---

## Identity File

```
.kitsu/identity.toml
```

Local persona configuration in TOML format. See [Identity & Cryptography](identity-and-crypto.md) for the full format specification.

---

## Git Bridge

```
.kitsu/git_bridge/       ← Created on first `kitsu push` to a Git URL
```

A standard Git repository used as an intermediary for pushing objects to GitHub/GitLab. See [Networking](networking.md) for details.

---

## Exclude File

```
.exclude                 ← Project root, NOT inside .kitsu/
```

A gitignore-compatible file for specifying paths to exclude from tracking. Kitsu uses the `ignore` crate to parse these patterns.

### Built-in Exclusions

In addition to patterns in `.exclude`, Kitsu always ignores:

| Pattern | Reason |
|---------|--------|
| `.kitsu` (or configured `DIR_NAME`) | The VCS directory itself |
| `.git` | Git metadata if present |
| `target` | Rust build output |

### Example `.exclude`

```gitignore
# Build artifacts
target/
dist/

# IDE files
.idea/
.vscode/

# OS files
.DS_Store
Thumbs.db

# Temporary files
*.tmp
*.swp
```

---

## Configuration System

### Build-time Constants

The `build.rs` script extracts values from `Cargo.toml` at compile time:

| Env Variable | Source | Default | Usage |
|-------------|--------|---------|-------|
| `APP_NAME` | `package.name` | `"kitsu"` | CLI name in help text |
| `DIR_NAME` | `package.metadata.kitsu.dir_name` | `".kitsu"` | Repository directory name |
| `ABOUT` | `package.description` | `"A modern VCS"` | CLI description |

### AppConfig Struct

```rust
pub struct AppConfig {
    pub app_name: String,      // "kitsu"
    pub about: String,         // "A modern version control system writed in Rust"
    pub dir_name: String,      // ".kitsu"
    pub stage_file: String,    // "stage"
    pub current_file: String,  // "CURRENT"
    pub streams_dir: String,   // "streams"
    pub objects_dir: String,   // "objects"
}
```

All values are currently hardcoded defaults. The `load()` method returns `AppConfig::default()`. Runtime configuration loading is planned for a future version.
