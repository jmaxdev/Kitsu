use anyhow::Result;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use crate::storage::Storage;
use crate::objects::{Map, MapEntry};
use crate::config::AppConfig;

#[derive(Debug, Clone)]
pub struct StageEntry {
    pub hash: String,
    pub path: String,
    pub mode: u32,
    pub size: u64,
}

pub struct Stage {
    pub entries: BTreeMap<String, StageEntry>,
    path: PathBuf,
    config: AppConfig,
}

impl Stage {
    pub fn load(root_dir: &Path, config: AppConfig) -> Result<Self> {
        let path = root_dir.join(&config.dir_name).join(&config.stage_file);
        let mut entries = BTreeMap::new();
        if path.exists() {
            let content = fs::read(&path)?;
            if content.len() >= 4 {
                let entry_count = u32::from_be_bytes(content[0..4].try_into()?);
                let mut pos = 4;
                for _ in 0..entry_count {
                    let path_len = u32::from_be_bytes(content[pos..pos+4].try_into()?) as usize;
                    pos += 4;
                    let path_str = String::from_utf8(content[pos..pos+path_len].to_vec())?;
                    pos += path_len;
                    let hash = String::from_utf8(content[pos..pos+64].to_vec())?;
                    pos += 64;
                    let mode = u32::from_be_bytes(content[pos..pos+4].try_into()?);
                    pos += 4;
                    let size = u64::from_be_bytes(content[pos..pos+8].try_into()?);
                    pos += 8;
                    entries.insert(path_str.clone(), StageEntry {
                        path: path_str,
                        hash,
                        mode,
                        size,
                    });
                }
            }
        }
        Ok(Self { entries, path, config })
    }

    pub fn add(&mut self, path: String, hash: String, mode: u32, size: u64) {
        self.entries.insert(path.clone(), StageEntry {
            path,
            hash,
            mode,
            size,
        });
    }

    pub fn save(&self) -> Result<()> {
        let mut data = Vec::new();
        data.extend_from_slice(&(self.entries.len() as u32).to_be_bytes());
        for entry in self.entries.values() {
            let path_bytes = entry.path.as_bytes();
            data.extend_from_slice(&(path_bytes.len() as u32).to_be_bytes());
            data.extend_from_slice(path_bytes);
            data.extend_from_slice(entry.hash.as_bytes());
            data.extend_from_slice(&entry.mode.to_be_bytes());
            data.extend_from_slice(&entry.size.to_be_bytes());
        }
        fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn write_map(&self, storage: &Storage) -> Result<String> {
        let mut tree_map: BTreeMap<String, Vec<StageEntry>> = BTreeMap::new();
        let mut root_entries = Vec::new();
        for entry in self.entries.values() {
            if let Some(first_slash) = entry.path.find(['/', '\\']) {
                let dir = &entry.path[..first_slash];
                let sub_path = &entry.path[first_slash+1..];
                let mut sub_entry = entry.clone();
                sub_entry.path = sub_path.to_string();
                tree_map.entry(dir.to_string()).or_default().push(sub_entry);
            } else {
                root_entries.push(entry.clone());
            }
        }
        let mut final_entries = Vec::new();
        for (dir, sub_entries) in tree_map {
            let sub_stage = Stage {
                entries: sub_entries.into_iter().map(|e| (e.path.clone(), e)).collect(),
                path: PathBuf::new(),
                config: self.config.clone(),
            };
            let sub_map_hash = sub_stage.write_map(storage)?;
            final_entries.push(MapEntry {
                mode: "40000".to_string(),
                name: dir,
                hash: sub_map_hash,
            });
        }
        for entry in root_entries {
            final_entries.push(MapEntry {
                mode: format!("{:o}", entry.mode),
                name: entry.path,
                hash: entry.hash,
            });
        }
        let map = Map::new(final_entries);
        map.save(storage)
    }
}
