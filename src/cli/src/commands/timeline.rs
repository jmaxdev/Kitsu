use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use kitsu_core::objects::Checkpoint;
use std::path::Path;

pub fn execute(current_dir: &Path) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let mut cur = repo.head_hash()?;
    let mut history = Vec::new();
    while let Some(h) = cur {
        history.push(h.clone());
        let (_, data) = repo.storage().read_object(&h)?;
        cur = Checkpoint::deserialize(&data)?.parent_hash;
    }
    let total = history.len();
    for (i, hash) in history.iter().enumerate() {
        let (_, data) = repo.storage().read_object(hash)?;
        let cp = Checkpoint::deserialize(&data)?;
        println!(
            "{}",
            format!("#{} checkpoint {}", total - 1 - i, hash).yellow()
        );
        println!(
            "Author: {}\nDate:   {}\nMap:    {}\nSignature: {}",
            cp.author,
            chrono::DateTime::from_timestamp(cp.timestamp, 0).unwrap(),
            cp.map_hash.cyan(),
            if cp.signature.is_some() {
                "VALID".green()
            } else {
                "NONE".red()
            }
        );
        println!("\n    {}\n", cp.message.trim());
    }
    Ok(())
}
