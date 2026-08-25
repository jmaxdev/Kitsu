# Object Model and Data Structures

Kitsu is built upon an immutable, content-addressable directed acyclic graph (DAG) consisting of three fundamental object types: **Chunk**, **Map**, and **Checkpoint**.

Every object in Kitsu is identified by the SHA-256 hash of its serialized payload (including a type and length header).

---

## 1. Chunk

A **Chunk** represents raw, uninterpreted file content (analogous to a Git blob).

### Structure
```rust
pub struct Chunk {
    pub content: Vec<u8>,
}
```

### Canonical Hash Calculation
The SHA-256 hash of a chunk is computed as:
```text
SHA-256("chunk " + ascii_length(content) + "\0" + content)
```

### Properties
- Chunks contain only raw file bytes.
- File names, directory paths, and file permissions are not stored in chunks; they are stored in Maps.
- Deduplication occurs automatically: identical file contents across different files or commits result in the same chunk hash.

---

## 2. Map and MapEntry

A **Map** represents a directory listing (analogous to a Git tree). Maps form a hierarchical tree where entries can reference either Chunks (files) or nested Maps (subdirectories).

### Structures
```rust
pub struct MapEntry {
    pub mode: String,  // Octal file mode, e.g., "100644" for files, "40000" for directories
    pub name: String,  // Filename or directory name (without slashes)
    pub hash: String,  // 64-character hex SHA-256 hash
}

pub struct Map {
    pub entries: Vec<MapEntry>,
}
```

### Binary Serialization Format
Entries are sorted lexicographically by `name` before serialization to guarantee deterministic hashing. Each entry is encoded as follows:

```text
+-------------------+---+----------------------+---+--------------------+
| Octal Mode (ASCII)| ' '| Filename (UTF-8)    | \0| 32-byte Raw SHA-256|
+-------------------+---+----------------------+---+--------------------+
```

- `mode`: ASCII string (e.g., `"100644"`, `"40000"`).
- ` `: Single space character (`0x20`).
- `name`: UTF-8 encoded name of the file or subdirectory.
- `\0`: Null terminator byte (`0x00`).
- `hash`: 32 bytes representing the raw binary SHA-256 digest (decoded from the 64-character hex string).

---

## 3. Checkpoint

A **Checkpoint** represents an immutable snapshot of the entire repository state at a specific point in time (analogous to a Git commit).

### Structure
```rust
pub struct Checkpoint {
    pub map_hash: String,           // SHA-256 hash of the root Map
    pub parent_hash: Option<String>,// SHA-256 hash of the parent Checkpoint (None for initial)
    pub author: String,             // Author identity, format: "Name <email>"
    pub message: String,            // Human-readable commit message
    pub timestamp: i64,             // Unix timestamp in seconds
    pub signature: Option<String>,  // Hex-encoded Ed25519 signature
}
```

### Text Wire Format
Checkpoints use a line-oriented UTF-8 format:

```text
map <map_hash>\n
parent <parent_hash>\n        [optional: omitted if root checkpoint]
author <author_name_and_email> <timestamp>\n
curator <author_name_and_email> <timestamp>\n
signature <hex_ed25519_sig>\n [optional: omitted if unsigned]
\n
<multiline_commit_message>\n
```

### Example Checkpoint Payload
```text
map e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
parent 8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4
author Alice Smith <alice@example.com> 1700000000
curator Alice Smith <alice@example.com> 1700000000
signature d85b8...[128 hex chars]...

feat: implement storage backend compression
```

---

## 4. Object Graph Relationships

```
[ Checkpoint ] (Root snapshot)
      |
      +---> parent: [ Parent Checkpoint ]
      |
      +---> map_hash: [ Map: Root Directory ]
                           |
                           +---> Entry ("src", mode "40000") ---> [ Map: src/ ]
                           |                                           |
                           |                                           +---> Entry ("main.rs", mode "100644") ---> [ Chunk ]
                           |
                           +---> Entry ("Cargo.toml", mode "100644") ---> [ Chunk ]
```
