# Glossary

Kitsu uses its own terminology to distinguish itself from Git. This glossary maps every Kitsu-specific term to its meaning and, where applicable, its Git equivalent.

---

## Core Concepts

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Ignite** | `git init` | Initialize a new Kitsu repository with an interactive setup wizard |
| **Track** | `git add` | Stage files for the next checkpoint by hashing and recording them |
| **Freeze** | `git commit` | Create an immutable snapshot (checkpoint) of all staged files |
| **Timeline** | `git log` | View the chronological history of checkpoints |
| **Rollback** | `git reset --hard` | Restore the working tree to a previous checkpoint |
| **Switch** | `git checkout` / `git switch` | Change the working tree to a different stream or checkpoint |
| **Copy** | `git clone` | Clone a repository from a remote registry |
| **Push** | `git push` | Upload objects and seals to a remote registry |
| **Pull** | `git pull` / `git fetch` | Download objects and seals from a remote registry |

---

## Data Model

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Chunk** | Blob | The most basic object type — stores the raw bytes of a single file |
| **Map** | Tree | A directory listing — contains entries pointing to Chunks (files) or other Maps (subdirectories) |
| **Checkpoint** | Commit | A snapshot object that references a root Map and includes metadata (author, message, timestamp, optional signature) |
| **Map Entry** | Tree entry | A single entry in a Map: mode + name + hash of the referenced object |

---

## Branching & Versioning

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Stream** | Branch | A named pointer to a checkpoint hash, representing a line of development. Automatically advances when new checkpoints are created while the stream is active |
| **Seal** | Tag | A named version marker pointing to a specific checkpoint. Uses semantic versioning (e.g., `1.0.0`). Seals are immutable once created |
| **Bump** | — | Auto-increment a seal version: `major`, `minor`, or `patch` |

---

## Repository Structure

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **CURRENT** | HEAD | A file that determines the active stream or checkpoint. Contains either `stream: <name>` (attached mode) or a raw hash (detached mode) |
| **Stage** | Index / Staging area | A binary file tracking which files are prepared for the next checkpoint. Updated by `kitsu track` |
| **Object Store** | `.git/objects/` | The `.kitsu/objects/` directory containing all content-addressable objects, sharded by hash prefix |
| **Attached Mode** | On a branch | When CURRENT contains `stream: <name>`, new checkpoints update the stream pointer |
| **Detached Mode** | Detached HEAD | When CURRENT contains a raw hash, new checkpoints don't update any stream |

---

## Identity & Security

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Persona** | User config (`user.name` / `user.email`) | A named identity with display name, email, and Ed25519 key pair. Multiple personas can be configured, one is always active |
| **Persona Store** | `.gitconfig` | TOML file containing all personas and the active persona ID. Can be local (`.kitsu/identity.toml`) or global (`~/.kitsu_identity.toml`) |
| **Signed Checkpoint** | Signed commit (`git commit -S`) | A checkpoint with an Ed25519 signature embedded in the `signature` field, created with `kitsu freeze -S` |
| **Curator** | Committer | The identity recorded as the curator of a checkpoint. Currently always the same as the author (reserved for future use) |

---

## Networking

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Sovereign Registry** | Self-hosted Git server | A remote object store accessible via SSH/SFTP on any server. No special server software needed — just SSH access to a filesystem |
| **Git Bridge** | — | A helper mechanism that wraps Kitsu objects in a Git repository and pushes them to a `vcontrol-data` branch on GitHub/GitLab |
| **Beam** | Remote (shorthand) | A shorthand command group (`kitsu beam`) for managing remote registries |
| **Registry** | Remote | A remote location where objects can be pushed to and pulled from |
| **Remote** | Remote | A named reference to a registry URL, stored in `.kitsu/remotes/` |

---

## Object Storage

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Content-Addressable Storage (CAS)** | Git object model | A storage paradigm where every object is identified by the cryptographic hash of its content, ensuring deduplication and integrity |
| **Object Hash** | SHA-1 hash (Git) | A 64-character hexadecimal SHA-256 hash that uniquely identifies an object |
| **Object Header** | Git object header | The type and length prefix prepended to object content: `"<type> <length>\0"` |
| **Hash Prefix** | Loose object directory | The first 2 characters of an object hash, used as a directory name for sharding |
| **Reachable Objects** | — | All objects that can be reached by traversing the graph from a given checkpoint (Checkpoint → Map → Chunk) |

---

## Commands (Unique to Kitsu)

| Term | Git Equivalent | Definition |
|------|----------------|------------|
| **Burn** | — | Delete an object from the local object store. Destructive and irreversible |
| **Peek** | `git cat-file -p` | Display the raw content of a stored object by its hash |
| **Contents** | `git ls-tree -r` | List all files in a checkpoint's Map tree with mode, hash, size, and name |
| **State** | `git status` | (WIP) Show the working tree state compared to the last checkpoint |
| **Export** | `git bundle create` | Package a checkpoint and all its reachable objects into a compressed tar.gz archive |
| **Import** | `git bundle unbundle` | Extract objects from a previously exported archive into the local object store |
| **Hash** | `git hash-object` | Compute and display the SHA-256 hash of a file as Kitsu would store it |

---

## Target Resolution

| Syntax | Name | Definition |
|--------|------|------------|
| `main` | Stream reference | Resolves to the hash stored in `.kitsu/streams/main` |
| `1.0.0` | Seal reference | Resolves to the hash stored in `.kitsu/seals/1.0.0` |
| `~N` | Relative reference | Walk N parents back from HEAD (e.g., `~1` = parent of HEAD) |
| `#N` | Absolute index | Index from the beginning of history (e.g., `#0` = first-ever checkpoint) |
| `abc123...` | Direct hash | Used as-is — a raw SHA-256 object hash |

---

## Build & Configuration

| Term | Definition |
|------|------------|
| **Build-time constants** | Values extracted from `Cargo.toml` by `build.rs` and compiled into the binary: `APP_NAME`, `DIR_NAME`, `ABOUT` |
| **AppConfig** | Runtime configuration struct holding directory names and paths. Currently populated from compile-time defaults |
| **Exclude** | A `.exclude` file (gitignore-compatible) at the project root that specifies patterns for files to ignore during tracking |
