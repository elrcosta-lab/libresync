use crate::conflict::resolver::ConflictResolution;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    BothModified,
    LocalDeletedRemoteModified,
    RemoteDeletedLocalModified,
    SimultaneousCreate,
}

#[derive(Debug, Clone)]
pub struct Conflict {
    pub id: String,
    pub kind: ConflictKind,
    pub file_entry_id: String,
    pub local_hash: String,
    pub remote_hash: String,
    pub local_modified_at: i64,
    pub remote_modified_at: i64,
    pub detected_at: i64,
    pub resolution: Option<ConflictResolution>,
}

#[derive(Debug, Clone)]
pub struct ConflictRecord {
    pub id: String,
    pub conflict_id: String,
    pub kept_path: String,
    pub conflict_copy_path: Option<String>,
    pub resolved_by: String,
    pub resolved_at: i64,
}
