//! Git repository importer for Kitsu.
//!
//! Converts existing Git repositories (local `.git` or cloned from GitHub/GitLab)
//! into fully native Kitsu repositories, transforming Git commits, trees, and blobs
//! into Kitsu Checkpoints, Maps, and Chunks, and linking origin remotes.

use anyhow::Result;
use std::fs;
use std::path::Path;

use crate::config::AppConfig;
use crate::global_registry::GlobalRegistry;
use crate::objects::{Checkpoint, Map, MapEntry};
use crate::remote::RemoteRegistry;
use crate::repository::Repository;
use crate::storage::{ObjectType, Storage};

/// Imports an existing Git repository into a Kitsu repository.
///
/// Converts the current Git HEAD commit and tree into Kitsu Checkpoint and Map
/// objects, sets stream `main`, imports configured Git remotes to Kitsu remotes
/// (with default data branch `kitsu-data`), and registers the repository in the global store.
///
/// # Errors
/// Returns an error if Git repository cannot be opened, has no commits, or file I/O fails.
pub fn import_git_repository(repo_path: &Path) -> Result<String> {
    let git_repo = git2::Repository::open(repo_path).map_err(|e| {
        anyhow::anyhow!(
            "Failed to open Git repository at {}: {}",
            repo_path.display(),
            e
        )
    })?;

    let head = git_repo
        .head()
        .map_err(|e| anyhow::anyhow!("Git repository HEAD not found: {}", e))?;

    let commit = head
        .peel_to_commit()
        .map_err(|e| anyhow::anyhow!("Failed to peel HEAD to Git commit: {}", e))?;

    let repo_dir = repo_path.join(".kitsu");
    if !repo_dir.exists() {
        Repository::init(repo_path)?;
    }

    let config = AppConfig::load();
    let storage = Storage::new(repo_path.to_path_buf(), config.clone());

    let tree = commit.tree()?;
    let mut stage_entries = Vec::new();
    let root_map_hash =
        import_git_tree_recursive(&git_repo, &tree, &storage, "", &mut stage_entries)?;

    let mut stage = crate::storage::Stage::load(repo_path, config.clone())?;
    for (rel_path, hash, mode, size) in stage_entries {
        stage.add(rel_path, hash, mode, size);
    }
    stage.save()?;

    let author_name = commit.author().name().unwrap_or("Git User").to_string();
    let author_email = commit
        .author()
        .email()
        .unwrap_or("git@example.com")
        .to_string();
    let author = format!("{} <{}>", author_name, author_email);
    let message = commit.message().unwrap_or("Imported from Git").to_string();
    let timestamp = commit.time().seconds();

    let checkpoint = Checkpoint {
        parent_hash: None,
        map_hash: root_map_hash,
        author,
        message,
        timestamp,
        signature: None,
    };

    let cp_hash = checkpoint.save(&storage)?;

    fs::write(
        repo_dir.join(&config.streams_dir).join("main"),
        format!("{}\n", cp_hash),
    )?;
    fs::write(repo_dir.join(&config.current_file), "stream: main\n")?;

    if let Ok(remotes) = git_repo.remotes() {
        for remote_name in remotes.iter().flatten() {
            if let Ok(remote) = git_repo.find_remote(remote_name)
                && let Some(url) = remote.url()
            {
                RemoteRegistry::add(&repo_dir, remote_name, url, Some("kitsu-data"))?;
                if remote_name == "origin" {
                    RemoteRegistry::set_default(&repo_dir, "origin")?;
                }
            }
        }
    }

    let _ = GlobalRegistry::register(repo_path);

    Ok(cp_hash)
}

/// Recursively converts a Git tree into Kitsu Map objects and Chunks.
fn import_git_tree_recursive(
    git_repo: &git2::Repository,
    tree: &git2::Tree,
    storage: &Storage,
    prefix: &str,
    stage_entries: &mut Vec<(String, String, u32, u64)>,
) -> Result<String> {
    let mut entries = Vec::new();

    for entry in tree.iter() {
        let name = match entry.name() {
            Some(n) => n.to_string(),
            None => continue,
        };

        if name == ".kitsu" || name == ".git" {
            continue;
        }

        let rel_path = if prefix.is_empty() {
            name.clone()
        } else {
            format!("{}/{}", prefix, name)
        };

        let obj = entry.to_object(git_repo)?;
        if let Some(sub_tree) = obj.as_tree() {
            let sub_map_hash =
                import_git_tree_recursive(git_repo, sub_tree, storage, &rel_path, stage_entries)?;
            entries.push(MapEntry {
                mode: "40000".to_string(),
                hash: sub_map_hash,
                name,
            });
        } else if let Some(blob) = obj.as_blob() {
            let file_mode = format!("{:o}", entry.filemode());
            let mode_num = if file_mode.starts_with("100755") {
                0o100755
            } else {
                0o100644
            };
            let chunk_hash = storage.hash_and_write(ObjectType::Chunk, blob.content())?;
            stage_entries.push((
                rel_path,
                chunk_hash.clone(),
                mode_num,
                blob.content().len() as u64,
            ));
            entries.push(MapEntry {
                mode: if file_mode.starts_with("100755") {
                    "100755".to_string()
                } else {
                    "100644".to_string()
                },
                hash: chunk_hash,
                name,
            });
        }
    }

    let map = Map { entries };
    let map_hash = map.save(storage)?;
    Ok(map_hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_import_git_repo() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path();

        let git_repo = git2::Repository::init(repo_path).unwrap();
        let file_path = repo_path.join("hello.txt");
        fs::write(&file_path, "Hello from Git to Kitsu!").unwrap();

        let mut index = git_repo.index().unwrap();
        index.add_path(Path::new("hello.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = git_repo.find_tree(tree_id).unwrap();

        let sig = git2::Signature::now("Test Committer", "committer@example.com").unwrap();
        git_repo
            .commit(Some("HEAD"), &sig, &sig, "Initial git commit", &tree, &[])
            .unwrap();

        let cp_hash = import_git_repository(repo_path).unwrap();
        assert!(!cp_hash.is_empty());

        let kitsu_repo = Repository::open(repo_path).unwrap();
        assert_eq!(kitsu_repo.head_hash().unwrap(), Some(cp_hash.clone()));

        let (obj_type, data) = kitsu_repo.storage().read_object(&cp_hash).unwrap();
        assert_eq!(obj_type, ObjectType::Checkpoint);
        let cp = Checkpoint::deserialize(&data).unwrap();
        assert_eq!(cp.author, "Test Committer <committer@example.com>");
        assert_eq!(cp.message, "Initial git commit");
    }
}
