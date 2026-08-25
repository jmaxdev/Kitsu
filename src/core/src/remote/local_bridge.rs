//! Local filesystem transport for offline / local folder remotes.
//!
//! Enables pushing and pulling objects directly to/from local directories,
//! mounted network drives, USB keys, and offline backup paths.

use crate::storage::Storage;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Local filesystem bridge for pushing and pulling objects between local directories.
pub struct LocalBridge;

impl LocalBridge {
    fn resolve_target_dir(remote_url: &str) -> PathBuf {
        let clean = remote_url.trim_start_matches("file://");
        let p = PathBuf::from(clean);
        if p.join(".kitsu").exists() {
            p.join(".kitsu")
        } else {
            p
        }
    }

    /// Pushes all reachable objects to a local directory.
    ///
    /// Copies loose objects and updates target seal pointers in the
    /// destination filesystem path.
    ///
    /// # Errors
    /// Returns an error if filesystem read or write operations fail.
    pub fn push(
        storage: &Storage,
        _repo_dir: &Path,
        remote_url: &str,
        target_name: &str,
        reachable: &HashSet<String>,
    ) -> Result<()> {
        let dest_dir = Self::resolve_target_dir(remote_url);
        let objects_dir = dest_dir.join("objects");
        let seals_dir = dest_dir.join("seals");
        fs::create_dir_all(&objects_dir)?;
        fs::create_dir_all(&seals_dir)?;

        for h in reachable {
            let data = storage.read_raw_object(h)?;
            let p = objects_dir.join(&h[..2]);
            fs::create_dir_all(&p)?;
            fs::write(p.join(&h[2..]), data)?;
        }

        if let Some(head_hash) = reachable.iter().next() {
            fs::write(seals_dir.join(target_name), format!("{}\n", head_hash))?;
        }

        Ok(())
    }

    /// Pulls objects from a local source directory.
    ///
    /// Imports loose objects and updates local seal pointers from
    /// the source filesystem path.
    ///
    /// # Errors
    /// Returns an error if source files cannot be read or local write fails.
    pub fn pull(
        storage: &Storage,
        repo_dir: &Path,
        remote_url: &str,
        target_name: &str,
    ) -> Result<Option<String>> {
        let src_dir = Self::resolve_target_dir(remote_url);
        let objects_dir = src_dir.join("objects");

        if objects_dir.exists() {
            for prefix_entry in fs::read_dir(&objects_dir)? {
                let prefix_entry = prefix_entry?;
                if prefix_entry.path().is_dir() {
                    let prefix = prefix_entry.file_name().to_string_lossy().to_string();
                    for obj_entry in fs::read_dir(prefix_entry.path())? {
                        let obj_entry = obj_entry?;
                        let suffix = obj_entry.file_name().to_string_lossy().to_string();
                        let hash = format!("{}{}", prefix, suffix);
                        let data = fs::read(obj_entry.path())?;
                        storage.write_raw(&hash, &data)?;
                    }
                }
            }
        }

        let seal_file = src_dir.join("seals").join(target_name);
        if seal_file.exists() {
            let hash = fs::read_to_string(&seal_file)?.trim().to_string();
            let local_seals = repo_dir.join("seals");
            fs::create_dir_all(&local_seals)?;
            fs::write(local_seals.join(target_name), format!("{}\n", hash))?;
            Ok(Some(hash))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use tempfile::tempdir;

    #[test]
    fn local_bridge_push_and_pull_cycle() {
        let src_dir = tempdir().unwrap();
        let dst_dir = tempdir().unwrap();

        let config = AppConfig::load();
        let src_storage = Storage::new(src_dir.path().to_path_buf(), config.clone());
        let dst_storage = Storage::new(dst_dir.path().to_path_buf(), config);

        let data = b"sample chunk content for local bridge";
        let hash = src_storage
            .hash_and_write(crate::storage::ObjectType::Chunk, data)
            .unwrap();

        let mut reachable = HashSet::new();
        reachable.insert(hash.clone());

        let dst_url = dst_dir.path().to_string_lossy().to_string();
        LocalBridge::push(&src_storage, src_dir.path(), &dst_url, "v1.0", &reachable).unwrap();

        // Verify dst_storage can pull it back
        let pulled_hash =
            LocalBridge::pull(&dst_storage, dst_dir.path(), &dst_url, "v1.0").unwrap();

        assert_eq!(pulled_hash, Some(hash.clone()));
        let (obj_type, read_data) = dst_storage.read_object(&hash).unwrap();
        assert_eq!(obj_type, crate::storage::ObjectType::Chunk);
        assert_eq!(read_data, data);
    }
}
