use anyhow::Result;
use colored::*;
use kitsu_core::Repository;
use semver::Version;
use std::path::Path;

use crate::app::BumpType;

pub fn execute(
    current_dir: &Path,
    version: Option<String>,
    bump: Option<BumpType>,
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
        let bump_str = match b {
            BumpType::Major => "major",
            BumpType::Minor => "minor",
            BumpType::Patch => "patch",
        };
        kitsu_core::refs::bump_version(bump_str, current_dir, dir_name)?
    } else if let Some(v) = version {
        Version::parse(&v)?
    } else {
        return Err(anyhow::anyhow!("No version specified"));
    };

    kitsu_core::refs::create_seal(&final_v, &head, current_dir, dir_name)?;
    println!("Sealed as {}", final_v);
    Ok(())
}
