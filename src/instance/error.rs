use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum InstanceError {
    AlreadyRunning(u32),
    IoError(String),
}

impl fmt::Display for InstanceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InstanceError::AlreadyRunning(pid) => {
                write!(f, "already running (PID {})", pid)
            }
            InstanceError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for InstanceError {}
