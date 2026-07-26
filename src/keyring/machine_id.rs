use crate::keyring::error::{KeyringError, KeyringResult};
use std::path::PathBuf;
use uuid::Uuid;

pub fn get_machine_id() -> KeyringResult<String> {
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

    let fallback_path = fallback_machine_id_path();
    if let Ok(id) = std::fs::read_to_string(&fallback_path) {
        let trimmed = id.trim().to_string();
        if !trimmed.is_empty() {
            return Ok(trimmed);
        }
    }

    let id = Uuid::new_v4().to_string();
    if let Some(parent) = fallback_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| KeyringError::IoError(e.to_string()))?;
    }
    std::fs::write(&fallback_path, &id)
        .map_err(|e| KeyringError::IoError(e.to_string()))?;
    Ok(id)
}

fn fallback_machine_id_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join(".config/libresync/machine-id")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_machine_id_fallback_creation() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("machine-id");
        let home = std::env::var("HOME").unwrap();
        let original = fallback_machine_id_path();

        let fake_home = tmp.path().to_str().unwrap().to_string();
        std::env::set_var("HOME", &fake_home);

        let result = get_machine_id();
        assert!(result.is_ok());
        let id = result.unwrap();
        assert!(!id.is_empty());
        assert!(path.exists());

        let second = get_machine_id().unwrap();
        assert_eq!(id, second);

        std::env::set_var("HOME", &home);
        let _ = fs::remove_dir_all(tmp.path());
        drop(tmp);
    }

    #[test]
    fn test_machine_id_from_etc() {
        if let Ok(id) = std::fs::read_to_string("/etc/machine-id") {
            let expected = id.trim().to_string();
            let result = get_machine_id().unwrap();
            assert_eq!(result, expected);
        }
    }
}
