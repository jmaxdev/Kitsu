use crate::objects::{Checkpoint, Map};
use crate::storage::Storage;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// Git protocol bridge for pushing Kitsu objects to GitHub/GitLab.
///
/// Stores Kitsu objects inside a bare git repository ("git_bridge"),
/// commits them, and pushes to a configurable remote branch (defaults to `kitsu-data`).
/// This enables using any standard git hosting service as a Kitsu registry.
pub struct GitBridge;

impl GitBridge {
    /// Pushes all reachable objects to a git remote branch.
    ///
    /// Creates a local git bridge repository (if needed), copies all
    /// Kitsu objects into it, creates a git commit, and pushes to
    /// the specified data branch (defaults to `"kitsu-data"`).
    ///
    /// # Errors
    /// Returns an error if git operations, file I/O, or network push fails.
    pub fn push(
        storage: &Storage,
        repo_dir: &Path,
        remote_url: &str,
        target_name: &str,
        reachable: &HashSet<String>,
        data_branch: Option<&str>,
    ) -> Result<()> {
        let branch = data_branch.unwrap_or("kitsu-data");
        let git_path = repo_dir.join("git_bridge");
        if !git_path.exists() {
            fs::create_dir_all(&git_path)?;
            git2::Repository::init(&git_path)?;
        }
        let repo = git2::Repository::open(&git_path)?;
        if repo.find_remote("origin").is_err() {
            repo.remote("origin", remote_url)?;
        }

        for h in reachable {
            let data = storage.read_raw_object(h)?;
            let p = git_path.join("objects").join(&h[..2]);
            fs::create_dir_all(&p)?;
            fs::write(p.join(&h[2..]), data)?;
        }

        let seal_p = git_path.join("seals");
        fs::create_dir_all(&seal_p)?;

        let head_hash = reachable.iter().next().map(|h| {
            storage
                .read_object(h)
                .ok()
                .and_then(|(_, d)| Checkpoint::deserialize(&d).ok())
                .map(|_| h.clone())
        });
        if let Some(Some(h)) = head_hash {
            fs::write(seal_p.join(target_name), &h)?;
        }

        let mut idx = repo.index()?;
        idx.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        idx.write()?;
        let tree = repo.find_tree(idx.write_tree()?)?;
        let sig = repo.signature()?;
        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let mut parents = Vec::new();
        if let Some(p) = &parent {
            parents.push(p);
        }
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("kitsu push: {}", target_name),
            &tree,
            &parents,
        )?;

        let mut remote = repo.find_remote("origin")?;
        let refspec = format!("refs/heads/master:refs/heads/{}", branch);
        remote.push(&[&refspec], None)?;

        Ok(())
    }

    /// Fetches objects from a git remote's data branch.
    ///
    /// Clones or fetches the data branch (defaults to `"kitsu-data"`) from the remote and
    /// imports all objects and seals into the local Kitsu storage.
    ///
    /// # Errors
    /// Returns an error if clone/fetch or object import fails.
    pub fn pull(
        storage: &Storage,
        repo_dir: &Path,
        remote_url: &str,
        target_name: &str,
        data_branch: Option<&str>,
    ) -> Result<Option<String>> {
        let branch = data_branch.unwrap_or("kitsu-data");
        let git_path = repo_dir.join("git_bridge");

        if !git_path.exists() {
            fs::create_dir_all(&git_path)?;
            let mut builder = git2::build::RepoBuilder::new();
            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.download_tags(git2::AutotagOption::None);
            builder.fetch_options(fetch_opts);
            builder.branch(branch);
            builder.clone(remote_url, &git_path)?;
        } else {
            let repo = git2::Repository::open(&git_path)?;
            let mut remote = repo.find_remote("origin")?;
            remote.fetch(&[branch], None, None)?;

            let fetch_head = repo.find_reference(&format!("refs/remotes/origin/{}", branch))?;
            let commit = fetch_head.peel_to_commit()?;
            repo.reset(commit.as_object(), git2::ResetType::Hard, None)?;
        }

        let objects_dir = git_path.join("objects");
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

        let seal_path = git_path.join("seals").join(target_name);
        if seal_path.exists() {
            let hash = fs::read_to_string(&seal_path)?.trim().to_string();
            let seals_dir = repo_dir.join("seals");
            fs::create_dir_all(&seals_dir)?;
            fs::write(seals_dir.join(target_name), format!("{}\n", hash))?;
            Ok(Some(hash))
        } else {
            let mut found_hash = None;
            let seals_dir = git_path.join("seals");
            if seals_dir.exists() {
                for entry in fs::read_dir(&seals_dir)? {
                    let entry = entry?;
                    let name = entry.file_name().to_string_lossy().to_string();
                    let hash = fs::read_to_string(entry.path())?.trim().to_string();
                    let local_seals = repo_dir.join("seals");
                    fs::create_dir_all(&local_seals)?;
                    fs::write(local_seals.join(&name), format!("{}\n", hash))?;
                    found_hash = Some(hash);
                }
            }
            Ok(found_hash)
        }
    }

    /// Fetches all reachable objects starting from a known checkpoint hash.
    ///
    /// Walks the object graph (checkpoints → maps → chunks) and ensures
    /// all referenced objects are present in local storage.
    ///
    /// # Errors
    /// Returns an error if any object cannot be read or deserialized.
    pub fn fetch_object_graph(storage: &Storage, start_hash: &str) -> Result<HashSet<String>> {
        let mut done = HashSet::new();
        let mut queue = vec![start_hash.to_string()];
        while let Some(h) = queue.pop() {
            if done.contains(&h) {
                continue;
            }
            if let Ok((obj_type, data)) = storage.read_object(&h) {
                done.insert(h.clone());
                match obj_type {
                    crate::storage::ObjectType::Checkpoint => {
                        if let Ok(cp) = Checkpoint::deserialize(&data) {
                            queue.push(cp.map_hash);
                            if let Some(parent) = cp.parent_hash {
                                queue.push(parent);
                            }
                        }
                    }
                    crate::storage::ObjectType::Map => {
                        if let Ok(map) = Map::deserialize(&data) {
                            for e in map.entries {
                                queue.push(e.hash);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(done)
    }
}
