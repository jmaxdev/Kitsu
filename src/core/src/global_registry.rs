//! Global repository tracking and inspection across the local system.
//!
//! Maintains a persistent list of all Kitsu repositories created or opened on the machine
//! at `~/.kitsu/repositories.toml`.

use anyhow::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::AppConfig;
use crate::identity::IdentityStore;
use crate::remote::{RemoteRegistry, default_remote_name};
use crate::repository::Repository;

/// Metadata for a tracked repository on the system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct RepositoryMeta {
    /// Canonical filesystem path of the repository root.
    pub path: String,
    /// Repository name (derived from directory name).
    pub name: String,
    /// ISO-8601 timestamp when this repo was first registered.
    pub registered_at: String,
    /// ISO-8601 timestamp when this repo was last accessed.
    pub last_seen: String,
    /// Whether this repository has a GitHub remote configured.
    pub is_github: bool,
    /// GitHub owner/repo identifier if applicable (e.g., `"jmaxdev/Kitsu"`).
    pub github_repo: Option<String>,
    /// Default remote URL if any.
    pub default_remote_url: Option<String>,
}

/// Full runtime details for a repository returned by the API.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RepositoryFullDetails {
    /// Basic metadata.
    #[serde(flatten)]
    pub meta: RepositoryMeta,
    /// Currently active persona name and email.
    pub active_persona: String,
    /// Current stream (branch) name.
    pub current_stream: Option<String>,
    /// HEAD checkpoint hash.
    pub head_hash: Option<String>,
    /// Total count of version seals (tags).
    pub seals_count: usize,
    /// Total count of stored objects.
    pub total_objects: u64,
    /// Total storage usage in bytes.
    pub storage_bytes: u64,
    /// Total local issues count.
    pub local_issues_count: usize,
}

#[derive(Serialize, Deserialize, Default)]
struct RegistryFile {
    repositories: Vec<RepositoryMeta>,
}

/// Global repository registry manager.
pub struct GlobalRegistry;

/// Normalizes and cleans a filesystem path, stripping Windows extended-length prefix (`\\?\` or `//?/`).
pub fn clean_path_string(path: &Path) -> String {
    let canonical = match path.canonicalize() {
        Ok(p) => p,
        Err(_) => path.to_path_buf(),
    };
    let s = canonical.to_string_lossy().to_string();
    let stripped = if let Some(rest) = s.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{}", rest)
    } else if let Some(rest) = s.strip_prefix(r"\\?\") {
        rest.to_string()
    } else if let Some(rest) = s.strip_prefix("//?/UNC/") {
        format!("//{}", rest)
    } else if let Some(rest) = s.strip_prefix("//?/") {
        rest.to_string()
    } else {
        s
    };
    stripped.replace('\\', "/")
}

impl GlobalRegistry {
    /// Returns the global registry file path (`~/.kitsu/repositories.toml`).
    pub fn file_path() -> Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let kitsu_dir = home.join(".kitsu");
        if !kitsu_dir.exists() {
            fs::create_dir_all(&kitsu_dir)?;
        }
        Ok(kitsu_dir.join("repositories.toml"))
    }

    /// Registers a repository path in the global tracking store.
    ///
    /// If the repository already exists in the registry, updates its `last_seen` timestamp
    /// and refreshed metadata.
    ///
    /// # Errors
    /// Returns an error if the directory doesn't exist or file writing fails.
    pub fn register(path: &Path) -> Result<()> {
        let path_str = clean_path_string(path);
        let repo_root = PathBuf::from(&path_str);
        let repo_dir = repo_root.join(".kitsu");

        // Verify that this is actually an initialized Kitsu repository
        if !repo_dir.join("CURRENT").exists() && !repo_dir.join("objects").exists() {
            return Ok(());
        }

        let name = repo_root
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed".to_string());

        let mut is_github = false;
        let mut github_repo = None;
        let mut default_remote_url = None;

        if repo_dir.exists()
            && let Ok(remotes) = RemoteRegistry::list(&repo_dir)
        {
            let def_name = default_remote_name(&repo_dir).unwrap_or_else(|_| "origin".into());
            for r in remotes {
                if r.name == def_name {
                    default_remote_url = Some(r.url.clone());
                }
                if r.url.contains("github.com") {
                    is_github = true;
                    if github_repo.is_none() {
                        github_repo = parse_github_slug(&r.url);
                    }
                }
            }
        }

        let now = Utc::now().to_rfc3339();
        let registry_path = Self::file_path()?;
        let mut registry = if registry_path.exists() {
            let content = fs::read_to_string(&registry_path)?;
            toml::from_str::<RegistryFile>(&content).unwrap_or_default()
        } else {
            RegistryFile::default()
        };

        if let Some(existing) = registry
            .repositories
            .iter_mut()
            .find(|r| r.path == path_str)
        {
            existing.last_seen = now;
            existing.is_github = is_github;
            existing.github_repo = github_repo;
            existing.default_remote_url = default_remote_url;
        } else {
            registry.repositories.push(RepositoryMeta {
                path: path_str,
                name,
                registered_at: now.clone(),
                last_seen: now,
                is_github,
                github_repo,
                default_remote_url,
            });
        }

        let content = toml::to_string(&registry)?;
        fs::write(registry_path, content)?;
        Ok(())
    }

    /// Unregisters a repository from the global tracking list.
    ///
    /// # Errors
    /// Returns an error if file I/O fails.
    pub fn unregister(path: &Path) -> Result<bool> {
        let path_str = clean_path_string(path);
        let registry_path = Self::file_path()?;
        if !registry_path.exists() {
            return Ok(false);
        }
        let content = fs::read_to_string(&registry_path)?;
        let mut registry: RegistryFile = toml::from_str(&content).unwrap_or_default();
        let initial_len = registry.repositories.len();
        registry.repositories.retain(|r| r.path != path_str);
        if registry.repositories.len() != initial_len {
            let new_content = toml::to_string(&registry)?;
            fs::write(registry_path, new_content)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns a list of all globally tracked repositories.
    ///
    /// # Errors
    /// Returns an error if reading the registry file fails.
    pub fn list() -> Result<Vec<RepositoryMeta>> {
        let registry_path = Self::file_path()?;
        if !registry_path.exists() {
            return Ok(Vec::new());
        }
        let content = fs::read_to_string(&registry_path)?;
        let mut registry: RegistryFile = toml::from_str(&content).unwrap_or_default();
        let initial_len = registry.repositories.len();

        // Clean stored path strings and prune repositories that no longer exist on disk
        for repo in &mut registry.repositories {
            let p = PathBuf::from(&repo.path);
            repo.path = clean_path_string(&p);
        }

        registry.repositories.retain(|r| {
            let p = PathBuf::from(&r.path);
            let repo_dir = p.join(".kitsu");
            repo_dir.join("CURRENT").exists() || repo_dir.join("objects").exists()
        });

        if registry.repositories.len() != initial_len
            && let Ok(new_content) = toml::to_string(&registry)
        {
            let _ = fs::write(&registry_path, new_content);
        }

        Ok(registry.repositories)
    }

    /// Collects detailed runtime statistics and metadata for a repository.
    ///
    /// # Errors
    /// Returns an error if the repository cannot be opened or read.
    pub fn get_details(meta: &RepositoryMeta) -> Result<RepositoryFullDetails> {
        let path = PathBuf::from(&meta.path);
        let repo = match Repository::open(&path) {
            Ok(r) => r,
            Err(_) => {
                return Ok(RepositoryFullDetails {
                    meta: meta.clone(),
                    active_persona: "unknown".into(),
                    current_stream: None,
                    head_hash: None,
                    seals_count: 0,
                    total_objects: 0,
                    storage_bytes: 0,
                    local_issues_count: 0,
                });
            }
        };
        let repo_dir = repo.repo_dir();
        let config = AppConfig::load();

        let id_store = IdentityStore::load(&path);
        let active = id_store.get_active();
        let active_persona = format!("{} <{}>", active.name, active.email);

        let current_stream = repo.current_stream().ok().flatten();
        let head_hash = repo.head_hash().ok().flatten();

        let seals_dir = repo_dir.join("seals");
        let seals_count = fs::read_dir(seals_dir).map(|d| d.count()).unwrap_or(0);

        let mut total_objects = 0u64;
        let mut storage_bytes = 0u64;
        let obj_dir = repo_dir.join(&config.objects_dir);
        if obj_dir.exists()
            && let Ok(entries) = fs::read_dir(obj_dir)
        {
            for entry in entries.flatten() {
                if entry.path().is_dir()
                    && let Ok(sub_entries) = fs::read_dir(entry.path())
                {
                    for obj in sub_entries.flatten() {
                        total_objects += 1;
                        if let Ok(m) = obj.metadata() {
                            storage_bytes += m.len();
                        }
                    }
                }
            }
        }

        let issues_dir = repo_dir.join("issues");
        let local_issues_count = fs::read_dir(issues_dir).map(|d| d.count()).unwrap_or(0);

        Ok(RepositoryFullDetails {
            meta: meta.clone(),
            active_persona,
            current_stream,
            head_hash,
            seals_count,
            total_objects,
            storage_bytes,
            local_issues_count,
        })
    }
}

/// Helper function to parse "owner/repo" from a GitHub URL.
pub fn parse_github_slug(url: &str) -> Option<String> {
    let clean = url.trim_end_matches(".git");
    if let Some(idx) = clean.find("github.com/") {
        let slug = &clean[idx + "github.com/".len()..];
        let parts: Vec<&str> = slug.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
    } else if let Some(idx) = clean.find("github.com:") {
        let slug = &clean[idx + "github.com:".len()..];
        let parts: Vec<&str> = slug.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            return Some(format!("{}/{}", parts[0], parts[1]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_github_slug() {
        assert_eq!(
            parse_github_slug("https://github.com/jmaxdev/Kitsu.git"),
            Some("jmaxdev/Kitsu".into())
        );
        assert_eq!(
            parse_github_slug("git@github.com:jmaxdev/Kitsu.git"),
            Some("jmaxdev/Kitsu".into())
        );
        assert_eq!(parse_github_slug("https://gitlab.com/user/repo.git"), None);
    }
}
