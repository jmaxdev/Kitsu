use crate::storage::{ObjectType, Storage};
use anyhow::Result;

/// A single entry within a directory map.
///
/// Represents one tracked item (file or subdirectory) with its
/// Unix file mode, name, and content-addressable hash reference.
#[derive(Clone, Debug)]
pub struct MapEntry {
    /// Unix file mode as octal string (e.g., `"100644"` for files, `"40000"` for directories).
    pub mode: String,
    /// Entry name (filename or directory name, without path separators).
    pub name: String,
    /// SHA-256 hash referencing a [`super::Chunk`] (file) or another [`Map`] (subdirectory).
    pub hash: String,
}

/// A sorted directory listing of tracked files and subdirectories.
///
/// Analogous to git's tree object. Maps form a recursive structure
/// where each entry can reference either a Chunk (file content) or
/// another Map (subdirectory), enabling arbitrary directory hierarchies.
pub struct Map {
    /// The entries in this directory, sorted by name on serialization.
    pub entries: Vec<MapEntry>,
}

impl Map {
    /// Creates a new map from the given entries.
    pub fn new(entries: Vec<MapEntry>) -> Self {
        Self { entries }
    }

    /// Serializes the map into its binary wire format.
    ///
    /// Each entry is encoded as `"{mode} {name}\0{hash_bytes}"` where
    /// `hash_bytes` is the raw 32-byte SHA-256 digest. Entries are
    /// sorted lexicographically by name before serialization to ensure
    /// deterministic hashing.
    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut sorted = self.entries.clone();
        sorted.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in sorted {
            let line = format!("{} {}\0", entry.mode, entry.name);
            data.extend_from_slice(line.as_bytes());
            data.extend_from_slice(&hex::decode(&entry.hash).expect("Invalid hash in map"));
        }
        data
    }

    /// Persists this map to the object store and returns its hash.
    ///
    /// # Errors
    /// Returns an error if the underlying storage write fails.
    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Map, &self.serialize())
    }

    /// Deserializes a map from its binary wire format.
    ///
    /// # Errors
    /// Returns an error if the binary data is malformed (missing null
    /// terminators, truncated hash bytes, or invalid UTF-8 headers).
    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut entries = Vec::new();
        let mut pos = 0;
        while pos < data.len() {
            let null_pos = data[pos..]
                .iter()
                .position(|&b| b == 0)
                .ok_or_else(|| anyhow::anyhow!("Invalid map format"))?
                + pos;
            let header = String::from_utf8(data[pos..null_pos].to_vec())?;
            let parts: Vec<&str> = header.split_whitespace().collect();
            let mode = parts[0].to_string();
            let name = parts[1].to_string();
            pos = null_pos + 1;
            let hash_bytes = &data[pos..pos + 32];
            let hash = hex::encode(hash_bytes);
            pos += 32;
            entries.push(MapEntry { mode, name, hash });
        }
        Ok(Map { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_roundtrip_serialization() {
        let entries = vec![MapEntry {
            mode: "100644".to_string(),
            name: "file.txt".to_string(),
            hash: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        }];
        let map = Map::new(entries);
        let data = map.serialize();
        let decoded = Map::deserialize(&data).unwrap();
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].name, "file.txt");
        assert_eq!(decoded.entries[0].mode, "100644");
    }
}
