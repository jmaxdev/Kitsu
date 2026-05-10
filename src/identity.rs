use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    pub id: String,
    pub name: String,
    pub email: String,
    pub public_key: Option<Vec<u8>>,
    pub private_key: Option<Vec<u8>>,
}

impl Identity {
    pub fn generate_keys(&mut self) {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();
        self.private_key = Some(signing_key.to_bytes().to_vec());
        self.public_key = Some(verifying_key.to_bytes().to_vec());
    }

    pub fn sign(&self, data: &[u8]) -> anyhow::Result<String> {
        let priv_bytes = self
            .private_key
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No private key"))?;
        let bytes: [u8; 32] = priv_bytes.as_slice().try_into()?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let signature = signing_key.sign(data);
        Ok(hex::encode(signature.to_bytes()))
    }

    #[allow(dead_code)]
    pub fn verify(
        public_key_bytes: &[u8],
        data: &[u8],
        signature_hex: &str,
    ) -> anyhow::Result<bool> {
        let sig_bytes = hex::decode(signature_hex)?;
        let sig_arr: [u8; 64] = sig_bytes.as_slice().try_into()?;
        let signature = Signature::from_bytes(&sig_arr);
        let pub_arr: [u8; 32] = public_key_bytes.try_into()?;
        let verifying_key = VerifyingKey::from_bytes(&pub_arr)?;
        Ok(verifying_key.verify_strict(data, &signature).is_ok())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IdentityStore {
    pub identities: Vec<Identity>,
    pub active_id: String,
}

impl IdentityStore {
    pub fn load(current_dir: &Path) -> Self {
        let local_path = current_dir.join(".kitsu/identity.toml");
        if local_path.exists() {
            if let Ok(content) = fs::read_to_string(local_path) {
                if let Ok(store) = toml::from_str(&content) {
                    return store;
                }
            }
        }
        let global_path = dirs::home_dir().map(|h| h.join(".kitsu_identity.toml"));
        if let Some(gp) = global_path {
            if gp.exists() {
                if let Ok(content) = fs::read_to_string(gp) {
                    if let Ok(store) = toml::from_str(&content) {
                        return store;
                    }
                }
            }
        }
        Self::default()
    }

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
