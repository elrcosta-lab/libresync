use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TransferError {
    JobNotFound,
    MaxRetriesExceeded,
    Sha256Mismatch { expected: String, actual: String },
    IntegrityCheckFailed,
    Cancelled,
    Paused,
    IoError(String),
    InvalidPath,
}

impl fmt::Display for TransferError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TransferError::JobNotFound => write!(f, "job not found"),
            TransferError::MaxRetriesExceeded => write!(f, "max retries exceeded"),
            TransferError::Sha256Mismatch { expected, actual } => {
                write!(f, "SHA256 mismatch: expected {}, got {}", expected, actual)
            }
            TransferError::IntegrityCheckFailed => write!(f, "integrity check failed"),
            TransferError::Cancelled => write!(f, "transfer cancelled"),
            TransferError::Paused => write!(f, "transfer paused"),
            TransferError::IoError(msg) => write!(f, "I/O error: {}", msg),
            TransferError::InvalidPath => write!(f, "invalid path"),
        }
    }
}

impl std::error::Error for TransferError {}
