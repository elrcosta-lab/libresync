use std::fmt;

#[derive(Debug, Clone)]
pub enum AuthError {
    NetworkError(String),
    TokenExpired,
    TokenRevoked,
    CsrfMismatch,
    PortUnavailable,
    LoginTimeout,
    LoginDenied,
    KeyringUnavailable,
    FallbackCorrupted,
    AccountNotFound,
    AccountDuplicated,
    InsufficientScope,
    RateLimited { retry_after: Option<u64> },
    StateError(String),
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthError::NetworkError(msg) => write!(f, "network error: {}", msg),
            AuthError::TokenExpired => write!(f, "token expired"),
            AuthError::TokenRevoked => write!(f, "token revoked"),
            AuthError::CsrfMismatch => write!(f, "CSRF state mismatch"),
            AuthError::PortUnavailable => write!(f, "callback port unavailable"),
            AuthError::LoginTimeout => write!(f, "login timeout"),
            AuthError::LoginDenied => write!(f, "login denied by user"),
            AuthError::KeyringUnavailable => write!(f, "keyring unavailable"),
            AuthError::FallbackCorrupted => write!(f, "encrypted fallback corrupted"),
            AuthError::AccountNotFound => write!(f, "account not found"),
            AuthError::AccountDuplicated => write!(f, "account already exists"),
            AuthError::InsufficientScope => write!(f, "insufficient OAuth scope"),
            AuthError::RateLimited { retry_after } => {
                if let Some(secs) = retry_after {
                    write!(f, "rate limited, retry after {}s", secs)
                } else {
                    write!(f, "rate limited")
                }
            }
            AuthError::StateError(msg) => write!(f, "state error: {}", msg),
        }
    }
}

impl std::error::Error for AuthError {}

pub type AuthResult<T> = Result<T, AuthError>;
