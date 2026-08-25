use crate::config::AppConfig;
use crate::refs::get_head_hash;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Creates a new stream (branch) pointing at the current HEAD.
///
/// # Errors
/// Returns an error if HEAD is empty or file I/O fails.
pub fn create_stream(name: &str, current_dir: &Path, config: &AppConfig) -> Result<()> {
    let stream_dir = current_dir.join(&config.dir_name).join(&config.streams_dir);
    fs::create_dir_all(&stream_dir)?;
    if let Some(h) = get_head_hash(current_dir, config)? {
        fs::write(stream_dir.join(name), h)?;
    }
    Ok(())
}

/// Lists all stream (branch) names in the repository.
///
/// # Errors
/// Returns an error if the streams directory cannot be read.
pub fn list_streams(current_dir: &Path, config: &AppConfig) -> Result<Vec<String>> {
    let stream_dir = current_dir.join(&config.dir_name).join(&config.streams_dir);
    fs::create_dir_all(&stream_dir)?;
    let mut names = Vec::new();
    for entry in fs::read_dir(stream_dir)? {
        names.push(entry?.file_name().to_string_lossy().to_string());
    }
    Ok(names)
}

/// Renames an existing stream.
///
/// # Errors
/// Returns an error if the source stream doesn't exist or file I/O fails.
pub fn rename_stream(old: &str, new: &str, current_dir: &Path, config: &AppConfig) -> Result<()> {
    let stream_dir = current_dir.join(&config.dir_name).join(&config.streams_dir);
    let old_path = stream_dir.join(old);
    if old_path.exists() {
        fs::rename(old_path, stream_dir.join(new))?;
    }
    Ok(())
}

/// Deletes a stream (branch) by name.
///
/// # Errors
/// Returns an error if file I/O fails.
pub fn delete_stream(name: &str, current_dir: &Path, config: &AppConfig) -> Result<()> {
    let stream_dir = current_dir.join(&config.dir_name).join(&config.streams_dir);
    let path = stream_dir.join(name);
    if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn stream_create_list_rename_delete_lifecycle() {
        let dir = tempdir().unwrap();
        let config = AppConfig::default();
        let repo_dir = dir.path().join(&config.dir_name);
        fs::create_dir_all(&repo_dir).unwrap();
        fs::write(repo_dir.join(&config.current_file), "0123456789\n").unwrap();

        create_stream("feature-auth", dir.path(), &config).unwrap();
        let list1 = list_streams(dir.path(), &config).unwrap();
        assert!(list1.contains(&"feature-auth".to_string()));

        rename_stream("feature-auth", "feature-login", dir.path(), &config).unwrap();
        let list2 = list_streams(dir.path(), &config).unwrap();
        assert!(!list2.contains(&"feature-auth".to_string()));
        assert!(list2.contains(&"feature-login".to_string()));

        delete_stream("feature-login", dir.path(), &config).unwrap();
        let list3 = list_streams(dir.path(), &config).unwrap();
        assert!(!list3.contains(&"feature-login".to_string()));
    }
}
