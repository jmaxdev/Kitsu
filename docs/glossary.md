# Terminology Glossary

This glossary maps Kitsu concepts, nomenclature, and commands to their Git and industry-standard version control equivalents.

---

## Data Structure Terminology

| Kitsu Term | Git Term | Description |
|---|---|---|
| **Chunk** | Blob | A content-addressable block of uninterpreted raw file bytes. |
| **Map** | Tree | A sorted directory listing containing file modes, names, and hashes. |
| **Checkpoint** | Commit | An immutable snapshot containing metadata, author identity, timestamp, parent pointer, and root Map hash. |
| **Stream** | Branch | A movable reference pointer that advances automatically as new checkpoints are frozen. |
| **Seal** | Tag / Release | A named semantic version pointer referencing a specific checkpoint hash. |
| **Stage** | Index / Staging Area | The intermediate binary index tracking queued file changes prior to checkpoint creation. |
| **HEAD / CURRENT** | HEAD | The reference pointer indicating the currently checked-out stream or detached checkpoint. |

---

## Command Terminology

| Kitsu Command | Git Equivalent | Function |
|---|---|---|
| `kitsu ignite` | `git init` | Initializes a new repository metadata structure and configuration. |
| `kitsu copy` | `git clone` | Clones a remote repository into a local directory. |
| `kitsu track` | `git add` | Stages file modifications into the binary staging index. |
| `kitsu freeze` | `git commit` | Writes staged files to storage and creates an immutable checkpoint. |
| `kitsu timeline` | `git log` | Displays chronological commit history and signature statuses. |
| `kitsu diff` | `git diff` | Shows line-by-line unified diffs between checkpoints or working tree. |
| `kitsu rollback` | `git reset --hard` | Discards changes and restores working directory to a checkpoint. |
| `kitsu seal` | `git tag` | Creates, lists, or auto-increments semantic version tags. |
| `kitsu switch` | `git checkout / switch` | Changes the active stream or checks out a historical checkpoint. |
| `kitsu state` | `git status` | Displays working tree status, staged changes, and untracked files. |
| `kitsu push` | `git push` | Transmits reachable objects and refs to a remote registry. |
| `kitsu pull` | `git pull` | Fetches objects and updates local refs from a remote registry. |
| `kitsu contents` | `git ls-tree -r` | Lists recursive directory contents of a checkpoint snapshot. |
| `kitsu hash` | `git hash-object` | Calculates the SHA-256 chunk hash of a local file. |
| `kitsu peek` | `git cat-file -p` | Inspects raw uncompressed object contents by hash. |
| `kitsu burn` | `git prune` | Deletes loose objects directly from object storage. |
| `kitsu repository` | `git remote / branch / fsck` | Administrative subcommands for remotes, streams, and integrity verification. |
| `kitsu persona` | `git config user.*` | Identity management for commit authorship and cryptographic keys. |
