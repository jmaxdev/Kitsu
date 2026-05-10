use crate::config::AppConfig;
use anyhow::Result;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy)]
pub enum ObjectType {
    Chunk,
    Map,
    Checkpoint,
}

impl ObjectType {
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Chunk => "chunk",
            ObjectType::Map => "map",
            ObjectType::Checkpoint => "checkpoint",
        }
    }

    pub fn from_str(s: &str) -> Result<Self> {
        match s {
            "chunk" => Ok(ObjectType::Chunk),
            "map" => Ok(ObjectType::Map),
            "checkpoint" => Ok(ObjectType::Checkpoint),
            _ => Err(anyhow::anyhow!("Unknown object type: {}", s)),
        }
    }
}

pub struct Storage {
    root_dir: PathBuf,
    config: AppConfig,
}

impl Storage {
    pub fn new(root_dir: PathBuf, config: AppConfig) -> Self {
        Self { root_dir, config }
    }

    pub fn get_object_path(&self, hash: &str) -> PathBuf {
        let (dir, file) = hash.split_at(2);
        self.root_dir
            .join(&self.config.dir_name)
            .join(&self.config.objects_dir)
            .join(dir)
            .join(file)
    }

    pub fn hash_and_write(&self, obj_type: ObjectType, data: &[u8]) -> Result<String> {
        let header = format!("{} {}\0", obj_type.as_str(), data.len());
        let mut full_data = Vec::new();
        full_data.extend_from_slice(header.as_bytes());
        full_data.extend_from_slice(data);
        let mut hasher = Sha256::new();
        hasher.update(&full_data);
        let hash = hex::encode(hasher.finalize());
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(&full_data)?;
        let compressed_data = encoder.finish()?;
        let path = self.get_object_path(&hash);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, compressed_data)?;
        }
        Ok(hash)
    }

    pub fn read_object(&self, hash: &str) -> Result<(ObjectType, Vec<u8>)> {
        let path = self.get_object_path(hash);
        let compressed_data = fs::read(path)?;
        let mut decoder = ZlibDecoder::new(&compressed_data[..]);
        let mut full_data = Vec::new();
        decoder.read_to_end(&mut full_data)?;
        let null_pos = full_data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid object format"))?;
        let header = String::from_utf8_lossy(&full_data[..null_pos]);
        let parts: Vec<&str> = header.split_whitespace().collect();
        let obj_type = ObjectType::from_str(parts[0])?;
        let content = full_data[null_pos + 1..].to_vec();
        Ok((obj_type, content))
    }

    pub fn write_raw(&self, hash: &str, full_data: &[u8]) -> Result<(ObjectType, Vec<u8>)> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(full_data)?;
        let compressed_data = encoder.finish()?;
        let path = self.get_object_path(hash);
        if !path.exists() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, compressed_data)?;
        }
        let null_pos = full_data
            .iter()
            .position(|&b| b == 0)
            .ok_or_else(|| anyhow::anyhow!("Invalid object format"))?;
        let header = String::from_utf8_lossy(&full_data[..null_pos]);
        let parts: Vec<&str> = header.split_whitespace().collect();
        let obj_type = ObjectType::from_str(parts[0])?;
        let content = full_data[null_pos + 1..].to_vec();
        Ok((obj_type, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_storage_write_read() {
        let dir = tempdir().unwrap();
        let config = AppConfig::default();
        let storage = Storage::new(dir.path().to_path_buf(), config);
        let data = b"kitsu storage test content";
        let hash = storage.hash_and_write(ObjectType::Chunk, data).unwrap();
        let (obj_type, read_data) = storage.read_object(&hash).unwrap();
        assert!(matches!(obj_type, ObjectType::Chunk));
        assert_eq!(read_data, data);
    }
}
