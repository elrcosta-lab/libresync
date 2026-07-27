use std::path::PathBuf;
use std::sync::Mutex;

use rusqlite::Connection;

use crate::db::DbError;

pub struct Database {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl Database {
    pub fn open(path: &str) -> Result<Self, DbError> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        let db = Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(path),
        };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_default() -> Result<Self, DbError> {
        let dir = dirs_data_dir().ok_or_else(|| {
            DbError::PathNotFound(
                std::path::PathBuf::from("~/.config/libresync"),
            )
        })?;
        std::fs::create_dir_all(&dir).map_err(|e| {
            DbError::PathNotFound(dir.join(e.to_string()))
        })?;
        let path = dir.join("data.db");
        Self::open(path.to_str().ok_or_else(|| {
            DbError::PathNotFound(path.clone())
        })?)
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn migrate(&self) -> Result<(), DbError> {
        let conn = self.conn.lock().map_err(|e| DbError::LockError(e.to_string()))?;
        conn.execute_batch(
            "
CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    email TEXT NOT NULL,
    display_name TEXT NOT NULL,
    avatar_url TEXT,
    scope TEXT NOT NULL DEFAULT 'drive.file',
    token_expires_at INTEGER NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'Active',
    is_active INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    last_sync_at INTEGER,
    quota_total INTEGER,
    quota_used INTEGER
);

CREATE TABLE IF NOT EXISTS sync_state (
    path TEXT PRIMARY KEY,
    local_modified_at INTEGER,
    remote_modified_at INTEGER,
    local_hash TEXT,
    remote_hash TEXT,
    last_sync_at INTEGER NOT NULL
);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY,
    file_path TEXT NOT NULL,
    remote_file_id TEXT,
    job_type TEXT NOT NULL,
    priority INTEGER NOT NULL DEFAULT 10,
    state TEXT NOT NULL DEFAULT 'Queued',
    retry_count INTEGER NOT NULL DEFAULT 0,
    max_retries INTEGER NOT NULL DEFAULT 5,
    created_at INTEGER NOT NULL,
    error_message TEXT
);
",
        )?;
        Ok(())
    }

    pub fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.conn.lock().expect("DB lock poisoned")
    }
}

fn dirs_data_dir() -> Option<std::path::PathBuf> {
    directories::ProjectDirs::from("com", "libresync", "LibreSync")
        .map(|d| d.data_dir().to_path_buf())
}
