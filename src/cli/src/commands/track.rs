use anyhow::Result;
use kitsu_core::Repository;
use kitsu_core::exclude::Exclude;
use kitsu_core::objects::Chunk;
use kitsu_core::storage::Stage;
use std::path::{Path, PathBuf};

pub fn execute(current_dir: &Path, files: Vec<PathBuf>) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let mut stage = Stage::load(current_dir, repo.config().clone())?;
    let exclude = Exclude::load(current_dir);

    for f in files {
        if !f.exists() {
            continue;
        }
        let rel = f.strip_prefix(current_dir).unwrap_or(&f);
        if exclude.is_ignored(rel, f.is_dir()) {
            continue;
        }
        let hash = Chunk::new(std::fs::read(&f)?).save(repo.storage())?;
        let meta = std::fs::metadata(&f)?;
        stage.add(
            rel.to_string_lossy().to_string(),
            hash,
            if meta.is_dir() { 0o40000 } else { 0o100644 },
            meta.len(),
        );
    }
    stage.save()?;
    Ok(())
}
