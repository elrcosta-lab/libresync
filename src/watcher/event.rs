use std::time::Instant;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    Created(String),
    Modified(String),
    Deleted(String),
    Renamed { from: String, to: String },
}

impl FileEvent {
    pub fn path(&self) -> &str {
        match self {
            FileEvent::Created(p) => p,
            FileEvent::Modified(p) => p,
            FileEvent::Deleted(p) => p,
            FileEvent::Renamed { from, .. } => from,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DebouncedEvent {
    pub events: Vec<FileEvent>,
    pub timestamp: Instant,
}

impl DebouncedEvent {
    pub fn new(events: Vec<FileEvent>) -> Self {
        Self {
            timestamp: Instant::now(),
            events,
        }
    }
}
