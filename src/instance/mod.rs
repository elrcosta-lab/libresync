mod error;
pub use error::InstanceError;

use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use fs2::FileExt;

#[derive(Debug)]
pub struct InstanceLock {
    #[allow(dead_code)]
    file: File,
    path: PathBuf,
}

impl InstanceLock {
    pub fn acquire() -> Result<Self, InstanceError> {
        Self::acquire_at("/tmp/libresync.pid")
    }

    pub fn acquire_at(path: impl AsRef<Path>) -> Result<Self, InstanceError> {
        let path = path.as_ref().to_path_buf();
        let file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|e| InstanceError::IoError(e.to_string()))?;

        file.try_lock_exclusive().map_err(|_| {
            let pid = fs::read_to_string(&path).ok();
            let pid: u32 = pid
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0);
            InstanceError::AlreadyRunning(pid)
        })?;

        file.set_len(0).ok();
        writeln!(&file, "{}", std::process::id())
            .map_err(|e| InstanceError::IoError(e.to_string()))?;
        file.sync_all().ok();

        Ok(Self { file, path })
    }

    pub fn release(self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
