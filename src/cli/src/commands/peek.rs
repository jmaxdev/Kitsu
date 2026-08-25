use anyhow::Result;
use kitsu_core::Repository;
use std::path::Path;

pub fn execute(current_dir: &Path, hash: &str) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let (_, d) = repo.storage().read_object(hash)?;
    println!("{}", String::from_utf8_lossy(&d));
    Ok(())
}
