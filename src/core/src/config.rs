//! Configuration types and default constants for Kitsu repositories.

use serde::{Deserialize, Serialize};

/// Default name for the Kitsu metadata directory.
pub const DIR_NAME: &str = ".kitsu";

/// Default name for the staging area file.
pub const STAGE_FILE: &str = "stage";

/// Default name for the HEAD reference file.
pub const CURRENT_FILE: &str = "CURRENT";

/// Default name for the streams (branches) directory.
pub const STREAMS_DIR: &str = "streams";

/// Default name for the content-addressable object store directory.
pub const OBJECTS_DIR: &str = "objects";

/// Configuration for a Kitsu repository instance.
///
/// Contains directory and file naming conventions used throughout the system.
/// All fields default to well-known values matching the standard Kitsu layout.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    /// Name of the metadata directory (e.g., `.kitsu`).
    pub dir_name: String,
    /// Name of the staging area file within the metadata directory.
    pub stage_file: String,
    /// Name of the HEAD reference file.
    pub current_file: String,
    /// Name of the streams (branches) subdirectory.
    pub streams_dir: String,
    /// Name of the objects storage subdirectory.
    pub objects_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            dir_name: DIR_NAME.to_string(),
            stage_file: STAGE_FILE.to_string(),
            current_file: CURRENT_FILE.to_string(),
            streams_dir: STREAMS_DIR.to_string(),
            objects_dir: OBJECTS_DIR.to_string(),
        }
    }
}

impl AppConfig {
    /// Loads the application configuration.
    ///
    /// Currently returns defaults. Future versions may read from a
    /// configuration file or environment variables.
    pub fn load() -> Self {
        AppConfig::default()
    }
}
