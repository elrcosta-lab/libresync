use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ConflictError {
    ResolutionNotFound,
    InvalidSuffix,
    EngineStopped,
}

impl fmt::Display for ConflictError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConflictError::ResolutionNotFound => write!(f, "resolution not found"),
            ConflictError::InvalidSuffix => write!(f, "invalid suffix"),
            ConflictError::EngineStopped => write!(f, "conflict engine stopped"),
        }
    }
}

impl std::error::Error for ConflictError {}
