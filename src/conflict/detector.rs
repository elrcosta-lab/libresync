use crate::conflict::models::{Conflict, ConflictKind};

#[derive(Debug, Clone)]
pub enum ConflictInput {
    BothModified {
        file_id: String,
        local_hash: String,
        remote_hash: String,
        local_modified_at: i64,
        remote_modified_at: i64,
    },
    LocalDeletedRemoteModified {
        file_id: String,
        remote_hash: String,
    },
    RemoteDeletedLocalModified {
        file_id: String,
        local_hash: String,
    },
    SimultaneousCreate {
        name: String,
        local_hash: String,
        remote_hash: String,
    },
}

pub struct ConflictDetector;

impl ConflictDetector {
    pub fn detect(input: ConflictInput) -> Option<Conflict> {
        match input {
            ConflictInput::BothModified {
                file_id,
                local_hash,
                remote_hash,
                local_modified_at,
                remote_modified_at,
            } => {
                if local_hash == remote_hash {
                    return None;
                }
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: ConflictKind::BothModified,
                    file_entry_id: file_id,
                    local_hash,
                    remote_hash,
                    local_modified_at,
                    remote_modified_at,
                    detected_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    resolution: None,
                })
            }
            ConflictInput::LocalDeletedRemoteModified { file_id, remote_hash } => {
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: ConflictKind::LocalDeletedRemoteModified,
                    file_entry_id: file_id,
                    local_hash: String::new(),
                    remote_hash,
                    local_modified_at: 0,
                    remote_modified_at: 0,
                    detected_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    resolution: None,
                })
            }
            ConflictInput::RemoteDeletedLocalModified { file_id, local_hash } => {
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: ConflictKind::RemoteDeletedLocalModified,
                    file_entry_id: file_id,
                    local_hash,
                    remote_hash: String::new(),
                    local_modified_at: 0,
                    remote_modified_at: 0,
                    detected_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    resolution: None,
                })
            }
            ConflictInput::SimultaneousCreate {
                name: _,
                local_hash,
                remote_hash,
            } => {
                if local_hash == remote_hash {
                    return None;
                }
                Some(Conflict {
                    id: uuid::Uuid::new_v4().to_string(),
                    kind: ConflictKind::SimultaneousCreate,
                    file_entry_id: String::new(),
                    local_hash,
                    remote_hash,
                    local_modified_at: 0,
                    remote_modified_at: 0,
                    detected_at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    resolution: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_both_modified_detects_conflict() {
        let result = ConflictDetector::detect(ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "def".into(),
            local_modified_at: 1000,
            remote_modified_at: 900,
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::BothModified);
    }

    #[test]
    fn test_local_deleted_remote_modified_detects_conflict() {
        let result = ConflictDetector::detect(ConflictInput::LocalDeletedRemoteModified {
            file_id: "f1".into(),
            remote_hash: "abc".into(),
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::LocalDeletedRemoteModified);
    }

    #[test]
    fn test_remote_deleted_local_modified_detects_conflict() {
        let result = ConflictDetector::detect(ConflictInput::RemoteDeletedLocalModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::RemoteDeletedLocalModified);
    }

    #[test]
    fn test_simultaneous_create_detects_conflict() {
        let result = ConflictDetector::detect(ConflictInput::SimultaneousCreate {
            name: "novo.txt".into(),
            local_hash: "abc".into(),
            remote_hash: "def".into(),
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::SimultaneousCreate);
    }

    #[test]
    fn test_only_one_side_modified_is_not_conflict() {
        let result = ConflictDetector::detect(ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "abc".into(),
            local_modified_at: 1000,
            remote_modified_at: 900,
        });
        assert!(result.is_none());
    }

    #[test]
    fn test_same_timestamp_is_still_conflict() {
        let result = ConflictDetector::detect(ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "def".into(),
            local_modified_at: 1000,
            remote_modified_at: 1000,
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::BothModified);
        assert_eq!(conflict.local_modified_at, conflict.remote_modified_at);
    }

    #[test]
    fn test_same_hash_not_conflict() {
        let result = ConflictDetector::detect(ConflictInput::BothModified {
            file_id: "f1".into(),
            local_hash: "abc".into(),
            remote_hash: "abc".into(),
            local_modified_at: 1000,
            remote_modified_at: 900,
        });
        assert!(result.is_none());
    }

    #[test]
    fn test_simultaneous_create_same_hash_not_conflict() {
        let result = ConflictDetector::detect(ConflictInput::SimultaneousCreate {
            name: "same.txt".into(),
            local_hash: "abc".into(),
            remote_hash: "abc".into(),
        });
        assert!(result.is_none());
    }

    #[test]
    fn test_simultaneous_create_different_hash_is_conflict() {
        let result = ConflictDetector::detect(ConflictInput::SimultaneousCreate {
            name: "diff.txt".into(),
            local_hash: "abc".into(),
            remote_hash: "xyz".into(),
        });
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.kind, ConflictKind::SimultaneousCreate);
    }
}
