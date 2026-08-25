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
   # Select "GitHub / GitLab" and enter username and repository name
   # Or configure manually:
   kitsu repository remote add origin https://github.com/username/my-project.git
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

## 3. Data Isolation

Kitsu pushes all data to a dedicated `kitsu-data` branch on the Git remote:
- The default branch on GitHub remains clean and unpolluted.
- Kitsu objects are committed as immutable binary artifacts inside `objects/` on the remote.
- Standard Git users can view the repository commit history or clone the data branch without conflicts.
