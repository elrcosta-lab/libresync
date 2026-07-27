mod common;

use async_trait::async_trait;
use common::create_test_engine;
use libresync_core::drive::client::DriveFile;
use libresync_core::drive::error::{DriveError, DriveResult};
use libresync_core::drive::DriveApi;
use libresync_core::sync::config::SyncConfig;
use libresync_core::sync::engine::SyncEngine;
use libresync_core::sync::job::{JobState, JobType, SyncJob};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

struct FileEntry {
    name: String,
    content: Vec<u8>,
    md5_checksum: Option<String>,
    exists: bool,
}

struct StatefulMockDriveApi {
    files: Mutex<HashMap<String, FileEntry>>,
}

struct FailingListDriveApi;

impl StatefulMockDriveApi {
    fn new() -> Self {
        Self {
            files: Mutex::new(HashMap::new()),
        }
    }

    fn add_file(&self, id: &str, name: &str, content: Vec<u8>, md5: Option<&str>) {
        let mut files = self.files.lock().unwrap();
        files.insert(
            id.to_string(),
            FileEntry {
                name: name.to_string(),
                content,
                md5_checksum: md5.map(|s| s.to_string()),
                exists: true,
            },
        );
    }
}

#[async_trait]
impl DriveApi for StatefulMockDriveApi {
    async fn list_files(&self, _parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>> {
        Ok(vec![])
    }

    async fn get_metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        let files = self.files.lock().unwrap();
        match files.get(file_id) {
            Some(entry) if entry.exists => Ok(DriveFile {
                id: file_id.to_string(),
                name: entry.name.clone(),
                mime_type: "text/plain".into(),
                size: None,
                created_time: None,
                modified_time: None,
                md5_checksum: entry.md5_checksum.clone(),
                parents: None,
                trashed: false,
            }),
            _ => Err(DriveError::NotFound(file_id.to_string())),
        }
    }

    async fn upload(
        &self,
        name: &str,
        content: &[u8],
        _mime_type: &str,
        _parent_id: Option<&str>,
    ) -> DriveResult<DriveFile> {
        let mut files = self.files.lock().unwrap();
        let id = format!("uploaded_{}", name);
        files.insert(
            id.clone(),
            FileEntry {
                name: name.to_string(),
                content: content.to_vec(),
                md5_checksum: None,
                exists: true,
            },
        );
        Ok(DriveFile {
            id,
            name: name.to_string(),
            mime_type: "text/plain".into(),
            size: None,
            created_time: None,
            modified_time: None,
            md5_checksum: None,
            parents: None,
            trashed: false,
        })
    }

    async fn download(&self, file_id: &str) -> DriveResult<Vec<u8>> {
        let files = self.files.lock().unwrap();
        match files.get(file_id) {
            Some(entry) if entry.exists => Ok(entry.content.clone()),
            _ => Ok(b"mock data".to_vec()),
        }
    }

    async fn delete(&self, file_id: &str) -> DriveResult<()> {
        let mut files = self.files.lock().unwrap();
        if let Some(entry) = files.get_mut(file_id) {
            entry.exists = false;
        }
        Ok(())
    }
}

#[async_trait]
impl DriveApi for FailingListDriveApi {
    async fn list_files(&self, _parent_id: Option<&str>) -> DriveResult<Vec<DriveFile>> {
        Err(DriveError::Auth("HTTP 401 Unauthorized".into()))
    }

    async fn get_metadata(&self, file_id: &str) -> DriveResult<DriveFile> {
        Err(DriveError::NotFound(file_id.to_string()))
    }

    async fn upload(
        &self,
        _name: &str,
        _content: &[u8],
        _mime_type: &str,
        _parent_id: Option<&str>,
    ) -> DriveResult<DriveFile> {
        Err(DriveError::Auth("HTTP 401 Unauthorized".into()))
    }

    async fn download(&self, _file_id: &str) -> DriveResult<Vec<u8>> {
        Err(DriveError::Auth("HTTP 401 Unauthorized".into()))
    }

    async fn delete(&self, _file_id: &str) -> DriveResult<()> {
        Err(DriveError::Auth("HTTP 401 Unauthorized".into()))
    }
}

fn create_engine_with_mock(mock: Arc<dyn DriveApi>) -> SyncEngine {
    let config = SyncConfig::default();
    SyncEngine::new(mock, config, "/tmp/libresync-test", None)
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::Digest;
    let hash = sha2::Sha256::digest(data);
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[test]
fn test_engine_creation() {
    let engine = create_test_engine();
    assert_eq!(engine.state().to_string(), "Idle");
    assert_eq!(engine.queue_len(), 0);
}

#[test]
fn test_engine_pause_resume() {
    let mut engine = create_test_engine();
    engine.pause().unwrap();
    assert_eq!(engine.state().to_string(), "Paused");

    engine.resume().unwrap();
    assert_eq!(engine.state().to_string(), "Idle");
}

#[tokio::test]
async fn test_detect_changes_no_errors() {
    let mut engine = create_test_engine();
    engine.detect_changes().await.unwrap();
    assert_eq!(engine.state().to_string(), "Queuing");
}

#[tokio::test]
async fn test_detect_changes_failure_returns_to_idle() {
    let mock = Arc::new(FailingListDriveApi);
    let mut engine = create_engine_with_mock(mock);

    let result = engine.detect_changes().await;

    assert!(result.is_err());
    assert_eq!(engine.state().to_string(), "Idle");
}

#[tokio::test]
async fn test_on_file_changed_enqueues_upload() {
    let mut engine = create_test_engine();
    engine.on_file_changed("/home/test/file.txt").await.unwrap();

    let jobs = engine.get_jobs_by_state(JobState::Queued);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_type, JobType::Upload);
    assert_eq!(jobs[0].file_path, "/home/test/file.txt");
}

#[tokio::test]
async fn test_on_remote_change_enqueues_download() {
    let mut engine = create_test_engine();
    engine
        .on_remote_change("remote-file-id-123")
        .await
        .unwrap();

    let jobs = engine.get_jobs_by_state(JobState::Queued);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_type, JobType::Download);
}

#[tokio::test]
async fn test_process_queue_completes_jobs() {
    let mut engine = create_test_engine();
    engine.on_file_changed("/sync/test.txt").await.unwrap();
    engine.on_remote_change("remote-id").await.unwrap();
    assert_eq!(engine.queue_len(), 2);

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 2);
}

#[tokio::test]
async fn test_engine_start_transitions_state() {
    let mut engine = create_test_engine();
    assert_eq!(engine.state().to_string(), "Idle");

    engine.start().await.unwrap();
    assert_eq!(engine.state().to_string(), "Queuing");
}

#[tokio::test]
async fn test_upload_without_conflict() {
    let mock = Arc::new(StatefulMockDriveApi::new());
    let mut engine = create_engine_with_mock(mock.clone());

    let job = SyncJob::new("/nonexistent/remote_file.txt", JobType::Upload);
    {
        let mut queue = engine.job_queue.lock().unwrap();
        queue.enqueue(job);
    }

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].job_type, JobType::Upload);
}

#[tokio::test]
async fn test_upload_with_conflict_resolved_local() {
    let mock = Arc::new(StatefulMockDriveApi::new());
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("conflito.txt");
    let local_content = b"local version content";
    tokio::fs::write(&file_path, local_content)
        .await
        .unwrap();

    let remote_hash = "different_remote_hash_value";
    mock.add_file("conflito.txt", "conflito.txt", b"remote version".to_vec(), Some(remote_hash));

    let mut engine = create_engine_with_mock(mock.clone());
    let job = SyncJob::new(file_path.to_str().unwrap(), JobType::Upload)
        .with_remote_file_id("conflito.txt");
    {
        let mut queue = engine.job_queue.lock().unwrap();
        queue.enqueue(job);
    }

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].job_type, JobType::Upload);
}

#[tokio::test]
async fn test_download_with_conflict_resolved_remote() {
    let mock = Arc::new(StatefulMockDriveApi::new());
    let dir = TempDir::new().unwrap();
    let file_path = dir.path().join("baixado.txt");

    tokio::fs::write(&file_path, b"local modified content")
        .await
        .unwrap();

    let local_hash = sha256_hex(b"local modified content");
    let remote_hash = "hash_diferente_do_local";
    assert_ne!(local_hash, remote_hash);

    mock.add_file("baixado.txt", "baixado.txt", b"remote content".to_vec(), Some(remote_hash));

    let mut engine = create_engine_with_mock(mock.clone());
    let job = SyncJob::new(file_path.to_str().unwrap(), JobType::Download)
        .with_remote_file_id("baixado.txt");
    {
        let mut queue = engine.job_queue.lock().unwrap();
        queue.enqueue(job);
    }

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].job_type, JobType::Download);
}

#[tokio::test]
async fn test_delete_with_restore() {
    let mock = Arc::new(StatefulMockDriveApi::new());

    mock.add_file("remoto.txt", "remoto.txt", b"remote content".to_vec(), Some("abc123"));

    let mut engine = create_engine_with_mock(mock.clone());
    let job = SyncJob::new("/local/remoto.txt", JobType::Delete)
        .with_remote_file_id("remoto.txt");
    {
        let mut queue = engine.job_queue.lock().unwrap();
        queue.enqueue(job);
    }

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].job_type, JobType::Delete);
}

#[tokio::test]
async fn test_download_job_writes_file_to_disk() {
    let mock = Arc::new(StatefulMockDriveApi::new());
    let dir = TempDir::new().unwrap();
    let sync_dir = dir.path().to_string_lossy().to_string();
    let remote_content = b"remote file content for download";

    mock.add_file("arquivo_remoto.txt", "arquivo_remoto.txt", remote_content.to_vec(), Some("abc123"));

    let mut engine = SyncEngine::new(mock.clone(), SyncConfig::default(), &sync_dir, None);
    let job = SyncJob::new("arquivo_remoto.txt", JobType::Download)
        .with_remote_file_id("arquivo_remoto.txt");
    {
        let mut queue = engine.job_queue.lock().unwrap();
        queue.enqueue(job);
    }

    engine.process_queue().await.unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 1);
    assert_eq!(completed[0].job_type, JobType::Download);

    let local_path = format!("{}/arquivo_remoto.txt", sync_dir);
    let local_content = tokio::fs::read(&local_path).await.unwrap();
    assert_eq!(local_content, remote_content, "downloaded file must be written to local sync dir");
}
