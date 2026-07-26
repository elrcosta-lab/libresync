use crate::keyring::encrypted_fallback::EncryptedFallback;
use crate::keyring::error::KeyringResult;
use crate::keyring::keyring_storage::KeyringStorage;
use std::path::PathBuf;

enum Backend {
    Keyring,
    Fallback { tokens_dir: PathBuf },
}

pub struct TokenStorage {
    backend: Backend,
}

impl TokenStorage {
    pub async fn new() -> Self {
        let backend = match KeyringStorage::store_token("__probe__", "{}").await {
            Ok(()) => {
                let _ = KeyringStorage::delete_token("__probe__").await;
                Backend::Keyring
            }
            Err(_) => {
                let tokens_dir = EncryptedFallback::token_path("probe")
                    .parent()
                    .unwrap_or(std::path::Path::new("/tmp"))
                    .to_path_buf();
                Backend::Fallback { tokens_dir }
            }
        };
        Self { backend }
    }

    pub async fn store(&self, email: &str, token_json: &str) -> KeyringResult<()> {
        match &self.backend {
            Backend::Keyring => KeyringStorage::store_token(email, token_json).await,
            Backend::Fallback { .. } => {
                let path = EncryptedFallback::token_path(email);
                EncryptedFallback::store(&path, token_json)
            }
        }
    }

    pub async fn load(&self, email: &str) -> KeyringResult<String> {
        match &self.backend {
            Backend::Keyring => KeyringStorage::load_token(email).await,
            Backend::Fallback { .. } => {
                let path = EncryptedFallback::token_path(email);
                EncryptedFallback::load(&path)
            }
        }
    }

    pub async fn delete(&self, email: &str) -> KeyringResult<()> {
        match &self.backend {
            Backend::Keyring => KeyringStorage::delete_token(email).await,
            Backend::Fallback { .. } => {
                let path = EncryptedFallback::token_path(email);
                EncryptedFallback::delete(&path)
            }
        }
    }

    pub fn is_keyring(&self) -> bool {
        matches!(self.backend, Backend::Keyring)
    }
}
