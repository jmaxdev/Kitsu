# Remote Networking and Protocols

Kitsu supports dual networking backends for remote repository synchronization:
1. **Git Bridge Protocol**: Native compatibility with standard Git hosting providers (GitHub, GitLab, Gitea, SourceHut).
2. **Sovereign SSH/SFTP Transport**: Direct synchronization with any self-hosted Linux/Unix server over SSH without requiring Git or server-side daemons.

---

## 1. Git Bridge Protocol (GitHub / GitLab)

The Git Bridge (`GitBridge`) enables using standard Git remotes as storage backends for Kitsu repositories.

### Remote Branch Architecture
Kitsu reserves a dedicated branch named **`kitsu-data`** on the remote Git repository.

```
Git Remote (e.g. github.com/user/repo)
└── Branch: kitsu-data
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
5. The commit is pushed via `git2` over HTTPS/SSH directly to `refs/heads/kitsu-data` on the remote.

### Pull Workflow
When executing `kitsu pull` from a Git remote:
1. The remote `kitsu-data` branch is fetched into `.kitsu/git_bridge/`.
2. All objects in `objects/xx/yy...` are read and imported into local Kitsu CAS storage (`.kitsu/objects/`).
3. Seal and stream references in `seals/` are updated locally.

---

## 2. Sovereign SSH / SFTP Transport

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

### Object Transfer
- `push_object`: Uploads raw object bytes directly to `<remote>/objects/<prefix>/<suffix>`.
- `fetch_object`: Downloads object bytes via SFTP and stores them locally.
- `push_seal` / `fetch_seal`: Synchronizes reference pointer files.
