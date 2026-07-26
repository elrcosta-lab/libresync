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

    #[test]
    fn test_machine_id_fallback_creation() {
        let result = get_machine_id();
        assert!(result.is_ok());
        let id = result.unwrap();
        assert!(!id.is_empty());
        assert!(id.len() >= 32, "machine-id should be at least 32 chars");

        let second = get_machine_id().unwrap();
        assert_eq!(id, second);
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
