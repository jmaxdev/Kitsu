# Storage Engine and Staging Index

This document details Kitsu's content-addressable storage (CAS) architecture, loose-object on-disk layout, compression pipeline, and the binary staging index format.

---

## 1. Directory Layout

All repository metadata is stored in the `.kitsu/` directory at the project root:

```
.kitsu/
├── CURRENT                     # HEAD pointer ("stream: main\n" or raw checkpoint hash)
├── stage                       # Binary staging index file
├── default_remote              # Name of default remote (e.g., "origin\n")
├── identity.toml               # Local repository persona configuration (optional)
├── objects/                    # Content-addressable object store
│   ├── e3/                     # Two-character hex directory prefix
│   │   └── b0c44298fc1c...     # 62-character hex filename (remaining hash)
│   └── 8f/
│       └── 434346648f6b...
├── streams/                    # Stream (branch) pointers
│   ├── main                    # Checkpoint hash
│   └── feature-auth            # Checkpoint hash
├── seals/                      # Seal (version tag) pointers
│   ├── 0.1.0                   # Checkpoint hash
│   └── 1.0.0                   # Checkpoint hash
└── remotes/                    # Remote definitions
    └── origin                  # Remote URL (e.g., "https://github.com/..." or "ssh://...")
```

---

## 2. Content-Addressable Storage (CAS)

### Loose Object Storage
Objects are stored as zlib-compressed files using a two-level directory fan-out matching the first 2 characters of their SHA-256 hash:

```text
Path: .kitsu/objects/<hash[0..2]>/<hash[2..64]>
```

This prevents single directories from containing hundreds of thousands of files on filesystems with directory entry bottlenecks.

### Object Serialization Pipeline

When an object is written via `Storage::hash_and_write(obj_type, data)`:
1. **Header Assembly**: A header string is formatted: `"<type> <size>\0"` where `<type>` is `"chunk"`, `"map"`, or `"checkpoint"`, and `<size>` is the byte length of `data`.
2. **Full Payload**: `full_data = [header_bytes, data_bytes]`.
3. **Hashing**: `hash = hex(SHA-256(full_data))`.
4. **Compression**: `compressed_data = zlib_compress(full_data, Compression::default())`.
5. **Atomic Write**: If `.kitsu/objects/<hash[0..2]>/<hash[2..]>` does not exist, parent directories are created and `compressed_data` is written to disk.

---

## 3. Binary Staging Index (`.kitsu/stage`)

The staging area (`Stage`) tracks files that have been staged for inclusion in the next checkpoint.

### Binary Layout

```text
+-------------------------+
| Entry Count (u32, BE)   |
+-------------------------+
| Repeated per entry:     |
|   Path Length (u32, BE) |
|   Path Bytes (UTF-8)    |
|   Hash (64 bytes ASCII) |
|   File Mode (u32, BE)   |
|   File Size (u64, BE)   |
+-------------------------+
```

### Field Specifications
| Field | Type | Encoding | Description |
|---|---|---|---|
| `entry_count` | `u32` | Big-Endian (4 bytes) | Total number of staged files |
| `path_len` | `u32` | Big-Endian (4 bytes) | Byte length of the relative file path |
| `path` | `[u8; path_len]` | UTF-8 | Relative file path (e.g., `src/core/lib.rs`) |
| `hash` | `[u8; 64]` | ASCII (64 bytes) | Hex-encoded SHA-256 chunk hash |
| `mode` | `u32` | Big-Endian (4 bytes) | Unix file mode (`0o100644` for files, `0o40000` for dirs) |
| `size` | `u64` | Big-Endian (8 bytes) | File size in bytes at staging time |

### Recursive Map Conversion
When `Stage::write_map(&storage)` is called during `kitsu freeze`:
1. Staged paths containing path separators (`/` or `\`) are grouped by their top-level directory.
2. For each directory, a sub-stage is created and recursively converted into a sub-Map.
3. Root files and sub-Map hashes are combined into the root `Map` and saved to the object store.
4. The resulting root `Map` hash is returned for inclusion in the `Checkpoint`.
