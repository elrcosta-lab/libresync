use rusqlite::params;

use crate::db::{Database, DbError};

#[derive(Debug, Clone)]
pub struct SyncStateEntry {
    pub path: String,
    pub local_modified_at: Option<i64>,
    pub remote_modified_at: Option<i64>,
    pub local_hash: Option<String>,
    pub remote_hash: Option<String>,
    pub last_sync_at: i64,
}

fn row_to_sync_state(row: &rusqlite::Row) -> Result<SyncStateEntry, rusqlite::Error> {
    Ok(SyncStateEntry {
        path: row.get("path")?,
        local_modified_at: row.get("local_modified_at")?,
        remote_modified_at: row.get("remote_modified_at")?,
        local_hash: row.get("local_hash")?,
        remote_hash: row.get("remote_hash")?,
        last_sync_at: row.get("last_sync_at")?,
    })
}

pub fn upsert_sync_state(db: &Database, entry: &SyncStateEntry) -> Result<(), DbError> {
    let conn = db.conn();
    conn.execute(
        "INSERT INTO sync_state (path, local_modified_at, remote_modified_at, local_hash, remote_hash, last_sync_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(path) DO UPDATE SET
            local_modified_at = excluded.local_modified_at,
            remote_modified_at = excluded.remote_modified_at,
            local_hash = excluded.local_hash,
            remote_hash = excluded.remote_hash,
            last_sync_at = excluded.last_sync_at",
        params![
            entry.path,
            entry.local_modified_at,
            entry.remote_modified_at,
            entry.local_hash,
            entry.remote_hash,
            entry.last_sync_at,
        ],
    )?;
    Ok(())
}

pub fn get_sync_state(db: &Database, path: &str) -> Result<Option<SyncStateEntry>, DbError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT path, local_modified_at, remote_modified_at, local_hash, remote_hash, last_sync_at FROM sync_state WHERE path = ?1",
    )?;
    let mut rows = stmt.query(params![path])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_sync_state(row)?)),
        None => Ok(None),
    }
}

pub fn list_sync_states(db: &Database) -> Result<Vec<SyncStateEntry>, DbError> {
    let conn = db.conn();
    let mut stmt = conn.prepare(
        "SELECT path, local_modified_at, remote_modified_at, local_hash, remote_hash, last_sync_at FROM sync_state ORDER BY path ASC",
    )?;
    let rows = stmt.query_map([], row_to_sync_state)?;
    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn delete_sync_state(db: &Database, path: &str) -> Result<(), DbError> {
    let conn = db.conn();
    let affected =
        conn.execute("DELETE FROM sync_state WHERE path = ?1", params![path])?;
    if affected == 0 {
        return Err(DbError::SyncStateNotFound(path.to_string()));
    }
    Ok(())
}
