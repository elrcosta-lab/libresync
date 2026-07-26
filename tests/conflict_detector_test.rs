use libresync_core::conflict::detector::{ConflictDetector, ConflictInput};
use libresync_core::conflict::models::ConflictKind;

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
