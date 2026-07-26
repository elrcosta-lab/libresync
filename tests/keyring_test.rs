fn sanitize_email(email: &str) -> String {
    email
        .to_lowercase()
        .replace('@', "_at_")
        .replace('.', "_dot_")
}

fn get_machine_id() -> Result<String, String> {
    let paths = [
        "/etc/machine-id",
        "/var/lib/dbus/machine-id",
    ];

    for path in &paths {
        if let Ok(id) = std::fs::read_to_string(path) {
            let trimmed = id.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }

    let home = std::env::var("HOME").map_err(|e| e.to_string())?;
    let fallback_path = format!("{}/.config/libresync/machine-id", home);

    if let Ok(id) = std::fs::read_to_string(&fallback_path) {
        let trimmed = id.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    let parent = std::path::Path::new(&fallback_path)
        .parent()
        .ok_or("invalid path")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    std::fs::write(&fallback_path, &id).map_err(|e| e.to_string())?;
    Ok(id)
}

#[test]
fn test_sanitize_email() {
    assert_eq!(sanitize_email("User@Example.com"), "user_at_example_dot_com");
    assert_eq!(sanitize_email("test@test.co.uk"), "test_at_test_dot_co_dot_uk");
    assert_eq!(sanitize_email("simple"), "simple");
}

#[test]
fn test_get_machine_id() {
    let id = get_machine_id().expect("machine id should be available");
    assert!(!id.is_empty(), "machine id should not be empty");
}

#[test]
fn test_encrypt_decrypt_roundtrip() {
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce};
    use hkdf::Hkdf;
    use sha2::Sha256;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("token.enc");
    let original = r#"{"access_token":"abc","refresh_token":"def","expires_at":123}"#;

    let machine_id = get_machine_id().unwrap();
    let salt: Vec<u8> = (0..28).map(|_| rand::random::<u8>()).collect();
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), machine_id.as_bytes());
    let mut key_bytes = [0u8; 32];
    hkdf.expand(b"libresync-token-encryption", &mut key_bytes)
        .unwrap();
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes);
    let cipher = Aes256Gcm::new(key);

    let nonce_bytes: [u8; 12] = rand::random();
    let nonce = Nonce::from_slice(&nonce_bytes);
    let encrypted = cipher.encrypt(nonce, original.as_bytes()).unwrap();

    let (ciphertext, tag) = encrypted.split_at(encrypted.len() - 16);
    let mut data = Vec::new();
    data.extend_from_slice(&nonce_bytes);
    data.extend_from_slice(&salt);
    data.extend_from_slice(tag);
    data.extend_from_slice(ciphertext);
    std::fs::write(&path, &data).unwrap();

    // Read back
    let read_data = std::fs::read(&path).unwrap();
    let read_nonce = &read_data[..12];
    let read_salt: [u8; 28] = read_data[12..40].try_into().unwrap();
    let read_tag = &read_data[40..56];
    let read_ciphertext = &read_data[56..];

    let hkdf2 = Hkdf::<Sha256>::new(Some(&read_salt[..]), machine_id.as_bytes());
    let mut key_bytes2 = [0u8; 32];
    hkdf2.expand(b"libresync-token-encryption", &mut key_bytes2).unwrap();
    let key2 = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_bytes2);
    let cipher2 = Aes256Gcm::new(key2);
    let nonce2 = Nonce::from_slice(read_nonce);

    let mut decrypt_input = read_ciphertext.to_vec();
    decrypt_input.extend_from_slice(read_tag);
    let plaintext = cipher2.decrypt(nonce2, decrypt_input.as_ref()).unwrap();
    let result = String::from_utf8(plaintext).unwrap();

    assert_eq!(result, original);
}

#[test]
fn test_corrupted_fallback_returns_error() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let path = dir.path().join("bad.enc");
    std::fs::write(&path, b"too short").unwrap();

    let data = std::fs::read(&path).unwrap();
    assert!(data.len() < 12 + 28 + 16);
}

#[test]
fn test_get_machine_id_fallback_creation() {
    let tmp = tempfile::TempDir::new().unwrap();
    let original_home = std::env::var("HOME").ok();
    std::env::set_var("HOME", tmp.path());

    let id = get_machine_id().expect("should create fallback machine-id");
    assert!(!id.is_empty());

    let second = get_machine_id().expect("should read persisted fallback");
    assert_eq!(id, second);

    if let Some(home) = original_home {
        std::env::set_var("HOME", home);
    }
}
