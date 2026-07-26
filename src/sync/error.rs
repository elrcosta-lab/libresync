use std::fmt;

use crate::sync::state::SyncState;

#[derive(Debug, Clone, PartialEq)]
pub enum SyncError {
    InvalidTransition { from: SyncState, to: SyncState },
    JobNotFound,
    QueueEmpty,
    SyncInProgress,
    AuthError(String),
    IoError(String),
    MaxRetriesExceeded,
}

impl fmt::Display for SyncError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SyncError::InvalidTransition { from, to } => {
                write!(f, "invalid transition from {:?} to {:?}", from, to)
            }
            SyncError::JobNotFound => write!(f, "job not found"),
            SyncError::QueueEmpty => write!(f, "queue is empty"),
            SyncError::SyncInProgress => write!(f, "sync already in progress"),
            SyncError::AuthError(msg) => write!(f, "auth error: {}", msg),
            SyncError::IoError(msg) => write!(f, "I/O error: {}", msg),
            SyncError::MaxRetriesExceeded => write!(f, "max retries exceeded"),
        }
    }
}

impl std::error::Error for SyncError {}
