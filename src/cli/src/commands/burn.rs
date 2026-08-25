use anyhow::Result;
use kitsu_core::Repository;
use std::path::Path;

pub fn execute(current_dir: &Path, hash: Option<String>, aggressive: bool) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let target = if let Some(h) = hash {
        h
    } else {
        repo.head_hash()?.unwrap()
    };
    let (d, f) = target.split_at(2);
    let p = repo
        .repo_dir()
        .join(&repo.config().objects_dir)
        .join(d)
        .join(f);
    if p.exists() {
        std::fs::remove_file(p)?;
    }
    if aggressive {
        println!("Aggressive cleanup...");
    }
    println!("Burned.");
    Ok(())
}
