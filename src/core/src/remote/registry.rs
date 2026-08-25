use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Remote registry entry with name, URL, and optional custom data branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteEntry {
    /// Short name (e.g., `"origin"`).
    pub name: String,
    /// Full URL (Git HTTPS/SSH, local directory path, or sovereign SSH/SFTP).
    pub url: String,
    /// Optional remote data branch name (defaults to `"kitsu-data"` for Git remotes).
    pub branch: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct RemoteConfigFile {
    url: String,
    branch: Option<String>,
}

/// Manages the set of named remote registries for a repository.
pub struct RemoteRegistry;

impl RemoteRegistry {
    /// Adds a new named remote with an optional custom data branch.
    ///
    /// # Errors
    /// Returns an error if directory creation or file writing fails.
    pub fn add(repo_dir: &Path, name: &str, url: &str, branch: Option<&str>) -> Result<()> {
        let rem_dir = repo_dir.join("remotes");
        fs::create_dir_all(&rem_dir)?;
        let config = RemoteConfigFile {
            url: url.to_string(),
            branch: branch.map(|b| b.to_string()),
        };
        let content = toml::to_string(&config)?;
        fs::write(rem_dir.join(name), content)?;
        Ok(())
    }

    /// Updates the URL and branch of an existing remote.
    ///
    /// # Errors
    /// Returns an error if the remote doesn't exist or file writing fails.
    pub fn edit(repo_dir: &Path, name: &str, url: &str, branch: Option<&str>) -> Result<()> {
        let path = repo_dir.join("remotes").join(name);
        if path.exists() {
            let config = RemoteConfigFile {
                url: url.to_string(),
                branch: branch.map(|b| b.to_string()),
            };
            let content = toml::to_string(&config)?;
            fs::write(path, content)?;
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

    /// Retrieves a specific remote entry by name.
    ///
    /// # Errors
    /// Returns an error if the remote does not exist.
    pub fn get(repo_dir: &Path, name: &str) -> Result<RemoteEntry> {
        let path = repo_dir.join("remotes").join(name);
        if !path.exists() {
            return Err(anyhow::anyhow!("Remote '{}' not found", name));
        }
        let raw = fs::read_to_string(path)?;
        if let Ok(config) = toml::from_str::<RemoteConfigFile>(&raw) {
            Ok(RemoteEntry {
                name: name.to_string(),
                url: config.url,
                branch: config.branch,
            })
        } else {
            Ok(RemoteEntry {
                name: name.to_string(),
                url: raw.trim().to_string(),
                branch: None,
            })
        }
    }

    /// Lists all configured remotes with their URLs and branches.
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
            if let Ok(remote) = Self::get(repo_dir, &name) {
                entries.push(remote);
            }
        }
        entries.sort_by(|a, b| a.name.cmp(&b.name));
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
    url.contains("github.com")
        || url.contains("gitlab.com")
        || url.ends_with(".git")
        || url.starts_with("git@")
}

/// Checks whether a target URL refers to a local filesystem directory.
pub fn is_local_path(url: &str) -> bool {
    let clean = url.trim_start_matches("file://");
    let path = Path::new(clean);
    path.is_absolute()
        || path.exists()
        || clean.starts_with('.')
        || clean.starts_with('/')
        || clean.starts_with('\\')
        || (clean.len() >= 2 && clean.as_bytes()[1] == b':')
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn remote_registry_add_list_get_cycle() {
        let dir = tempdir().unwrap();
        let repo_dir = dir.path().join(".kitsu");
        fs::create_dir_all(&repo_dir).unwrap();

        RemoteRegistry::add(
            &repo_dir,
            "origin",
            "https://github.com/user/repo.git",
            Some("custom-data"),
        )
        .unwrap();

        let remote = RemoteRegistry::get(&repo_dir, "origin").unwrap();
        assert_eq!(remote.name, "origin");
        assert_eq!(remote.url, "https://github.com/user/repo.git");
        assert_eq!(remote.branch.as_deref(), Some("custom-data"));

        let list = RemoteRegistry::list(&repo_dir).unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0], remote);
    }
}
