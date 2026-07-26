use std::fs;
use std::path::{Path, PathBuf};

pub struct Autostart;

#[derive(Debug)]
pub enum AutostartError {
    IoError(String),
    NoConfigDir,
}

impl std::fmt::Display for AutostartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AutostartError::IoError(msg) => write!(f, "I/O error: {}", msg),
            AutostartError::NoConfigDir => write!(f, "could not determine config directory"),
        }
    }
}

impl std::error::Error for AutostartError {}

impl Autostart {
    fn autostart_dir() -> Option<PathBuf> {
        directories::BaseDirs::new()
            .map(|d| d.config_dir().join("autostart"))
    }

    pub fn install(bin_path: &str) -> Result<(), AutostartError> {
        let dir = Self::autostart_dir().ok_or(AutostartError::NoConfigDir)?;
        Self::install_in(bin_path, &dir)
    }

    pub fn install_in(bin_path: &str, dir: &Path) -> Result<(), AutostartError> {
        fs::create_dir_all(dir)
            .map_err(|e| AutostartError::IoError(e.to_string()))?;

        let content = format!(
            "[Desktop Entry]\n\
             Type=Application\n\
             Name=LibreSync\n\
             Comment=Google Drive sync client\n\
             Exec={}\n\
             Terminal=false\n\
             X-GNOME-Autostart-enabled=true\n",
            bin_path
        );

        let path = dir.join("libresync.desktop");
        fs::write(&path, content)
            .map_err(|e| AutostartError::IoError(e.to_string()))?;

        Ok(())
    }

    pub fn uninstall() -> Result<(), AutostartError> {
        let dir = Self::autostart_dir().ok_or(AutostartError::NoConfigDir)?;
        Self::uninstall_in(&dir)
    }

    pub fn uninstall_in(dir: &Path) -> Result<(), AutostartError> {
        let path = dir.join("libresync.desktop");
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| AutostartError::IoError(e.to_string()))?;
        }
        Ok(())
    }

    pub fn is_installed() -> bool {
        Self::autostart_dir()
            .map(|d| d.join("libresync.desktop").exists())
            .unwrap_or(false)
    }

    pub fn is_installed_in(dir: &Path) -> bool {
        dir.join("libresync.desktop").exists()
    }
}
