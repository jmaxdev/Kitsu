use anyhow::Result;
use std::fs;
use std::path::Path;

/// Remote registry entry with name and URL.
pub struct RemoteEntry {
    /// Short name (e.g., `"origin"`).
    pub name: String,
    /// Full URL (SSH or HTTPS).
    pub url: String,
}

/// Manages the set of named remote registries for a repository.
pub struct RemoteRegistry;

impl RemoteRegistry {
    /// Adds a new named remote.
    ///
    /// # Errors
    /// Returns an error if directory creation or file writing fails.
    pub fn add(repo_dir: &Path, name: &str, url: &str) -> Result<()> {
        let rem_dir = repo_dir.join("remotes");
        fs::create_dir_all(&rem_dir)?;
        fs::write(rem_dir.join(name), url)?;
        Ok(())
    }

    /// Updates the URL of an existing remote.
    ///
    /// # Errors
    /// Returns an error if the remote doesn't exist or file writing fails.
    pub fn edit(repo_dir: &Path, name: &str, url: &str) -> Result<()> {
        let path = repo_dir.join("remotes").join(name);
        if path.exists() {
            fs::write(path, url)?;
            Ok(())
        } else {
            Err(anyhow::anyhow!("Remote '{}' not found", name))
        }
    }

    /// Removes a named remote.
    ///
    /// # Errors
    /// Returns an error if file deletion fails.
    pub fn remove(repo_dir: &Path, name: &str) -> Result<()> {
        let path = repo_dir.join("remotes").join(name);
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    /// Lists all configured remotes with their URLs.
    ///
    /// # Errors
    /// Returns an error if the remotes directory cannot be read.
    pub fn list(repo_dir: &Path) -> Result<Vec<RemoteEntry>> {
        let rem_dir = repo_dir.join("remotes");
        fs::create_dir_all(&rem_dir)?;
        let mut entries = Vec::new();
        for entry in fs::read_dir(&rem_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            let url = fs::read_to_string(rem_dir.join(&name))?.trim().to_string();
            entries.push(RemoteEntry { name, url });
        }
        Ok(entries)
    }

    /// Sets the default remote name for push/pull operations.
    ///
    /// # Errors
    /// Returns an error if file writing fails.
    pub fn set_default(repo_dir: &Path, name: &str) -> Result<()> {
        fs::write(repo_dir.join("default_remote"), name)?;
        Ok(())
    }
}

/// Reads the default remote name, falling back to `"origin"`.
///
/// # Errors
/// Returns an error if the file exists but cannot be read.
pub fn default_remote_name(repo_dir: &Path) -> Result<String> {
    let def_path = repo_dir.join("default_remote");
    if def_path.exists() {
        Ok(fs::read_to_string(def_path)?.trim().to_string())
    } else {
        Ok("origin".to_string())
    }
}

/// Checks whether a URL points to a git hosting service.
///
/// Returns `true` for URLs containing `github.com`, `gitlab.com`,
/// or ending with `.git`.
pub fn is_git_url(url: &str) -> bool {
    url.contains("github.com") || url.contains("gitlab.com") || url.ends_with(".git")
}
