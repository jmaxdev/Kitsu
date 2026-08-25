use crate::storage::{ObjectType, Storage};
use anyhow::Result;
use sha2::{Digest, Sha256};

/// A content-addressable blob of raw file data.
///
/// Represents the lowest-level storage unit in Kitsu's object model,
/// analogous to git's blob object. Each chunk stores the raw bytes
/// of a single file version and is identified by its SHA-256 hash.
pub struct Chunk {
    /// The raw byte content of the file.
    pub content: Vec<u8>,
}

impl Chunk {
    /// Creates a new chunk wrapping the given byte content.
    pub fn new(content: Vec<u8>) -> Self {
        Self { content }
    }

    /// Persists this chunk to the object store and returns its hash.
    ///
    /// If an object with the same hash already exists, the write is
    /// skipped (content-addressable deduplication).
    ///
    /// # Errors
    /// Returns an error if the underlying storage write fails.
    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Chunk, &self.content)
    }

    /// Computes the SHA-256 content hash without persisting to storage.
    ///
    /// Uses the canonical hashing scheme: `SHA-256("chunk {len}\0{content}")`.
    /// This matches the hash that [`Storage::hash_and_write`] would produce.
    pub fn hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"chunk ");
        hasher.update(self.content.len().to_string().as_bytes());
        hasher.update([0]);
        hasher.update(&self.content);
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_hash_is_deterministic_and_64_hex_chars() {
        let chunk = Chunk::new(b"hello kitsu".to_vec());
        let hash = chunk.hash();
        assert_eq!(hash.len(), 64);
        assert!(!hash.is_empty());

        let same_chunk = Chunk::new(b"hello kitsu".to_vec());
        assert_eq!(hash, same_chunk.hash());
    }
}
