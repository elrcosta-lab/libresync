use crate::conflict::detector::{ConflictDetector, ConflictInput};
use crate::conflict::error::ConflictError;
use crate::conflict::models::Conflict;
use crate::conflict::resolver::{ConflictResolution, ConflictResolver};

pub type ConflictCallback = Box<dyn Fn(Conflict) + Send + Sync>;

pub struct ConflictEngine {
    pub config: crate::conflict::config::ConflictConfig,
    callback: Option<ConflictCallback>,
}

impl ConflictEngine {
    pub fn new(config: crate::conflict::config::ConflictConfig) -> Self {
        Self {
            config,
            callback: None,
        }
    }

    pub fn with_callback(
        config: crate::conflict::config::ConflictConfig,
        callback: ConflictCallback,
    ) -> Self {
        Self {
            config,
            callback: Some(callback),
        }
    }

    pub fn set_callback(&mut self, callback: ConflictCallback) {
        self.callback = Some(callback);
    }

    pub fn handle_conflict(&self, input: ConflictInput) -> Result<ConflictResolution, ConflictError> {
        let conflict = ConflictDetector::detect(input).ok_or(ConflictError::ResolutionNotFound)?;
        let resolution = ConflictResolver::resolve(&conflict);
        if let Some(ref cb) = self.callback {
            cb(conflict);
        }
        Ok(resolution)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conflict::config::ConflictConfig;
    use crate::conflict::detector::ConflictInput;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn test_engine_detects_and_resolves() {
        let config = ConflictConfig {
            suffix_local: " (conflito maria)".into(),
            suffix_remote: " (conflito drive)".into(),
            auto_resolve: true,
        };
        let engine = ConflictEngine::new(config);
        let input = ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "def".into(),
            local_modified_at: 2000,
            remote_modified_at: 1000,
        };
        let resolution = engine.handle_conflict(input).unwrap();
        assert_eq!(
            resolution,
            ConflictResolution::KeepLocal {
                conflict_copy: Some("f1 (conflito drive)".into()),
            }
        );
    }

    #[test]
    fn test_engine_callback_invoked() {
        let config = ConflictConfig {
            suffix_local: " (conflito maria)".into(),
            suffix_remote: " (conflito drive)".into(),
            auto_resolve: true,
        };
        let call_count = Arc::new(AtomicUsize::new(0));
        let count_clone = call_count.clone();
        let callback: ConflictCallback = Box::new(move |_conflict| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });
        let engine = ConflictEngine::with_callback(config, callback);
        let input = ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "def".into(),
            local_modified_at: 2000,
            remote_modified_at: 1000,
        };
        let _ = engine.handle_conflict(input);
        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_engine_returns_error_for_no_conflict() {
        let config = ConflictConfig {
            suffix_local: " (conflito maria)".into(),
            suffix_remote: " (conflito drive)".into(),
            auto_resolve: true,
        };
        let engine = ConflictEngine::new(config);
        let input = ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "abc".into(),
            local_modified_at: 2000,
            remote_modified_at: 1000,
        };
        let result = engine.handle_conflict(input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ConflictError::ResolutionNotFound);
    }
}
