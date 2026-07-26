mod common;

use common::create_test_engine;
use libresync_core::sync::job::{JobState, JobType};

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

#[test]
fn test_detect_changes_enqueues_upload() {
    let mut engine = create_test_engine();
    engine.detect_changes().unwrap();

    let uploads = engine.get_jobs_by_state(JobState::Queued);
    assert!(!uploads.is_empty());
    assert!(uploads.iter().any(|j| j.job_type == JobType::Upload));
}

#[test]
fn test_on_file_changed_enqueues_upload() {
    let mut engine = create_test_engine();
    engine.on_file_changed("/home/test/file.txt").unwrap();

    let jobs = engine.get_jobs_by_state(JobState::Queued);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_type, JobType::Upload);
    assert_eq!(jobs[0].file_path, "/home/test/file.txt");
}

#[test]
fn test_on_remote_change_enqueues_download() {
    let mut engine = create_test_engine();
    engine.on_remote_change("remote-file-id-123").unwrap();

    let jobs = engine.get_jobs_by_state(JobState::Queued);
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].job_type, JobType::Download);
}

#[test]
fn test_process_queue_completes_jobs() {
    let mut engine = create_test_engine();
    engine.on_file_changed("/sync/test.txt").unwrap();
    engine.on_remote_change("remote-id").unwrap();
    assert_eq!(engine.queue_len(), 2);

    engine.process_queue().unwrap();

    let completed = engine.get_jobs_by_state(JobState::Completed);
    assert_eq!(completed.len(), 2);
}

#[test]
fn test_engine_start_transitions_state() {
    let mut engine = create_test_engine();
    assert_eq!(engine.state().to_string(), "Idle");

    engine.start().unwrap();
    assert_eq!(engine.state().to_string(), "Queuing");
}
