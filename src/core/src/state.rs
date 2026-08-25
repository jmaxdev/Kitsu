//! Working directory state calculation and status inspection.

use crate::config::AppConfig;
use crate::exclude::Exclude;
use crate::objects::{Checkpoint, Chunk, Map};
use crate::refs::get_head_hash;
use crate::storage::{Stage, Storage};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

/// Computed working tree state: what changed, what's staged, what's untracked.
///
/// This struct holds the result of comparing HEAD, the staging area,
/// and the working directory. It contains no presentation logic —
/// the CLI layer handles formatting and display.
pub struct WorkingState {
    /// New files staged but not in HEAD.
    pub staged_added: Vec<String>,
    /// Files staged with different content than HEAD.
    pub staged_modified: Vec<String>,
    /// Files in HEAD but removed from the stage.
    pub staged_deleted: Vec<String>,
    /// Files in the working directory with different content than the stage.
    pub unstaged_modified: Vec<String>,
    /// Staged/tracked files missing from the working directory.
    pub unstaged_deleted: Vec<String>,
    /// Files in the working directory not present in any tracking context.
    pub untracked: Vec<String>,
}

impl WorkingState {
    /// Returns `true` if there are no changes of any kind.
    pub fn is_clean(&self) -> bool {
        self.staged_added.is_empty()
            && self.staged_modified.is_empty()
            && self.staged_deleted.is_empty()
            && self.unstaged_modified.is_empty()
            && self.unstaged_deleted.is_empty()
            && self.untracked.is_empty()
    }
}

/// Computes the full working state by comparing HEAD, stage, and working directory.
///
/// # Errors
/// Returns an error if object reads, stage deserialization, or directory
/// traversal fails.
pub fn compute_state(
    current_dir: &Path,
    config: &AppConfig,
    storage: &Storage,
    exclude: &Exclude,
) -> Result<WorkingState> {
    let mut head_files: BTreeMap<String, String> = BTreeMap::new();

    let head_hash = get_head_hash(current_dir, config)?;
    if let Some(hash) = head_hash
        && let Ok((_, data)) = storage.read_object(&hash)
        && let Ok(cp) = Checkpoint::deserialize(&data)
    {
        collect_map_files(storage, &cp.map_hash, "", &mut head_files)?;
    }

    let stage = Stage::load(current_dir, config.clone())?;
    let mut staged_files: BTreeMap<String, String> = BTreeMap::new();
    for (path, entry) in &stage.entries {
        staged_files.insert(path.clone(), entry.hash.clone());
    }

    let mut wd_files: BTreeSet<String> = BTreeSet::new();
    collect_wd_files(current_dir, current_dir, exclude, &mut wd_files)?;

    let mut staged_added = Vec::new();
    let mut staged_modified = Vec::new();
    let mut staged_deleted = Vec::new();
    let mut unstaged_modified = Vec::new();
    let mut unstaged_deleted = Vec::new();
    let mut untracked = Vec::new();

    let all_tracked: BTreeSet<String> = head_files
        .keys()
        .chain(staged_files.keys())
        .cloned()
        .collect();

    for path in &all_tracked {
        let in_head = head_files.get(path);
        let in_stage = staged_files.get(path);
        match (in_head, in_stage) {
            (Some(h_hash), Some(s_hash)) => {
                if h_hash != s_hash {
                    staged_modified.push(path.clone());
                }
            }
            (None, Some(_)) => staged_added.push(path.clone()),
            (Some(_), None) => staged_deleted.push(path.clone()),
            (None, None) => unreachable!(),
        }
    }

    for path in &all_tracked {
        let in_stage = staged_files.get(path).or_else(|| head_files.get(path));
        let in_wd = wd_files.contains(path);
        if let Some(expected_hash) = in_stage {
            if in_wd {
                let full_path = current_dir.join(path);
                if let Ok(content) = fs::read(&full_path) {
                    let actual_hash = Chunk::new(content).hash();
                    if actual_hash != *expected_hash {
                        unstaged_modified.push(path.clone());
                    }
                }
            } else {
                unstaged_deleted.push(path.clone());
            }
        }
    }

    for path in &wd_files {
        if !all_tracked.contains(path) {
            untracked.push(path.clone());
        }
    }

    Ok(WorkingState {
        staged_added,
        staged_modified,
        staged_deleted,
        unstaged_modified,
        unstaged_deleted,
        untracked,
    })
}

/// Recursively collects all file paths and hashes from a map tree.
fn collect_map_files(
    storage: &Storage,
    map_hash: &str,
    prefix: &str,
    files: &mut BTreeMap<String, String>,
) -> Result<()> {
    let (_, data) = storage.read_object(map_hash)?;
    let map = Map::deserialize(&data)?;
    for entry in map.entries {
        let path = if prefix.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", prefix, entry.name)
        };
        if entry.mode == "40000" {
            collect_map_files(storage, &entry.hash, &path, files)?;
        } else {
            files.insert(path, entry.hash);
        }
    }
    Ok(())
}

/// Recursively collects all file paths in the working directory.
fn collect_wd_files(
    root: &Path,
    current: &Path,
    exclude: &Exclude,
    files: &mut BTreeSet<String>,
) -> Result<()> {
    if !current.exists() || !current.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel_path = path.strip_prefix(root).unwrap_or(&path);
        let is_dir = path.is_dir();
        if exclude.is_ignored(rel_path, is_dir) {
            continue;
        }
        if is_dir {
            collect_wd_files(root, &path, exclude, files)?;
        } else {
            files.insert(rel_path.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}
