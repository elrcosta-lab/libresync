use std::sync::{Arc, Mutex};

use crate::auth::provider::AuthProvider;
use crate::sync::config::SyncConfig;
use crate::sync::error::SyncError;
use crate::sync::job::{JobQueue, JobState, JobType, SyncJob};
use crate::sync::state::{SyncState, SyncStateMachine};

#[allow(dead_code)]
pub struct SyncEngine {
    state_machine: SyncStateMachine,
    job_queue: Arc<Mutex<JobQueue>>,
    auth_provider: Arc<dyn AuthProvider>,
    config: SyncConfig,
}

impl SyncEngine {
    pub fn new(auth_provider: Arc<dyn AuthProvider>, config: SyncConfig) -> Self {
        Self {
            state_machine: SyncStateMachine::new(),
            job_queue: Arc::new(Mutex::new(JobQueue::new())),
            auth_provider,
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

    pub fn start(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Scanning)?;
        self.state_machine.transition(SyncState::Queuing)?;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Paused)
    }

    pub fn resume(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Idle)
    }

    pub fn detect_changes(&mut self) -> Result<(), SyncError> {
        self.state_machine.transition(SyncState::Scanning)?;
        let job = SyncJob::new("/sync/detected_file.txt", JobType::Upload);
        self.job_queue.lock().unwrap().enqueue(job);
        self.state_machine.transition(SyncState::Queuing)?;
        Ok(())
    }

    pub fn process_queue(&mut self) -> Result<(), SyncError> {
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

        let mut queue = self.job_queue.lock().unwrap();
        let has_uploads = queue
            .get_by_state(JobState::Queued)
            .iter()
            .any(|j| j.job_type == JobType::Upload);
        let has_downloads = queue
            .get_by_state(JobState::Queued)
            .iter()
            .any(|j| j.job_type == JobType::Download);
        let has_conflict = queue
            .get_by_state(JobState::Queued)
            .iter()
            .any(|j| j.job_type == JobType::Move);

        if has_uploads {
            self.state_machine.transition(SyncState::Uploading)?;
        } else if has_downloads {
            self.state_machine.transition(SyncState::Downloading)?;
        } else if has_conflict {
            self.state_machine.transition(SyncState::Conflict)?;
        }

        for job in queue.jobs_mut() {
            if job.state == JobState::Queued {
                job.state = JobState::Running;
                job.state = JobState::Completed;
            }
        }
        drop(queue);

        self.state_machine.transition(SyncState::Verifying)?;
        self.state_machine.transition(SyncState::Idle)?;
        Ok(())
    }

    pub fn on_file_changed(&mut self, path: &str) -> Result<(), SyncError> {
        let job = SyncJob::new(path, JobType::Upload);
        self.job_queue.lock().unwrap().enqueue(job);
        Ok(())
    }

    pub fn on_remote_change(&mut self, file_id: &str) -> Result<(), SyncError> {
        let job = SyncJob::new(file_id, JobType::Download);
        self.job_queue.lock().unwrap().enqueue(job);
        Ok(())
    }
}
