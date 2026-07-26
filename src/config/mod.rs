use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibreSyncConfig {
    pub google: GoogleConfig,
    pub sync: SyncSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoogleConfig {
    pub client_id: String,
    pub client_secret: Option<String>,
    pub refresh_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSettings {
    pub local_dir: PathBuf,
    #[serde(default = "default_sync_interval")]
    pub poll_interval_secs: u64,
    #[serde(default = "default_true")]
    pub auto_start: bool,
}

fn default_sync_interval() -> u64 {
    30
}

fn default_true() -> bool {
    true
}

impl Default for LibreSyncConfig {
    fn default() -> Self {
        Self {
            google: GoogleConfig {
                client_id: String::new(),
                client_secret: None,
                refresh_token: None,
            },
            sync: SyncSettings {
                local_dir: PathBuf::from("LibreSync"),
                poll_interval_secs: 30,
                auto_start: true,
            },
        }
    }
}

impl LibreSyncConfig {
    pub fn config_dir() -> PathBuf {
        if let Some(dirs) = directories::ProjectDirs::from("", "", "libresync") {
            dirs.config_dir().to_path_buf()
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".config").join("libresync")
        } else {
            PathBuf::from(".config/libresync")
        }
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.toml")
    }

    pub fn load() -> Result<Self, ConfigError> {
        let path = Self::config_path();
        let content =
            std::fs::read_to_string(&path).map_err(|e| ConfigError::Io(path.clone(), e))?;
        toml::from_str(&content).map_err(|e| ConfigError::Parse(path, e))
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        let path = Self::config_path();
        let dir = path.parent().unwrap_or(&path);
        std::fs::create_dir_all(dir).map_err(|e| ConfigError::Io(path.clone(), e))?;
        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(e.to_string()))?;
        std::fs::write(&path, content).map_err(|e| ConfigError::Io(path, e))
    }
}

#[derive(Debug)]
pub enum ConfigError {
    Io(PathBuf, std::io::Error),
    Parse(PathBuf, toml::de::Error),
    Serialize(String),
    MissingField(&'static str),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(path, e) => write!(f, "{}: {}", path.display(), e),
            Self::Parse(path, e) => write!(f, "{}: {}", path.display(), e),
            Self::Serialize(msg) => write!(f, "serialize: {}", msg),
            Self::MissingField(name) => write!(f, "missing: {}", name),
        }
    }
}

impl std::error::Error for ConfigError {}
