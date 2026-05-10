use serde::{Deserialize, Serialize};

pub const APP_NAME: &str = env!("APP_NAME");
pub const DIR_NAME: &str = env!("DIR_NAME");
pub const ABOUT: &str = env!("ABOUT");

pub const STAGE_FILE: &str = "stage";
pub const CURRENT_FILE: &str = "CURRENT";
pub const STREAMS_DIR: &str = "streams";
pub const OBJECTS_DIR: &str = "objects";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub app_name: String,
    pub about: String,
    pub dir_name: String,
    pub stage_file: String,
    pub current_file: String,
    pub streams_dir: String,
    pub objects_dir: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: APP_NAME.to_string(),
            about: ABOUT.to_string(),
            dir_name: DIR_NAME.to_string(),
            stage_file: STAGE_FILE.to_string(),
            current_file: CURRENT_FILE.to_string(),
            streams_dir: STREAMS_DIR.to_string(),
            objects_dir: OBJECTS_DIR.to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        AppConfig::default()
    }
}
