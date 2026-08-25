use anyhow::Result;
use colored::*;
use kitsu_core::objects::Chunk;
use std::path::Path;

pub fn execute(file: &Path) -> Result<()> {
    if !file.exists() {
        return Err(anyhow::anyhow!("File not found"));
    }
    let data = std::fs::read(file)?;
    let hash = Chunk::new(data).hash();
    println!("{}", hash.green().bold());
    Ok(())
}
