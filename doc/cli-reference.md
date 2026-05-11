# CLI Reference

Complete reference for every Kitsu command, subcommand, flag, and argument.

---

## Global Options

Kitsu inherits standard options from `clap`:

| Flag | Description |
|------|-------------|
| `--version` | Print the Kitsu version (`0.0.1-alpha`) |
| `--help` | Print help information |
| `-h` | Print short help |

---

## Core Commands

### `kitsu ignite`

Initialize a new Kitsu repository in the current directory.

**Git equivalent:** `git init`

**What it does:**
1. Creates the `.kitsu/` directory structure (`objects/`, `streams/`, `seals/`, `remotes/`)
2. Sets `CURRENT` to `stream: main`
3. Launches an interactive wizard to optionally configure a remote registry

**Interactive wizard options:**
- **GitHub / GitLab** — Prompts for username and repository name, creates URL like `https://github.com/user/repo.git`
- **Custom SSH Server** — Prompts for host, user (default: `root`), and path (default: `/opt/vcontrol/repo`)

**Example:**
```bash
$ kitsu ignite
--- Kitsu Ignite Assistant ---
? Configure a remote registry now? Yes
? Select registry type GitHub / GitLab
? GitHub/GitLab username jmaxdev
? Repository name my-project
SUCCESS Remote 'origin' configured: https://github.com/jmaxdev/my-project.git
Repository ignited successfully.
```

---

### `kitsu copy <url> [directory]`

Clone a repository from a remote registry.

**Git equivalent:** `git clone`

| Argument | Required | Description |
|----------|----------|-------------|
| `url` | Yes | Remote URL (SSH or Git) |
| `directory` | No | Local directory name (defaults to repo name from URL) |

**Behavior:**
- Creates the target directory with full `.kitsu/` structure
- Sets `origin` remote to the source URL
- For SSH/SFTP: fetches all reachable objects starting from `latest` or `main` seal
- For Git URLs: prints WIP message (not yet implemented)
- Reconstructs the working tree from the fetched checkpoint

**Example:**
```bash
$ kitsu copy ssh://root@myserver.com/opt/vcontrol/repo my-project
Copying from ssh://root@myserver.com/opt/vcontrol/repo...
Done. Project copied to "my-project"
```

---

### `kitsu track <files...>`

Stage files for the next checkpoint.

**Git equivalent:** `git add`

| Argument | Required | Description |
|----------|----------|-------------|
| `files` | Yes | One or more file paths to stage |

**Behavior:**
- Reads each file, creates a `Chunk` object (content hash + compressed storage)
- Records the file's path, hash, mode (file=`100644`, dir=`40000`), and size in the stage
- Skips files matching `.exclude` patterns
- Skips non-existent files silently

**Example:**
```bash
$ kitsu track src/main.rs src/config.rs README.md
```

> **Note:** There is no `kitsu track .` (recursive add) yet. You must specify files individually.

---

### `kitsu freeze -m <message> [-S]`

Create a new checkpoint (immutable snapshot) from the current stage.

**Git equivalent:** `git commit`

| Flag | Required | Description |
|------|----------|-------------|
| `-m <message>` | Yes | Checkpoint message |
| `-S` / `--sign` | No | Sign the checkpoint with the active persona's Ed25519 key |

**Behavior:**
1. Loads the staging area
2. Builds a hierarchical Map tree from all staged entries
3. Creates a Checkpoint object with: map hash, parent hash (from HEAD), author, timestamp, message
4. If `-S`: signs the serialized checkpoint data and embeds the signature
5. Writes the checkpoint object to storage
6. Updates the current stream pointer to the new hash

**Example:**
```bash
$ kitsu freeze -m "Add user authentication module"
[freeze a1b2c3d4e5f6...] Add user authentication module

$ kitsu freeze -m "Security patch" -S
[freeze f7e8d9c0b1a2...] Security patch
```

---

### `kitsu timeline`

Display the checkpoint history from HEAD backwards.

**Git equivalent:** `git log`

**Output format:**
```
#N checkpoint <hash>
Author: Name <email>
Date:   YYYY-MM-DD HH:MM:SS UTC
Map:    <map_hash>
Signature: VALID | NONE

    Checkpoint message
```

**Example:**
```bash
$ kitsu timeline
#1 checkpoint a1b2c3d4e5f6...
Author: John Doe <john@example.com>
Date:   2026-05-10 20:00:00 UTC
Map:    9f8e7d6c5b4a...
Signature: NONE

    Add user authentication module

#0 checkpoint f7e8d9c0b1a2...
Author: John Doe <john@example.com>
Date:   2026-05-10 19:30:00 UTC
Map:    3a2b1c0d9e8f...
Signature: VALID

    Initial checkpoint
```

---

### `kitsu diff [old] [new]`

Show differences between checkpoints or between HEAD and the current stage.

**Git equivalent:** `git diff`

| Argument | Required | Description |
|----------|----------|-------------|
| `old` | No | Source checkpoint/stream/seal (defaults to HEAD) |
| `new` | No | Target checkpoint/stream/seal (defaults to current stage) |

**Behavior:**
- Compares two Map trees recursively
- Shows added files in green, deleted files in red
- For modified files, shows a line-level diff using the `similar` algorithm

**Example:**
```bash
# Diff HEAD vs. current stage
$ kitsu diff

# Diff between two checkpoints
$ kitsu diff ~2 ~0

# Diff between a seal and HEAD
$ kitsu diff 1.0.0 main
```

---

### `kitsu rollback [target]`

Restore the working tree to a previous checkpoint.

**Git equivalent:** `git reset --hard`

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | No | Checkpoint reference (defaults to parent of HEAD) |

**Behavior:**
- Resolves the target to a checkpoint hash
- Reads the checkpoint's Map tree
- Removes files/directories not in the map (respecting `.exclude`)
- Writes all files from the map to disk
- Updates the current stream/HEAD pointer

**Example:**
```bash
# Rollback to the previous checkpoint
$ kitsu rollback

# Rollback to a specific checkpoint
$ kitsu rollback ~3

# Rollback to a seal
$ kitsu rollback 1.0.0
```

> **Warning:** This is destructive — it overwrites the working tree.

---

### `kitsu switch <target>`

Switch the working tree to a different stream or checkpoint.

**Git equivalent:** `git checkout` / `git switch`

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | Yes | Stream name, seal, or checkpoint hash |

**Behavior:**
- Resolves the target to a checkpoint hash
- Reconstructs the working tree from the checkpoint's Map
- If target is a stream name, sets CURRENT to `stream: <name>` (attached mode)
- Otherwise, sets CURRENT to the raw hash (detached mode)

**Example:**
```bash
$ kitsu switch feature-auth
Switched to feature-auth

$ kitsu switch 1.0.0
Switched to 1.0.0
```

---

### `kitsu seal [version] [-b bump] [-l]`

Create or list semantic version seals (tags).

**Git equivalent:** `git tag`

| Argument/Flag | Required | Description |
|---------------|----------|-------------|
| `version` | No | Explicit version string (e.g., `1.2.3`) |
| `-b` / `--bump` | No | Auto-bump type: `major`, `minor`, or `patch` |
| `-l` / `--list` | No | List all seals |

**Behavior:**
- With `-l`: Lists all seals sorted by version, showing `version -> hash`
- With `-b`: Finds the latest seal version, increments accordingly, and creates a new seal
- With explicit version: Creates a seal with the given version pointing to HEAD

**Examples:**
```bash
# Create an explicit seal
$ kitsu seal 1.0.0
Sealed as 1.0.0

# Auto-bump patch version (1.0.0 → 1.0.1)
$ kitsu seal -b patch
Sealed as 1.0.1

# Auto-bump minor version (1.0.1 → 1.1.0)
$ kitsu seal -b minor
Sealed as 1.1.0

# List all seals
$ kitsu seal -l
  1.0.0 -> a1b2c3d4...
  1.0.1 -> e5f6a7b8...
  1.1.0 -> c9d0e1f2...
```

---

### `kitsu export <target> <output>`

Export a checkpoint and all its reachable objects to a compressed archive.

**Git equivalent:** `git bundle create`

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | Yes | Checkpoint reference to export |
| `output` | Yes | Output file path (`.tar.gz`) |

**Example:**
```bash
$ kitsu export main backup.tar.gz
Exported main to "backup.tar.gz"
```

---

### `kitsu import <input>`

Import objects from a previously exported archive.

| Argument | Required | Description |
|----------|----------|-------------|
| `input` | Yes | Path to `.tar.gz` archive |

**Example:**
```bash
$ kitsu import backup.tar.gz
Import complete.
```

---

### `kitsu push [remote] [target]`

Upload objects and seals to a remote registry.

**Git equivalent:** `git push`

| Argument | Required | Description |
|----------|----------|-------------|
| `remote` | No | Remote name (defaults to `default_remote` or `origin`) |
| `target` | No | Stream/seal name (defaults to current stream or `latest`) |

**Behavior:**
- Collects all reachable objects from the target checkpoint
- **SSH/SFTP mode**: Connects via SSH, pushes each object individually, then pushes the seal
- **Git mode**: Creates a local Git bridge repo, commits objects, pushes to `vcontrol-data` branch

---

### `kitsu pull [remote] [target]`

Download objects and seals from a remote registry.

**Git equivalent:** `git pull` / `git fetch`

| Argument | Required | Description |
|----------|----------|-------------|
| `remote` | No | Remote name (defaults to `default_remote` or `origin`) |
| `target` | No | Seal name to pull (defaults to `latest`) |

---

### `kitsu contents [target]`

List all files in a checkpoint's Map tree.

**Git equivalent:** `git ls-tree -r`

| Argument | Required | Description |
|----------|----------|-------------|
| `target` | No | Checkpoint reference (defaults to HEAD) |

**Output format:**
```
MODE       SHA-256 HASH                                                     SIZE       NAME
--------------------------------------------------------------------------------------------------------------
100644     a1b2c3d4e5f6...                                                  1234       src/main.rs
100644     9f8e7d6c5b4a...                                                  567        README.md
```

---

### `kitsu hash <file>`

Compute and display the SHA-256 hash of a file (as Kitsu would store it).

**Git equivalent:** `git hash-object`

| Argument | Required | Description |
|----------|----------|-------------|
| `file` | Yes | Path to the file |

**Example:**
```bash
$ kitsu hash README.md
a1b2c3d4e5f6789012345678901234567890abcdef1234567890abcdef12345678
```

---

### `kitsu peek <hash>`

Display the raw content of a stored object.

**Git equivalent:** `git cat-file -p`

| Argument | Required | Description |
|----------|----------|-------------|
| `hash` | Yes | Full SHA-256 hash of the object |

---

### `kitsu burn [hash] [-a]`

Delete an object from local storage.

| Argument/Flag | Required | Description |
|---------------|----------|-------------|
| `hash` | No | Object hash to delete (defaults to HEAD) |
| `-a` / `--aggressive` | No | Aggressive cleanup mode |

> **Warning:** Burning objects is irreversible and may break the object graph.

---

### `kitsu state`

Show the working tree state compared to the last checkpoint.

**Git equivalent:** `git status`

> **Status:** WIP — currently prints a placeholder message.

---

## Subcommand Groups

### `kitsu repository`

Repository management commands.

#### `kitsu repository info`

Display repository information: active persona, default remote, seal count, HEAD hash.

#### `kitsu repository stats`

Display storage statistics: total object count and storage usage in MB.

#### `kitsu repository verify`

Verify the integrity of all objects in the store by reading and decompressing each one.

#### `kitsu repository vacuum`

Clean up the repository (placeholder — not yet fully implemented).

---

### `kitsu repository remote`

CRUD operations for remote registries.

#### `kitsu repository remote add <name> <url>`

Add a new remote registry.

```bash
$ kitsu repository remote add origin ssh://root@server.com/opt/vcontrol/repo
Remote 'origin' added: ssh://root@server.com/opt/vcontrol/repo
```

#### `kitsu repository remote edit <name> <url>`

Update the URL of an existing remote.

#### `kitsu repository remote default <name>`

Set a remote as the default for push/pull operations.

#### `kitsu repository remote list`

List all configured remotes and their URLs.

#### `kitsu repository remote remove <name>`

Delete a remote configuration.

---

### `kitsu repository stream`

Stream (branch) management.

#### `kitsu repository stream new <name>`

Create a new stream from the current HEAD.

```bash
$ kitsu repository stream new feature-auth
Stream 'feature-auth' created from HEAD.
```

#### `kitsu repository stream list`

List all streams.

#### `kitsu repository stream rename <old> <new>`

Rename a stream.

#### `kitsu repository stream delete <name>`

Delete a stream.

---

### `kitsu beam`

Shorthand remote management (alias for common `repository remote` operations).

#### `kitsu beam add <name> <url>`

Add a remote.

#### `kitsu beam list`

List remotes.

#### `kitsu beam default <name>`

Set the default remote.

---

### `kitsu persona`

Identity management for commit authorship and cryptographic signing.

#### `kitsu persona` (no subcommand)

Display the active persona's name and email.

```bash
$ kitsu persona
John Doe <john@example.com>
```

#### `kitsu persona add <id> <name> <email> [-g]`

Create a new persona with auto-generated Ed25519 keypair.

| Flag | Description |
|------|-------------|
| `-g` / `--global` | Save to global config (`~/.kitsu_identity.toml`) instead of local |

```bash
$ kitsu persona add work "John Doe" "john@company.com"
$ kitsu persona add personal "John" "john@home.com" -g
```

#### `kitsu persona list`

List all configured personas.

```bash
$ kitsu persona list
  work - John Doe <john@company.com>
  personal - John <john@home.com>
```

#### `kitsu persona use <id> [-g]`

Switch the active persona.

```bash
$ kitsu persona use work
```

#### `kitsu persona edit <id> [-n name] [-e email] [-g]`

Edit an existing persona's name and/or email.

#### `kitsu persona github <username> [id] [-g]`

Configure a persona with a GitHub username (sets name and email from GitHub conventions).

#### `kitsu persona keys`

Regenerate the Ed25519 keypair for the active persona.

---

## Target Resolution Syntax

Many commands accept a `<target>` argument. Kitsu resolves targets in this order:

1. **Stream name** — If a file exists at `.kitsu/streams/<target>`, read its hash
2. **Seal name** — If a file exists at `.kitsu/seals/<target>`, read its hash
3. **Relative `~N`** — Walk N parents back from HEAD (e.g., `~0` = HEAD, `~1` = parent)
4. **Absolute `#N`** — Index from the beginning of history (e.g., `#0` = first checkpoint)
5. **Raw hash** — Use as a direct object hash
