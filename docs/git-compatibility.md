# Git Compatibility and Interoperability

Kitsu is designed as a standalone alternative to Git while preserving full interoperability with Git hosting ecosystems and developer conventions.

---

## 1. File Exclusion Engine (`.gitignore` and `.exclude`)

Kitsu provides built-in support for both standard `.gitignore` files and Kitsu-native `.exclude` files via the `ignore` crate.

### Resolution Precedence
When evaluating whether a path is excluded:
1. **Built-in Exclusions**: `.kitsu`, `.git`, and `target` directories are always excluded.
2. **Kitsu Native**: `.exclude` in repository root.
3. **Git Compatible**: `.gitignore` in repository root.

### Example Rules
```gitignore
# Excluded by default
.kitsu
.git
target/

# Standard patterns supported
*.log
*.tmp
node_modules/
.env
```

---

## 2. GitHub and GitLab Workflow

Kitsu projects can be hosted directly on GitHub, GitLab, or Gitea repositories using the built-in Git Bridge.

### Step-by-Step Setup

1. **Create an empty repository on GitHub**:
   ```text
   https://github.com/username/my-project.git
   ```

2. **Initialize and configure remote in Kitsu**:
   ```bash
   kitsu ignite
   # Select "GitHub repository" and enter username, repository name, and optional custom data branch
   # Or configure manually:
   kitsu repository remote add origin https://github.com/username/my-project.git -b kitsu-data
   ```

3. **Make checkpoints and push**:
   ```bash
   kitsu track .
   kitsu freeze -m "Initial commit from Kitsu"
   kitsu push origin main
   ```

4. **Cloning an existing Kitsu project from GitHub**:
   ```bash
   kitsu copy https://github.com/username/my-project.git
   ```

---

## 3. Data Isolation and Custom Branching

By default, Kitsu pushes all data to a dedicated `kitsu-data` branch on the Git remote, though the branch name can be customized to any identifier (e.g. `vcontrol-data`, `archive`, `kitsu-main`):
- The default branch on GitHub remains clean and unpolluted.
- Kitsu objects are committed as immutable binary artifacts inside `objects/` on the remote.
- Standard Git users can view the repository commit history or clone the data branch without conflicts.
- Local-only repositories can be initialized without any remote, keeping 100% of data offline on your local machine.

---

## 4. Migrating Existing Git Repositories (`import git`)

If you have an existing Git repository with commit history, Kitsu can convert it directly into native Kitsu objects:

```bash
# Inside an existing Git directory (or pass path as argument)
kitsu repository import git
```

- **Tree Conversion**: Recursively converts Git trees to Kitsu Maps and Git blobs to SHA-256 Chunks.
- **Checkpoint Creation**: Converts the Git HEAD commit into a Kitsu Checkpoint preserving commit message, author name, and author email.
- **Stage Index Sync**: Initializes and populates `.kitsu/stage` so your working directory remains clean.
- **Remote Mapping**: Preserves existing Git remotes under Kitsu's remote registry targeting the `kitsu-data` branch.

### Auto-Detection in `kitsu copy`
When running `kitsu copy <URL>`, if the target is a standard Git repository that has not been initialized with Kitsu yet (i.e. lacking a `kitsu-data` branch), Kitsu automatically clones the Git tree and runs `import git` seamlessly.

---

## 5. GitHub Issues & Pull Requests Bridge

Kitsu integrates directly with GitHub's REST API for issue and PR management:

```bash
# List / view / manage issues
kitsu repository issue
kitsu repository issue 123
kitsu repository issue open "New Feature" "Detailed description"
kitsu repository issue close 123 "Resolved in v0.0.4"

# List / view / open pull requests
kitsu repository pr
kitsu repository pr 45
kitsu repository pr open "feat: add oauth" "PR description" "feat-branch" "main"
kitsu repository pr close 45
```

