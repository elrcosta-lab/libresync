use std::sync::{Arc, Mutex};

use crate::drive::DriveApi;
use crate::sync::config::SyncConfig;
use crate::sync::error::SyncError;
use crate::sync::job::{JobQueue, JobState, JobType, SyncJob};
use crate::sync::state::{SyncState, SyncStateMachine};

pub struct SyncEngine {
    state_machine: SyncStateMachine,
    job_queue: Arc<Mutex<JobQueue>>,
    drive_client: Arc<dyn DriveApi>,
    #[allow(dead_code)]
    config: SyncConfig,
}

impl SyncEngine {
    pub fn new(drive_client: Arc<dyn DriveApi>, config: SyncConfig) -> Self {
        Self {
            state_machine: SyncStateMachine::new(),
            job_queue: Arc::new(Mutex::new(JobQueue::new())),
            drive_client,
            config,
        }
    }

    pub fn state(&self) -> SyncState {
        self.state_machine.current()
    }

    pub fn queue_len(&self) -> usize {
        self.job_queue.lock().unwrap().len()
    }

    pub fn get_jobs_by_state(&self, state: JobState) -> Vec<SyncJob> {
        self.job_queue.lock().unwrap().get_by_state(state)
    }

    pub fn pause(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Paused)
    }

    pub fn resume(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Idle)
    }

    pub async fn start(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Scanning)?;
        self.state_machine.transition(SyncState::Queuing)?;
        Ok(())
    }

    pub async fn detect_changes(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Scanning)?;

        let remote_files = self
            .drive_client
            .list_files(None)
            .await
            .map_err(|e| SyncError::EngineError(format!("list: {}", e)))?;

        let mut queue = self.job_queue.lock().unwrap();
        for f in &remote_files {
            let job = SyncJob::new(&f.name, JobType::Download);
            queue.enqueue(job);
        }
        drop(queue);

        self.state_machine.transition(SyncState::Queuing)?;
        Ok(())
    }

    pub async fn process_queue(&mut self) -> Result<(), SyncError> {
        let has_work = {
            let queue = self.job_queue.lock().unwrap();
            !queue.get_by_state(JobState::Queued).is_empty()
        };

        if !has_work {
            return Ok(());
        }

        if self.state_machine.current() == SyncState::Idle {
            self.state_machine.transition(SyncState::Scanning)?;
            self.state_machine.transition(SyncState::Queuing)?;
        }

        let jobs: Vec<SyncJob> = {
            let queue = self.job_queue.lock().unwrap();
            let has_uploads = queue
                .get_by_state(JobState::Queued)
                .iter()
                .any(|j| j.job_type == JobType::Upload);
            let has_downloads = queue
                .get_by_state(JobState::Queued)
                .iter()
                .any(|j| j.job_type == JobType::Download);
            let has_deletes = queue
                .get_by_state(JobState::Queued)
                .iter()
                .any(|j| j.job_type == JobType::Delete);

            if has_uploads {
                self.state_machine
                    .transition(SyncState::Uploading)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
            } else if has_downloads {
                self.state_machine
                    .transition(SyncState::Downloading)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
            } else if has_deletes {
                self.state_machine
                    .transition(SyncState::Conflict)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
            }

            queue
                .get_by_state(JobState::Queued)
                .drain(..)
                .collect()
        };

        for mut job in jobs {
            job.state = JobState::Running;
            let result: Result<(), crate::drive::error::DriveError> = match job.job_type {
                JobType::Upload => {
                    let content = format!("sync upload: {}", job.file_path);
                    self.drive_client
                        .upload(&job.file_path, content.as_bytes(), "text/plain", None)
                        .await
                        .map(|_| ())
                }
                JobType::Download => {
                    self.drive_client.download(&job.file_path).await.map(|_| ())
                }
                JobType::Delete => self.drive_client.delete(&job.file_path).await,
                JobType::Move => Ok(()),
                JobType::Metadata => Ok(()),
            };

            match result {
                Ok(_) => {
                    job.state = JobState::Completed;
                    self.job_queue.lock().unwrap().enqueue(job);
                }
                Err(e) => {
                    job.state = JobState::Failed;
                    job.error_message = Some(e.to_string());
                    self.job_queue.lock().unwrap().enqueue(job);
                }
            }
        }

        self.state_machine
            .transition(SyncState::Verifying)
            .map_err(|e| SyncError::EngineError(e.to_string()))?;
        self.state_machine
            .transition(SyncState::Idle)
            .map_err(|e| SyncError::EngineError(e.to_string()))?;
        Ok(())
    }

    pub async fn on_file_changed(&mut self, path: &str) -> Result<(), SyncError> {
        let content = tokio::fs::read(path).await.unwrap_or_default();
        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(path);

        self.drive_client
            .upload(name, &content, "application/octet-stream", None)
            .await
            .map_err(|e| SyncError::EngineError(format!("upload: {}", e)))?;

        let job = SyncJob::new(path, JobType::Upload);
        self.job_queue.lock().unwrap().enqueue(job);
        Ok(())
    }

    pub async fn on_remote_change(&mut self, file_id: &str) -> Result<(), SyncError> {
        let data = self
            .drive_client
            .download(file_id)
            .await
            .map_err(|e| SyncError::EngineError(format!("download: {}", e)))?;

        let local_path = format!("/tmp/libresync/{}", file_id);
        if let Some(parent) = std::path::Path::new(&local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| SyncError::EngineError(format!("mkdir: {}", e)))?;
        }
        tokio::fs::write(&local_path, &data)
            .await
            .map_err(|e| SyncError::EngineError(format!("write: {}", e)))?;

        let job = SyncJob::new(&local_path, JobType::Download);
        self.job_queue.lock().unwrap().enqueue(job);
        Ok(())
    }
}
