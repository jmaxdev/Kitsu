use anyhow::Result;
use kitsu_core::Repository;
use kitsu_core::objects::{Checkpoint, MapEntry};
use kitsu_core::storage::Stage;
use std::path::Path;

pub fn execute(current_dir: &Path, old: Option<String>, new: Option<String>) -> Result<()> {
    let repo = Repository::open(current_dir)?;

    let old_map = if let Some(t) = old {
        let h = repo.resolve_target(&t)?;
        let (_, data) = repo.storage().read_object(&h)?;
        Some(Checkpoint::deserialize(&data)?.map_hash)
    } else {
        repo.head_hash()?.and_then(|h| {
            let (_, data) = repo.storage().read_object(&h).ok()?;
            Some(Checkpoint::deserialize(&data).ok()?.map_hash)
        })
    };

    if let Some(t) = new {
        let h = repo.resolve_target(&t)?;
        let (_, data) = repo.storage().read_object(&h)?;
        kitsu_core::diff::diff_maps(
            repo.storage(),
            old_map.as_deref(),
            &Checkpoint::deserialize(&data)?.map_hash,
            "",
        )?;
    } else {
        let stage = Stage::load(current_dir, repo.config().clone())?;
        let entries = stage
            .entries
            .values()
            .map(|e| MapEntry {
                mode: format!("{:o}", e.mode),
                name: e.path.clone(),
                hash: e.hash.clone(),
            })
            .collect();
        let hash = kitsu_core::objects::Map::new(entries).save(repo.storage())?;
        kitsu_core::diff::diff_maps(repo.storage(), old_map.as_deref(), &hash, "")?;
    }
    Ok(())
}
