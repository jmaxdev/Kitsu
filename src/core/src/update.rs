//! Release update notifications and in-place binary self-updating.
//!
//! Provides non-blocking release checking against the official GitHub
//! repository (`https://github.com/jmaxdev/Kitsu`) with local caching to
//! avoid API rate limits, and safe in-place binary updating via `self_replace`.

use anyhow::{Context, Result};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::PathBuf;

/// Official GitHub repository URL.
pub const REPO_URL: &str = "https://github.com/jmaxdev/Kitsu";

/// GitHub API endpoint for releases.
pub const RELEASES_API_URL: &str = "https://api.github.com/repos/jmaxdev/Kitsu/releases";

/// Minimum interval between network update checks (2 hours).
pub const CHECK_INTERVAL_SECONDS: i64 = 2 * 3600;

/// Information about a downloaded release asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseAsset {
    /// Asset filename (e.g., `kitsu-x86_64-pc-windows-msvc.zip`).
    pub name: String,
    /// Direct download URL for the asset.
    pub browser_download_url: String,
    /// Asset size in bytes.
    pub size: u64,
}

/// Metadata for a published GitHub release.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// Git tag name (e.g., `v0.0.4-alpha`).
    pub tag_name: String,
    /// Human-readable release title.
    pub name: Option<String>,
    /// Web URL to view the release on GitHub.
    pub html_url: String,
    /// Whether this release is marked as a prerelease on GitHub.
    pub prerelease: bool,
    /// List of attached artifact assets.
    pub assets: Vec<ReleaseAsset>,
}

impl ReleaseInfo {
    /// Parses the semantic version from the tag name (stripping leading 'v').
    pub fn parse_version(&self) -> Option<Version> {
        let clean = self.tag_name.trim_start_matches('v');
        Version::parse(clean).ok()
    }
}

/// Disk cache structure for update checks to prevent rate limiting.
#[derive(Serialize, Deserialize)]
struct UpdateCache {
    last_check_timestamp: i64,
    latest_known_version: Option<String>,
    latest_known_tag: Option<String>,
    latest_html_url: Option<String>,
}

/// Returns the path to the local update check cache file.
fn cache_file_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".kitsu_update_cache.json"))
}

/// Loads the cache file from disk if it exists.
fn load_cache() -> Option<UpdateCache> {
    let path = cache_file_path()?;
    if path.exists() {
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Saves the cache file to disk.
fn save_cache(cache: &UpdateCache) {
    if let Some(path) = cache_file_path()
        && let Ok(json) = serde_json::to_string(cache)
    {
        let _ = fs::write(path, json);
    }
}

/// Checks if a newer version of Kitsu is available on GitHub.
///
/// Uses a local cache to avoid querying GitHub more than once every 6 hours.
/// Returns `Some(ReleaseInfo)` if an update is available, or `None` if
/// the current version is up-to-date, network is unavailable, or rate limits occur.
pub fn check_for_update(current_version_str: &str) -> Option<ReleaseInfo> {
    let current_version = Version::parse(current_version_str.trim_start_matches('v')).ok()?;
    let now = chrono::Utc::now().timestamp();

    if let Some(cache) = load_cache() {
        let elapsed = now - cache.last_check_timestamp;
        if elapsed < CHECK_INTERVAL_SECONDS {
            if let Some(latest_tag) = &cache.latest_known_tag
                && let Some(latest_ver_str) = &cache.latest_known_version
                && let Ok(latest_ver) = Version::parse(latest_ver_str)
                && latest_ver > current_version
            {
                return Some(ReleaseInfo {
                    tag_name: latest_tag.clone(),
                    name: None,
                    html_url: cache
                        .latest_html_url
                        .clone()
                        .unwrap_or_else(|| REPO_URL.to_string()),
                    prerelease: latest_ver.pre.is_empty().not(),
                    assets: Vec::new(),
                });
            } else {
                return None;
            }
        }
    }

    let releases = fetch_releases(current_version_str).ok()?;
    let latest_release = find_highest_release(&releases)?;
    let latest_ver = latest_release.parse_version()?;

    let is_newer = latest_ver > current_version;

    let cache = UpdateCache {
        last_check_timestamp: now,
        latest_known_version: if is_newer {
            Some(latest_ver.to_string())
        } else {
            None
        },
        latest_known_tag: if is_newer {
            Some(latest_release.tag_name.clone())
        } else {
            None
        },
        latest_html_url: if is_newer {
            Some(latest_release.html_url.clone())
        } else {
            None
        },
    };
    save_cache(&cache);

    if is_newer { Some(latest_release) } else { None }
}

/// Fetches the list of releases from the GitHub Releases API.
fn fetch_releases(current_version: &str) -> Result<Vec<ReleaseInfo>> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(5))
        .build();

    let response: Vec<ReleaseInfo> = agent
        .get(RELEASES_API_URL)
        .set(
            "User-Agent",
            &format!("kitsu/{} (+{})", current_version, REPO_URL),
        )
        .set("Accept", "application/vnd.github.v3+json")
        .call()
        .context("Failed to connect to GitHub Releases API")?
        .into_json()
        .context("Failed to parse GitHub Releases JSON")?;

    Ok(response)
}

/// Finds the release with the highest semantic version.
fn find_highest_release(releases: &[ReleaseInfo]) -> Option<ReleaseInfo> {
    let mut parsed: Vec<(Version, &ReleaseInfo)> = releases
        .iter()
        .filter_map(|r| r.parse_version().map(|v| (v, r)))
        .collect();

    parsed.sort_by(|a, b| a.0.cmp(&b.0));
    parsed.last().map(|(_, r)| (*r).clone())
}

/// Identifies the asset filename target suffix for the current platform.
pub fn get_platform_asset_pattern() -> &'static str {
    if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc.zip"
    } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
        "x86_64-unknown-linux-gnu.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin.tar.gz"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin.tar.gz"
    } else {
        "unknown"
    }
}

/// Performs a self-update of the currently running Kitsu binary.
///
/// Downloads the release archive for the current operating system and
/// architecture, extracts the executable, and atomically replaces the
/// running binary on disk using `self_replace`.
///
/// # Errors
/// Returns an error if the network request fails, no matching asset is found,
/// or binary replacement fails.
pub fn perform_update(
    current_version_str: &str,
    target_tag: Option<&str>,
) -> Result<(ReleaseInfo, Version)> {
    let releases = fetch_releases(current_version_str)?;
    let target_release = if let Some(tag) = target_tag {
        releases
            .into_iter()
            .find(|r| r.tag_name == tag || r.tag_name == format!("v{}", tag))
            .ok_or_else(|| anyhow::anyhow!("Release with tag '{}' not found", tag))?
    } else {
        find_highest_release(&releases)
            .ok_or_else(|| anyhow::anyhow!("No releases found on GitHub"))?
    };

    let target_version = target_release
        .parse_version()
        .ok_or_else(|| anyhow::anyhow!("Invalid semver in tag '{}'", target_release.tag_name))?;

    let pattern = get_platform_asset_pattern();
    if pattern == "unknown" {
        return Err(anyhow::anyhow!(
            "Unsupported platform architecture for automatic binary update"
        ));
    }

    let asset = target_release
        .assets
        .iter()
        .find(|a| a.name.contains(pattern))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No compatible binary archive found for target '{}' in release {}",
                pattern,
                target_release.tag_name
            )
        })?;

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(60))
        .build();

    let mut response = agent
        .get(&asset.browser_download_url)
        .set(
            "User-Agent",
            &format!("kitsu/{} (+{})", current_version_str, REPO_URL),
        )
        .call()
        .context("Failed to download release archive")?
        .into_reader();

    let mut archive_bytes = Vec::new();
    response.read_to_end(&mut archive_bytes)?;

    let temp_dir = tempfile::tempdir()?;
    let extracted_binary = temp_dir
        .path()
        .join(if cfg!(windows) { "kitsu.exe" } else { "kitsu" });

    if asset.name.ends_with(".zip") {
        let cursor = std::io::Cursor::new(archive_bytes);
        let mut zip = zip::ZipArchive::new(cursor)?;
        for i in 0..zip.len() {
            let mut file = zip.by_index(i)?;
            if file.name().ends_with("kitsu") || file.name().ends_with("kitsu.exe") {
                let mut out = fs::File::create(&extracted_binary)?;
                std::io::copy(&mut file, &mut out)?;
                break;
            }
        }
    } else if asset.name.ends_with(".tar.gz") {
        let cursor = std::io::Cursor::new(archive_bytes);
        let dec = flate2::read::GzDecoder::new(cursor);
        let mut tar = tar::Archive::new(dec);
        for entry in tar.entries()? {
            let mut entry = entry?;
            let path = entry.path()?;
            if path
                .file_name()
                .map(|f| f.to_string_lossy() == "kitsu" || f.to_string_lossy() == "kitsu.exe")
                .unwrap_or(false)
            {
                entry.unpack(&extracted_binary)?;
                break;
            }
        }
    } else {
        return Err(anyhow::anyhow!(
            "Unknown archive format for asset: {}",
            asset.name
        ));
    }

    if !extracted_binary.exists() {
        return Err(anyhow::anyhow!(
            "Failed to extract kitsu binary from archive"
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&extracted_binary)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&extracted_binary, perms)?;
    }

    self_replace::self_replace(&extracted_binary)
        .context("Failed to replace running executable on disk")?;

    if let Some(path) = cache_file_path() {
        let _ = fs::remove_file(path);
    }

    Ok((target_release, target_version))
}

trait BoolExt {
    fn not(self) -> bool;
}

impl BoolExt for bool {
    fn not(self) -> bool {
        !self
    }
}
