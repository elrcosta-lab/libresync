use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::conflict::config::ConflictConfig;
use crate::conflict::detector::ConflictInput;
use crate::conflict::engine::ConflictEngine;
use crate::conflict::resolver::ConflictResolution;
use crate::db::Database;
use crate::drive::error::DriveError;
use crate::drive::DriveApi;
use crate::sync::config::SyncConfig;
use crate::sync::error::SyncError;
use crate::sync::job::{JobQueue, JobState, JobType, SyncJob};
use crate::sync::state::{SyncState, SyncStateMachine};

pub struct SyncEngine {
    state_machine: SyncStateMachine,
    pub job_queue: Arc<Mutex<JobQueue>>,
    drive_client: Arc<dyn DriveApi>,
    #[allow(dead_code)]
    config: SyncConfig,
    sync_dir: String,
    conflict_engine: ConflictEngine,
    #[allow(dead_code)]
    db: Option<Arc<Database>>,
}

const FOLDER_MIME: &str = "application/vnd.google-apps.folder";

fn resolve_remote_path(
    parents: &Option<Vec<String>>,
    file_name: &str,
    folder_map: &HashMap<String, (String, Option<Vec<String>>)>,
) -> String {
    if let Some(parents_list) = parents {
        if let Some(parent_id) = parents_list.first() {
            if let Some((parent_name, parent_parents)) = folder_map.get(parent_id) {
                let parent_path = resolve_remote_path(parent_parents, parent_name, folder_map);
                return format!("{}/{}", parent_path, file_name);
            }
        }
    }
    file_name.to_string()
}

fn compute_local_hash(data: &[u8]) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn local_modified_timestamp(path: &str) -> i64 {
    tokio::fs::metadata(path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

impl SyncEngine {
    pub fn new(
        drive_client: Arc<dyn DriveApi>,
        config: SyncConfig,
        sync_dir: &str,
    #[allow(dead_code)]
    db: Option<Arc<Database>>,
    ) -> Self {
        let conflict_config = ConflictConfig {
            suffix_local: " (conflito maria)".into(),
            suffix_remote: " (conflito drive)".into(),
            auto_resolve: true,
        };
        Self {
            state_machine: SyncStateMachine::new(),
            job_queue: Arc::new(Mutex::new(JobQueue::new())),
            drive_client,
            config,
            sync_dir: sync_dir.to_string(),
            conflict_engine: ConflictEngine::new(conflict_config),
            db,
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
        // Garantir que estamos em Idle antes de iniciar scan
        let current = self.state_machine.current();
        if current == SyncState::Queuing {
            let _ = self.state_machine.transition(SyncState::Idle);
        }
        
        self.state_machine.transition(SyncState::Scanning)?;

        println!("[detect_changes] Listando arquivos remotos...");
        let remote_files = match self.drive_client.list_files(None).await {
            Ok(files) => files,
            Err(e) => {
                let _ = self.state_machine.transition(SyncState::Error);
                let _ = self.state_machine.transition(SyncState::Idle);
                return Err(SyncError::EngineError(format!("list: {}", e)));
            }
        };

        println!("[detect_changes] {} arquivos remotos encontrados", remote_files.len());

        let mut folder_map: HashMap<String, (String, Option<Vec<String>>)> = HashMap::new();
        for f in &remote_files {
            if f.mime_type == FOLDER_MIME {
                folder_map.insert(f.id.clone(), (f.name.clone(), f.parents.clone()));
            }
        }

        let mut queue = self.job_queue.lock().unwrap();
        for f in &remote_files {
            if f.mime_type == FOLDER_MIME {
                continue;
            }
            let remote_path = resolve_remote_path(&f.parents, &f.name, &folder_map);
            println!("[detect_changes] Criando job download para '{}' (path: {})", f.name, remote_path);
            let job = SyncJob::new(&remote_path, JobType::Download)
                .with_remote_file_id(&f.id);
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
            println!("[process_queue] Nenhum job na fila");
            return Ok(());
        }

        println!("[process_queue] Iniciando processamento da fila...");

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
            let result: Result<(), DriveError> = match job.job_type {
                JobType::Upload => self.handle_upload_job(&job).await,
                JobType::Download => self.handle_download_job(&job).await,
                JobType::Delete => self.handle_delete_job(&job).await,
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

        match self.state_machine.current() {
            SyncState::Conflict => {
                self.state_machine
                    .transition(SyncState::Resolving)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
                self.state_machine
                    .transition(SyncState::Idle)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
            }
            _ => {
                self.state_machine
                    .transition(SyncState::Verifying)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
                self.state_machine
                    .transition(SyncState::Idle)
                    .map_err(|e| SyncError::EngineError(e.to_string()))?;
            }
        }
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

    async fn resolve_remote_path_for_file(&self, parents: &Option<Vec<String>>, file_name: &str) -> String {
        let mut segments: Vec<String> = vec![file_name.to_string()];
        let mut current_parents = parents.clone();
        loop {
            match current_parents.as_ref().and_then(|p| p.first().cloned()) {
                Some(parent_id) => {
                    match self.drive_client.get_metadata(&parent_id).await {
                        Ok(parent_meta) => {
                            segments.push(parent_meta.name);
                            current_parents = parent_meta.parents;
                        }
                        Err(_) => break,
                    }
                }
                None => break,
            }
        }
        segments.reverse();
        segments.join("/")
    }

    pub async fn on_remote_change(&mut self, file_id: &str) -> Result<(), SyncError> {
        let meta = self
            .drive_client
            .get_metadata(file_id)
            .await
            .map_err(|e| SyncError::EngineError(format!("metadata: {}", e)))?;

        let data = self
            .drive_client
            .download(file_id)
            .await
            .map_err(|e| SyncError::EngineError(format!("download: {}", e)))?;

        let remote_path = self.resolve_remote_path_for_file(&meta.parents, &meta.name).await;
        let local_path = format!("{}/{}", self.sync_dir, remote_path);
        if let Some(parent) = std::path::Path::new(&local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| SyncError::EngineError(format!("mkdir: {}", e)))?;
        }
        tokio::fs::write(&local_path, &data)
            .await
            .map_err(|e| SyncError::EngineError(format!("write: {}", e)))?;

        let job = SyncJob::new(&remote_path, JobType::Download)
            .with_remote_file_id(file_id);
        self.job_queue.lock().unwrap().enqueue(job);
        Ok(())
    }

    async fn handle_upload_job(&self, job: &SyncJob) -> Result<(), DriveError> {
        let local_path = &job.file_path;
        let name = std::path::Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(local_path);

        let local_content = tokio::fs::read(local_path).await.unwrap_or_else(|_| {
            format!("sync upload: {}", job.file_path).into_bytes()
        });
        let local_hash = compute_local_hash(&local_content);

        let remote_lookup = match &job.remote_file_id {
            Some(id) => self.drive_client.get_metadata(id).await,
            None => Err(DriveError::NotFound(name.to_string())),
        };

        match remote_lookup {
            Ok(remote_file) => {
                if let Some(ref remote_hash) = remote_file.md5_checksum {
                    if remote_hash != &local_hash {
                        let input = ConflictInput::BothModified {
                            file_id: remote_file.id.clone(),
                            local_hash,
                            remote_hash: remote_hash.clone(),
                            local_modified_at: local_modified_timestamp(local_path).await,
                            remote_modified_at: 0,
                        };
                        return match self.conflict_engine.handle_conflict(input) {
                            Ok(ConflictResolution::KeepLocal { conflict_copy }) => {
                                if let Some(ref copy_name) = conflict_copy {
                                    self.drive_client
                                        .upload(copy_name, &local_content, "application/octet-stream", None)
                                        .await?;
                                }
                                self.drive_client
                                    .upload(name, &local_content, "application/octet-stream", None)
                                    .await?;
                                Ok(())
                            }
                            Ok(ConflictResolution::KeepRemote { .. }) => {
                                let data = self.drive_client.download(&remote_file.id).await?;
                                self.write_downloaded_file(name, &data).await?;
                                Ok(())
                            }
                            Ok(ConflictResolution::KeepBoth { remote_copy_path, .. }) => {
                                self.drive_client
                                    .upload(&remote_copy_path, &local_content, "application/octet-stream", None)
                                    .await?;
                                Ok(())
                            }
                            Ok(ConflictResolution::RestoreRemote) => {
                                let data = self.drive_client.download(&remote_file.id).await?;
                                self.write_downloaded_file(name, &data).await?;
                                Ok(())
                            }
                            Err(_) => {
                                self.drive_client
                                    .upload(name, &local_content, "application/octet-stream", None)
                                    .await?;
                                Ok(())
                            }
                        };
                    }
                }
                self.drive_client
                    .upload(name, &local_content, "application/octet-stream", None)
                    .await?;
                Ok(())
            }
            Err(DriveError::NotFound(_)) => {
                self.drive_client
                    .upload(name, &local_content, "application/octet-stream", None)
                    .await?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn write_downloaded_file(&self, name: &str, data: &[u8]) -> Result<(), DriveError> {
        let local_path = format!("{}/{}", self.sync_dir, name);
        println!("[write_downloaded_file] Escrevendo {} bytes em '{}'", data.len(), local_path);
        if let Some(parent) = std::path::Path::new(&local_path).parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DriveError::Network(format!("mkdir: {}", e)))?;
        }
        tokio::fs::write(&local_path, data)
            .await
            .map_err(|e| DriveError::Network(format!("write: {}", e)))?;
        println!("[write_downloaded_file] Arquivo '{}' escrito com sucesso", local_path);
        Ok(())
    }

    async fn handle_download_job(&self, job: &SyncJob) -> Result<(), DriveError> {
        let local_path = &job.file_path;
        let name = std::path::Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(local_path);

        let file_id = match &job.remote_file_id {
            Some(id) => id.as_str(),
            None => name,
        };

        println!("[handle_download_job] Baixando '{}' (file_id: {})", name, file_id);

        match self.drive_client.get_metadata(file_id).await {
            Ok(remote_file) => {
                println!("[handle_download_job] Metadata OK para '{}'", remote_file.name);
                if let Ok(local_content) = tokio::fs::read(local_path).await {
                    let local_hash = compute_local_hash(&local_content);
                    if let Some(ref remote_hash) = remote_file.md5_checksum {
                        if remote_hash != &local_hash {
                            let input = ConflictInput::BothModified {
                                file_id: remote_file.id.clone(),
                                local_hash,
                                remote_hash: remote_hash.clone(),
                                local_modified_at: local_modified_timestamp(local_path).await,
                                remote_modified_at: 0,
                            };
                            return match self.conflict_engine.handle_conflict(input) {
                                Ok(ConflictResolution::KeepRemote { .. }) => {
                                    let data = self.drive_client.download(&remote_file.id).await?;
                                    self.write_downloaded_file(name, &data).await?;
                                    Ok(())
                                }
                                Ok(ConflictResolution::KeepLocal { .. }) => Ok(()),
                                Ok(ConflictResolution::KeepBoth { remote_copy_path, .. }) => {
                                    let remote_content =
                                        self.drive_client.download(&remote_file.id).await?;
                                    self.drive_client
                                        .upload(&remote_copy_path, &remote_content, "application/octet-stream", None)
                                        .await?;
                                    self.write_downloaded_file(name, &local_content).await?;
                                    Ok(())
                                }
                                Ok(ConflictResolution::RestoreRemote) => {
                                    let data = self.drive_client.download(&remote_file.id).await?;
                                    self.write_downloaded_file(name, &data).await?;
                                    Ok(())
                                }
                                Err(_) => {
                                    let data = self.drive_client.download(&remote_file.id).await?;
                                    self.write_downloaded_file(name, &data).await?;
                                    Ok(())
                                }
                            };
                        }
                    }
                }
                let data = self.drive_client.download(&remote_file.id).await?;
                self.write_downloaded_file(name, &data).await?;
                Ok(())
            }
            Err(DriveError::NotFound(_)) => {
                let data = self.drive_client.download(file_id).await?;
                self.write_downloaded_file(name, &data).await?;
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    async fn handle_delete_job(&self, job: &SyncJob) -> Result<(), DriveError> {
        let local_path = &job.file_path;
        let name = std::path::Path::new(local_path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(local_path);

        let local_exists = tokio::fs::metadata(local_path).await.is_ok();

        let file_id = match &job.remote_file_id {
            Some(id) => id.as_str(),
            None => name,
        };

        match self.drive_client.get_metadata(file_id).await {
            Ok(remote_file) => {
                if !local_exists {
                    let input = ConflictInput::LocalDeletedRemoteModified {
                        file_id: remote_file.id.clone(),
                        remote_hash: remote_file.md5_checksum.clone().unwrap_or_default(),
                    };
                    if let Ok(ConflictResolution::RestoreRemote) = self.conflict_engine.handle_conflict(input) {
                        let data = self.drive_client.download(&remote_file.id).await?;
                        self.write_downloaded_file(name, &data).await?;
                        return Ok(());
                    }
                } else {
                    let local_content = tokio::fs::read(local_path).await.unwrap_or_default();
                    let input = ConflictInput::RemoteDeletedLocalModified {
                        file_id: remote_file.id.clone(),
                        local_hash: compute_local_hash(&local_content),
                    };
                    if let Ok(ConflictResolution::KeepLocal { .. }) = self.conflict_engine.handle_conflict(input) {
                        return Ok(());
                    }
                }
                self.drive_client.delete(&remote_file.id).await
            }
            Err(DriveError::NotFound(_)) => self.drive_client.delete(file_id).await,
            Err(e) => Err(e),
        }
    }
}
