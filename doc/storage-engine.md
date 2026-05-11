# Storage Engine

The storage engine is the low-level persistence layer of Kitsu. It implements a **content-addressable store** (CAS) using SHA-256 hashing and zlib compression.

**Source:** `src/storage.rs` (134 lines)

---

## Concepts

### Content-Addressable Storage

In a CAS, every object is identified by the cryptographic hash of its content. This provides:

- **Deduplication** — If two files have identical content, they produce the same hash and are stored only once
- **Integrity** — Any modification to stored data changes the hash, making corruption detectable
- **Immutability** — Objects are write-once; they are never updated in place
- **Efficient sync** — When pushing/pulling, only objects whose hashes are not present on the target need to be transferred

### Object Format

Every object stored on disk follows this format:

```
┌─────────────────────────────────────────────────┐
│ zlib_compress(                                  │
│   "<type> <content_length>\0<content_bytes>"    │
│ )                                               │
└─────────────────────────────────────────────────┘
```

Where:
- `<type>` is one of: `chunk`, `map`, `checkpoint`
- `<content_length>` is the decimal string length of the content
- `\0` is a null byte separator
- `<content_bytes>` is the raw content (binary for Chunk/Map, text for Checkpoint)

The SHA-256 hash is computed over the **uncompressed** full data (header + content).

---

## Object Types

```rust
#[derive(Debug, Clone, Copy)]
pub enum ObjectType {
    Chunk,       // Raw file content
    Map,         // Directory tree
    Checkpoint,  // Snapshot with metadata
}
```

| Variant | String | Description |
|---------|--------|-------------|
| `Chunk` | `"chunk"` | Raw file content blob |
| `Map` | `"map"` | Directory listing with entries |
| `Checkpoint` | `"checkpoint"` | Snapshot referencing a Map with metadata |

### Type Conversion

```rust
impl ObjectType {
    pub fn as_str(&self) -> &str;           // Chunk → "chunk"
    pub fn from_str(s: &str) -> Result<Self>; // "chunk" → Chunk
}
```

---

## Disk Layout

Objects are stored in a two-level directory structure under `.kitsu/objects/`:

```
.kitsu/objects/
├── a1/
│   ├── b2c3d4e5f6...    ← object with hash starting with "a1"
│   └── f7e8d9c0b1...
├── 3f/
│   └── 2e1d0c9b8a...
└── ...
```

The first two characters of the hex hash form the **directory prefix**, and the remaining characters form the **filename**. This is the same sharding strategy Git uses, preventing any single directory from containing too many files.

---

## Storage Struct

```rust
pub struct Storage {
    root_dir: PathBuf,    // Project root directory
    config: AppConfig,    // Configuration (dir_name, objects_dir, etc.)
}
```

### Constructor

```rust
pub fn new(root_dir: PathBuf, config: AppConfig) -> Self
```

Creates a new `Storage` instance. Does not create any directories — the caller (`ignite`, `copy`) is responsible for directory creation.

---

## Core Operations

### `get_object_path(hash) → PathBuf`

Computes the filesystem path for a given object hash.

```rust
pub fn get_object_path(&self, hash: &str) -> PathBuf {
    let (dir, file) = hash.split_at(2);
    self.root_dir
        .join(&self.config.dir_name)    // .kitsu
        .join(&self.config.objects_dir)  // objects
        .join(dir)                       // first 2 hex chars
        .join(file)                      // remaining chars
}
```

**Example:** Hash `a1b2c3d4...` → `.kitsu/objects/a1/b2c3d4...`

---

### `hash_and_write(obj_type, data) → Result<String>`

The primary write operation. Hashes raw content, wraps it with a typed header, compresses it, and writes to disk.

**Algorithm:**

```
1. header = "<type> <data.len()>\0"
2. full_data = header + data
3. hash = SHA-256(full_data)           → 64-char hex string
4. compressed = zlib_compress(full_data)
5. path = get_object_path(hash)
6. if !path.exists():
7.     create parent directories
8.     write compressed data to path
9. return hash
```

**Key behaviors:**
- **Idempotent** — If an object with the same hash already exists, it is NOT overwritten
- **Auto-sharding** — Parent directories (the 2-char prefix dir) are created automatically
- Returns the hex-encoded SHA-256 hash

---

### `read_object(hash) → Result<(ObjectType, Vec<u8>)>`

Reads and decompresses an object, returning its type and raw content.

**Algorithm:**

```
1. path = get_object_path(hash)
2. compressed_data = read_file(path)
3. full_data = zlib_decompress(compressed_data)
4. Find null byte position in full_data
5. header = full_data[..null_pos]        → "chunk 1234"
6. Parse header: split by space → [type, length]
7. content = full_data[null_pos+1..]
8. return (ObjectType, content)
```

**Note:** The returned content does NOT include the header — only the raw content bytes.

---

### `write_raw(hash, full_data) → Result<(ObjectType, Vec<u8>)>`

Writes pre-formatted object data (header + content already assembled). Used during `import` and `pull` operations where the data arrives already in object format.

**Algorithm:**

```
1. compressed = zlib_compress(full_data)
2. path = get_object_path(hash)
3. if !path.exists():
4.     create parent directories
5.     write compressed data to path
6. Parse header from full_data (same as read_object)
7. return (ObjectType, content)
```

**Difference from `hash_and_write`:** This method does NOT compute the hash — it trusts the provided hash. The caller is responsible for hash correctness.

---

## Compression

Kitsu uses **zlib** compression (via the `flate2` crate) for all objects:

| Operation | Crate API |
|-----------|-----------|
| Compress | `ZlibEncoder::new(Vec::new(), Compression::default())` |
| Decompress | `ZlibDecoder::new(&compressed_data[..])` |

The default compression level is used (`Compression::default()`), which balances speed and compression ratio.

### Why zlib?

- Same algorithm as Git, proven for VCS workloads
- Good compression ratio for text files (source code)
- Fast decompression for read-heavy workflows
- The `flate2` crate is mature and well-maintained

---

## Hash Algorithm

Kitsu uses **SHA-256** (via the `sha2` crate) for all object hashing:

- Hash length: 32 bytes → 64 hex characters
- The hash covers the full object (header + content), not just the content
- This means the same file content hashed as a `Chunk` and a `Map` would produce different hashes (due to different headers)

---

## Deduplication in Practice

```
File A: "hello world" → hash: abc123...
File B: "hello world" → hash: abc123... (same!)

Only one object is stored at .kitsu/objects/ab/c123...
Both stage entries reference the same hash.
```

When `hash_and_write` is called:
1. It computes the hash
2. Checks if `get_object_path(hash)` already exists
3. If it exists → skip writing, just return the hash
4. If it doesn't exist → compress and write

---

## Tests

### `test_storage_write_read`

Uses `tempfile::tempdir()` to create an isolated test environment:

1. Creates a `Storage` instance with default config
2. Writes a `Chunk` with content `"kitsu storage test content"`
3. Reads the object back by its hash
4. Asserts the type is `Chunk` and the content matches
