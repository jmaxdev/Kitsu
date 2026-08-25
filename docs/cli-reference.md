# CLI Reference Manual

This manual provides complete documentation for all subcommands in the Kitsu CLI (`kitsu`).

---

## Command Overview

```
kitsu <COMMAND> [OPTIONS]
```

### Top-Level Subcommands

- [`ignite`](#kitsu-ignite) — Initialize a new Kitsu repository
- [`copy`](#kitsu-copy) — Clone a repository from a remote URL
- [`track`](#kitsu-track) — Stage files for the next checkpoint
- [`freeze`](#kitsu-freeze) — Create a new checkpoint from staged changes
- [`timeline`](#kitsu-timeline) — Show checkpoint history
- [`diff`](#kitsu-diff) — Show differences between checkpoints or working tree
- [`rollback`](#kitsu-rollback) — Roll back to a previous checkpoint
- [`seal`](#kitsu-seal) — Create or list version seals (tags)
- [`switch`](#kitsu-switch) — Switch to a different checkpoint or stream
- [`export`](#kitsu-export) — Export objects to a portable archive
- [`import`](#kitsu-import) — Import objects from a portable archive
- [`push`](#kitsu-push) — Push objects to a remote registry
- [`pull`](#kitsu-pull) — Pull objects from a remote registry
- [`contents`](#kitsu-contents) — Show contents of a checkpoint's file tree
- [`hash`](#kitsu-hash) — Compute the SHA-256 hash of a file
- [`state`](#kitsu-state) — Show working tree status
- [`peek`](#kitsu-peek) — Inspect raw object content by hash
- [`burn`](#kitsu-burn) — Delete objects from the store
- [`repository`](#kitsu-repository) — Repository inspection and management
- [`persona`](#kitsu-persona) — Manage identity personas

---

## Command Details

### `kitsu ignite`
Initializes a new Kitsu repository in the current working directory.

```bash
kitsu ignite
```
- Creates `.kitsu/` metadata directory and subdirectories (`objects/`, `streams/`, `seals/`, `remotes/`).
- Initializes `CURRENT` pointing to `stream: main`.
- Prompts with an interactive wizard to configure a remote registry (GitHub/GitLab or SSH).

---

### `kitsu copy`
Clones an existing repository from a remote URL.

```bash
kitsu copy <URL> [DIRECTORY]
```
- **`<URL>`**: SSH URL (`ssh://[user@]host[:port]/path`) or Git URL (`https://github.com/...`).
- **`[DIRECTORY]`**: Destination directory (defaults to the repository name extracted from the URL).

---

### `kitsu track`
Stages files for inclusion in the next checkpoint.

```bash
kitsu track <FILES...>
```
- Computes chunk hashes for specified files, saves chunks to object storage, and adds entries to `.kitsu/stage`.
- Files matching `.gitignore` or `.exclude` rules are automatically skipped.

---

### `kitsu freeze`
Creates a new checkpoint snapshot from staged files.

```bash
kitsu freeze -m <MESSAGE> [--sign / -S]
```
- **`-m, --message <MESSAGE>`**: Human-readable description of the checkpoint (required).
- **`-S, --sign`**: Cryptographically sign the checkpoint using the active persona's Ed25519 private key.
- Updates the active stream or `CURRENT` pointer to the new checkpoint hash.

---

### `kitsu timeline`
Displays chronological history of checkpoints starting from HEAD back to the initial root checkpoint.

```bash
kitsu timeline
```
- Displays index numbers (`#0`, `#1`, ...), checkpoint hashes, author metadata, creation timestamps, root map hashes, and signature verification status.

---

### `kitsu diff`
Displays textual and structural differences between two checkpoints, or between a checkpoint and the current staging index.

```bash
kitsu diff [OLD] [NEW]
```
- If both `OLD` and `NEW` are omitted: diffs HEAD against staged changes.
- If `OLD` is provided: diffs `OLD` against staged changes.
- If both are provided: diffs `OLD` against `NEW`.

---

### `kitsu rollback`
Restores the working directory and HEAD to a specified checkpoint.

```bash
kitsu rollback [TARGET]
```
- **`[TARGET]`**: Target checkpoint identifier (hash, stream name, seal version, `~N` ancestor, `#N` index). Defaults to parent checkpoint (`~1`).

---

### `kitsu seal`
Creates or lists semantic version seals (tags).

```bash
kitsu seal [VERSION] [--bump / -b <major|minor|patch>] [--list / -l]
```
- **`[VERSION]`**: Explicit semver string (e.g., `1.0.0`).
- **`-b, --bump <BUMP>`**: Auto-increments the specified component (`major`, `minor`, `patch`) relative to the highest existing seal.
- **`-l, --list`**: Lists all existing seals sorted by semver ascending.

---

### `kitsu switch`
Switches the working directory and HEAD pointer to a different stream, seal, or checkpoint.

```bash
kitsu switch <TARGET>
```
- If `<TARGET>` is a stream name, attaches HEAD to the stream (`stream: <TARGET>`).
- If `<TARGET>` is a checkpoint hash or seal, detaches HEAD directly to the resolved hash.

---

### `kitsu export`
Exports all reachable objects for a given target into a compressed tar archive (`.tar.gz`).

```bash
kitsu export <TARGET> <OUTPUT_FILE>
```

---

### `kitsu import`
Imports objects from a compressed archive into local storage. If the repository is empty, automatically checks out the imported snapshot.

```bash
kitsu import <INPUT_FILE>
```

---

### `kitsu push`
Pushes reachable objects and reference pointers to a configured remote.

```bash
kitsu push [REMOTE] [TARGET]
```
- **`[REMOTE]`**: Remote name (defaults to the configured default remote, or `origin`).
- **`[TARGET]`**: Stream name or seal to push (defaults to active stream or `latest`).

---

### `kitsu pull`
Pulls objects and reference pointers from a configured remote.

```bash
kitsu pull [REMOTE] [TARGET]
```

---

### `kitsu contents`
Lists all files, Unix file modes, file sizes, and chunk hashes stored within a checkpoint's directory tree.

```bash
kitsu contents [TARGET]
```

---

### `kitsu hash`
Calculates and outputs the SHA-256 chunk hash for a file on disk without writing to storage.

```bash
kitsu hash <FILE>
```

---

### `kitsu state`
Inspects and displays working tree status:
- Changes staged for freeze (new files, modified files, deleted files).
- Changes not staged for freeze (working directory modifications).
- Untracked files.

```bash
kitsu state
```

---

### `kitsu peek`
Outputs the raw uncompressed content of any stored object by its SHA-256 hash.

```bash
kitsu peek <HASH>
```

---

### `kitsu burn`
Removes an object file from `.kitsu/objects/`.

```bash
kitsu burn [HASH] [--aggressive / -a]
```

---

### `kitsu repository`
Repository maintenance and administrative operations.

```bash
kitsu repository info               # Display repository metadata
kitsu repository stats              # Show object count and disk storage usage
kitsu repository verify             # Validate SHA-256 integrity of all stored objects
kitsu repository vacuum             # Clean unreachable objects
kitsu repository remote add <N> <U> # Add a named remote
kitsu repository remote list        # List configured remotes
kitsu repository remote remove <N>  # Remove a remote
kitsu repository stream new <N>     # Create a new stream from HEAD
kitsu repository stream list        # List all stream names
kitsu repository stream rename <O> <N> # Rename a stream
kitsu repository stream delete <N>  # Delete a stream
```

---

### `kitsu persona`
Identity persona management for checkpoint authorship and cryptographic signing.

```bash
kitsu persona                       # Show active persona details
kitsu persona list                  # List all configured personas
kitsu persona add <ID> <NAME> <EMAIL> [--global / -g]
kitsu persona use <ID> [--global / -g]
kitsu persona edit <ID> [-n <NAME>] [-e <EMAIL>] [--global / -g]
kitsu persona github <USERNAME> [ID] [--global / -g]
kitsu persona keys                  # Regenerate Ed25519 signing keys
```
