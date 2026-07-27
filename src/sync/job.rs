use std::time::SystemTime;

use crate::sync::error::SyncError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobType {
    Upload,
    Download,
    Delete,
    Move,
    Metadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobState {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct SyncJob {
    pub id: String,
    pub file_path: String,
    pub job_type: JobType,
    pub priority: u8,
    pub state: JobState,
    pub retry_count: u32,
    pub max_retries: u32,
    pub created_at: u64,
    pub error_message: Option<String>,
    pub remote_file_id: Option<String>,
}

impl SyncJob {
    pub fn new(file_path: &str, job_type: JobType) -> Self {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            file_path: file_path.to_string(),
            job_type,
            priority: 10,
            state: JobState::Queued,
            retry_count: 0,
            max_retries: 5,
            created_at: now,
            error_message: None,
            remote_file_id: None,
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_remote_file_id(mut self, remote_file_id: &str) -> Self {
        self.remote_file_id = Some(remote_file_id.to_string());
        self
    }
}

#[derive(Debug, Clone)]
pub struct JobQueue {
    jobs: Vec<SyncJob>,
}

impl JobQueue {
    pub fn new() -> Self {
        Self { jobs: Vec::new() }
    }

    pub fn enqueue(&mut self, job: SyncJob) {
        self.jobs.push(job);
    }

    pub fn dequeue(&mut self) -> Option<SyncJob> {
        let idx = self
            .jobs
            .iter()
            .enumerate()
            .filter(|(_, j)| j.state == JobState::Queued)
            .max_by_key(|(_, j)| j.priority)
            .map(|(i, _)| i)?;
        let mut job = self.jobs.remove(idx);
        job.state = JobState::Running;
        Some(job)
    }

    pub fn peek(&self) -> Option<&SyncJob> {
        self.jobs
            .iter()
            .filter(|j| j.state == JobState::Queued)
            .max_by_key(|j| j.priority)
    }

    pub fn cancel(&mut self, id: &str) -> Result<(), SyncError> {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or(SyncError::JobNotFound)?;
        job.state = JobState::Cancelled;
        Ok(())
    }

    pub fn retry(&mut self, id: &str) -> Result<(), SyncError> {
        let job = self
            .jobs
            .iter_mut()
            .find(|j| j.id == id)
            .ok_or(SyncError::JobNotFound)?;
        if job.retry_count < job.max_retries {
            job.retry_count += 1;
            job.state = JobState::Queued;
            Ok(())
        } else {
            Err(SyncError::MaxRetriesExceeded)
        }
    }

    pub fn len(&self) -> usize {
        self.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.jobs.is_empty()
    }

    pub fn get_by_state(&self, state: JobState) -> Vec<SyncJob> {
        self.jobs
            .iter()
            .filter(|j| j.state == state)
            .cloned()
            .collect()
    }

    pub fn jobs_mut(&mut self) -> &mut [SyncJob] {
        &mut self.jobs
    }
}

impl Default for JobQueue {
    fn default() -> Self {
        Self::new()
    }
}
