use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum KeyringError {
    ServiceUnavailable,
    EncryptionError,
    DecryptionError,
    IoError(String),
    TokenNotFound,
    InvalidFormat,
}

impl fmt::Display for KeyringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KeyringError::ServiceUnavailable => write!(f, "keyring service unavailable"),
            KeyringError::EncryptionError => write!(f, "encryption error"),
            KeyringError::DecryptionError => write!(f, "decryption error"),
            KeyringError::IoError(msg) => write!(f, "I/O error: {}", msg),
            KeyringError::TokenNotFound => write!(f, "token not found"),
            KeyringError::InvalidFormat => write!(f, "invalid token format"),
        }
    }
}

impl std::error::Error for KeyringError {}

impl From<std::io::Error> for KeyringError {
    fn from(e: std::io::Error) -> Self {
        KeyringError::IoError(e.to_string())
    }
}

pub type KeyringResult<T> = Result<T, KeyringError>;
