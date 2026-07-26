use rusqlite::{params, Connection};

use crate::db::DbError;
use crate::sync::job::{JobState, SyncJob, JobType};

fn job_type_to_str(t: &JobType) -> &'static str {
    match t {
        JobType::Upload => "Upload",
        JobType::Download => "Download",
        JobType::Delete => "Delete",
        JobType::Move => "Move",
        JobType::Metadata => "Metadata",
    }
}

fn job_type_from_str(s: &str) -> Result<JobType, DbError> {
    match s {
        "Upload" => Ok(JobType::Upload),
        "Download" => Ok(JobType::Download),
        "Delete" => Ok(JobType::Delete),
        "Move" => Ok(JobType::Move),
        "Metadata" => Ok(JobType::Metadata),
        other => Err(DbError::InvalidJobType(other.to_string())),
    }
}

fn job_state_to_str(s: &JobState) -> &'static str {
    match s {
        JobState::Queued => "Queued",
        JobState::Running => "Running",
        JobState::Paused => "Paused",
        JobState::Completed => "Completed",
        JobState::Failed => "Failed",
        JobState::Cancelled => "Cancelled",
    }
}

fn job_state_from_str(s: &str) -> Result<JobState, DbError> {
    match s {
        "Queued" => Ok(JobState::Queued),
        "Running" => Ok(JobState::Running),
        "Paused" => Ok(JobState::Paused),
        "Completed" => Ok(JobState::Completed),
        "Failed" => Ok(JobState::Failed),
        "Cancelled" => Ok(JobState::Cancelled),
        other => Err(DbError::InvalidJobState(other.to_string())),
    }
}

fn row_to_job(row: &rusqlite::Row) -> Result<SyncJob, rusqlite::Error> {
    Ok(SyncJob {
        id: row.get("id")?,
        file_path: row.get("file_path")?,
        job_type: job_type_from_str(row.get::<_, String>("job_type")?.as_str())
            .unwrap_or(JobType::Upload),
        priority: row.get::<_, i32>("priority")? as u8,
        state: job_state_from_str(row.get::<_, String>("state")?.as_str())
            .unwrap_or(JobState::Queued),
        retry_count: row.get::<_, i32>("retry_count")? as u32,
        max_retries: row.get::<_, i32>("max_retries")? as u32,
        created_at: row.get::<_, i64>("created_at")? as u64,
        error_message: row.get("error_message")?,
    })
}

pub fn insert_job(conn: &Connection, job: &SyncJob) -> Result<(), DbError> {
    conn.execute(
        "INSERT INTO jobs (id, file_path, job_type, priority, state, retry_count, max_retries, created_at, error_message)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            job.id,
            job.file_path,
            job_type_to_str(&job.job_type),
            job.priority as i32,
            job_state_to_str(&job.state),
            job.retry_count as i32,
            job.max_retries as i32,
            job.created_at as i64,
            job.error_message,
        ],
    )?;
    Ok(())
}

pub fn get_job(conn: &Connection, id: &str) -> Result<Option<SyncJob>, DbError> {
    let mut stmt = conn.prepare(
        "SELECT id, file_path, job_type, priority, state, retry_count, max_retries, created_at, error_message FROM jobs WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_job(row)?)),
        None => Ok(None),
    }
}

pub fn list_jobs(conn: &Connection, state: Option<JobState>) -> Result<Vec<SyncJob>, DbError> {
    let mut jobs = Vec::new();
    match state {
        Some(s) => {
            let mut stmt = conn.prepare(
                "SELECT id, file_path, job_type, priority, state, retry_count, max_retries, created_at, error_message FROM jobs WHERE state = ?1 ORDER BY priority DESC, created_at ASC",
            )?;
            let rows = stmt.query_map(params![job_state_to_str(&s)], row_to_job)?;
            for row in rows {
                jobs.push(row?);
            }
        }
        None => {
            let mut stmt = conn.prepare(
                "SELECT id, file_path, job_type, priority, state, retry_count, max_retries, created_at, error_message FROM jobs ORDER BY priority DESC, created_at ASC",
            )?;
            let rows = stmt.query_map([], row_to_job)?;
            for row in rows {
                jobs.push(row?);
            }
        }
    }
    Ok(jobs)
}

pub fn update_job_state(
    conn: &Connection,
    id: &str,
    state: JobState,
    error: Option<&str>,
) -> Result<(), DbError> {
    let affected = conn.execute(
        "UPDATE jobs SET state = ?2, error_message = ?3 WHERE id = ?1",
        params![id, job_state_to_str(&state), error],
    )?;
    if affected == 0 {
        return Err(DbError::JobNotFound(id.to_string()));
    }
    Ok(())
}

pub fn delete_job(conn: &Connection, id: &str) -> Result<(), DbError> {
    let affected = conn.execute("DELETE FROM jobs WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(DbError::JobNotFound(id.to_string()));
    }
    Ok(())
}

pub fn clear_completed_jobs(conn: &Connection) -> Result<(), DbError> {
    conn.execute(
        "DELETE FROM jobs WHERE state = ?1 OR state = ?2",
        params![job_state_to_str(&JobState::Completed), job_state_to_str(&JobState::Cancelled)],
    )?;
    Ok(())
}
