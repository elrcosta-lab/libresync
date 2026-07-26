use libresync_core::conflict::models::{Conflict, ConflictKind};
use libresync_core::conflict::resolver::{ConflictResolution, ConflictResolver};

fn make_conflict(
    kind: ConflictKind,
    file_entry_id: &str,
    local_modified_at: i64,
    remote_modified_at: i64,
    local_hash: &str,
    remote_hash: &str,
) -> Conflict {
    Conflict {
        id: "test-id".into(),
        kind,
        file_entry_id: file_entry_id.into(),
        local_hash: local_hash.into(),
        remote_hash: remote_hash.into(),
        local_modified_at,
        remote_modified_at,
        detected_at: 0,
        resolution: None,
    }
}

#[test]
fn test_both_modified_local_wins() {
    let conflict = make_conflict(
        ConflictKind::BothModified,
        "relatorio.docx",
        2000,
        1000,
        "abc",
        "def",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(
        resolution,
        ConflictResolution::KeepLocal {
            conflict_copy: Some("relatorio (conflito drive).docx".into()),
        }
    );
}

#[test]
fn test_both_modified_remote_wins() {
    let conflict = make_conflict(
        ConflictKind::BothModified,
        "relatorio.docx",
        1000,
        2000,
        "abc",
        "def",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(
        resolution,
        ConflictResolution::KeepRemote {
            conflict_copy: Some("relatorio (conflito maria).docx".into()),
        }
    );
}

#[test]
fn test_local_deleted_remote_modified_restores_remote() {
    let conflict = make_conflict(
        ConflictKind::LocalDeletedRemoteModified,
        "foto.png",
        0,
        0,
        "",
        "abc",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(resolution, ConflictResolution::RestoreRemote);
}

#[test]
fn test_remote_deleted_local_modified_keeps_local() {
    let conflict = make_conflict(
        ConflictKind::RemoteDeletedLocalModified,
        "foto.png",
        0,
        0,
        "abc",
        "",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(
        resolution,
        ConflictResolution::KeepLocal {
            conflict_copy: None,
        }
    );
}

#[test]
fn test_simultaneous_create_keeps_both() {
    let conflict = make_conflict(
        ConflictKind::SimultaneousCreate,
        "novo.txt",
        0,
        0,
        "abc",
        "def",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(
        resolution,
        ConflictResolution::KeepBoth {
            local_path: "novo.txt".into(),
            remote_copy_path: "novo (conflito drive).txt".into(),
        }
    );
}

#[test]
fn test_tie_timestamp_keeps_both() {
    let conflict = make_conflict(
        ConflictKind::BothModified,
        "empate.txt",
        1000,
        1000,
        "abc",
        "def",
    );
    let resolution = ConflictResolver::resolve(&conflict);
    assert_eq!(
        resolution,
        ConflictResolution::KeepBoth {
            local_path: "empate.txt".into(),
            remote_copy_path: "empate (conflito drive).txt".into(),
        }
    );
}
