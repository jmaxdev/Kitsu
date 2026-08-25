# Remote Networking and Protocols

Kitsu supports three transport backends for repository synchronization:
1. **Git Bridge Protocol**: Native compatibility with standard Git hosting providers (GitHub, GitLab, Gitea, SourceHut) with **configurable data branch names**.
2. **Local Filesystem Remotes**: 100% offline synchronization with local directories, external backup drives, network shares, and local Kitsu repositories.
3. **Sovereign SSH/SFTP Transport**: Direct synchronization with any self-hosted Linux/Unix server over SSH without requiring Git or server-side daemons.

---

## 1. Git Bridge Protocol (GitHub / GitLab)

The Git Bridge (`GitBridge`) enables using standard Git remotes as storage backends for Kitsu repositories.

### Configurable Remote Data Branch
By default, Kitsu syncs with a dedicated branch named **`kitsu-data`** on the remote Git repository. The branch name can be customized per remote or overridden per command:

```bash
# Add a remote with a custom branch name
kitsu repository remote add origin https://github.com/user/repo.git -b my-custom-branch

# Or override the branch at push/pull time
kitsu push origin main -b my-custom-branch
kitsu pull origin main -b my-custom-branch
```

### Remote Branch Layout
```
Git Remote (e.g. github.com/user/repo)
└── Branch: <data-branch> (default: kitsu-data)
    ├── objects/
    │   ├── e3/b0c442...        # Stored Kitsu objects
    │   └── ...
    └── seals/
        ├── main                # Reference pointer containing checkpoint hash
        └── 1.0.0               # Seal version pointer
```

### Push Workflow
When executing `kitsu push` to a Git remote:
1. A local bare Git repository is managed inside `.kitsu/git_bridge/`.
2. All reachable Kitsu objects (`Chunk`, `Map`, `Checkpoint`) are written to `.kitsu/git_bridge/objects/`.
3. Reference files (stream or seal names) are updated in `.kitsu/git_bridge/seals/`.
4. A standard Git commit is created referencing the tree.
5. The commit is pushed via `git2` over HTTPS/SSH directly to `refs/heads/<data-branch>` on the remote.

### Pull Workflow
When executing `kitsu pull` from a Git remote:
1. The remote `<data-branch>` branch is fetched into `.kitsu/git_bridge/`.
2. All objects in `objects/xx/yy...` are read and imported into local Kitsu CAS storage (`.kitsu/objects/`).
3. Seal and stream references in `seals/` are updated locally.

---

## 2. Local Filesystem Remotes (100% Offline / Backup)

Kitsu allows configuring local directories as remotes for offline backup, air-gapped environments, USB drives, or multi-directory development.

### Setup and Usage
```bash
# Add a local directory or backup drive as remote
kitsu repository remote add backup D:\backups\kitsu-repo
# or
kitsu repository remote add backup /mnt/usb/kitsu-backup

# Push to local backup
kitsu push backup main

# Pull from local backup
kitsu pull backup main

# Clone from a local directory
kitsu copy D:\backups\kitsu-repo ./restored-project
```

---

## 3. Sovereign SSH / SFTP Transport

For self-hosted, sovereign setups, Kitsu communicates directly with any SSH-accessible host without requiring any Kitsu or Git binary installed on the remote server.

### URL Format
```text
ssh://[user@]hostname[:port]/path/to/remote_repo
```

### Authentication Hierarchy
`SshTransport` attempts authentication methods in the following order:
1. **SSH Agent**: Queries the running `ssh-agent` or OpenSSH authentication daemon.
2. **Password Authentication**: Prompts interactively via `rpassword` if agent auth is unavailable.
3. **Public Key File**: Inspects `~/.ssh/id_rsa` in the user's home directory.

### Remote Directory Hierarchy
On the remote host, Kitsu initializes and maintains the following structure via SFTP:

```
<remote_path>/kitsu_repo/
├── objects/
│   ├── e3/
│   └── ...
├── seals/
└── streams/
```
