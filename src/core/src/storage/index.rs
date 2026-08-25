use crate::config::AppConfig;
use crate::objects::{Map, MapEntry};
use crate::storage::Storage;
use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A single entry in the staging index.
///
/// Tracks a file's path, content hash, Unix mode, and size as recorded
/// at the time of `kitsu track`.
#[derive(Debug, Clone)]
pub struct StageEntry {
    /// Content hash of the staged file.
    pub hash: String,
    /// Relative path from the repository root.
    pub path: String,
    /// Unix file mode (e.g., `0o100644` for regular files).
    pub mode: u32,
    /// File size in bytes at staging time.
    pub size: u64,
}

/// The staging area (index) that tracks files queued for the next checkpoint.
///
/// The stage is persisted as a compact binary format within the repository
/// metadata directory. Entries are keyed by their relative path to ensure
/// uniqueness and enable efficient lookups.
pub struct Stage {
    /// Staged entries keyed by relative file path.
    pub entries: BTreeMap<String, StageEntry>,
    path: PathBuf,
    config: AppConfig,
}

impl Stage {
    /// Loads the stage from disk, or returns an empty stage if none exists.
    ///
    /// The binary format is: `[entry_count: u32]` followed by repeated
    /// `[path_len: u32][path: bytes][hash: 64 bytes][mode: u32][size: u64]`.
    ///
    /// # Errors
    /// Returns an error if the file exists but cannot be read or parsed.
    pub fn load(root_dir: &Path, config: AppConfig) -> Result<Self> {
        let path = root_dir.join(&config.dir_name).join(&config.stage_file);
        let mut entries = BTreeMap::new();
        if path.exists() {
            let content = fs::read(&path)?;
            if content.len() >= 4 {
                let entry_count = u32::from_be_bytes(content[0..4].try_into()?) as usize;
                let mut pos = 4;
                for _ in 0..entry_count {
                    let path_len = u32::from_be_bytes(content[pos..pos + 4].try_into()?) as usize;
                    pos += 4;
                    let path_str = String::from_utf8(content[pos..pos + path_len].to_vec())?;
                    pos += path_len;
                    let hash = String::from_utf8(content[pos..pos + 64].to_vec())?;
                    pos += 64;
                    let mode = u32::from_be_bytes(content[pos..pos + 4].try_into()?);
                    pos += 4;
                    let size = u64::from_be_bytes(content[pos..pos + 8].try_into()?);
                    pos += 8;
                    entries.insert(
                        path_str.clone(),
                        StageEntry {
                            path: path_str,
                            hash,
                            mode,
                            size,
                        },
                    );
                }
            }
        }
        Ok(Self {
            entries,
            path,
            config,
        })
    }

    /// Adds or updates a file entry in the staging area.
    pub fn add(&mut self, path: String, hash: String, mode: u32, size: u64) {
        self.entries.insert(
            path.clone(),
            StageEntry {
                path,
                hash,
                mode,
                size,
            },
        );
    }

    /// Persists the current staging state to disk in binary format.
    ///
    /// # Errors
    /// Returns an error if the file cannot be written.
    pub fn save(&self) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());
        for entry in self.entries.values() {
            let path_bytes = entry.path.as_bytes();
            data.extend_from_slice(&(path_bytes.len() as u32).to_be_bytes());
            data.extend_from_slice(path_bytes);
            data.extend_from_slice(entry.hash.as_bytes());
            data.extend_from_slice(&entry.mode.to_be_bytes());
            data.extend_from_slice(&entry.size.to_be_bytes());
        }
        fs::write(&self.path, data)?;
        Ok(())
    }

    /// Converts the flat staging entries into a recursive [`Map`] tree
    /// and writes all map objects to the store.
    ///
    /// Directory structure is inferred from path separators: entries
    /// containing `/` or `\` are grouped into subdirectory maps.
    ///
    /// # Errors
    /// Returns an error if any map object cannot be written to storage.
    pub fn write_map(&self, storage: &Storage) -> Result<String> {
        let mut tree_map: BTreeMap<String, Vec<StageEntry>> = BTreeMap::new();
        let mut root_entries = Vec::new();

        for entry in self.entries.values() {
            if let Some(first_slash) = entry.path.find(['/', '\\']) {
                let dir = &entry.path[..first_slash];
                let sub_path = &entry.path[first_slash + 1..];
                let mut sub_entry = entry.clone();
                sub_entry.path = sub_path.to_string();
                tree_map.entry(dir.to_string()).or_default().push(sub_entry);
            } else {
                root_entries.push(entry.clone());
            }
        }

        let mut final_entries = Vec::new();
        for (dir, sub_entries) in tree_map {
            let sub_stage = Stage {
                entries: sub_entries
                    .into_iter()
                    .map(|e| (e.path.clone(), e))
                    .collect(),
                path: PathBuf::new(),
                config: self.config.clone(),
            };
            let sub_map_hash = sub_stage.write_map(storage)?;
            final_entries.push(MapEntry {
                mode: "40000".to_string(),
                name: dir,
                hash: sub_map_hash,
            });
        }
        for entry in root_entries {
            final_entries.push(MapEntry {
                mode: format!("{:o}", entry.mode),
                name: entry.path,
                hash: entry.hash,
            });
        }
        let map = Map::new(final_entries);
        map.save(storage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn stage_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let config = AppConfig::default();
        let repo_dir = dir.path().join(&config.dir_name);
        fs::create_dir_all(&repo_dir).unwrap();

        let mut stage = Stage::load(dir.path(), config.clone()).unwrap();
        assert!(stage.entries.is_empty());

        let fake_hash = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
        stage.add("src/main.rs".into(), fake_hash.into(), 0o100644, 1024);
        stage.add("Cargo.toml".into(), fake_hash.into(), 0o100644, 256);
        stage.save().unwrap();

        let loaded = Stage::load(dir.path(), config).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries.get("src/main.rs").unwrap().size, 1024);
        assert_eq!(loaded.entries.get("Cargo.toml").unwrap().mode, 0o100644);
    }

    #[test]
    fn stage_write_map_creates_nested_maps() {
        let dir = tempdir().unwrap();
        let config = AppConfig::default();
        let storage = Storage::new(dir.path().to_path_buf(), config.clone());

        let mut stage = Stage::load(dir.path(), config).unwrap();
        let chunk_hash = storage
            .hash_and_write(crate::storage::ObjectType::Chunk, b"code")
            .unwrap();
        stage.add("src/lib.rs".into(), chunk_hash.clone(), 0o100644, 4);
        stage.add("README.md".into(), chunk_hash.clone(), 0o100644, 4);

        let root_map_hash = stage.write_map(&storage).unwrap();
        let (t, data) = storage.read_object(&root_map_hash).unwrap();
        assert_eq!(t, crate::storage::ObjectType::Map);
        let root_map = Map::deserialize(&data).unwrap();
        assert_eq!(root_map.entries.len(), 2);
        let src_entry = root_map.entries.iter().find(|e| e.name == "src").unwrap();
        assert_eq!(src_entry.mode, "40000");
    }
}
