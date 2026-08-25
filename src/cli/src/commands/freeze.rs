use anyhow::Result;
use kitsu_core::Repository;
use kitsu_core::identity::IdentityStore;
use kitsu_core::objects::Checkpoint;
use kitsu_core::storage::Stage;
use std::path::Path;

pub fn execute(current_dir: &Path, message: &str, sign: bool) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let stage = Stage::load(current_dir, repo.config().clone())?;
    let map_hash = stage.write_map(repo.storage())?;
    let id_store = IdentityStore::load(current_dir);
    let active = id_store.get_active();
    let parent = repo.head_hash()?;

    let mut cp = Checkpoint {
        map_hash,
        parent_hash: parent,
        author: format!("{} <{}>", active.name, active.email),
        message: message.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        signature: None,
    };
    if sign {
        cp.signature = Some(active.sign(&cp.serialize())?);
    }
    let hash = cp.save(repo.storage())?;
    repo.update_head(&hash)?;
    println!("[freeze {}] {}", hash, cp.message);
    Ok(())
}
