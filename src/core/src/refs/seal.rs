use anyhow::Result;
use semver::Version;
use std::fs;
use std::path::Path;

/// A resolved seal entry with its parsed semver version and checkpoint hash.
pub struct SealEntry {
    /// Parsed semantic version of this seal.
    pub version: Version,
    /// Checkpoint hash that this seal points to.
    pub hash: String,
}

/// Creates a new seal (version tag) pointing at the given checkpoint hash.
///
/// The seal file is named after the version string and contains the hash.
///
/// # Errors
/// Returns an error if directory creation or file writing fails.
pub fn create_seal(
    version: &Version,
    hash: &str,
    current_dir: &Path,
    dir_name: &str,
) -> Result<()> {
    let seals_dir = current_dir.join(dir_name).join("seals");
    fs::create_dir_all(&seals_dir)?;
    fs::write(seals_dir.join(version.to_string()), format!("{}\n", hash))?;
    Ok(())
}

/// Lists all seals (version tags) sorted by semver ascending.
///
/// Non-semver filenames in the seals directory are silently skipped.
///
/// # Errors
/// Returns an error if the seals directory cannot be read.
pub fn list_seals(current_dir: &Path, dir_name: &str) -> Result<Vec<SealEntry>> {
    let seals_dir = current_dir.join(dir_name).join("seals");
    fs::create_dir_all(&seals_dir)?;
    let mut seals = Vec::new();
    for entry in fs::read_dir(&seals_dir)? {
        let name = entry?.file_name().to_string_lossy().to_string();
        if let Ok(version) = Version::parse(&name) {
            let hash = fs::read_to_string(seals_dir.join(&name))?
                .trim()
                .to_string();
            seals.push(SealEntry { version, hash });
        }
    }
    seals.sort_by(|a, b| a.version.cmp(&b.version));
    Ok(seals)
}

/// Computes the next version by bumping the specified component.
///
/// Finds the latest existing seal version and increments the requested
/// component (major/minor/patch), resetting lower components to zero.
/// If no seals exist, starts from `0.0.0`.
///
/// # Errors
/// Returns an error if the seals directory cannot be read.
pub fn bump_version(bump: &str, current_dir: &Path, dir_name: &str) -> Result<Version> {
    let seals = list_seals(current_dir, dir_name)?;
    let mut latest = seals
        .last()
        .map(|s| s.version.clone())
        .unwrap_or_else(|| Version::new(0, 0, 0));
    match bump {
        "major" => {
            latest.major += 1;
            latest.minor = 0;
            latest.patch = 0;
        }
        "minor" => {
            latest.minor += 1;
            latest.patch = 0;
        }
        "patch" => {
            latest.patch += 1;
        }
        _ => return Err(anyhow::anyhow!("Invalid bump type: {}", bump)),
    }
    Ok(latest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn seal_create_list_and_bump_cycle() {
        let dir = tempdir().unwrap();
        let dir_name = ".kitsu";
        let v1 = Version::parse("0.1.0").unwrap();
        create_seal(&v1, "hash1", dir.path(), dir_name).unwrap();

        let seals = list_seals(dir.path(), dir_name).unwrap();
        assert_eq!(seals.len(), 1);
        assert_eq!(seals[0].version, v1);
        assert_eq!(seals[0].hash, "hash1");

        let bumped_patch = bump_version("patch", dir.path(), dir_name).unwrap();
        assert_eq!(bumped_patch, Version::parse("0.1.1").unwrap());

        let bumped_minor = bump_version("minor", dir.path(), dir_name).unwrap();
        assert_eq!(bumped_minor, Version::parse("0.2.0").unwrap());

        let bumped_major = bump_version("major", dir.path(), dir_name).unwrap();
        assert_eq!(bumped_major, Version::parse("1.0.0").unwrap());
    }
}
