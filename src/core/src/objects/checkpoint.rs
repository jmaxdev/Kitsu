use crate::storage::{ObjectType, Storage};
use anyhow::Result;

/// A versioned snapshot of the entire repository state.
///
/// Analogous to git's commit object. Each checkpoint captures a complete
/// directory map hash, an optional parent for history chaining, author
/// metadata, a human-readable message, and an optional Ed25519 signature.
pub struct Checkpoint {
    /// SHA-256 hash of the root [`super::Map`] for this snapshot.
    pub map_hash: String,
    /// Parent checkpoint hash for history chaining. `None` for the initial checkpoint.
    pub parent_hash: Option<String>,
    /// Author identity in `"Name <email>"` format.
    pub author: String,
    /// Human-readable description of the changes in this snapshot.
    pub message: String,
    /// Unix timestamp (seconds since epoch) of checkpoint creation.
    pub timestamp: i64,
    /// Hex-encoded Ed25519 signature over the serialized content, if signed.
    pub signature: Option<String>,
}

impl Checkpoint {
    /// Serializes the checkpoint into its text wire format.
    ///
    /// The format is line-oriented and human-readable:
    /// ```text
    /// map {map_hash}
    /// parent {parent_hash}          // omitted if None
    /// author {author} {timestamp}
    /// curator {author} {timestamp}
    /// signature {hex_signature}     // omitted if None
    ///
    /// {message}
    /// ```
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

    /// Persists this checkpoint to the object store and returns its hash.
    ///
    /// # Errors
    /// Returns an error if the underlying storage write fails.
    pub fn save(&self, storage: &Storage) -> Result<String> {
        storage.hash_and_write(ObjectType::Checkpoint, &self.serialize())
    }

    /// Deserializes a checkpoint from its text wire format.
    ///
    /// Tolerates unknown header fields (silently ignored) for forward
    /// compatibility. The `curator` field is currently parsed but not
    /// stored separately from `author`.
    ///
    /// # Errors
    /// Returns an error if the data is not valid UTF-8 or if the
    /// timestamp cannot be parsed as an integer.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checkpoint_roundtrip_with_parent_and_signature() {
        let cp = Checkpoint {
            map_hash: "a".repeat(64),
            parent_hash: Some("b".repeat(64)),
            author: "Senior Dev <senior@kitsu.dev>".into(),
            message: "feat: initial commit\n\nFull details here.".into(),
            timestamp: 1700000000,
            signature: Some("c".repeat(128)),
        };
        let bytes = cp.serialize();
        let decoded = Checkpoint::deserialize(&bytes).unwrap();
        assert_eq!(decoded.map_hash, cp.map_hash);
        assert_eq!(decoded.parent_hash, cp.parent_hash);
        assert_eq!(decoded.author, cp.author);
        assert_eq!(decoded.message, cp.message);
        assert_eq!(decoded.timestamp, cp.timestamp);
        assert_eq!(decoded.signature, cp.signature);
    }

    #[test]
    fn checkpoint_roundtrip_initial_commit_no_parent_no_sig() {
        let cp = Checkpoint {
            map_hash: "1".repeat(64),
            parent_hash: None,
            author: "Senior Dev <senior@kitsu.dev>".into(),
            message: "Initial commit".into(),
            timestamp: 1700000000,
            signature: None,
        };
        let bytes = cp.serialize();
        let decoded = Checkpoint::deserialize(&bytes).unwrap();
        assert_eq!(decoded.map_hash, cp.map_hash);
        assert!(decoded.parent_hash.is_none());
        assert!(decoded.signature.is_none());
        assert_eq!(decoded.message, "Initial commit");
    }
}
