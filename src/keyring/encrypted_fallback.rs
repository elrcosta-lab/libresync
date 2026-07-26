use crate::keyring::error::{KeyringError, KeyringResult};
use crate::keyring::machine_id::get_machine_id;
use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{Aes256Gcm, Nonce};
use hkdf::Hkdf;
use sha2::Sha256;
use std::path::{Path, PathBuf};
use std::fs;

const SALT_LEN: usize = 28;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;

pub struct EncryptedFallback;

impl EncryptedFallback {
    pub fn store(path: &Path, token_json: &str) -> KeyringResult<()> {
        let salt = OsRng.gen_salt(SALT_LEN);
        let key = derive_key(&salt)?;
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes: [u8; NONCE_LEN] = OsRng.gen_nonce();
        let nonce = Nonce::from_slice(&nonce_bytes);

        let encrypted = cipher
            .encrypt(nonce, token_json.as_bytes())
            .map_err(|_| KeyringError::EncryptionError)?;

        let (ciphertext, tag) = encrypted.split_at(encrypted.len() - TAG_LEN);

        let mut data = Vec::with_capacity(NONCE_LEN + SALT_LEN + TAG_LEN + ciphertext.len());
        data.extend_from_slice(&nonce_bytes);
        data.extend_from_slice(&salt);
        data.extend_from_slice(tag);
        data.extend_from_slice(ciphertext);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, &data)?;
        Ok(())
    }

    pub fn load(path: &Path) -> KeyringResult<String> {
        let data = fs::read(path)?;

        if data.len() < NONCE_LEN + SALT_LEN + TAG_LEN {
            return Err(KeyringError::InvalidFormat);
        }

        let nonce_bytes: [u8; NONCE_LEN] = data[..NONCE_LEN].try_into()
            .map_err(|_| KeyringError::InvalidFormat)?;
        let salt: [u8; SALT_LEN] = data[NONCE_LEN..NONCE_LEN + SALT_LEN].try_into()
            .map_err(|_| KeyringError::InvalidFormat)?;
        let tag = &data[NONCE_LEN + SALT_LEN..NONCE_LEN + SALT_LEN + TAG_LEN];
        let ciphertext = &data[NONCE_LEN + SALT_LEN + TAG_LEN..];

        let key = derive_key(&salt)?;
        let cipher = Aes256Gcm::new(&key);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut input = ciphertext.to_vec();
        input.extend_from_slice(tag);

        let plaintext = cipher
            .decrypt(nonce, input.as_ref())
            .map_err(|_| KeyringError::DecryptionError)?;

        String::from_utf8(plaintext).map_err(|_| KeyringError::InvalidFormat)
    }

    pub fn delete(path: &Path) -> KeyringResult<()> {
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    pub fn token_path(email: &str) -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
        let sanitized = sanitize_email(email);
        PathBuf::from(home).join(format!(".config/libresync/tokens/{}.enc", sanitized))
    }
}

fn derive_key(salt: &[u8]) -> KeyringResult<aes_gcm::Key<Aes256Gcm>> {
    let machine_id = get_machine_id()?;
    let hkdf = Hkdf::<Sha256>::new(Some(salt), machine_id.as_bytes());
    let mut key_bytes = [0u8; 32];
    hkdf.expand(b"libresync-token-encryption", &mut key_bytes)
        .map_err(|_| KeyringError::EncryptionError)?;
    Ok(*aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes))
}

pub fn sanitize_email(email: &str) -> String {
    email
        .to_lowercase()
        .replace('@', "_at_")
        .replace('.', "_dot_")
}

trait OsRngExt {
    fn gen_salt(&self, len: usize) -> Vec<u8>;
    fn gen_nonce(&self) -> [u8; NONCE_LEN];
}

impl OsRngExt for OsRng {
    fn gen_salt(&self, len: usize) -> Vec<u8> {
        use rand::RngCore;
        let mut buf = vec![0u8; len];
        OsRng.fill_bytes(&mut buf);
        buf
    }

    fn gen_nonce(&self) -> [u8; NONCE_LEN] {
        use rand::RngCore;
        let mut buf = [0u8; NONCE_LEN];
        OsRng.fill_bytes(&mut buf);
        buf
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_sanitize_email() {
        assert_eq!(sanitize_email("User@Example.com"), "user_at_example_dot_com");
        assert_eq!(sanitize_email("test@test.co.uk"), "test_at_test_dot_co_dot_uk");
        assert_eq!(sanitize_email("simple"), "simple");
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.enc");

        let original = r#"{"access_token":"abc","refresh_token":"def","expires_at":123}"#;
        EncryptedFallback::store(&path, original).unwrap();
        let loaded = EncryptedFallback::load(&path).unwrap();
        assert_eq!(loaded, original);
    }

    #[test]
    fn test_corrupted_file_returns_error() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("corrupted.enc");

        fs::write(&path, b"not enough bytes").unwrap();
        let result = EncryptedFallback::load(&path);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), KeyringError::InvalidFormat);
    }

    #[test]
    fn test_delete_nonexistent() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nonexistent.enc");
        assert!(EncryptedFallback::delete(&path).is_ok());
    }

    #[test]
    fn test_delete_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("to_delete.enc");
        EncryptedFallback::store(&path, "data").unwrap();
        assert!(path.exists());
        EncryptedFallback::delete(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_token_path_format() {
        let path = EncryptedFallback::token_path("User@Example.com");
        let path_str = path.to_string_lossy();
        assert!(path_str.contains("user_at_example_dot_com.enc"));
    }
}
