use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use semver::Version;
use std::path::Path;

pub fn execute(
    current_dir: &Path,
    version: Option<String>,
    bump: Option<String>,
    list: bool,
) -> Result<()> {
    let repo = Repository::open(current_dir)?;
    let dir_name = &repo.config().dir_name;

    if list {
        let seals = kitsu_core::refs::list_seals(current_dir, dir_name)?;
        for s in seals {
            println!("  {} -> {}", s.version.to_string().green(), s.hash.yellow());
        }
        return Ok(());
    }

    let head = repo
        .head_hash()?
        .ok_or_else(|| anyhow::anyhow!("No head"))?;

    let final_v = if let Some(b) = bump {
        kitsu_core::refs::bump_version(&b, current_dir, dir_name)?
    } else if let Some(v) = version {
        let clean_v = v.trim_start_matches('v');
        Version::parse(clean_v)?
    } else {
        return Err(anyhow::anyhow!(
            "No version specified (provide version or --bump)"
        ));
    };

    kitsu_core::refs::create_seal(&final_v, &head, current_dir, dir_name)?;
    println!("Sealed as {}", final_v);
    Ok(())
}
