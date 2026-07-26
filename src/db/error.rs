use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("account not found: {0}")]
    AccountNotFound(String),
    #[error("job not found: {0}")]
    JobNotFound(String),
    #[error("sync state not found: {0}")]
    SyncStateNotFound(String),
    #[error("invalid account status: {0}")]
    InvalidAccountStatus(String),
    #[error("invalid job type: {0}")]
    InvalidJobType(String),
    #[error("invalid job state: {0}")]
    InvalidJobState(String),
    #[error("database path not found: {0}")]
    PathNotFound(PathBuf),
}
