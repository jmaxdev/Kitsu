use crate::identity::crypto;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// A single identity (persona) with optional Ed25519 keypair.
///
/// Represents an author/committer identity akin to git's `user.name`
/// and `user.email`, extended with Ed25519 keys for checkpoint signing.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    /// Short identifier for this persona (e.g., `"work"`, `"personal"`).
    pub id: String,
    /// Display name for commit metadata.
    pub name: String,
    /// Email address for commit metadata.
    pub email: String,
    /// Ed25519 public key bytes, if generated.
    pub public_key: Option<Vec<u8>>,
    /// Ed25519 private key bytes, if generated.
    pub private_key: Option<Vec<u8>>,
}

impl Identity {
    /// Generates a fresh Ed25519 keypair and stores it in this identity.
    pub fn generate_keys(&mut self) {
        let (private_key, public_key) = crypto::generate_keypair();
        self.private_key = Some(private_key);
        self.public_key = Some(public_key);
    }

    /// Signs the given data using this identity's private key.
    ///
    /// # Errors
    /// Returns an error if no private key is available or if signing fails.
    pub fn sign(&self, data: &[u8]) -> anyhow::Result<String> {
        let priv_bytes = self
            .private_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No private key"))?;
        crypto::sign_data(priv_bytes, data)
    }
}

/// Persistent store for multiple identity personas.
///
/// Supports local (per-repository) and global (home directory) storage.
/// One identity is always designated as active for checkpoint authoring.
#[derive(Serialize, Deserialize, Debug)]
pub struct IdentityStore {
    /// All registered personas.
    pub identities: Vec<Identity>,
    /// ID of the currently active persona.
    pub active_id: String,
}

impl IdentityStore {
    /// Loads the identity store, checking local then global paths.
    ///
    /// Resolution order:
    /// 1. `{repo_root}/.kitsu/identity.toml` (local)
    /// 2. `~/.kitsu_identity.toml` (global)
    /// 3. Default store with a generated `"default"` persona.
    pub fn load(current_dir: &Path) -> Self {
        let local_path = current_dir.join(".kitsu/identity.toml");
        if local_path.exists()
            && let Ok(content) = fs::read_to_string(local_path)
            && let Ok(store) = toml::from_str(&content)
        {
            return store;
        }
        let global_path = dirs::home_dir().map(|h| h.join(".kitsu_identity.toml"));
        if let Some(gp) = global_path
            && gp.exists()
            && let Ok(content) = fs::read_to_string(gp)
            && let Ok(store) = toml::from_str(&content)
        {
            return store;
        }
        Self::default()
    }

    /// Persists the identity store to either local or global path.
    ///
    /// # Errors
    /// Returns an error if serialization or file writing fails.
    pub fn save(&self, current_dir: &Path, global: bool) -> anyhow::Result<()> {
        let content = toml::to_string(self)?;
        if global {
            let path = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("No home dir"))?
                .join(".kitsu_identity.toml");
            fs::write(path, content)?;
        } else {
            let path = current_dir.join(".kitsu/identity.toml");
            fs::create_dir_all(path.parent().unwrap())?;
            fs::write(path, content)?;
        }
        Ok(())
    }

    /// Returns the currently active identity.
    ///
    /// Falls back to the first identity if the active ID doesn't match.
    pub fn get_active(&self) -> &Identity {
        self.identities
            .iter()
            .find(|i| i.id == self.active_id)
            .unwrap_or(&self.identities[0])
    }
}

impl Default for IdentityStore {
    fn default() -> Self {
        let mut default_id = Identity {
            id: "default".into(),
            name: "Kitsu User".into(),
            email: "kitsu@example.com".into(),
            public_key: None,
            private_key: None,
        };
        default_id.generate_keys();
        Self {
            identities: vec![default_id],
            active_id: "default".into(),
        }
    }
}
