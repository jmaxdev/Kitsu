//! Central repository orchestrator for Kitsu.

use crate::config::AppConfig;
use crate::exclude::Exclude;
use crate::objects::{Checkpoint, Map};
use crate::storage::{ObjectType, Storage};
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Central orchestrator for a Kitsu repository.
///
/// Provides a unified entry point for all repository operations,
/// encapsulating the storage engine, configuration, and exclusion
/// filter. Most CLI commands operate through this struct.
pub struct Repository {
    root: PathBuf,
    config: AppConfig,
    storage: Storage,
    exclude: Exclude,
}

impl Repository {
    /// Opens an existing repository at the given path.
    ///
    /// Verifies the metadata directory exists before returning.
    ///
    /// # Errors
    /// Returns an error if no repository is found at the path.
    pub fn open(path: &Path) -> Result<Self> {
        let config = AppConfig::load();
        let repo_dir = path.join(&config.dir_name);
        if !repo_dir.exists() {
            return Err(anyhow::anyhow!(
                "Repository not found at {}",
                path.display()
            ));
        }
        let storage = Storage::new(path.to_path_buf(), config.clone());
        let exclude = Exclude::load(path);
        Ok(Self {
            root: path.to_path_buf(),
            config,
            storage,
            exclude,
        })
    }

    /// Creates and initializes a new repository at the given path.
    ///
    /// Creates the metadata directory structure: objects, streams,
    /// seals, remotes, and the CURRENT file pointing to `main`.
    ///
    /// # Errors
    /// Returns an error if directory creation or file writing fails.
    pub fn init(path: &Path) -> Result<Self> {
        let config = AppConfig::load();
        let repo_dir = path.join(&config.dir_name);
        fs::create_dir_all(repo_dir.join(&config.objects_dir))?;
        fs::create_dir_all(repo_dir.join(&config.streams_dir))?;
        fs::create_dir_all(repo_dir.join("seals"))?;
        fs::create_dir_all(repo_dir.join("remotes"))?;
        let cur = repo_dir.join(&config.current_file);
        if !cur.exists() {
            fs::write(cur, "stream: main\n")?;
        }
        let storage = Storage::new(path.to_path_buf(), config.clone());
        let exclude = Exclude::load(path);
        Ok(Self {
            root: path.to_path_buf(),
            config,
            storage,
            exclude,
        })
    }

    /// Returns the repository root path.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the metadata directory path (e.g., `.kitsu/`).
    pub fn repo_dir(&self) -> PathBuf {
        self.root.join(&self.config.dir_name)
    }

    /// Returns a reference to the storage engine.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Returns a reference to the configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Returns a reference to the exclusion filter.
    pub fn exclude(&self) -> &Exclude {
        &self.exclude
    }

    /// Resolves HEAD to a checkpoint hash.
    ///
    /// # Errors
    /// Returns an error if file I/O fails.
    pub fn head_hash(&self) -> Result<Option<String>> {
        crate::refs::get_head_hash(&self.root, &self.config)
    }

    /// Resolves a target string (stream/seal/ancestor/index/hash) to a checkpoint hash.
    ///
    /// # Errors
    /// Returns an error if the target cannot be resolved.
    pub fn resolve_target(&self, target: &str) -> Result<String> {
        crate::refs::resolve_target(target, &self.root, &self.config, &self.storage)
    }

    /// Reads the current stream name, if HEAD is attached to a stream.
    pub fn current_stream(&self) -> Result<Option<String>> {
        let cur_path = self.repo_dir().join(&self.config.current_file);
        if !cur_path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(&cur_path)?;
        if content.starts_with("stream: ") {
            Ok(Some(
                content.trim_start_matches("stream: ").trim().to_string(),
            ))
        } else {
            Ok(None)
        }
    }

    /// Updates HEAD (or the current stream) to point at a new hash.
    ///
    /// # Errors
    /// Returns an error if file I/O fails.
    pub fn update_head(&self, hash: &str) -> Result<()> {
        let cur_path = self.repo_dir().join(&self.config.current_file);
        let cur_content = fs::read_to_string(&cur_path)?;
        if cur_content.starts_with("stream: ") {
            let stream = cur_content.trim_start_matches("stream: ").trim();
            fs::write(
                self.repo_dir().join(&self.config.streams_dir).join(stream),
                format!("{}\n", hash),
            )?;
        } else {
            fs::write(cur_path, format!("{}\n", hash))?;
        }
        Ok(())
    }

    /// Restores the working directory to match a given map hash.
    ///
    /// Removes files not present in the map and writes/overwrites
    /// files from the object store. Recursively handles subdirectories.
    ///
    /// # Errors
    /// Returns an error if object reads or file system operations fail.
    pub fn apply_map_to_disk(&self, map_hash: &str, target_dir: &Path) -> Result<()> {
        apply_map_recursive(&self.storage, map_hash, target_dir, &self.exclude)
    }

    /// Collects all object hashes reachable from a given root hash.
    ///
    /// Walks checkpoints → maps → chunks to build a complete set of
    /// objects needed to fully reconstruct the snapshot.
    ///
    /// # Errors
    /// Returns an error if any object cannot be read.
    pub fn collect_reachable(&self, hash: &str) -> Result<HashSet<String>> {
        let mut objects = HashSet::new();
        collect_reachable_recursive(&self.storage, hash, &mut objects)?;
        Ok(objects)
    }
}

fn apply_map_recursive(
    storage: &Storage,
    map_hash: &str,
    target_dir: &Path,
    exclude: &Exclude,
) -> Result<()> {
    let (_, map_data) = storage.read_object(map_hash)?;
    let map = Map::deserialize(&map_data)?;
    let mut entries = std::collections::HashSet::new();
    for e in &map.entries {
        entries.insert(e.name.clone());
    }
    if target_dir.exists() {
        for entry in fs::read_dir(target_dir)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();
            if exclude.is_ignored(Path::new(&name), entry.path().is_dir()) {
                continue;
            }
            if !entries.contains(&name) {
                if entry.path().is_dir() {
                    fs::remove_dir_all(entry.path())?;
                } else {
                    fs::remove_file(entry.path())?;
                }
            }
        }
    }
    for e in map.entries {
        let path = target_dir.join(&e.name);
        if e.mode == "40000" {
            fs::create_dir_all(&path)?;
            apply_map_recursive(storage, &e.hash, &path, exclude)?;
        } else {
            let (_, data) = storage.read_object(&e.hash)?;
            fs::write(&path, data)?;
        }
    }
    Ok(())
}

fn collect_reachable_recursive(
    storage: &Storage,
    hash: &str,
    objects: &mut HashSet<String>,
) -> Result<()> {
    if objects.contains(hash) {
        return Ok(());
    }
    objects.insert(hash.to_string());
    let (obj_type, data) = storage.read_object(hash)?;
    match obj_type {
        ObjectType::Checkpoint => {
            let cp = Checkpoint::deserialize(&data)?;
            collect_reachable_recursive(storage, &cp.map_hash, objects)?;
        }
        ObjectType::Map => {
            let map = Map::deserialize(&data)?;
            for e in map.entries {
                collect_reachable_recursive(storage, &e.hash, objects)?;
            }
        }
        _ => {}
    }
    Ok(())
}
