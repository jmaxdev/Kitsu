use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use kitsu_core::objects::Checkpoint;
use std::path::Path;

pub fn execute(current_dir: &Path, target: Option<String>) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let hash = if let Some(t) = target {
        repo.resolve_target(&t)?
    } else {
        repo.head_hash()?
            .ok_or_else(|| anyhow::anyhow!("No head"))?
    };
    let (_, cp_data) = repo.storage().read_object(&hash)?;
    let cp = Checkpoint::deserialize(&cp_data)?;
    println!(
        "{}",
        format!("--- Contents of Checkpoint {} ---", hash)
            .cyan()
            .bold()
    );
    println!(
        "{:<10} {:<64} {:<10} {:<20}",
        "MODE", "SHA-256 HASH", "SIZE", "NAME"
    );
    println!("{}", "-".repeat(110));

    fn list_recursive(
        storage: &kitsu_core::storage::Storage,
        map_hash: &str,
        prefix: &str,
    ) -> Result<()> {
        let (_, data) = storage.read_object(map_hash)?;
        let map = kitsu_core::objects::Map::deserialize(&data)?;
        for e in map.entries {
            let full_path = if prefix.is_empty() {
                e.name.clone()
            } else {
                format!("{}/{}", prefix, e.name)
            };
            if e.mode == "40000" {
                list_recursive(storage, &e.hash, &full_path)?;
            } else {
                let (_, blob) = storage.read_object(&e.hash)?;
                println!(
                    "{:<10} {:<64} {:<10} {:<20}",
                    e.mode,
                    e.hash,
                    blob.len(),
                    full_path
                );
            }
        }
        Ok(())
    }
    list_recursive(repo.storage(), &cp.map_hash, "")?;
    Ok(())
}
