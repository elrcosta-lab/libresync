use std::fmt;

#[derive(Debug)]
pub enum DriveError {
    Network(String),
    Auth(String),
    NotFound(String),
    RateLimited { retry_after: Option<u64> },
    Api { status: u16, body: String },
    Serialization(String),
    Config(String),
}

impl fmt::Display for DriveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(msg) => write!(f, "network: {}", msg),
            Self::Auth(msg) => write!(f, "auth: {}", msg),
            Self::NotFound(id) => write!(f, "not found: {}", id),
            Self::RateLimited { retry_after: Some(s) } => {
                write!(f, "rate limited, retry after {}s", s)
            }
            Self::RateLimited { retry_after: None } => write!(f, "rate limited"),
            Self::Api { status, body } => write!(f, "API {}: {}", status, body),
            Self::Serialization(msg) => write!(f, "serialization: {}", msg),
            Self::Config(msg) => write!(f, "config: {}", msg),
        }
    }
}

impl std::error::Error for DriveError {}

pub type DriveResult<T> = Result<T, DriveError>;
