use crate::config::AppConfig;
use anyhow::Result;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::str::FromStr;

/// Discriminant for the three Kitsu object types.
///
/// Encoded as the first token in every stored object's header,
/// enabling type-safe deserialization after reading from disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    /// Raw file content (analogous to git blob).
    Chunk,
    /// Directory listing (analogous to git tree).
    Map,
    /// Versioned snapshot (analogous to git commit).
    Checkpoint,
}

impl ObjectType {
    /// Returns the canonical string representation used in object headers.
    pub fn as_str(&self) -> &str {
        match self {
            ObjectType::Chunk => "chunk",
            ObjectType::Map => "map",
            ObjectType::Checkpoint => "checkpoint",
        }
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ObjectType {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s {
            "chunk" => Ok(ObjectType::Chunk),
            "map" => Ok(ObjectType::Map),
            "checkpoint" => Ok(ObjectType::Checkpoint),
            _ => Err(anyhow::anyhow!("Unknown object type: {}", s)),
        }
    }
}

/// Content-addressable object store backed by the filesystem.
///
/// Objects are stored as zlib-compressed files under a two-level directory
/// hierarchy: the first two hex characters of the hash form the directory
/// name, and the remaining characters form the filename. This mirrors
/// git's loose object layout.
///
/// Each stored object has the wire format: `"{type} {size}\0{content}"`,
/// which is hashed with SHA-256 to produce the content address.
pub struct Storage {
    root_dir: PathBuf,
    config: AppConfig,
}

impl Storage {
    /// Creates a new storage instance rooted at the given directory.
    pub fn new(root_dir: PathBuf, config: AppConfig) -> Self {
        Self { root_dir, config }
    }

    /// Returns the root directory of the repository.
    pub fn root_dir(&self) -> &PathBuf {
        &self.root_dir
    }

    /// Returns the configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Computes the filesystem path for a given object hash.
    ///
    /// Uses a two-level fan-out: `{repo_dir}/objects/{hash[0..2]}/{hash[2..]}`.
    pub fn get_object_path(&self, hash: &str) -> PathBuf {
        let (dir, file) = hash.split_at(2);
        self.root_dir
            .join(&self.config.dir_name)
            .join(&self.config.objects_dir)
            .join(dir)
            .join(file)
    }

    /// Hashes content with a type header and writes to the object store.
    ///
    /// The full data hashed is `"{type} {size}\0{content}"`. If an object
    /// with the resulting hash already exists, the write is skipped
    /// (content-addressable deduplication).
    ///
    /// # Errors
    /// Returns an error if directory creation or file writing fails.
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

    /// Reads and decompresses an object from the store.
    ///
    /// Returns the object type and raw content (without the header).
    ///
    /// # Errors
    /// Returns an error if the object file doesn't exist, decompression
    /// fails, or the header format is invalid.
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
        let obj_type: ObjectType = parts[0].parse()?;
        let content = full_data[null_pos + 1..].to_vec();
        Ok((obj_type, content))
    }

    /// Writes pre-formatted raw data to the store under the given hash.
    ///
    /// Unlike [`hash_and_write`](Self::hash_and_write), this method does not compute
    /// the hash — the caller provides it. Used for importing objects from
    /// remotes where the hash is already known.
    ///
    /// # Errors
    /// Returns an error if writing fails or the raw data header is invalid.
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
        let obj_type: ObjectType = parts[0].parse()?;
        let content = full_data[null_pos + 1..].to_vec();
        Ok((obj_type, content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn storage_roundtrip_write_then_read() {
        let dir = tempdir().unwrap();
        let config = AppConfig::default();
        let storage = Storage::new(dir.path().to_path_buf(), config);
        let data = b"kitsu storage test content";
        let hash = storage.hash_and_write(ObjectType::Chunk, data).unwrap();
        let (obj_type, read_data) = storage.read_object(&hash).unwrap();
        assert!(matches!(obj_type, ObjectType::Chunk));
        assert_eq!(read_data, data);
    }

    #[test]
    fn object_type_roundtrip_from_str() {
        assert_eq!("chunk".parse::<ObjectType>().unwrap(), ObjectType::Chunk);
        assert_eq!("map".parse::<ObjectType>().unwrap(), ObjectType::Map);
        assert_eq!(
            "checkpoint".parse::<ObjectType>().unwrap(),
            ObjectType::Checkpoint
        );
        assert!("bogus".parse::<ObjectType>().is_err());
    }
}
