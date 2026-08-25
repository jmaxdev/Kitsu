use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use kitsu_core::objects::Checkpoint;
use kitsu_core::storage::ObjectType;
use std::io::Read;
use std::path::Path;

pub fn execute(current_dir: &Path, input: &Path) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let file = std::fs::File::open(input)?;
    let dec = flate2::read::GzDecoder::new(file);
    let mut tar = tar::Archive::new(dec);

    let mut target_name = None;
    let mut target_hash = None;

    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;

        if path == "MANIFEST" {
            let manifest_str = String::from_utf8_lossy(&data);
            for line in manifest_str.lines() {
                if let Some(name) = line.strip_prefix("target_name: ") {
                    target_name = Some(name.to_string());
                }
                if let Some(hash) = line.strip_prefix("target_hash: ") {
                    target_hash = Some(hash.to_string());
                }
            }
            continue;
        }

        let hash = path.split(':').next().unwrap();
        repo.storage().write_raw(hash, &data)?;
    }
    println!("Import complete.");

    if let Some(hash) = target_hash {
        let name = target_name.unwrap_or_else(|| "unknown".to_string());
        println!("Image target: {} ({})", name.cyan(), hash.bright_black());

        let head_hash = repo.head_hash().unwrap_or(None);
        if head_hash.is_none() {
            println!(
                "{}",
                "Repository is empty. Auto-applying imported image...".yellow()
            );
            if let Ok((ObjectType::Checkpoint, cp_data)) = repo.storage().read_object(&hash)
                && let Ok(cp) = Checkpoint::deserialize(&cp_data)
            {
                if repo.apply_map_to_disk(&cp.map_hash, current_dir).is_ok() {
                    std::fs::write(
                        repo.repo_dir().join(&repo.config().current_file),
                        format!("{}\n", hash),
                    )
                    .ok();
                    println!("Switched to {}", hash.green());
                } else {
                    println!("Failed to apply working tree.");
                }
            }
        } else {
            println!(
                "To apply it to your working tree, run: {} {}",
                "kitsu switch".bold(),
                hash
            );
        }
    }
    Ok(())
}
