use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use anyhow::Result;
use ed25519_dalek::{SigningKey, Signer};
use rand::rngs::OsRng;
use rand::RngCore;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub email: String,
    pub public_key: Option<String>, // Hex encoded
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>, // Hex encoded
}

impl Identity {
    pub fn generate_keys(&mut self) {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();
        
        self.private_key = Some(hex::encode(signing_key.to_bytes()));
        self.public_key = Some(hex::encode(verifying_key.to_bytes()));
    }

    pub fn sign(&self, data: &[u8]) -> Result<String> {
        let priv_hex = self.private_key.as_ref().ok_or_else(|| anyhow::anyhow!("No private key available"))?;
        let priv_bytes = hex::decode(priv_hex)?;
        let secret_bytes: [u8; 32] = priv_bytes.as_slice().try_into()?;
        let signing_key = SigningKey::from_bytes(&secret_bytes);
        let signature = signing_key.sign(data);
        Ok(hex::encode(signature.to_bytes()))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct IdentityStore {
    pub active_id: String,
    pub identities: Vec<Identity>,
}

impl Default for IdentityStore {
    fn default() -> Self {
        Self {
            active_id: "default".to_string(),
            identities: vec![Identity {
                id: "default".to_string(),
                name: "Anonymous".to_string(),
                email: "anon@example.com".to_string(),
                public_key: None,
                private_key: None,
            }],
        }
    }
}

impl IdentityStore {
    pub fn load(root_dir: &Path) -> Self {
        let vcs_dir = crate::config::DIR_NAME;
        let local_path = root_dir.join(vcs_dir).join("identities");
        
        if local_path.exists() {
            let content = fs::read_to_string(local_path).unwrap_or_default();
            return toml::from_str(&content).unwrap_or_else(|_| Self::default());
        }

        let global_path = dirs::home_dir()
            .map(|p| p.join(".vcontrol_identities"))
            .unwrap_or_default();
            
        if global_path.exists() {
            let content = fs::read_to_string(global_path).unwrap_or_default();
            return toml::from_str(&content).unwrap_or_else(|_| Self::default());
        }

        Self::default()
    }

    pub fn save(&self, root_dir: &Path, global: bool) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        if global {
            let path = dirs::home_dir()
                .ok_or_else(|| anyhow::anyhow!("Home directory not found"))?
                .join(".vcontrol_identities");
            fs::write(path, content)?;
        } else {
            let vcs_dir = crate::config::DIR_NAME;
            let path = root_dir.join(vcs_dir).join("identities");
            fs::write(path, content)?;
        }
        Ok(())
    }

    pub fn get_active(&self) -> &Identity {
        self.identities
            .iter()
            .find(|i| i.id == self.active_id)
            .unwrap_or(&self.identities[0])
    }

    pub fn get_active_mut(&mut self) -> &mut Identity {
        let active_id = self.active_id.clone();
        let pos = self.identities.iter().position(|i| i.id == active_id).unwrap_or(0);
        &mut self.identities[pos]
    }
}
