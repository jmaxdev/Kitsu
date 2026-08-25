use crate::config::AppConfig;
use crate::objects::Checkpoint;
use crate::storage::Storage;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Resolves the current HEAD to a checkpoint hash.
///
/// HEAD can be in one of two states:
/// - **Attached**: `"stream: {name}\n"` — follows a named stream (branch).
///   The actual hash is read from the stream file.
/// - **Detached**: a raw hash string — points directly to a checkpoint.
///
/// # Errors
/// Returns an error if file I/O fails. Returns `Ok(None)` if the
/// repository is empty (no checkpoints yet) or the referenced stream
/// has no checkpoints.
pub fn get_head_hash(current_dir: &Path, config: &AppConfig) -> Result<Option<String>> {
    let repo_dir = current_dir.join(&config.dir_name);
    let current_path = repo_dir.join(&config.current_file);
    if !current_path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&current_path)?;
    if content.starts_with("stream: ") {
        let stream = content.trim_start_matches("stream: ").trim();
        let path = repo_dir.join(&config.streams_dir).join(stream);
        if path.exists() {
            Ok(Some(fs::read_to_string(path)?.trim().to_string()))
        } else {
            Ok(None)
        }
    } else {
        Ok(Some(content.trim().to_string()))
    }
}

/// Resolves a user-provided target string to a concrete checkpoint hash.
///
/// Supports multiple resolution strategies, tried in order:
/// 1. **Stream name**: if a file exists under `streams/{target}`.
/// 2. **Seal name**: if a file exists under `seals/{target}`.
/// 3. **Ancestor notation** (`~N`): walks N parents back from HEAD.
/// 4. **Index notation** (`#N`): selects the Nth checkpoint from root (0-indexed).
/// 5. **Raw hash**: returned as-is if none of the above match.
///
/// # Errors
/// Returns an error if the referenced stream/seal cannot be read,
/// the ancestor depth exceeds the history, or the index is out of bounds.
pub fn resolve_target(
    target: &str,
    current_dir: &Path,
    config: &AppConfig,
    storage: &Storage,
) -> Result<String> {
    let repo_dir = current_dir.join(&config.dir_name);

    let stream_path = repo_dir.join(&config.streams_dir).join(target);
    if stream_path.exists() {
        return Ok(fs::read_to_string(stream_path)?.trim().to_string());
    }

    let seal_path = repo_dir.join("seals").join(target);
    if seal_path.exists() {
        return Ok(fs::read_to_string(seal_path)?.trim().to_string());
    }

    if let Some(stripped) = target.strip_prefix('~') {
        let n: usize = stripped.parse()?;
        let mut current =
            get_head_hash(current_dir, config)?.ok_or_else(|| anyhow::anyhow!("No history"))?;
        for _ in 0..n {
            let (_, content) = storage.read_object(&current)?;
            current = Checkpoint::deserialize(&content)?
                .parent_hash
                .ok_or_else(|| anyhow::anyhow!("No parent"))?;
        }
        return Ok(current);
    }

    if let Some(stripped) = target.strip_prefix('#') {
        let n: usize = stripped.parse()?;
        let head =
            get_head_hash(current_dir, config)?.ok_or_else(|| anyhow::anyhow!("No history"))?;
        let mut history = Vec::new();
        let mut cur = Some(head);
        while let Some(h) = cur {
            history.push(h.clone());
            let (_, content) = storage.read_object(&h)?;
            cur = Checkpoint::deserialize(&content)?.parent_hash;
        }
        history.reverse();
        return history
            .get(n)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("Index out of bounds"));
    }

    Ok(target.to_string())
}
