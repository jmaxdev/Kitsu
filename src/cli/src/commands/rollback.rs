use anyhow::Result;
use kitsu_core::Repository;
use kitsu_core::objects::Checkpoint;
use std::path::Path;

pub fn execute(current_dir: &Path, target: Option<String>) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let hash = if let Some(t) = target {
        repo.resolve_target(&t)?
    } else {
        let head = repo
            .head_hash()?
            .ok_or_else(|| anyhow::anyhow!("No head"))?;
        Checkpoint::deserialize(&repo.storage().read_object(&head)?.1)?
            .parent_hash
            .ok_or_else(|| anyhow::anyhow!("No parent"))?
    };
    let cp = Checkpoint::deserialize(&repo.storage().read_object(&hash)?.1)?;
    repo.apply_map_to_disk(&cp.map_hash, current_dir)?;
    repo.update_head(&hash)?;
    println!("Rolled back to {}", hash);
    Ok(())
}
