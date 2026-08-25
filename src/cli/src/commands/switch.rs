use anyhow::Result;
use kitsu_core::Repository;
use kitsu_core::objects::Checkpoint;
use std::fs;
use std::path::Path;

pub fn execute(current_dir: &Path, target: &str) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let hash = repo.resolve_target(target)?;
    let cp = Checkpoint::deserialize(&repo.storage().read_object(&hash)?.1)?;
    repo.apply_map_to_disk(&cp.map_hash, current_dir)?;

    let streams_path = repo
        .repo_dir()
        .join(&repo.config().streams_dir)
        .join(target);
    if streams_path.exists() {
        fs::write(
            repo.repo_dir().join(&repo.config().current_file),
            format!("stream: {}\n", target),
        )?;
    } else {
        fs::write(
            repo.repo_dir().join(&repo.config().current_file),
            format!("{}\n", hash),
        )?;
    }
    println!("Switched to {}", target);
    Ok(())
}
