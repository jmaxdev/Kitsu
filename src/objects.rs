use anyhow::Result;
use crate::storage::{Storage, ObjectType};

pub struct Chunk {
    pub content: Vec<u8>,
}

impl Chunk {
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Chunk, &self.content)
    }

    pub fn hash(&self) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"chunk ");
        hasher.update(self.content.len().to_string().as_bytes());
        hasher.update(&[0]);
        hasher.update(&self.content);
        hex::encode(hasher.finalize())
    }
}

#[derive(Clone)]
pub struct MapEntry {
    pub mode: String,
    pub name: String,
    pub hash: String,
}

pub struct Map {
    pub entries: Vec<MapEntry>,
}

impl Map {
    pub fn new(entries: Vec<MapEntry>) -> Self {
        Self { entries }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut data = Vec::new();
        let mut sorted_entries = self.entries.clone();
        sorted_entries.sort_by(|a, b| a.name.cmp(&b.name));
        for entry in sorted_entries {
            let line = format!("{} {}\0", entry.mode, entry.name);
            data.extend_from_slice(line.as_bytes());
            data.extend_from_slice(&hex::decode(&entry.hash).expect("Invalid hash in map"));
        }
        data
    }

    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Map, &self.serialize())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let mut entries = Vec::new();
        let mut pos = 0;
        while pos < data.len() {
            let null_pos = data[pos..].iter().position(|&b| b == 0).ok_or_else(|| anyhow::anyhow!("Invalid map format"))? + pos;
            let header = String::from_utf8(data[pos..null_pos].to_vec())?;
            let parts: Vec<&str> = header.split_whitespace().collect();
            let mode = parts[0].to_string();
            let name = parts[1].to_string();
            pos = null_pos + 1;
            let hash_bytes = &data[pos..pos+32];
            let hash = hex::encode(hash_bytes);
            pos += 32;
            entries.push(MapEntry { mode, name, hash });
        }
        Ok(Map { entries })
    }
}

pub struct Checkpoint {
    pub map_hash: String,
    pub parent_hash: Option<String>,
    pub author: String,
    pub message: String,
    pub timestamp: i64,
    pub signature: Option<String>,
}

impl Checkpoint {
    pub fn serialize(&self) -> Vec<u8> {
        let mut content = format!("map {}\n", self.map_hash);
        if let Some(parent) = &self.parent_hash {
            content.push_str(&format!("parent {}\n", parent));
        }
        content.push_str(&format!("author {} {}\n", self.author, self.timestamp));
        content.push_str(&format!("curator {} {}\n", self.author, self.timestamp));
        if let Some(sig) = &self.signature {
            content.push_str(&format!("signature {}\n", sig));
        }
        content.push_str(&format!("\n{}\n", self.message));
        content.into_bytes()
    }

    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Checkpoint, &self.serialize())
    }

    pub fn deserialize(data: &[u8]) -> Result<Self> {
        let content = String::from_utf8(data.to_vec())?;
        let mut map_hash = String::new();
        let mut parent_hash = None;
        let mut author = String::new();
        let mut timestamp = 0;
        let mut message = String::new();
        let mut signature = None;
        let mut lines = content.lines();
        while let Some(line) = lines.next() {
            if line.is_empty() {
                message = lines.collect::<Vec<&str>>().join("\n");
                break;
            }
            let parts: Vec<&str> = line.splitn(2, ' ').collect();
            match parts[0] {
                "map" => map_hash = parts[1].to_string(),
                "parent" => parent_hash = Some(parts[1].to_string()),
                "signature" => signature = Some(parts[1].to_string()),
                "author" => {
                    let author_parts: Vec<&str> = parts[1].rsplitn(2, ' ').collect();
                    timestamp = author_parts[0].parse()?;
                    author = author_parts[1].to_string();
                }
                _ => {}
            }
        }
        Ok(Checkpoint {
            map_hash,
            parent_hash,
            author,
            message,
            timestamp,
            signature,
        })
    }
}
