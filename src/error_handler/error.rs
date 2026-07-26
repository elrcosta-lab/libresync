use std::fmt;

#[derive(Debug)]
pub enum HandlerError {
    RetryExhausted(String),
    ConnectivityLost(String),
}

impl fmt::Display for HandlerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RetryExhausted(msg) => write!(f, "retry exhausted: {}", msg),
            Self::ConnectivityLost(msg) => write!(f, "connectivity lost: {}", msg),
        }
    }
}

impl std::error::Error for HandlerError {}
