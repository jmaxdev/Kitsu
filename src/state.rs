use crate::config::AppConfig;
use crate::exclude::Exclude;
use crate::index::Stage;
use crate::objects::{Checkpoint, Map};
use crate::storage::Storage;
use anyhow::Result;
use colored::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

pub fn run_state(
    current_dir: &Path,
    config: &AppConfig,
    storage: &Storage,
    exclude: &Exclude,
) -> Result<()> {
    let mut head_files: BTreeMap<String, String> = BTreeMap::new();

    // 1. Load HEAD Map
    let head_hash = crate::get_head_hash(current_dir, config)?;
    if let Some(hash) = head_hash
        && let Ok((_, data)) = storage.read_object(&hash)
        && let Ok(cp) = Checkpoint::deserialize(&data)
    {
        collect_map_files(storage, &cp.map_hash, "", &mut head_files)?;
    }

    // 2. Load Stage
    let stage = Stage::load(current_dir, config.clone())?;
    let mut staged_files: BTreeMap<String, String> = BTreeMap::new();
    for (path, entry) in &stage.entries {
        staged_files.insert(path.clone(), entry.hash.clone());
    }

    // 3. Scan Working Directory
    let mut wd_files: BTreeSet<String> = BTreeSet::new();
    collect_wd_files(current_dir, current_dir, exclude, &mut wd_files)?;

    // 4. Compare
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

    // Check Staged Changes (Stage vs HEAD)
    for path in &all_tracked {
        let in_head = head_files.get(path);
        let in_stage = staged_files.get(path);

        match (in_head, in_stage) {
            (Some(h_hash), Some(s_hash)) => {
                if h_hash != s_hash {
                    staged_modified.push(path.clone());
                }
            }
            (None, Some(_)) => {
                staged_added.push(path.clone());
            }
            (Some(_), None) => {
                staged_deleted.push(path.clone());
            }
            (None, None) => unreachable!(),
        }
    }

    // Check Unstaged Changes (WD vs Stage)
    for path in &all_tracked {
        let in_stage = staged_files.get(path).or_else(|| head_files.get(path));
        let in_wd = wd_files.contains(path);

        if let Some(expected_hash) = in_stage {
            if in_wd {
                let full_path = current_dir.join(path);
                if let Ok(content) = fs::read(&full_path) {
                    let chunk = crate::objects::Chunk::new(content);
                    let actual_hash = chunk.hash();
                    if actual_hash != *expected_hash {
                        unstaged_modified.push(path.clone());
                    }
                }
            } else {
                unstaged_deleted.push(path.clone());
            }
        }
    }

    // Check Untracked (WD vs All Tracked)
    for path in &wd_files {
        if !all_tracked.contains(path) {
            untracked.push(path.clone());
        }
    }

    // Print Status
    println!("{}", "Kitsu Working State".bold());

    if !staged_added.is_empty() || !staged_modified.is_empty() || !staged_deleted.is_empty() {
        println!("\nChanges to be frozen:");
        println!("  (use \"kitsu rollback\" to unstage)");
        for p in &staged_added {
            println!("\t{}", format!("new file:   {}", p).green());
        }
        for p in &staged_modified {
            println!("\t{}", format!("modified:   {}", p).green());
        }
        for p in &staged_deleted {
            println!("\t{}", format!("deleted:    {}", p).green());
        }
    }

    if !unstaged_modified.is_empty() || !unstaged_deleted.is_empty() {
        println!("\nChanges not staged for freeze:");
        println!("  (use \"kitsu track <file>...\" to update what will be frozen)");
        for p in &unstaged_modified {
            println!("\t{}", format!("modified:   {}", p).red());
        }
        for p in &unstaged_deleted {
            println!("\t{}", format!("deleted:    {}", p).red());
        }
    }

    if !untracked.is_empty() {
        println!("\nUntracked files:");
        println!("  (use \"kitsu track <file>...\" to include in what will be frozen)");
        for p in &untracked {
            println!("\t{}", p.red());
        }
    }

    if staged_added.is_empty()
        && staged_modified.is_empty()
        && staged_deleted.is_empty()
        && unstaged_modified.is_empty()
        && unstaged_deleted.is_empty()
        && untracked.is_empty()
    {
        println!("\n{}", "nothing to freeze, working tree clean".green());
    }

    Ok(())
}

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
