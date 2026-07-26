use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum WatcherError {
    PathNotFound(String),
    WatchAlreadyActive,
    WatchNotActive,
    ChannelClosed,
    IoError(String),
}

impl fmt::Display for WatcherError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WatcherError::PathNotFound(p) => write!(f, "path not found: {}", p),
            WatcherError::WatchAlreadyActive => write!(f, "watch already active"),
            WatcherError::WatchNotActive => write!(f, "watch is not active"),
            WatcherError::ChannelClosed => write!(f, "event channel closed"),
            WatcherError::IoError(msg) => write!(f, "I/O error: {}", msg),
        }
    }
}

impl std::error::Error for WatcherError {}
