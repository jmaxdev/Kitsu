use anyhow::Result;
use semver::{Prerelease, Version};
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

/// Computes the next version by bumping the specified component or prerelease tag.
///
/// Supports:
/// - `"major"`: increments major component and resets minor, patch, and prerelease.
/// - `"minor"`: increments minor component and resets patch and prerelease.
/// - `"patch"`: if a prerelease is active, finalizes into a stable release; otherwise increments patch.
/// - Prerelease identifiers (`"alpha"`, `"beta"`, `"rc"`, `"alpha.0"`, etc.):
///   - If current version already matches the prerelease tag, increments the numeric index.
///   - If starting a new prerelease cycle, bumps patch and initializes `<tag>.0`.
///
/// # Errors
/// Returns an error if the seals directory cannot be read or if the prerelease tag is invalid.
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
            latest.pre = Prerelease::EMPTY;
        }
        "minor" => {
            latest.minor += 1;
            latest.patch = 0;
            latest.pre = Prerelease::EMPTY;
        }
        "patch" => {
            if latest.pre.is_empty() {
                latest.patch += 1;
            } else {
                latest.pre = Prerelease::EMPTY;
            }
        }
        raw_tag => {
            let clean_tag = raw_tag.trim_start_matches('-');
            if clean_tag.contains('.') {
                if latest.pre.is_empty() {
                    latest.patch += 1;
                }
                latest.pre = Prerelease::new(clean_tag)?;
            } else {
                let current_pre = latest.pre.as_str();
                let prefix = format!("{}.", clean_tag);
                if !current_pre.is_empty()
                    && (current_pre == clean_tag || current_pre.starts_with(&prefix))
                {
                    let num = current_pre
                        .strip_prefix(&prefix)
                        .and_then(|n| n.parse::<u64>().ok())
                        .unwrap_or(0);
                    latest.pre = Prerelease::new(&format!("{}.{}", clean_tag, num + 1))?;
                } else {
                    if latest.pre.is_empty() {
                        latest.patch += 1;
                    }
                    latest.pre = Prerelease::new(&format!("{}.0", clean_tag))?;
                }
            }
        }
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

    #[test]
    fn seal_prerelease_bump_cycle() {
        let dir = tempdir().unwrap();
        let dir_name = ".kitsu";
        let v0 = Version::parse("0.1.0").unwrap();
        create_seal(&v0, "hash0", dir.path(), dir_name).unwrap();

        let v_alpha0 = bump_version("alpha", dir.path(), dir_name).unwrap();
        assert_eq!(v_alpha0, Version::parse("0.1.1-alpha.0").unwrap());
        create_seal(&v_alpha0, "hash_a0", dir.path(), dir_name).unwrap();

        let v_alpha1 = bump_version("alpha", dir.path(), dir_name).unwrap();
        assert_eq!(v_alpha1, Version::parse("0.1.1-alpha.1").unwrap());
        create_seal(&v_alpha1, "hash_a1", dir.path(), dir_name).unwrap();

        let v_rc0 = bump_version("rc", dir.path(), dir_name).unwrap();
        assert_eq!(v_rc0, Version::parse("0.1.1-rc.0").unwrap());
        create_seal(&v_rc0, "hash_rc0", dir.path(), dir_name).unwrap();

        let v_rc1 = bump_version("rc", dir.path(), dir_name).unwrap();
        assert_eq!(v_rc1, Version::parse("0.1.1-rc.1").unwrap());
        create_seal(&v_rc1, "hash_rc1", dir.path(), dir_name).unwrap();

        let v_final = bump_version("patch", dir.path(), dir_name).unwrap();
        assert_eq!(v_final, Version::parse("0.1.1").unwrap());
    }
}
